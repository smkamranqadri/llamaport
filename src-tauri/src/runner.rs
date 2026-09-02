use std::collections::{HashMap, VecDeque};
use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

use crate::speeds::{SpeedKey, SpeedRecord};
use crate::store;
use crate::sysmem::{self, Pressure};

/// The runner reports through this rather than talking to Tauri directly, so its
/// lifecycle can be driven by tests without a running app.
pub trait EventSink: Send + Sync + 'static {
    fn emit(&self, event: &str, payload: serde_json::Value);
}

pub type Events = Arc<dyn EventSink>;

const LOG_CAPACITY: usize = 2000;
const CRASH_TAIL: usize = 20;
const HEALTH_INTERVAL: Duration = Duration::from_millis(500);
const TELEMETRY_INTERVAL: Duration = Duration::from_secs(1);
const START_TIMEOUT: Duration = Duration::from_secs(900);

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
    /// Activity Monitor's Memory column for the child. Undercounts GPU-resident weights.
    pub process_footprint_bytes: Option<u64>,
    pub pressure: Pressure,
    /// Whether the server answered *just now*, as distinct from the process being alive.
    pub health_ok: bool,
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
    /// What this run will be filed under if it produces a reading. Built by the caller
    /// from the profile, so the runner records what it is told to rather than knowing
    /// which settings can move a number.
    pub speed_key: SpeedKey,
    pub llama_version: Option<String>,
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
    started_secs: Option<u64>,
    port: Option<u16>,
    server_ctx: Option<u64>,
    /// The last totals this run reported. Only the telemetry loop writes them and it only
    /// runs after Ready, so their presence is what says a run got far enough to be worth
    /// recording.
    counters: Option<Counters>,
}

impl Inner {
    fn snapshot(&self) -> RunnerSnapshot {
        RunnerSnapshot {
            state: self.state,
            model_id: self.spec.as_ref().map(|s| s.model_id.clone()),
            model_name: self.spec.as_ref().map(|s| s.model_name.clone()),
            alias: self.spec.as_ref().map(|s| s.alias.clone()),
            port: self.port,
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
}

pub struct Runner {
    inner: Arc<Mutex<Inner>>,
    events: Events,
}

impl Runner {
    pub fn new(events: Events) -> Self {
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
                started_secs: None,
                port: None,
                server_ctx: None,
                counters: None,
            })),
        }
    }

    pub fn snapshot(&self) -> RunnerSnapshot {
        self.inner.lock().expect("runner lock").snapshot()
    }

    pub fn logs(&self) -> Vec<String> {
        let inner = self.inner.lock().expect("runner lock");
        inner.logs.iter().cloned().collect()
    }

    pub fn start(&self, spec: LaunchSpec) -> Result<(), String> {
        self.stop()?;
        spawn_run(&self.inner, &self.events, spec, false)
    }

    pub fn stop(&self) -> Result<(), String> {
        let (mut child, before) = {
            let mut inner = self.inner.lock().expect("runner lock");
            let before = inner.state;
            if inner.child.is_none() {
                inner.state = RunState::Idle;
            } else {
                inner.stopping = true;
                inner.state = RunState::Stopping;
            }
            (inner.child.take(), before)
        };

        if let Some(child) = child.as_mut() {
            let _ = child.kill();
            let _ = child.wait();

            let mut inner = self.inner.lock().expect("runner lock");
            inner.state = RunState::Idle;
            inner.stopping = false;
            inner.port = None;
            inner.started_secs = None;
            inner.server_ctx = None;
            drop(inner);
            clear_pidfile();
            record_speed(&self.inner);
        }

        // On a real transition, on every path out. The tray learns the state only from this
        // stream, so a silent Ready -> Idle left it advertising a model that had stopped.
        // Guarded on the transition because `start` stops first, and an idle-to-idle stop
        // would put a spurious Idle in front of every launch.
        if self.inner.lock().expect("runner lock").state != before {
            emit_state(&self.events, &self.inner);
        }
        Ok(())
    }
}

