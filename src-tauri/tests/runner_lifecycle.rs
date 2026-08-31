//! Drives the runner against a stand-in server rather than a real model, so the state
//! machine, health gate, telemetry parsing and crash classification are all exercised
//! without loading tens of gigabytes of weights.
//!
//! `python3 -m http.server` is the stand-in: files named `health`, `props` and `metrics`
//! are served at exactly the paths the runner polls.

use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use llamaport_lib::profile::Profile;
use llamaport_lib::runner::{log_path, EventSink, LaunchSpec, RunState, Runner};
use llamaport_lib::speeds::{Source, SpeedKey, SpeedRecord};
use llamaport_lib::store;

mod common;

#[derive(Default)]
struct Recorder {
    events: Mutex<Vec<(String, serde_json::Value)>>,
}

impl EventSink for Recorder {
    fn emit(&self, event: &str, payload: serde_json::Value) {
        self.events
            .lock()
            .expect("recorder lock")
            .push((event.to_string(), payload));
    }
}

impl Recorder {
    fn of_type(&self, kind: &str) -> Vec<serde_json::Value> {
        self.events
            .lock()
            .expect("recorder lock")
            .iter()
            .filter(|(name, _)| name == kind)
            .map(|(_, payload)| payload.clone())
            .collect()
    }
}

const METRICS: &str = "\
# HELP llamacpp:kv_cache_usage_ratio KV-cache usage
# TYPE llamacpp:kv_cache_usage_ratio gauge
llamacpp:kv_cache_usage_ratio 0.25
llamacpp:tokens_predicted_total 100
llamacpp:tokens_predicted_seconds_total 2
llamacpp:prompt_tokens_total 500
llamacpp:prompt_seconds_total 1
llamacpp:requests_processing 1
llamacpp:requests_deferred 0
";

