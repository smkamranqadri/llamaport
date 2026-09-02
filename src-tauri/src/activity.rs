//! What this app is costing the machine right now.
//!
//! One row per `llama-server` the app knows about — the model it is running, the server a
//! measurement launches on its own port, and any stray one the orphan scan found — plus
//! the machine-wide figures underneath them.
//!
//! **There is no GPU column and there will not be one.** Per-process GPU has no public
//! macOS API, and neither does overall utilisation; the artboard draws both and both are
//! dropped rather than invented. What the GPU *can* answer is how much memory it will
//! hand out, which is the figure a launch has to fit inside anyway.

use std::sync::Mutex;

use serde::Serialize;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};

use crate::sysmem::{self, Pressure};

/// What the caller already knows about a process, before the machine is asked anything.
pub struct Known {
    pub pid: u32,
    pub name: String,
    pub kind: &'static str,
    pub port: Option<u16>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessRow {
    pub pid: u32,
    pub name: String,
    /// `model`, `measurement` or `stray` — what the screen draws the row as.
    pub kind: String,
    pub port: Option<u16>,
    /// Activity Monitor's Memory column. It undercounts a fully offloaded model, whose
    /// weights sit in Metal buffers charged to the kernel, so the GPU card below the
    /// table carries what the launch actually asks of the GPU.
    pub memory_bytes: Option<u64>,
    /// Percent of one core, the way Activity Monitor counts it: a process using two cores
    /// flat out reads 200.
    pub cpu_percent: Option<f32>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    pub processes: Vec<ProcessRow>,
    /// Across the whole machine, 0 to 100 whatever the core count.
    pub total_cpu_percent: Option<f32>,
    pub memory_used_bytes: Option<u64>,
    pub memory_total_bytes: Option<u64>,
    pub swap_used_bytes: Option<u64>,
    pub pressure: Pressure,
    /// What a launch must fit inside, off the same `--list-devices` call the launch plan
    /// reads. `None` where llama-server has not been found.
    pub device_budget_bytes: Option<u64>,
    /// What the running model's launch asks of that budget, `None` when nothing runs.
    pub gpu_wanted_bytes: Option<u64>,
}

/// Which processes the table lists, in the order it lists them: the model being run, the
/// server a measurement launched, then every stray the scan found.
///
/// The measurement's own server is excluded from the strays. The orphan scan is told to
/// skip the runner's child and knows nothing about Tune's, so without this the app
/// reports its own measurement as somebody else's leftover — which is the same defect
/// the stray banner had, one layer up.
pub fn known_processes(
    snapshot: &crate::runner::RunnerSnapshot,
    tune: &crate::tune::Report,
    tune_pid: Option<u32>,
    orphans: &[crate::runner::Orphan],
) -> Vec<Known> {
    let mut known = Vec::new();

    let live = matches!(
        snapshot.state,
        crate::runner::RunState::Starting | crate::runner::RunState::Ready
    );
    if let (Some(pid), true) = (snapshot.pid, live) {
        known.push(Known {
            pid,
            name: snapshot
                .model_name
                .clone()
                .unwrap_or_else(|| "the running model".to_string()),
            kind: "model",
            port: snapshot.port,
        });
    }

    if let Some(pid) = tune_pid {
        let name = match tune.model_name.clone() {
            Some(model) => format!("measuring {model}"),
            None => "measuring best speed".to_string(),
        };
        known.push(Known {
            pid,
            name,
            kind: "measurement",
            port: Some(crate::tune::PORT),
        });
    }

    for orphan in orphans {
        if Some(orphan.pid) == tune_pid {
            continue;
        }
        known.push(Known {
            pid: orphan.pid,
            name: orphan
                .model
                .clone()
                .unwrap_or_else(|| "llama-server".to_string()),
            kind: "stray",
            port: orphan.port,
        });
    }

    known
}

/// A `System` that lives between polls.
///
/// A CPU percentage is a difference between two samples of the same counter. A fresh
/// `System` every call has nothing to subtract from, and reports either zero or the
/// average since boot — which is not what anybody watching a model is asking about.
pub struct Monitor {
    system: Mutex<System>,
}

impl Default for Monitor {
    fn default() -> Self {
        Self::new()
    }
}

impl Monitor {
    pub fn new() -> Self {
        Self {
            system: Mutex::new(System::new()),
        }
    }

