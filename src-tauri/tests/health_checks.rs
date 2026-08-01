//! Drives the health checks against a stand-in OpenAI-compatible server.
//!
//! A hand-rolled listener rather than a real llama-server: the point is to control the
//! responses — including the failure shapes — which a real model cannot be asked to
//! produce on demand.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

use llama_cpp_hub_lib::health::{self, CheckStatus, Reasoning, Target, Verdict};

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Healthy,
    UnknownAlias,
    ChatRejected,
}

const MODELS: &str = r#"{"object":"list","data":[{"id":"test-alias","object":"model"}]}"#;
const OTHER_MODELS: &str = r#"{"object":"list","data":[{"id":"something-else"}]}"#;
const CHAT: &str = r#"{"choices":[{"message":{"role":"assistant","content":"ready","reasoning_content":"considering the question"}}],"usage":{"prompt_tokens":12,"completion_tokens":3},"timings":{"prompt_per_second":301.4,"predicted_per_second":15.5}}"#;

struct Fake {
    port: u16,
}

fn respond(stream: &mut TcpStream, status: &str, body: &str) {
    let _ = write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
}

fn respond_stream(stream: &mut TcpStream) {
    let _ = write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n"
    );
    let _ = stream.flush();

    // A visible gap before the first token, so time-to-first-token is measurable rather
    // than rounding to zero.
    thread::sleep(Duration::from_millis(60));
    for piece in ["re", "ady"] {
        let _ = write!(
            stream,
            "data: {{\"choices\":[{{\"delta\":{{\"content\":\"{piece}\"}}}}]}}\n\n"
        );
        let _ = stream.flush();
        thread::sleep(Duration::from_millis(20));
    }
    let _ = write!(stream, "data: [DONE]\n\n");
    let _ = stream.flush();
}

fn start_fake(mode: Mode) -> Fake {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind");
    let port = listener.local_addr().expect("addr").port();

    thread::spawn(move || {
        for connection in listener.incoming() {
            let Ok(mut stream) = connection else { continue };
            let mut reader = BufReader::new(stream.try_clone().expect("clone"));

            let mut request_line = String::new();
            if reader.read_line(&mut request_line).is_err() {
                continue;
            }

            let mut content_length = 0usize;
            loop {
                let mut header = String::new();
                if reader.read_line(&mut header).unwrap_or(0) == 0 || header.trim().is_empty() {
                    break;
                }
                let lower = header.to_ascii_lowercase();
                if let Some(value) = lower.strip_prefix("content-length:") {
                    content_length = value.trim().parse().unwrap_or(0);
                }
            }

            let mut body = vec![0u8; content_length];
            if content_length > 0 {
                let _ = reader.read_exact(&mut body);
            }
            let body = String::from_utf8_lossy(&body).to_string();

            let path = request_line.split_whitespace().nth(1).unwrap_or("");
            match path {
                "/health" => respond(&mut stream, "200 OK", r#"{"status":"ok"}"#),
                "/v1/models" => {
                    let models = if mode == Mode::UnknownAlias {
                        OTHER_MODELS
                    } else {
                        MODELS
                    };
                    respond(&mut stream, "200 OK", models)
                }
                "/v1/chat/completions" => {
                    if mode == Mode::ChatRejected {
                        respond(
                            &mut stream,
                            "500 Internal Server Error",
                            r#"{"error":"no"}"#,
                        );
                    } else if body.contains("\"stream\":true") {
                        respond_stream(&mut stream);
                    } else {
                        respond(&mut stream, "200 OK", CHAT);
                    }
                }
                _ => respond(&mut stream, "404 Not Found", "{}"),
            }
        }
    });

    Fake { port }
}

fn target(port: u16) -> Target {
    Target {
        host: "127.0.0.1".into(),
        port,
        alias: "test-alias".into(),
        pid: Some(std::process::id()),
    }
}

fn find<'a>(report: &'a health::HealthReport, name: &str) -> &'a health::Check {
    report
        .checks
        .iter()
        .find(|check| check.name == name)
        .unwrap_or_else(|| panic!("missing check {name}: {:?}", report.checks))
}

