use std::collections::{HashMap, VecDeque};
use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sysinfo::{Pid, ProcessesToUpdate, System};

use crate::estimate::CalibrationSample;
use crate::safety::{self, Assessment, Inputs};
use crate::sysmem::{self, Pressure};

/// The runner reports through this rather than talking to Tauri directly, so its
/// lifecycle can be driven by tests without a running app.
pub trait EventSink: Send + Sync + 'static {
    fn emit(&self, event: &str, payload: serde_json::Value);
}

pub type Events = Arc<dyn EventSink>;

const LOG_CAPACITY: usize = 2000;
const CRASH_TAIL: usize = 20;
const PORT_SEARCH_RANGE: u16 = 20;
const HEALTH_INTERVAL: Duration = Duration::from_millis(500);
const TELEMETRY_INTERVAL: Duration = Duration::from_secs(1);
const START_TIMEOUT: Duration = Duration::from_secs(900);

pub type SampleSink = Arc<dyn Fn(CalibrationSample) + Send + Sync>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RunState {
    Idle,
    Starting,
    Ready,
    Stopping,
    Crashed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunnerSnapshot {
    pub state: RunState,
    pub model_id: Option<String>,
    pub model_name: Option<String>,
    pub alias: Option<String>,
    pub port: Option<u16>,
    pub requested_port: Option<u16>,
    pub pid: Option<u32>,
    pub started_secs: Option<u64>,
    pub error: Option<String>,
    pub crash_tail: Vec<String>,
    pub restarted: bool,
    pub server_ctx: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Telemetry {
    pub kv_cache_usage: Option<f64>,
    /// Rates measured live between polls; zero while idle.
    pub prompt_tps: Option<f64>,
    pub gen_tps: Option<f64>,
    /// The server's own per-request figures, which persist after a request finishes.
    pub last_prompt_tps: Option<f64>,
    pub last_gen_tps: Option<f64>,
    pub tokens_generated: Option<f64>,
    pub tokens_prompt: Option<f64>,
    pub requests_processing: Option<f64>,
    pub requests_deferred: Option<f64>,
    /// Machine-wide, because on Apple Silicon `-ngl all` puts weights and KV cache in
    /// Metal buffers that land in wired memory rather than the process footprint.
    pub system_used_bytes: Option<u64>,
    pub system_total_bytes: Option<u64>,
    pub swap_used_bytes: Option<u64>,
    pub model_delta_bytes: Option<u64>,
    /// Activity Monitor's Memory column for the child. Undercounts GPU-resident weights.
    pub process_footprint_bytes: Option<u64>,
    pub pressure: Pressure,
    pub safety: Option<Assessment>,
    pub uptime_secs: u64,
}

#[derive(Debug, Clone)]
pub struct LaunchSpec {
    pub model_id: String,
    pub model_name: String,
    pub binary: String,
    pub args: Vec<String>,
    pub alias: String,
    pub host: String,
    pub port: u16,
    pub ctx: u64,
    pub cache_type_k: String,
    pub cache_type_v: String,
    pub predicted_base: u64,
}

struct Inner {
    generation: u64,
    state: RunState,
    child: Option<Child>,
    spec: Option<LaunchSpec>,
    logs: VecDeque<String>,
    error: Option<String>,
    crash_tail: Vec<String>,
    restarted: bool,
    stopping: bool,
    baseline_used: u64,
    peak_used: u64,
    peak_footprint: u64,
    peak_swap: u64,
    started_secs: Option<u64>,
    port: Option<u16>,
    requested_port: Option<u16>,
    server_ctx: Option<u64>,
}

impl Inner {
    fn snapshot(&self) -> RunnerSnapshot {
        RunnerSnapshot {
            state: self.state,
            model_id: self.spec.as_ref().map(|s| s.model_id.clone()),
            model_name: self.spec.as_ref().map(|s| s.model_name.clone()),
            alias: self.spec.as_ref().map(|s| s.alias.clone()),
            port: self.port,
            requested_port: self.requested_port,
            pid: self.child.as_ref().map(|c| c.id()),
            started_secs: self.started_secs,
            error: self.error.clone(),
            crash_tail: self.crash_tail.clone(),
            restarted: self.restarted,
            server_ctx: self.server_ctx,
        }
    }

    fn push_log(&mut self, line: String) {
        if self.logs.len() == LOG_CAPACITY {
            self.logs.pop_front();
        }
        self.logs.push_back(line);
    }

    fn tail(&self, n: usize) -> Vec<String> {
        self.logs.iter().rev().take(n).rev().cloned().collect()
    }

    /// A run that reached `Ready` is worth recording whatever it observed. Zero or
    /// implausible growth is filtered when the residency is fitted, not here — judging
    /// it at capture time is how the previous design ended up discarding every sample.
    fn calibration_sample(&self) -> Option<CalibrationSample> {
        if self.state != RunState::Ready {
            return None;
        }
        let spec = self.spec.as_ref()?;
        Some(CalibrationSample {
            model_id: spec.model_id.clone(),
            ctx: spec.ctx,
            cache_type_k: spec.cache_type_k.clone(),
            cache_type_v: spec.cache_type_v.clone(),
            predicted_base: spec.predicted_base,
            observed_total: self.peak_used.saturating_sub(self.baseline_used),
        })
    }
}

pub struct Runner {
    inner: Arc<Mutex<Inner>>,
    sink: SampleSink,
    events: Events,
}

impl Runner {
    pub fn new(events: Events, sink: SampleSink) -> Self {
        Self {
            events,
            inner: Arc::new(Mutex::new(Inner {
                generation: 0,
                state: RunState::Idle,
                child: None,
                spec: None,
                logs: VecDeque::with_capacity(256),
                error: None,
                crash_tail: Vec::new(),
                restarted: false,
                stopping: false,
                baseline_used: 0,
                peak_used: 0,
                peak_footprint: 0,
                peak_swap: 0,
                started_secs: None,
                port: None,
                requested_port: None,
                server_ctx: None,
            })),
            sink,
        }
    }

    pub fn snapshot(&self) -> RunnerSnapshot {
        self.inner.lock().expect("runner lock").snapshot()
    }

    pub fn logs(&self) -> Vec<String> {
        let inner = self.inner.lock().expect("runner lock");
        inner.logs.iter().cloned().collect()
    }

    pub fn is_busy(&self) -> bool {
        let state = self.inner.lock().expect("runner lock").state;
        matches!(state, RunState::Starting | RunState::Ready)
    }

    /// Peak values seen across the current run, for the benchmark record. `None` when
    /// telemetry has not sampled yet.
    pub fn peaks(&self) -> (Option<u64>, Option<u64>) {
        let guard = self.inner.lock().expect("runner lock");
        (
            Some(guard.peak_footprint).filter(|v| *v > 0),
            Some(guard.peak_swap).filter(|v| *v > 0),
        )
    }

    /// Machine-wide memory growth attributable to the running model, for subtracting
    /// before projecting a replacement launch. `None` when nothing is running.
    pub fn current_model_bytes(&self) -> Option<u64> {
        let guard = self.inner.lock().expect("runner lock");
        if guard.state != RunState::Ready {
            return None;
        }
        Some(guard.peak_used.saturating_sub(guard.baseline_used))
    }

    pub fn start(&self, spec: LaunchSpec) -> Result<(), String> {
        self.stop()?;
        spawn_run(&self.inner, &self.sink, &self.events, spec, false)
    }

    pub fn stop(&self) -> Result<(), String> {
        let (mut child, sample) = {
            let mut inner = self.inner.lock().expect("runner lock");
            if inner.child.is_none() {
                inner.state = RunState::Idle;
                return Ok(());
            }
            // Captured before the state changes: an explicit stop is the normal end of a
            // run, and the waiter thread will not see this exit because the child is
            // taken here.
            let sample = inner.calibration_sample();
            inner.stopping = true;
            inner.state = RunState::Stopping;
            (inner.child.take(), sample)
        };

        if let Some(child) = child.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }

        if let Some(sample) = sample {
            (self.sink)(sample);
        }

        let mut inner = self.inner.lock().expect("runner lock");
        inner.state = RunState::Idle;
        inner.stopping = false;
        inner.port = None;
        inner.started_secs = None;
        inner.server_ctx = None;
        clear_pidfile();
        Ok(())
    }
}

fn emit_state(events: &Events, inner: &Arc<Mutex<Inner>>) {
    let snapshot = inner.lock().expect("runner lock").snapshot();
    if let Ok(payload) = serde_json::to_value(snapshot) {
        events.emit("runner:state", payload);
    }
}

fn find_free_port(host: &str, start: u16) -> Option<u16> {
    (start..start.saturating_add(PORT_SEARCH_RANGE))
        .find(|port| TcpListener::bind((host, *port)).is_ok())
}

fn spawn_run(
    inner: &Arc<Mutex<Inner>>,
    sink: &SampleSink,
    events: &Events,
    mut spec: LaunchSpec,
    is_restart: bool,
) -> Result<(), String> {
    let requested_port = spec.port;
    let port = find_free_port(&spec.host, spec.port)
        .ok_or_else(|| format!("no free port near {}", spec.port))?;

    if port != spec.port {
        spec.port = port;
        replace_port_arg(&mut spec.args, port);
    }

    let baseline_used = {
        let mut system = System::new();
        system.refresh_memory();
        system.used_memory()
    };

    let mut child = Command::new(&spec.binary)
        .args(&spec.args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("could not start {}: {e}", spec.binary))?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let pid = child.id();

    let generation = {
        let mut guard = inner.lock().expect("runner lock");
        guard.generation += 1;
        guard.state = RunState::Starting;
        guard.error = None;
        guard.crash_tail.clear();
        guard.stopping = false;
        guard.baseline_used = baseline_used;
        guard.peak_used = 0;
        guard.peak_footprint = 0;
        guard.peak_swap = 0;
        guard.port = Some(port);
        guard.requested_port = Some(requested_port);
        guard.server_ctx = None;
        guard.restarted = is_restart;
        guard.started_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .map(|d| d.as_secs());
        if !is_restart {
            guard.logs.clear();
        }
        guard.spec = Some(spec.clone());
        guard.child = Some(child);
        guard.generation
    };

    write_pidfile(pid, port, &spec.model_id);
    emit_state(events, inner);

    if let Some(out) = stdout {
        spawn_log_reader(inner.clone(), events.clone(), generation, out);
    }
    if let Some(err) = stderr {
        spawn_log_reader(inner.clone(), events.clone(), generation, err);
    }

    spawn_health_and_telemetry(inner.clone(), events.clone(), generation, spec.clone());
    spawn_waiter(
        inner.clone(),
        sink.clone(),
        events.clone(),
        generation,
        spec,
    );

    Ok(())
}

fn replace_port_arg(args: &mut [String], port: u16) {
    if let Some(index) = args.iter().position(|a| a == "--port") {
        if let Some(value) = args.get_mut(index + 1) {
            *value = port.to_string();
        }
    }
}

fn spawn_log_reader<R: std::io::Read + Send + 'static>(
    inner: Arc<Mutex<Inner>>,
    events: Events,
    generation: u64,
    stream: R,
) {
    thread::spawn(move || {
        let reader = BufReader::new(stream);
        for line in reader.lines().map_while(Result::ok) {
            {
                let mut guard = inner.lock().expect("runner lock");
                if guard.generation != generation {
                    return;
                }
                guard.push_log(line.clone());
            }
            events.emit("runner:log", serde_json::Value::String(line));
        }
    });
}

