//! "Test model": an ordered list of checks against a running server, each timed and
//! reported separately so a partial failure says which part failed.
//!
//! Nothing here assumes a response shape it has not seen. The reasoning field in
//! particular is *detected* rather than assumed, because llama.cpp exposes it under
//! different names depending on build and `--reasoning-format`.

use std::io::{BufRead, BufReader};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::redact::Redacted;

/// Short, deterministic, and cheap: a long prompt would distort the timings it is
/// meant to measure and would eat context the user wanted for real work.
const TEST_PROMPT: &str = "Reply with exactly one word: ready";
/// Reasoning models spend tokens thinking before they answer, so a budget that only
/// covers an answer produces an empty `content` and looks like a broken server.
const MAX_TOKENS: u32 = 96;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

/// Enforced at compile time: a probe that grew large would distort the timings it
/// exists to measure and would consume context the user wanted for real work.
const _: () = assert!(MAX_TOKENS <= 128);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CheckStatus {
    Passed,
    Warning,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Verdict {
    Passed,
    PassedWithWarnings,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Reasoning {
    /// A dedicated field, e.g. `message.reasoning_content`.
    SeparateField,
    /// Wrapped in the content itself, e.g. `<think>…</think>`.
    Inline,
    NotReturned,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Check {
    pub name: String,
    pub status: CheckStatus,
    pub detail: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Timings {
    pub time_to_first_token_ms: Option<u64>,
    pub total_response_ms: Option<u64>,
    pub prompt_tokens: Option<u64>,
    pub generated_tokens: Option<u64>,
    pub prompt_tps: Option<f64>,
    pub gen_tps: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthReport {
    pub verdict: Verdict,
    pub checks: Vec<Check>,
    pub timings: Timings,
    pub reasoning: Reasoning,
}

pub struct Target {
    pub host: String,
    pub port: u16,
    pub alias: String,
    pub pid: Option<u32>,
    pub api_key: Option<Redacted>,
}

impl Target {
    fn base(&self) -> String {
        format!("http://{}:{}", self.host, self.port)
    }
}

pub fn verdict(checks: &[Check]) -> Verdict {
    if checks.iter().any(|c| c.status == CheckStatus::Failed) {
        Verdict::Failed
    } else if checks.iter().any(|c| c.status == CheckStatus::Warning) {
        Verdict::PassedWithWarnings
    } else {
        Verdict::Passed
    }
}

fn check(name: &str, status: CheckStatus, detail: impl Into<String>, started: Instant) -> Check {
    Check {
        name: name.to_string(),
        status,
        detail: detail.into(),
        duration_ms: started.elapsed().as_millis() as u64,
    }
}

fn authorised(request: ureq::Request, key: Option<&Redacted>) -> ureq::Request {
    match key {
        Some(key) if !key.is_empty() => {
            request.set("Authorization", &format!("Bearer {}", key.expose()))
        }
        _ => request,
    }
}

fn get(url: &str, key: Option<&Redacted>) -> Result<String, String> {
    authorised(ureq::get(url).timeout(REQUEST_TIMEOUT), key)
        .call()
        .map_err(|e| describe(&e))?
        .into_string()
        .map_err(|e| e.to_string())
}

/// Error text must never echo the request, which would carry the Authorization header.
fn describe(error: &ureq::Error) -> String {
    match error {
        ureq::Error::Status(code, _) => format!("HTTP {code}"),
        ureq::Error::Transport(transport) => format!("transport error: {}", transport.kind()),
    }
}

fn chat_body(alias: &str, stream: bool) -> Value {
    json!({
        "model": alias,
        "messages": [{ "role": "user", "content": TEST_PROMPT }],
        "max_tokens": MAX_TOKENS,
        "temperature": 0,
        "stream": stream,
    })
}

/// Looks for reasoning wherever this build happens to put it.
fn detect_reasoning(message: &Value) -> Reasoning {
    let separate = ["reasoning_content", "reasoning", "thinking"]
        .iter()
        .any(|field| {
            message
                .get(*field)
                .and_then(Value::as_str)
                .is_some_and(|value| !value.trim().is_empty())
        });
    if separate {
        return Reasoning::SeparateField;
    }

    let content = message.get("content").and_then(Value::as_str).unwrap_or("");
    if content.contains("<think>") || content.contains("<thinking>") {
        return Reasoning::Inline;
    }
    Reasoning::NotReturned
}

struct StreamOutcome {
    first_token_ms: u64,
    total_ms: u64,
    chunks: u64,
}

fn stream_chat(base: &str, alias: &str, key: Option<&Redacted>) -> Result<StreamOutcome, String> {
    let started = Instant::now();
    let response = authorised(
        ureq::post(&format!("{base}/v1/chat/completions")).timeout(REQUEST_TIMEOUT),
        key,
    )
    .set("Content-Type", "application/json")
    .send_string(&chat_body(alias, true).to_string())
    .map_err(|e| describe(&e))?;

    let reader = BufReader::new(response.into_reader());
    let mut first_token_ms = None;
    let mut chunks = 0u64;

    for line in reader.lines().map_while(Result::ok) {
        let Some(payload) = line.strip_prefix("data: ") else {
            continue;
        };
        if payload.trim() == "[DONE]" {
            break;
        }
        let Ok(value) = serde_json::from_str::<Value>(payload) else {
            continue;
        };

        // Reasoning models emit `reasoning_content` deltas before any `content`.
        // Counting only `content` reports a working stream as empty.
        let delta = ["content", "reasoning_content", "reasoning", "thinking"]
            .iter()
            .filter_map(|field| {
                value
                    .pointer(&format!("/choices/0/delta/{field}"))
                    .and_then(Value::as_str)
            })
            .find(|text| !text.is_empty());

        let Some(_) = delta else { continue };

        chunks += 1;
        if first_token_ms.is_none() {
            first_token_ms = Some(started.elapsed().as_millis() as u64);
        }
    }

    if chunks == 0 {
        return Err("stream produced no content".to_string());
    }

    Ok(StreamOutcome {
        first_token_ms: first_token_ms.unwrap_or(0),
        total_ms: started.elapsed().as_millis() as u64,
        chunks,
    })
}

pub fn run(target: &Target) -> HealthReport {
    let base = target.base();
    let key = target.api_key.as_ref();
    let mut checks = Vec::new();
    let mut timings = Timings::default();
    let mut reasoning = Reasoning::NotReturned;

    // 1. Process
    let started = Instant::now();
    match target.pid {
        Some(pid) if crate::sysmem::process_exists(pid) => checks.push(check(
            "Process running",
            CheckStatus::Passed,
            format!("pid {pid}"),
            started,
        )),
        Some(pid) => checks.push(check(
            "Process running",
            CheckStatus::Failed,
            format!("pid {pid} is gone"),
            started,
        )),
        None => checks.push(check(
            "Process running",
            CheckStatus::Skipped,
            "no pid recorded",
            started,
        )),
    }

    // 2. TCP
    let started = Instant::now();
    let address = format!("{}:{}", target.host, target.port)
        .to_socket_addrs()
        .ok()
        .and_then(|mut addrs| addrs.next());

    match address.map(|addr| TcpStream::connect_timeout(&addr, CONNECT_TIMEOUT)) {
        Some(Ok(_)) => checks.push(check(
            "Port reachable",
            CheckStatus::Passed,
            format!("{}:{}", target.host, target.port),
            started,
        )),
        Some(Err(e)) => {
            checks.push(check(
                "Port reachable",
                CheckStatus::Failed,
                e.to_string(),
                started,
            ));
            return finish(checks, timings, reasoning);
        }
        None => {
            checks.push(check(
                "Port reachable",
                CheckStatus::Failed,
                "could not resolve host",
                started,
            ));
            return finish(checks, timings, reasoning);
        }
    }

    // 3. Health endpoint
    let started = Instant::now();
    match get(&format!("{base}/health"), key) {
        Ok(_) => checks.push(check("Health endpoint", CheckStatus::Passed, "ok", started)),
        Err(e) => {
            checks.push(check("Health endpoint", CheckStatus::Failed, e, started));
            return finish(checks, timings, reasoning);
        }
    }

    // 4 and 5. Model list, and whether our alias is in it
    let started = Instant::now();
    match get(&format!("{base}/v1/models"), key) {
        Ok(body) => {
            let ids: Vec<String> = serde_json::from_str::<Value>(&body)
                .ok()
                .and_then(|v| v.get("data").cloned())
                .and_then(|d| d.as_array().cloned())
                .unwrap_or_default()
                .iter()
                .filter_map(|entry| entry.get("id").and_then(Value::as_str).map(String::from))
                .collect();

            checks.push(check(
                "Model list",
                CheckStatus::Passed,
                format!("{} model(s) advertised", ids.len()),
                started,
            ));

            let started = Instant::now();
            if ids.iter().any(|id| id == &target.alias) {
                checks.push(check(
                    "Alias available",
                    CheckStatus::Passed,
                    target.alias.clone(),
                    started,
                ));
            } else {
                // Not fatal: the request still works, the server just advertises a
                // different id than the alias we passed.
                checks.push(check(
                    "Alias available",
                    CheckStatus::Warning,
                    format!("server advertises {ids:?}, not {}", target.alias),
                    started,
                ));
            }
        }
        Err(e) => checks.push(check("Model list", CheckStatus::Warning, e, started)),
    }

    // 6. Chat completion
    let started = Instant::now();
    let completion = authorised(
        ureq::post(&format!("{base}/v1/chat/completions")).timeout(REQUEST_TIMEOUT),
        key,
    )
    .set("Content-Type", "application/json")
    .send_string(&chat_body(&target.alias, false).to_string());

    match completion {
        Ok(response) => {
            let elapsed = started.elapsed().as_millis() as u64;
            let body: Value = response
                .into_string()
                .ok()
                .and_then(|text| serde_json::from_str(&text).ok())
                .unwrap_or(Value::Null);

            let message = body
                .pointer("/choices/0/message")
                .cloned()
                .unwrap_or(Value::Null);
            let content = message.get("content").and_then(Value::as_str).unwrap_or("");

            timings.total_response_ms = Some(elapsed);
            timings.prompt_tokens = body.pointer("/usage/prompt_tokens").and_then(Value::as_u64);
            timings.generated_tokens = body
                .pointer("/usage/completion_tokens")
                .and_then(Value::as_u64);

            // llama.cpp reports its own timings when it can; preferred over anything we
            // could infer from wall clock.
            timings.prompt_tps = body
                .pointer("/timings/prompt_per_second")
                .and_then(Value::as_f64);
            timings.gen_tps = body
                .pointer("/timings/predicted_per_second")
                .and_then(Value::as_f64);

            reasoning = detect_reasoning(&message);

            if content.trim().is_empty() && reasoning != Reasoning::NotReturned {
                // The server answered; it simply spent the token budget reasoning.
                checks.push(check(
                    "Chat completion",
                    CheckStatus::Warning,
                    "only reasoning returned — the token budget was spent thinking",
                    started,
                ));
            } else if content.trim().is_empty() {
                checks.push(check(
                    "Chat completion",
                    CheckStatus::Failed,
                    "response contained no content",
                    started,
                ));
            } else {
                checks.push(check(
                    "Chat completion",
                    CheckStatus::Passed,
                    format!("{elapsed} ms, {} chars", content.trim().len()),
                    started,
                ));
            }
        }
        Err(e) => checks.push(check(
            "Chat completion",
            CheckStatus::Failed,
            describe(&e),
            started,
        )),
    }

    // 7. Streaming
    let started = Instant::now();
    match stream_chat(&base, &target.alias, key) {
        Ok(outcome) => {
            timings.time_to_first_token_ms = Some(outcome.first_token_ms);
            if timings.total_response_ms.is_none() {
                timings.total_response_ms = Some(outcome.total_ms);
            }
            if timings.gen_tps.is_none() {
                let generating_ms = outcome.total_ms.saturating_sub(outcome.first_token_ms);
                if generating_ms > 0 {
                    timings.gen_tps = Some(outcome.chunks as f64 / (generating_ms as f64 / 1000.0));
                }
            }
            checks.push(check(
                "Streaming",
                CheckStatus::Passed,
                format!("first token in {} ms", outcome.first_token_ms),
                started,
            ));
        }
        Err(e) => checks.push(check("Streaming", CheckStatus::Warning, e, started)),
    }

    // 8. Reasoning — informational only; its absence is a configuration choice.
    let started = Instant::now();
    let detail = match reasoning {
        Reasoning::SeparateField => "returned in a separate field",
        Reasoning::Inline => "returned inline in the content",
        Reasoning::NotReturned => "not returned by this server",
    };
    checks.push(check(
        "Reasoning output",
        CheckStatus::Skipped,
        detail,
        started,
    ));

    finish(checks, timings, reasoning)
}

fn finish(checks: Vec<Check>, timings: Timings, reasoning: Reasoning) -> HealthReport {
    HealthReport {
        verdict: verdict(&checks),
        checks,
        timings,
        reasoning,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn made(status: CheckStatus) -> Check {
        Check {
            name: "x".into(),
            status,
            detail: String::new(),
            duration_ms: 0,
        }
    }

    #[test]
    fn all_passing_is_a_pass() {
        assert_eq!(
            verdict(&[made(CheckStatus::Passed), made(CheckStatus::Passed)]),
            Verdict::Passed
        );
    }

    #[test]
    fn a_skipped_check_does_not_downgrade_the_verdict() {
        assert_eq!(
            verdict(&[made(CheckStatus::Passed), made(CheckStatus::Skipped)]),
            Verdict::Passed
        );
    }

    #[test]
    fn a_warning_downgrades_but_does_not_fail() {
        assert_eq!(
            verdict(&[made(CheckStatus::Passed), made(CheckStatus::Warning)]),
            Verdict::PassedWithWarnings
        );
    }

    #[test]
    fn any_failure_fails_the_run() {
        assert_eq!(
            verdict(&[
                made(CheckStatus::Passed),
                made(CheckStatus::Warning),
                made(CheckStatus::Failed),
            ]),
            Verdict::Failed
        );
    }

    #[test]
    fn reasoning_is_found_in_a_dedicated_field() {
        let message = json!({ "content": "ready", "reasoning_content": "thinking about it" });
        assert_eq!(detect_reasoning(&message), Reasoning::SeparateField);
    }

    #[test]
    fn reasoning_is_found_under_alternative_field_names() {
        for field in ["reasoning", "thinking"] {
            let message = json!({ "content": "ready", field: "considered" });
            assert_eq!(
                detect_reasoning(&message),
                Reasoning::SeparateField,
                "{field}"
            );
        }
    }

    #[test]
    fn reasoning_is_found_inline() {
        let message = json!({ "content": "<think>hmm</think> ready" });
        assert_eq!(detect_reasoning(&message), Reasoning::Inline);
    }

    #[test]
    fn an_empty_reasoning_field_does_not_count_as_present() {
        let message = json!({ "content": "ready", "reasoning_content": "   " });
        assert_eq!(detect_reasoning(&message), Reasoning::NotReturned);
    }

    #[test]
    fn the_test_prompt_stays_small() {
        assert!(
            TEST_PROMPT.len() < 80,
            "the probe must not consume meaningful context"
        );
    }

    #[test]
    fn the_request_body_is_deterministic() {
        let body = chat_body("alias", false);
        assert_eq!(body["temperature"], json!(0));
        assert_eq!(body["max_tokens"], json!(MAX_TOKENS));
        assert_eq!(body["stream"], json!(false));
    }
}