fn have_python() -> bool {
    Command::new("python3")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn fixture_dir(name: &str) -> PathBuf {
    fixture_dir_reporting(name, METRICS)
}

fn fixture_dir_reporting(name: &str, metrics: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("llama-hub-test-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("fixture dir");
    std::fs::write(dir.join("health"), r#"{"status":"ok"}"#).expect("health");
    std::fs::write(
        dir.join("props"),
        r#"{"default_generation_settings":{"n_ctx":4096}}"#,
    )
    .expect("props");
    std::fs::write(dir.join("metrics"), metrics).expect("metrics");
    dir
}

/// A server that came up and was never asked for anything.
const IDLE_METRICS: &str = "\
llamacpp:kv_cache_usage_ratio 0
llamacpp:tokens_predicted_total 0
llamacpp:tokens_predicted_seconds_total 0
llamacpp:prompt_tokens_total 0
llamacpp:prompt_seconds_total 0
llamacpp:requests_processing 0
llamacpp:requests_deferred 0
";

fn rows_for(model_id: &str) -> Vec<SpeedRecord> {
    store::load_speeds(&store::speeds_path())
        .into_iter()
        .filter(|row| row.key.model_id == model_id)
        .collect()
}

/// Serves the stand-in until the returned runner is stopped, and returns once it is Ready
/// and has reported at least once — without a telemetry tick there are no totals to keep.
fn ready_and_reporting(runner: &Runner, recorder: &Arc<Recorder>, model_id: &str, dir: &Path) {
    let served = dir.to_string_lossy().into_owned();
    start_on_free_port(runner, |port| {
        spec_for(
            model_id,
            "python3",
            vec![
                "-m".into(),
                "http.server".into(),
                port.to_string(),
                "--directory".into(),
                served.clone(),
            ],
            port,
        )
    });
    assert!(
        wait_for(|| runner.snapshot().state == RunState::Ready, 20),
        "never became ready: {:?}",
        runner.snapshot()
    );
    assert!(
        wait_for(|| !recorder.of_type("runner:telemetry").is_empty(), 5),
        "no telemetry emitted"
    );
}

/// A port kept bound until the moment it is handed to the runner.
///
/// Choosing one by binding port 0 and closing immediately leaves a window in which a
/// sibling test — these run in parallel — is handed the same port, and the launch is then
/// refused as occupied, which is `inspect_port` working correctly on a port the test
/// believed was free.
struct Port {
    listener: Option<TcpListener>,
    number: u16,
}

impl Port {
    fn reserve() -> Self {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind");
        let number = listener.local_addr().expect("addr").port();
        Self {
            listener: Some(listener),
            number,
        }
    }

    /// Frees the port and returns it, to be called immediately before the launch.
    fn release(&mut self) -> u16 {
        self.listener = None;
        self.number
    }
}

/// Serialises reserve-release-launch, so the gap between releasing a port and the runner
/// claiming it cannot be filled by another test.
static HANDOVER: Mutex<()> = Mutex::new(());

fn handover() -> std::sync::MutexGuard<'static, ()> {
    HANDOVER
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Launches on a port this test proved free, and returns it.
///
/// `HANDOVER` only reaches threads in this binary. `cargo test` runs each integration
/// target as its own process, and siblings such as `download_engine` bind ephemeral
/// ports throughout, so the port can still be lost between `release` and the launch.
/// Only the refusal naming this exact port is retried, so a genuine conflict still fails.
fn start_on_free_port(runner: &Runner, spec_for: impl Fn(u16) -> LaunchSpec) -> u16 {
    const ATTEMPTS: usize = 8;
    let mut refusal = String::new();

    for _ in 0..ATTEMPTS {
        let guard = handover();
        let mut reserved = Port::reserve();
        let port = reserved.release();
        let outcome = runner.start(spec_for(port));
        drop(guard);

        match outcome {
            Ok(()) => return port,
            Err(error) if error.starts_with(&format!("port {port} is already")) => refusal = error,
            Err(error) => panic!("start: {error}"),
        }
    }

    panic!("no reserved port survived the handover in {ATTEMPTS} attempts: {refusal}");
}

fn spec(binary: &str, args: Vec<String>, port: u16) -> LaunchSpec {
    spec_for("test-model", binary, args, port)
}

/// A model id of its own, for the tests that read `speeds.json` back: every test in this
/// binary shares one config directory, so a row is found by whose model it names.
fn spec_for(model_id: &str, binary: &str, args: Vec<String>, port: u16) -> LaunchSpec {
    let profile = Profile {
        ctx: 4096,
        cache_type_k: "q8_0".into(),
        cache_type_v: "q8_0".into(),
        ..Profile::default()
    };
    LaunchSpec {
        model_id: model_id.into(),
        model_name: "Test Model".into(),
        binary: binary.into(),
        args,
        alias: "test-alias".into(),
        host: "127.0.0.1".into(),
        port,
        ctx: 4096,
        cache_type_k: "q8_0".into(),
        cache_type_v: "q8_0".into(),
        predicted_base: 1_000,
        speed_key: SpeedKey::of(model_id, &profile),
        llama_version: Some("b10360".into()),
    }
}

fn wait_for(mut check: impl FnMut() -> bool, seconds: u64) -> bool {
    let deadline = Instant::now() + Duration::from_secs(seconds);
    while Instant::now() < deadline {
        if check() {
            return true;
        }
        thread::sleep(Duration::from_millis(100));
    }
    false
}

/// Every runner in this binary is built here, which is what keeps the isolation from
/// depending on each test remembering it.
fn runner_with(recorder: Arc<Recorder>) -> Runner {
    common::isolate_config_dir();
    Runner::new(recorder)
}

#[test]
fn reaches_ready_reports_telemetry_and_stops() {
    if !have_python() {
        eprintln!("skipping: python3 not available");
        return;
    }

    let dir = fixture_dir("lifecycle");
    let recorder = Arc::new(Recorder::default());
    let runner = runner_with(recorder.clone());

    let port = start_on_free_port(&runner, |port| {
        spec(
            "python3",
            vec![
                "-m".into(),
                "http.server".into(),
                port.to_string(),
                "--directory".into(),
                dir.to_string_lossy().into_owned(),
            ],
            port,
        )
    });

    assert!(
        wait_for(|| runner.snapshot().state == RunState::Ready, 20),
        "never became ready: {:?}",
        runner.snapshot()
    );

    let snapshot = runner.snapshot();
    assert_eq!(snapshot.port, Some(port));
    assert_eq!(snapshot.model_id.as_deref(), Some("test-model"));
    assert_eq!(snapshot.alias.as_deref(), Some("test-alias"));
    assert!(snapshot.pid.is_some());
    assert_eq!(
        snapshot.server_ctx,
        Some(4096),
        "should have read n_ctx from /props"
    );

    assert!(
        wait_for(|| !recorder.of_type("runner:telemetry").is_empty(), 5),
        "no telemetry emitted"
    );

    let telemetry = recorder.of_type("runner:telemetry");
    let latest = telemetry.last().expect("telemetry payload");
    assert_eq!(latest["kvCacheUsage"].as_f64(), Some(0.25));
    assert_eq!(latest["requestsProcessing"].as_f64(), Some(1.0));
    assert!(latest["systemUsedBytes"].as_u64().unwrap_or(0) > 0);
    assert!(latest["systemTotalBytes"].as_u64().unwrap_or(0) > 0);

    let states: Vec<String> = recorder
        .of_type("runner:state")
        .iter()
        .filter_map(|s| s["state"].as_str().map(str::to_string))
        .collect();
    assert_eq!(states.first().map(String::as_str), Some("starting"));
    assert!(states.contains(&"ready".to_string()));

    runner.stop().expect("stop");
    assert_eq!(runner.snapshot().state, RunState::Idle);

    let announced: Vec<String> = recorder
        .of_type("runner:state")
        .iter()
        .filter_map(|s| s["state"].as_str().map(str::to_string))
        .collect();
    assert_eq!(
        announced.last().map(String::as_str),
        Some("idle"),
        "stopping must be announced and not merely returned: the tray only listens"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// The other half of announcing a stop: `start` stops first, so a stop that changes
/// nothing must stay silent or every launch would be preceded by a spurious Idle.
/// The defect this guards: the runner mirrors its log and writes its pidfile through
/// `store::config_dir`, so a suite that drives the real runner writes both into the
/// directory the installed app is using.
#[test]
fn the_runner_writes_where_the_override_points() {
    let isolated = common::isolate_config_dir();
    let recorder = Arc::new(Recorder::default());
    let runner = runner_with(recorder);

    assert!(
        log_path().starts_with(&isolated),
        "the run log is outside the isolated directory: {}",
        log_path().display()
    );

    start_on_free_port(&runner, |port| {
        spec(
            "/bin/sh",
            vec!["-c".into(), "echo 'wrote a line' >&2; exit 3".into()],
            port,
        )
    });
    assert!(
        wait_for(|| runner.snapshot().state == RunState::Crashed, 10),
        "never reached crashed: {:?}",
        runner.snapshot()
    );

    assert!(
        isolated.join("last-run.log").exists(),
        "nothing was written under {}",
        isolated.display()
    );
}

/// The measurement every run already performs, which until now died with the process.
#[test]
fn a_run_that_generated_something_is_recorded_when_it_stops() {
    if !have_python() {
        eprintln!("skipping: python3 not available");
        return;
    }

    let dir = fixture_dir("recorded");
    let recorder = Arc::new(Recorder::default());
    let runner = runner_with(recorder.clone());
    ready_and_reporting(&runner, &recorder, "recorded-model", &dir);

    assert!(
        rows_for("recorded-model").is_empty(),
        "nothing should be written until the run settles"
    );

    runner.stop().expect("stop");

    let rows = rows_for("recorded-model");
    assert_eq!(rows.len(), 1, "one row per run");
    let row = &rows[0];
    assert_eq!(row.gen_tokens, 100.0);
    assert_eq!(row.gen_seconds, 2.0);
    assert_eq!(row.prompt_tokens, 500.0);
    assert_eq!(row.prompt_seconds, 1.0);
    assert_eq!(row.source, Source::Observed);
    assert_eq!(row.llama_version.as_deref(), Some("b10360"));
    assert_eq!(row.key.ctx, 4096);
    assert_eq!(row.gen_tps(), Some(50.0));

    let _ = std::fs::remove_dir_all(&dir);
}

/// A server that was launched and never used measured nothing, and a row saying zero
/// would be ranked against rows that did work.
#[test]
fn a_run_that_generated_nothing_is_not_recorded() {
    if !have_python() {
        eprintln!("skipping: python3 not available");
        return;
    }

    let dir = fixture_dir_reporting("idle", IDLE_METRICS);
    let recorder = Arc::new(Recorder::default());
    let runner = runner_with(recorder.clone());
    ready_and_reporting(&runner, &recorder, "idle-model", &dir);

    runner.stop().expect("stop");

    assert!(
        rows_for("idle-model").is_empty(),
        "a run that generated nothing has nothing to report"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// The other settle path: the server exits on its own, and the runner restarts it once.
/// The restarted process starts its counters from zero, so the first process's reading
/// has to be written before the restart rather than after it.
#[test]
fn a_run_is_recorded_before_it_is_restarted() {
    if !have_python() {
        eprintln!("skipping: python3 not available");
        return;
    }

    let dir = fixture_dir("restarted");
    let recorder = Arc::new(Recorder::default());
    let runner = runner_with(recorder.clone());
    ready_and_reporting(&runner, &recorder, "restarted-model", &dir);

    let pid = runner.snapshot().pid.expect("pid");
    Command::new("kill")
        .args(["-9", &pid.to_string()])
        .status()
        .expect("kill");

    assert!(
        wait_for(|| !rows_for("restarted-model").is_empty(), 10),
        "the exit path wrote nothing"
    );
    assert_eq!(rows_for("restarted-model")[0].gen_tokens, 100.0);

    runner.stop().expect("stop");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A launch that never served cannot have measured anything.
#[test]
fn a_run_that_never_became_ready_is_not_recorded() {
    let recorder = Arc::new(Recorder::default());
    let runner = runner_with(recorder);

    start_on_free_port(&runner, |port| {
        spec_for(
            "never-ready-model",
            "/bin/sh",
            vec!["-c".into(), "echo 'no' >&2; exit 3".into()],
            port,
        )
    });
    assert!(
        wait_for(|| runner.snapshot().state == RunState::Crashed, 10),
        "never reached crashed: {:?}",
        runner.snapshot()
    );

    assert!(rows_for("never-ready-model").is_empty());
}

#[test]
fn stopping_when_nothing_runs_announces_nothing() {
    let recorder = Arc::new(Recorder::default());
    let runner = runner_with(recorder.clone());

    runner.stop().expect("stop");

    assert!(
        recorder.of_type("runner:state").is_empty(),
        "idle to idle is not a transition"
    );
}

#[test]
fn crash_before_ready_is_not_restarted() {
    let recorder = Arc::new(Recorder::default());
    let runner = runner_with(recorder.clone());

    start_on_free_port(&runner, |port| {
        spec(
            "/bin/sh",
            vec![
                "-c".into(),
                "echo 'error: failed to load model' >&2; exit 3".into(),
            ],
            port,
        )
    });

    assert!(
        wait_for(|| runner.snapshot().state == RunState::Crashed, 10),
        "never reached crashed: {:?}",
        runner.snapshot()
    );

    let snapshot = runner.snapshot();
    assert!(
        snapshot
            .error
            .as_deref()
            .is_some_and(|e| e.contains("before becoming ready")),
        "unexpected error: {:?}",
        snapshot.error
    );
    assert!(
        !snapshot.restarted,
        "a launch that never became ready must not be retried"
    );
    assert!(
        snapshot
            .crash_tail
            .iter()
            .any(|l| l.contains("failed to load model")),
        "stderr should be surfaced in the crash tail: {:?}",
        snapshot.crash_tail
    );
}

/// Falling forward to a free port produced a second server on a port no client was
/// configured for, and twice left two copies of the same model resident.
#[test]
fn an_occupied_port_refuses_to_launch_rather_than_moving() {
    let guard = handover();
    let hold = TcpListener::bind(("127.0.0.1", 0)).expect("hold port");
    let taken = hold.local_addr().expect("addr").port();

    let recorder = Arc::new(Recorder::default());
    let runner = runner_with(recorder);

    let outcome = runner.start(spec("/bin/sh", vec!["-c".into(), "sleep 5".into()], taken));
    drop(guard);

    let error = outcome.expect_err("a busy port must not silently move");
    assert!(error.contains(&taken.to_string()), "{error}");
    assert_eq!(
        runner.snapshot().state,
        RunState::Idle,
        "nothing should have been started"
    );
}

#[test]
fn a_free_port_still_launches() {
    let recorder = Arc::new(Recorder::default());
    let runner = runner_with(recorder);

    let port = start_on_free_port(&runner, |port| {
        spec("/bin/sh", vec!["-c".into(), "sleep 5".into()], port)
    });

    let snapshot = runner.snapshot();
    assert_eq!(snapshot.port, Some(port));
    runner.stop().expect("stop");
}