fn spawn_health_and_telemetry(
    inner: Arc<Mutex<Inner>>,
    events: Events,
    generation: u64,
    spec: LaunchSpec,
) {
    thread::spawn(move || {
        let base = format!("http://{}:{}", spec.host, spec.port);
        let deadline = Instant::now() + START_TIMEOUT;

        loop {
            thread::sleep(HEALTH_INTERVAL);

            {
                let guard = inner.lock().expect("runner lock");
                if guard.generation != generation || guard.state != RunState::Starting {
                    return;
                }
            }

            if Instant::now() > deadline {
                let mut guard = inner.lock().expect("runner lock");
                if guard.generation == generation {
                    let tail = guard.tail(CRASH_TAIL);
                    guard.state = RunState::Crashed;
                    guard.error = Some("timed out waiting for the server to become ready".into());
                    guard.crash_tail = tail;
                }
                drop(guard);
                emit_state(&events, &inner);
                return;
            }

            if http_get(&format!("{base}/health")).is_ok() {
                break;
            }
        }

        let server_ctx = http_get(&format!("{base}/props"))
            .ok()
            .and_then(|body| serde_json::from_str::<serde_json::Value>(&body).ok())
            .and_then(|v| {
                v.get("default_generation_settings")
                    .and_then(|g| g.get("n_ctx"))
                    .and_then(serde_json::Value::as_u64)
                    .or_else(|| v.get("n_ctx").and_then(serde_json::Value::as_u64))
            });

        {
            let mut guard = inner.lock().expect("runner lock");
            if guard.generation != generation {
                return;
            }
            guard.state = RunState::Ready;
            guard.server_ctx = server_ctx;
        }
        emit_state(&events, &inner);

        let ctx = server_ctx.unwrap_or(spec.ctx);
        telemetry_loop(inner, events, generation, base, ctx);
    });
}