#[test]
fn a_healthy_server_passes_every_check() {
    let fake = start_fake(Mode::Healthy);
    let report = health::run(&target(fake.port));

    assert_eq!(report.verdict, Verdict::Passed, "{:#?}", report.checks);
    assert_eq!(find(&report, "Process running").status, CheckStatus::Passed);
    assert_eq!(find(&report, "Port reachable").status, CheckStatus::Passed);
    assert_eq!(find(&report, "Health endpoint").status, CheckStatus::Passed);
    assert_eq!(find(&report, "Model list").status, CheckStatus::Passed);
    assert_eq!(find(&report, "Alias available").status, CheckStatus::Passed);
    assert_eq!(find(&report, "Chat completion").status, CheckStatus::Passed);
    assert_eq!(find(&report, "Streaming").status, CheckStatus::Passed);
}

#[test]
fn every_check_reports_its_own_duration() {
    let fake = start_fake(Mode::Healthy);
    let report = health::run(&target(fake.port));

    assert!(report.checks.len() >= 7);
    assert!(
        find(&report, "Streaming").duration_ms >= 60,
        "streaming took at least the fixture's first-token delay"
    );
}

#[test]
fn timings_prefer_the_servers_own_figures() {
    let fake = start_fake(Mode::Healthy);
    let report = health::run(&target(fake.port));

    assert_eq!(report.timings.prompt_tokens, Some(12));
    assert_eq!(report.timings.generated_tokens, Some(3));
    assert_eq!(report.timings.prompt_tps, Some(301.4));
    assert_eq!(report.timings.gen_tps, Some(15.5));

    let ttft = report
        .timings
        .time_to_first_token_ms
        .expect("ttft measured");
    assert!(
        ttft >= 60,
        "first token should not appear before it was sent"
    );
    assert!(report.timings.total_response_ms.is_some());
}

#[test]
fn reasoning_is_detected_without_being_assumed() {
    let fake = start_fake(Mode::Healthy);
    let report = health::run(&target(fake.port));
    assert_eq!(report.reasoning, Reasoning::SeparateField);
}

#[test]
fn an_unadvertised_alias_warns_rather_than_fails() {
    let fake = start_fake(Mode::UnknownAlias);
    let report = health::run(&target(fake.port));

    assert_eq!(report.verdict, Verdict::PassedWithWarnings);
    assert_eq!(
        find(&report, "Alias available").status,
        CheckStatus::Warning
    );
    assert_eq!(find(&report, "Chat completion").status, CheckStatus::Passed);
}

#[test]
fn a_rejected_chat_request_fails_the_run() {
    let fake = start_fake(Mode::ChatRejected);
    let report = health::run(&target(fake.port));

    assert_eq!(report.verdict, Verdict::Failed);
    assert_eq!(find(&report, "Chat completion").status, CheckStatus::Failed);
    assert!(find(&report, "Chat completion").detail.contains("500"));
}

#[test]
fn an_unreachable_port_fails_fast_without_pretending_to_test_more() {
    let closed = TcpListener::bind(("127.0.0.1", 0)).expect("bind");
    let port = closed.local_addr().expect("addr").port();
    drop(closed);

    let report = health::run(&target(port));

    assert_eq!(report.verdict, Verdict::Failed);
    assert!(
        report.checks.iter().all(|c| c.name != "Chat completion"),
        "later checks must not be reported as passed when the port is dead"
    );
}

#[test]
fn a_dead_process_is_reported_but_does_not_prevent_the_http_checks() {
    let fake = start_fake(Mode::Healthy);
    let mut spec = target(fake.port);
    spec.pid = Some(0x7FFF_FFFF);

    let report = health::run(&spec);

    assert_eq!(find(&report, "Process running").status, CheckStatus::Failed);
    assert_eq!(find(&report, "Health endpoint").status, CheckStatus::Passed);
    assert_eq!(report.verdict, Verdict::Failed);
}
