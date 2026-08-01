//! Drives the runner against a stand-in server rather than a real model, so the state
//! machine, health gate, telemetry parsing and crash classification are all exercised
//! without loading tens of gigabytes of weights.
//!
//! `python3 -m http.server` is the stand-in: files named `health`, `props` and `metrics`
//! are served at exactly the paths the runner polls.

use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use llama_cpp_hub_lib::runner::{EventSink, LaunchSpec, RunState, Runner};

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
    let dir = std::env::temp_dir().join(format!("llama-hub-test-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("fixture dir");
    std::fs::write(dir.join("health"), r#"{"status":"ok"}"#).expect("health");
    std::fs::write(
        dir.join("props"),
        r#"{"default_generation_settings":{"n_ctx":4096}}"#,
    )
    .expect("props");
    std::fs::write(dir.join("metrics"), METRICS).expect("metrics");
    dir
}

fn free_port() -> u16 {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind");
    listener.local_addr().expect("addr").port()
}

fn spec(binary: &str, args: Vec<String>, port: u16) -> LaunchSpec {
    LaunchSpec {
        model_id: "test-model".into(),
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

fn runner_with(recorder: Arc<Recorder>) -> (Runner, Arc<Mutex<Vec<u64>>>) {
    let samples = Arc::new(Mutex::new(Vec::new()));
    let captured = samples.clone();
    let runner = Runner::new(
        recorder,
        Arc::new(move |sample| {
            captured
                .lock()
                .expect("samples lock")
                .push(sample.observed_total);
        }),
    );
    (runner, samples)
}

#[test]
fn reaches_ready_reports_telemetry_and_stops() {
    if !have_python() {
        eprintln!("skipping: python3 not available");
        return;
    }

    let dir = fixture_dir("lifecycle");
    let port = free_port();
    let recorder = Arc::new(Recorder::default());
    let (runner, _samples) = runner_with(recorder.clone());

    runner
        .start(spec(
            "python3",
            vec![
                "-m".into(),
                "http.server".into(),
                port.to_string(),
                "--directory".into(),
                dir.to_string_lossy().into_owned(),
            ],
            port,
        ))
        .expect("start");

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

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn crash_before_ready_is_not_restarted() {
    let recorder = Arc::new(Recorder::default());
    let (runner, _samples) = runner_with(recorder.clone());

    runner
        .start(spec(
            "/bin/sh",
            vec![
                "-c".into(),
                "echo 'error: failed to load model' >&2; exit 3".into(),
            ],
            free_port(),
        ))
        .expect("start");

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
        snapshot.crash_tail.iter().any(|l| l.contains("failed to load model")),
        "stderr should be surfaced in the crash tail: {:?}",
        snapshot.crash_tail
    );
}

#[test]
fn occupied_port_falls_forward_and_reports_it() {
    let hold = TcpListener::bind(("127.0.0.1", 0)).expect("hold port");
    let taken = hold.local_addr().expect("addr").port();

    let recorder = Arc::new(Recorder::default());
    let (runner, _samples) = runner_with(recorder);

    runner
        .start(spec("/bin/sh", vec!["-c".into(), "sleep 5".into()], taken))
        .expect("start");

    let snapshot = runner.snapshot();
    assert_eq!(snapshot.requested_port, Some(taken));
    assert_ne!(
        snapshot.port,
        Some(taken),
        "should not claim the occupied port"
    );

    runner.stop().expect("stop");
}