struct Counters {
    prompt_tokens: f64,
    prompt_seconds: f64,
    gen_tokens: f64,
    gen_seconds: f64,
}

fn read_counters(metrics: &HashMap<String, f64>) -> Counters {
    Counters {
        prompt_tokens: *metrics.get("llamacpp:prompt_tokens_total").unwrap_or(&0.0),
        prompt_seconds: *metrics.get("llamacpp:prompt_seconds_total").unwrap_or(&0.0),
        gen_tokens: *metrics
            .get("llamacpp:tokens_predicted_total")
            .unwrap_or(&0.0),
        gen_seconds: *metrics
            .get("llamacpp:tokens_predicted_seconds_total")
            .unwrap_or(&0.0),
    }
}

/// Counters are cumulative, so throughput is a delta. A counter that goes backwards means
/// the process restarted and the baseline has to be dropped rather than differenced.
fn rate(
    current_tokens: f64,
    previous_tokens: f64,
    current_secs: f64,
    previous_secs: f64,
) -> Option<f64> {
    if current_tokens < previous_tokens || current_secs < previous_secs {
        return None;
    }
    let tokens = current_tokens - previous_tokens;
    let seconds = current_secs - previous_secs;
    if tokens <= 0.0 || seconds <= 0.0 {
        return Some(0.0);
    }
    Some(tokens / seconds)
}