    /// Rows for the processes named, in the order given, and what the machine is doing
    /// under them. A process that has gone since the caller looked it up is dropped
    /// rather than reported with empty figures.
    pub fn poll(&self, known: &[Known]) -> (Vec<ProcessRow>, Option<f32>) {
        let mut system = self.system.lock().expect("activity lock");
        system.refresh_cpu_usage();
        system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing().with_cpu(),
        );

        let rows = known
            .iter()
            .filter_map(|entry| {
                let process = system.process(sysinfo::Pid::from_u32(entry.pid))?;
                Some(ProcessRow {
                    pid: entry.pid,
                    name: entry.name.clone(),
                    kind: entry.kind.to_string(),
                    port: entry.port,
                    memory_bytes: sysmem::process_footprint_bytes(entry.pid),
                    cpu_percent: Some(process.cpu_usage()),
                })
            })
            .collect();

        (rows, Some(system.global_cpu_usage()))
    }

    /// The machine-wide figures. Memory comes from the same places the running model's
    /// own screen reads, so the two cannot disagree about what the Mac is using.
    pub fn machine(&self) -> Activity {
        let mut system = self.system.lock().expect("activity lock");
        system.refresh_memory();

        Activity {
            memory_used_bytes: Some(system.used_memory()),
            memory_total_bytes: sysmem::installed_bytes().or_else(|| Some(system.total_memory())),
            swap_used_bytes: sysmem::swap_used_bytes().or_else(|| Some(system.used_swap())),
            pressure: sysmem::pressure(),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::{Orphan, RunState, RunnerSnapshot};
    use crate::tune::Report;

    fn idle() -> RunnerSnapshot {
        RunnerSnapshot {
            state: RunState::Idle,
            model_id: None,
            model_name: None,
            alias: None,
            port: None,
            pid: None,
            started_secs: None,
            error: None,
            crash_tail: Vec::new(),
            restarted: false,
            server_ctx: None,
        }
    }

    /// The scan is told to skip the runner's child and knows nothing about Tune's, so
    /// without the exclusion the app reports its own measurement as somebody's leftover.
    #[test]
    fn the_measurements_own_server_is_never_a_stray() {
        let orphans = vec![
            Orphan {
                pid: 42,
                port: Some(crate::tune::PORT),
                model: Some("phi-4".into()),
            },
            Orphan {
                pid: 99,
                port: Some(8080),
                model: None,
            },
        ];

        let rows = known_processes(&idle(), &Report::default(), Some(42), &orphans);

        let strays: Vec<u32> = rows
            .iter()
            .filter(|row| row.kind == "stray")
            .map(|row| row.pid)
            .collect();
        assert_eq!(
            strays,
            vec![99],
            "the measurement's own pid was called a stray"
        );

        // It appears once, as what it is.
        let mine: Vec<&str> = rows
            .iter()
            .filter(|row| row.pid == 42)
            .map(|row| row.kind)
            .collect();
        assert_eq!(mine, vec!["measurement"]);
        assert_eq!(
            rows[1].name, "llama-server",
            "a stray with no model in its command"
        );
    }

    #[test]
    fn the_running_model_leads_the_table_and_the_measurement_follows() {
        let snapshot = RunnerSnapshot {
            state: RunState::Ready,
            model_name: Some("qwen2.5-7b-instruct".into()),
            pid: Some(7),
            port: Some(8080),
            ..idle()
        };
        let report = Report {
            model_name: Some("phi-4".into()),
            ..Default::default()
        };

        let rows = known_processes(&snapshot, &report, Some(8), &[]);

        let seen: Vec<(&str, &str)> = rows
            .iter()
            .map(|row| (row.kind, row.name.as_str()))
            .collect();
        assert_eq!(
            seen,
            vec![
                ("model", "qwen2.5-7b-instruct"),
                ("measurement", "measuring phi-4"),
            ]
        );
        assert_eq!(rows[1].port, Some(crate::tune::PORT));
    }

    /// A crashed process keeps its pid in the snapshot. Listing it would put a dead row
    /// in a table whose whole subject is what is running now.
    #[test]
    fn a_model_that_is_not_running_has_no_row() {
        let snapshot = RunnerSnapshot {
            state: RunState::Crashed,
            pid: Some(7),
            model_name: Some("qwen2.5-7b-instruct".into()),
            ..idle()
        };

        assert!(known_processes(&snapshot, &Report::default(), None, &[]).is_empty());
    }
}