/// One row per run that reached Ready and generated something.
///
/// Taking the counters is what makes it once per run: both settle paths call this, and a
/// stop that follows an exit finds nothing left to write. A run that never became Ready
/// never had counters at all.
fn record_speed(inner: &Arc<Mutex<Inner>>) {
    let (counters, spec) = {
        let mut guard = inner.lock().expect("runner lock");
        (guard.counters.take(), guard.spec.clone())
    };
    let (Some(counters), Some(spec)) = (counters, spec) else {
        return;
    };
    if counters.gen_tokens <= 0.0 {
        return;
    }

    let record = SpeedRecord {
        timestamp_secs: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        key: spec.speed_key,
        llama_version: spec.llama_version,
        source: crate::speeds::Source::Observed,
        prompt_tokens: counters.prompt_tokens,
        prompt_seconds: counters.prompt_seconds,
        gen_tokens: counters.gen_tokens,
        gen_seconds: counters.gen_seconds,
    };
    // `parse_metrics` accepts anything f64 parses, which includes NaN and inf.
    if !record.is_sound() {
        return;
    }
    let _ = store::append_speed(&store::speeds_path(), record);
}

fn emit_state(events: &Events, inner: &Arc<Mutex<Inner>>) {
    let snapshot = inner.lock().expect("runner lock").snapshot();
    if let Ok(payload) = serde_json::to_value(snapshot) {
        events.emit("runner:state", payload);
    }
}

/// Who, if anyone, already holds a port. Distinguishes "another llama-server" from
/// "something else entirely", because the remedy differs: the first is usually an
/// orphan of this app, the second is not ours to touch.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortConflict {
    pub port: u16,
    pub responds_to_health: bool,
    pub is_llama_server: bool,
}

pub fn inspect_port(host: &str, port: u16) -> Option<PortConflict> {
    if TcpListener::bind((host, port)).is_ok() {
        return None;
    }

    let responds_to_health = http_get(&format!("http://{host}:{port}/health")).is_ok();
    // /props is llama.cpp specific, so a positive answer identifies the occupant.
    let is_llama_server = http_get(&format!("http://{host}:{port}/props"))
        .map(|body| body.contains("default_generation_settings") || body.contains("model_path"))
        .unwrap_or(false);

    Some(PortConflict {
        port,
        responds_to_health,
        is_llama_server,
    })
}

fn describe_conflict(conflict: &PortConflict) -> String {
    if conflict.is_llama_server {
        format!(
            "port {} is already serving another llama-server — stop it first, or choose \
             a different port",
            conflict.port
        )
    } else {
        format!(
            "port {} is already in use by another process",
            conflict.port
        )
    }
}