fn telemetry_loop(
    inner: Arc<Mutex<Inner>>,
    events: Events,
    generation: u64,
    base: String,
    ctx: u64,
) {
    let mut system = System::new();
    let mut previous: Option<Counters> = None;
    let started = Instant::now();

    loop {
        thread::sleep(TELEMETRY_INTERVAL);

        {
            let guard = inner.lock().expect("runner lock");
            if guard.generation != generation || guard.state != RunState::Ready {
                return;
            }
        }

        let mut telemetry = Telemetry {
            uptime_secs: started.elapsed().as_secs(),
            ..Default::default()
        };

        system.refresh_memory();
        let used = system.used_memory();
        let installed = sysmem::installed_bytes().or_else(|| Some(system.total_memory()));
        let swap = sysmem::swap_used_bytes().or_else(|| Some(system.used_swap()));

        telemetry.system_used_bytes = Some(used);
        telemetry.system_total_bytes = installed;
        telemetry.swap_used_bytes = swap;
        telemetry.pressure = sysmem::pressure();

        let pid = {
            let mut guard = inner.lock().expect("runner lock");
            guard.peak_used = guard.peak_used.max(used);
            telemetry.model_delta_bytes = Some(used.saturating_sub(guard.baseline_used));
            guard.child.as_ref().map(|c| c.id())
        };

        telemetry.process_footprint_bytes = pid.and_then(sysmem::process_footprint_bytes);

        {
            let mut guard = inner.lock().expect("runner lock");
            if let Some(footprint) = telemetry.process_footprint_bytes {
                guard.peak_footprint = guard.peak_footprint.max(footprint);
            }
            if let Some(swap) = swap {
                guard.peak_swap = guard.peak_swap.max(swap);
            }
        }
        telemetry.safety = Some(safety::assess(Inputs {
            installed,
            used: Some(used),
            swap_used: swap,
            pressure: telemetry.pressure,
            running_model_bytes: None,
            predicted_total: None,
        }));

        if let Ok(body) = http_get(&format!("{base}/metrics")) {
            let metrics = parse_metrics(&body);
            telemetry.kv_cache_usage = kv_usage(&metrics, ctx);
            telemetry.requests_processing = metrics.get("llamacpp:requests_processing").copied();
            telemetry.requests_deferred = metrics.get("llamacpp:requests_deferred").copied();
            telemetry.last_gen_tps = metrics.get("llamacpp:predicted_tokens_seconds").copied();
            telemetry.last_prompt_tps = metrics.get("llamacpp:prompt_tokens_seconds").copied();
            telemetry.tokens_generated = metrics.get("llamacpp:tokens_predicted_total").copied();
            telemetry.tokens_prompt = metrics.get("llamacpp:prompt_tokens_total").copied();

            let current = read_counters(&metrics);
            if let Some(prev) = &previous {
                telemetry.gen_tps = rate(
                    current.gen_tokens,
                    prev.gen_tokens,
                    current.gen_seconds,
                    prev.gen_seconds,
                );
                telemetry.prompt_tps = rate(
                    current.prompt_tokens,
                    prev.prompt_tokens,
                    current.prompt_seconds,
                    prev.prompt_seconds,
                );
            }
            previous = Some(current);
        }

        if let Ok(payload) = serde_json::to_value(&telemetry) {
            events.emit("runner:telemetry", payload);
        }
    }
}

