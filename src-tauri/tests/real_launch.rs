//! End-to-end launch against the real llama-server and a real model. Ignored by default
//! because it loads a model into memory; run it deliberately:
//!
//!     cargo test --test real_launch -- --ignored --nocapture

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use llama_cpp_hub_lib::catalog;
use llama_cpp_hub_lib::probe;
use llama_cpp_hub_lib::profile::{self, Profile};
use llama_cpp_hub_lib::runner::{EventSink, LaunchSpec, RunState, Runner};

struct Printer {
    telemetry: Mutex<Vec<serde_json::Value>>,
}

impl EventSink for Printer {
    fn emit(&self, event: &str, payload: serde_json::Value) {
        match event {
            "runner:telemetry" => self.telemetry.lock().unwrap().push(payload),
            "runner:state" => println!("[state] {}", payload["state"]),
            _ => {}
        }
    }
}

#[test]
#[ignore]
fn launches_the_real_server_with_the_generated_command() {
    let binary = probe::discover(None).expect("llama-server not found");
    let caps = probe::probe(&binary).expect("probe failed");
    println!("binary  {}", caps.binary);
    println!("version {}", caps.version.clone().unwrap_or_default());

    let dir = std::path::PathBuf::from(std::env::var("HOME").unwrap()).join("models");
    let models = catalog::scan(&dir);
    let model = models
        .iter()
        .filter(|m| m.error.is_none())
        .min_by_key(|m| m.size_bytes)
        .expect("no usable model");

    println!("model   {} ({} bytes)", model.file_name, model.size_bytes);

    let launch = Profile {
        alias: profile::default_alias(&model.display_name),
        ctx: 8192,
        port: 8899,
        ..Profile::default()
    };

    let args = launch.args(&model.path, &caps);
    println!("command {}", profile::render_command(&caps.binary, &args));

    let printer = Arc::new(Printer {
        telemetry: Mutex::new(Vec::new()),
    });
    let runner = Runner::new(printer.clone());

    runner
        .start(LaunchSpec {
            model_id: model.id.clone(),
            model_name: model.display_name.clone(),
            binary: caps.binary.clone(),
            args,
            alias: launch.alias.clone(),
            host: launch.host.clone(),
            port: launch.port,
            ctx: launch.ctx,
            cache_type_k: launch.cache_type_k.clone(),
            cache_type_v: launch.cache_type_v.clone(),
            predicted_base: model.size_bytes,
        })
        .expect("start");

    let deadline = Instant::now() + Duration::from_secs(300);
    while Instant::now() < deadline {
        let snapshot = runner.snapshot();
        if snapshot.state == RunState::Ready {
            break;
        }
        assert_ne!(
            snapshot.state,
            RunState::Crashed,
            "crashed: {:?}\n{}",
            snapshot.error,
            snapshot.crash_tail.join("\n")
        );
        thread::sleep(Duration::from_millis(500));
    }

    let snapshot = runner.snapshot();
    assert_eq!(snapshot.state, RunState::Ready, "never became ready");
    println!(
        "ready on port {:?}, server context {:?}",
        snapshot.port, snapshot.server_ctx
    );

    thread::sleep(Duration::from_secs(3));
    let telemetry = printer.telemetry.lock().unwrap().clone();
    let latest = telemetry.last().expect("no telemetry");
    println!("telemetry {latest}");
    assert!(
        latest["kvCacheUsage"].as_f64().is_some(),
        "metrics endpoint produced no KV usage — is --metrics being passed?"
    );
    assert!(latest["systemUsedBytes"].as_u64().unwrap_or(0) > 0);
    assert!(
        latest["tokensGenerated"].as_f64().is_some(),
        "totals should come from the metrics endpoint"
    );

    runner.stop().expect("stop");
    assert_eq!(runner.snapshot().state, RunState::Idle);
}