fn spawn_run(
    inner: &Arc<Mutex<Inner>>,
    events: &Events,
    spec: LaunchSpec,
    is_restart: bool,
) -> Result<(), String> {
    // Falling forward to the next free port was the original design and it was wrong:
    // this app runs one model at a time, and a silent substitution produces a second
    // server on a port no client is configured for. Twice this has left two copies of
    // the same 15 GB model resident at once.
    if let Some(conflict) = inspect_port(&spec.host, spec.port) {
        return Err(describe_conflict(&conflict));
    }
    let port = spec.port;

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
        guard.port = Some(port);
        guard.server_ctx = None;
        guard.restarted = is_restart;
        guard.started_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .map(|d| d.as_secs());
        if !is_restart {
            guard.logs.clear();
            reset_log_file();
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
    spawn_waiter(inner.clone(), events.clone(), generation, spec);

    Ok(())
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
            append_log_line(&line);
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

#[derive(Clone)]
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
            let guard = inner.lock().expect("runner lock");
            guard.child.as_ref().map(|c| c.id())
        };

        telemetry.process_footprint_bytes = pid.and_then(sysmem::process_footprint_bytes);

        telemetry.health_ok = http_get(&format!("{base}/health")).is_ok();

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
            {
                let mut guard = inner.lock().expect("runner lock");
                if guard.generation != generation {
                    return;
                }
                guard.counters = Some(current.clone());
            }
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

fn spawn_waiter(inner: Arc<Mutex<Inner>>, events: Events, generation: u64, spec: LaunchSpec) {
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

        let (was_ready, already_restarted) = {
            let mut guard = inner.lock().expect("runner lock");
            guard.child = None;
            (guard.state == RunState::Ready, guard.restarted)
        };

        clear_pidfile();
        record_speed(&inner);

        // A crash before Ready is a configuration problem; restarting reproduces it.
        if was_ready && !already_restarted {
            let mut guard = inner.lock().expect("runner lock");
            guard.push_log("[hub] server exited unexpectedly, restarting once".into());
            drop(guard);
            if spawn_run(&inner, &events, spec.clone(), true).is_ok() {
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

pub fn log_path() -> std::path::PathBuf {
    crate::store::config_dir().join("last-run.log")
}

/// Logs are mirrored to disk as they arrive so the last run survives a crash of either
/// the server or the app itself — the in-memory ring buffer dies with the process, and
/// the lines that explain a crash are exactly the ones worth keeping.
fn append_log_line(line: &str) {
    use std::io::Write;
    let path = log_path();
    let _ = std::fs::create_dir_all(crate::store::config_dir());
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        let _ = writeln!(file, "{line}");
    }
}

fn reset_log_file() {
    let _ = std::fs::write(log_path(), "");
}

/// The previous run's output, for showing after an unexpected exit.
pub fn previous_logs() -> Vec<String> {
    std::fs::read_to_string(log_path())
        .map(|text| {
            let lines: Vec<String> = text.lines().map(String::from).collect();
            let start = lines.len().saturating_sub(LOG_CAPACITY);
            lines[start..].to_vec()
        })
        .unwrap_or_default()
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
    pub port: Option<u16>,
    pub model: Option<String>,
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

/// Pulls the port and model out of a llama-server command line.
///
/// The alias wins over the file name where there is one: it is the id a client addresses
/// and the name the person who started the server chose, where `-m` gives a path whose
/// last segment is long enough to wrap the banner it is printed in.
pub fn parse_server_command(args: &[String]) -> (Option<u16>, Option<String>) {
    let value_after = |flag: &str| -> Option<String> {
        args.iter()
            .position(|arg| arg == flag)
            .and_then(|index| args.get(index + 1))
            .cloned()
    };

    let port = value_after("--port").and_then(|value| value.parse().ok());
    let from_path = value_after("-m")
        .or_else(|| value_after("--model"))
        .map(|path| {
            let name = path.rsplit('/').next().unwrap_or(&path).to_string();
            name.strip_suffix(".gguf").unwrap_or(&name).to_string()
        });
    let model = value_after("--alias").or(from_path);
    (port, model)
}

/// Finds every running llama-server, not just one this app remembers starting.
///
/// The pidfile only ever knew about the last process written to it, so a hard kill of
/// the app — which `tauri dev` does on every rebuild — left servers running that nothing
/// could see. Two 15 GB copies accumulated that way in a single evening.
pub fn detect_orphans(exclude_pid: Option<u32>) -> Vec<Orphan> {
    let mut system = System::new();
    // Specifics, not the plain refresh: that one leaves `cmd()` empty, so every orphan the
    // app has ever reported named an unknown model on an unknown port.
    system.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::everything(),
    );

    system
        .processes()
        .values()
        .filter(|process| process.name().to_string_lossy().contains("llama-server"))
        .filter(|process| Some(process.pid().as_u32()) != exclude_pid)
        .map(|process| {
            let args: Vec<String> = process
                .cmd()
                .iter()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect();
            let (port, model) = parse_server_command(&args);
            Orphan {
                pid: process.pid().as_u32(),
                port,
                model,
            }
        })
        .collect()
}

/// Stops a server the user explicitly chose to stop, re-verifying it first.
pub fn stop_orphan(pid: u32) -> Result<(), String> {
    if !is_live_server(pid) {
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
    fn a_command_line_yields_its_port_and_model() {
        let args: Vec<String> = [
            "-m",
            "/Users/me/models/Qwen3.6-35B-A3B-UD-Q3_K_XL.gguf",
            "--host",
            "127.0.0.1",
            "--port",
            "8889",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();

        let (port, model) = parse_server_command(&args);
        assert_eq!(port, Some(8889));
        assert_eq!(model.as_deref(), Some("Qwen3.6-35B-A3B-UD-Q3_K_XL"));
    }

    #[test]
    fn an_alias_is_preferred_to_the_file_name() {
        let args: Vec<String> = [
            "-m",
            "/Users/me/models/Qwen_Qwen3.5-2B-Q4_K_M.gguf",
            "--alias",
            "qwen3.5-2b",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();

        assert_eq!(parse_server_command(&args).1.as_deref(), Some("qwen3.5-2b"));
    }

    #[test]
    fn a_command_line_without_the_flags_yields_nothing_rather_than_guessing() {
        let args = vec!["llama-server".to_string()];
        assert_eq!(parse_server_command(&args), (None, None));
    }

    #[test]
    fn a_busy_port_is_described_by_who_holds_it() {
        let llama = PortConflict {
            port: 8888,
            responds_to_health: true,
            is_llama_server: true,
        };
        assert!(describe_conflict(&llama).contains("another llama-server"));

        let other = PortConflict {
            port: 8888,
            responds_to_health: false,
            is_llama_server: false,
        };
        assert!(describe_conflict(&other).contains("another process"));
    }
}