fn spawn_waiter(
    inner: Arc<Mutex<Inner>>,
    sink: SampleSink,
    events: Events,
    generation: u64,
    spec: LaunchSpec,
) {
    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(250));

        let exit = {
            let mut guard = inner.lock().expect("runner lock");
            if guard.generation != generation {
                return;
            }
            match guard.child.as_mut() {
                Some(child) => match child.try_wait() {
                    Ok(Some(status)) => Some(status.code()),
                    Ok(None) => None,
                    Err(_) => Some(None),
                },
                None => return,
            }
        };

        let Some(code) = exit else { continue };

        let (was_ready, already_restarted, sample) = {
            let mut guard = inner.lock().expect("runner lock");
            let sample = guard.calibration_sample();
            guard.child = None;
            (guard.state == RunState::Ready, guard.restarted, sample)
        };

        clear_pidfile();

        if let Some(sample) = sample {
            sink(sample);
        }

        // A crash before Ready is a configuration problem; restarting reproduces it.
        if was_ready && !already_restarted {
            let mut guard = inner.lock().expect("runner lock");
            guard.push_log("[hub] server exited unexpectedly, restarting once".into());
            drop(guard);
            if spawn_run(&inner, &sink, &events, spec.clone(), true).is_ok() {
                return;
            }
        }

        {
            let mut guard = inner.lock().expect("runner lock");
            let tail = guard.tail(CRASH_TAIL);
            guard.state = RunState::Crashed;
            guard.crash_tail = tail;
            guard.error = Some(match code {
                Some(code) if was_ready => {
                    format!("server exited with code {code} after running")
                }
                Some(code) => format!("server exited with code {code} before becoming ready"),
                None => "server was terminated".to_string(),
            });
            guard.port = None;
        }
        emit_state(&events, &inner);
        return;
    });
}

/// Build 10090 dropped `kv_cache_usage_ratio` in favour of `n_tokens_max`, so the
/// occupancy figure is read where available and derived against the context size
/// otherwise. Older builds keep working unchanged.
fn kv_usage(metrics: &HashMap<String, f64>, ctx: u64) -> Option<f64> {
    if let Some(ratio) = metrics.get("llamacpp:kv_cache_usage_ratio") {
        return Some(*ratio);
    }
    if ctx == 0 {
        return None;
    }
    let tokens = metrics.get("llamacpp:n_tokens_max")?;
    Some(tokens / ctx as f64)
}

fn parse_metrics(body: &str) -> HashMap<String, f64> {
    let mut out = HashMap::new();
    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((name, value)) = line.rsplit_once(char::is_whitespace) else {
            continue;
        };
        if let Ok(parsed) = value.trim().parse::<f64>() {
            out.insert(name.trim().to_string(), parsed);
        }
    }
    out
}

fn http_get(url: &str) -> Result<String, String> {
    let response = ureq::get(url)
        .timeout(Duration::from_secs(3))
        .call()
        .map_err(|e| e.to_string())?;
    response.into_string().map_err(|e| e.to_string())
}

#[derive(serde::Serialize, serde::Deserialize)]
struct PidFile {
    pid: u32,
    port: u16,
    model_id: String,
}

fn pidfile_path() -> std::path::PathBuf {
    crate::store::config_dir().join("runner.pid")
}

fn write_pidfile(pid: u32, port: u16, model_id: &str) {
    let _ = std::fs::create_dir_all(crate::store::config_dir());
    let record = PidFile {
        pid,
        port,
        model_id: model_id.to_string(),
    };
    if let Ok(json) = serde_json::to_string(&record) {
        let _ = std::fs::write(pidfile_path(), json);
    }
}

fn clear_pidfile() {
    let _ = std::fs::remove_file(pidfile_path());
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Orphan {
    pub pid: u32,
    pub port: u16,
    pub model_id: String,
}

/// True when the pid is alive and still looks like our server. A pid alone is weak
/// evidence because pids are recycled, so the name is always rechecked.
fn is_live_server(pid: u32) -> bool {
    let pid = Pid::from_u32(pid);
    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);

    system
        .process(pid)
        .map(|process| process.name().to_string_lossy().contains("llama-server"))
        .unwrap_or(false)
}

/// A live pidfile on startup means the app died without stopping its server.
///
/// This reports and never kills: the app must not terminate a process the user did not
/// ask it to terminate, and a name check cannot distinguish our orphan from a server the
/// user started by hand. A stale pidfile (process gone, or the pid now belongs to
/// something else) is cleared silently.
pub fn detect_orphan() -> Option<Orphan> {
    let raw = std::fs::read_to_string(pidfile_path()).ok()?;
    let record: PidFile = match serde_json::from_str::<PidFile>(&raw) {
        Ok(record) => record,
        Err(_) => {
            clear_pidfile();
            return None;
        }
    };

    if !is_live_server(record.pid) {
        clear_pidfile();
        return None;
    }

    Some(Orphan {
        pid: record.pid,
        port: record.port,
        model_id: record.model_id,
    })
}

/// Stops an orphan the user explicitly chose to stop, re-verifying the process first.
pub fn stop_orphan(pid: u32) -> Result<(), String> {
    if !is_live_server(pid) {
        clear_pidfile();
        return Err("that process is no longer running".to_string());
    }

    let target = Pid::from_u32(pid);
    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::Some(&[target]), true);

    let process = system
        .process(target)
        .ok_or_else(|| "process disappeared".to_string())?;

    if !process.kill() {
        return Err("could not stop the process".to_string());
    }
    clear_pidfile();
    Ok(())
}

/// Leaves the process running and forgets about it.
pub fn dismiss_orphan() {
    clear_pidfile();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_prometheus_lines() {
        let body = "\
# HELP llamacpp:kv_cache_usage_ratio KV-cache usage
# TYPE llamacpp:kv_cache_usage_ratio gauge
llamacpp:kv_cache_usage_ratio 0.34
llamacpp:tokens_predicted_total 1024
llamacpp:requests_deferred 2
garbage line
";
        let metrics = parse_metrics(body);
        assert_eq!(metrics.get("llamacpp:kv_cache_usage_ratio"), Some(&0.34));
        assert_eq!(
            metrics.get("llamacpp:tokens_predicted_total"),
            Some(&1024.0)
        );
        assert_eq!(metrics.get("llamacpp:requests_deferred"), Some(&2.0));
        assert!(!metrics.contains_key("garbage"));
    }

    #[test]
    fn kv_usage_prefers_the_reported_ratio() {
        let metrics = HashMap::from([
            ("llamacpp:kv_cache_usage_ratio".to_string(), 0.5),
            ("llamacpp:n_tokens_max".to_string(), 100.0),
        ]);
        assert_eq!(kv_usage(&metrics, 8192), Some(0.5));
    }

    #[test]
    fn kv_usage_falls_back_to_tokens_over_context() {
        let metrics = HashMap::from([("llamacpp:n_tokens_max".to_string(), 2048.0)]);
        assert_eq!(kv_usage(&metrics, 8192), Some(0.25));
        assert_eq!(kv_usage(&metrics, 0), None);
        assert_eq!(kv_usage(&HashMap::new(), 8192), None);
    }

    #[test]
    fn throughput_is_a_delta_over_generation_time() {
        assert_eq!(rate(1000.0, 900.0, 12.0, 10.0), Some(50.0));
    }

    #[test]
    fn idle_period_reports_zero_not_a_spike() {
        assert_eq!(rate(900.0, 900.0, 10.0, 10.0), Some(0.0));
    }

    #[test]
    fn counter_reset_is_not_differenced() {
        assert_eq!(rate(10.0, 900.0, 1.0, 10.0), None);
    }

    #[test]
    fn port_substitution_rewrites_the_rendered_args() {
        let mut args = vec![
            "--host".to_string(),
            "127.0.0.1".to_string(),
            "--port".to_string(),
            "8888".to_string(),
        ];
        replace_port_arg(&mut args, 8889);
        assert_eq!(args[3], "8889");
    }
}
