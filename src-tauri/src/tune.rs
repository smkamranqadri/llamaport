//! Measuring, because a memory sum cannot say a launch is good.
//!
//! Arithmetic picks the largest context and most precise cache that fits. On the model
//! this app was built around that is the slowest of three candidates — 30.5 tok/s where
//! 65,536 with a full-precision cache gives 41.6. So the ladder below is run for real:
//! every candidate is launched, asked the same question, and timed.
//!
//! `tools/fits.py --run` is the oracle this is checked against, the relationship
//! `estimate.rs` already has with it. `candidates` and `prompt_of` are ports of its
//! `candidates()` and `long_prompt()`, and a disagreement about a real file is a finding.

use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use serde::Serialize;

use crate::estimate;
use crate::gguf::GgufMetadata;
use crate::probe::Capabilities;
use crate::profile::Profile;

/// llama.cpp's own `--fit-target` default: what it keeps free on each device.
const MARGIN_MIB: u64 = 1024;
/// A rough allowance for compute buffers, which the header cannot describe.
const COMPUTE_MIB: u64 = 96;
const MIB: u64 = 1024 * 1024;

/// The rungs worth trying. The model's own maximum is added to these.
const LADDER: [u64; 7] = [4096, 8192, 16384, 32768, 65536, 131072, 262144];
/// Ordered as fits.py orders them, so a disagreement with the oracle is about the
/// arithmetic rather than about which of two equal candidates was picked first.
const CACHES: [&str; 2] = ["q8_0", "f16"];

const HEALTH_TIMEOUT: Duration = Duration::from_secs(300);
const PREDICT: u64 = 64;

/// The port Tune launches on. Its own, so a measurement never disturbs the port a client
/// is configured for, and fixed so a stranded server is findable at a known address.
pub const PORT: u16 = 9977;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub ctx: u64,
    pub cache_k: String,
    pub cache_v: String,
}

impl Candidate {
    fn cache(&self) -> &str {
        &self.cache_k
    }
}

/// What a launch has to fit inside, or `None` where the build did not say — in which
/// case nothing is measured, because every candidate would be a guess.
fn budget_bytes(caps: &Capabilities) -> Option<u64> {
    let device = caps.device_budget_mib()?;
    Some(device.saturating_sub(MARGIN_MIB) * MIB)
}

fn fits(md: &GgufMetadata, file_size: u64, ctx: u64, cache: &str, budget: u64) -> bool {
    let Some(sized) = estimate::estimate(md, file_size, ctx, cache, cache) else {
        return false;
    };
    sized.total_bytes + COMPUTE_MIB * MIB <= budget
}

/// A few that fit, spread across the axes that actually differ: how much context, and
/// how precise the cache. Measuring every rung would take all afternoon.
pub fn candidates(md: &GgufMetadata, file_size: u64, caps: &Capabilities) -> Vec<Candidate> {
    let Some(budget) = budget_bytes(caps) else {
        return Vec::new();
    };
    let max_ctx = md.context_length.unwrap_or(0);

    let mut contexts: Vec<u64> = LADDER.iter().copied().filter(|c| *c <= max_ctx).collect();
    if max_ctx > 0 && !contexts.contains(&max_ctx) {
        contexts.push(max_ctx);
    }

    let mut fitting: Vec<Candidate> = Vec::new();
    for cache in CACHES {
        for ctx in &contexts {
            if fits(md, file_size, *ctx, cache, budget) {
                fitting.push(Candidate {
                    ctx: *ctx,
                    cache_k: cache.to_string(),
                    cache_v: cache.to_string(),
                });
            }
        }
    }

    let Some(widest) = fitting
        .iter()
        .max_by_key(|c| (c.ctx, c.cache() == "f16"))
        .cloned()
    else {
        return Vec::new();
    };

    let mut wanted = vec![widest.clone()];
    if let Some(small) = fitting
        .iter()
        .filter(|c| c.ctx <= 8192)
        .max_by_key(|c| (c.cache() == "f16", c.ctx))
    {
        wanted.push(small.clone());
    }
    if let Some(mid) = fitting
        .iter()
        .filter(|c| (16384..=65536).contains(&c.ctx))
        .max_by_key(|c| (c.cache() == "f16", c.ctx))
    {
        wanted.push(mid.clone());
    }
    wanted.extend(
        fitting
            .iter()
            .filter(|c| c.ctx == widest.ctx && c.cache() != widest.cache())
            .cloned(),
    );

    let mut picked: Vec<Candidate> = Vec::new();
    for candidate in wanted {
        if !picked.contains(&candidate) {
            picked.push(candidate);
        }
    }
    picked
}

/// Varied text rather than one repeated line, because a repeated line is not what a
/// coding agent sends and is not what the cache does with it.
const WORDS: [&str; 27] = [
    "model",
    "context",
    "window",
    "memory",
    "cache",
    "attention",
    "layer",
    "token",
    "embedding",
    "gradient",
    "kernel",
    "buffer",
    "offload",
    "quantise",
    "inference",
    "throughput",
    "latency",
    "batch",
    "server",
    "prompt",
    "decode",
    "residual",
    "weights",
    "tensor",
    "matrix",
    "vector",
    "scalar",
];

/// Roughly `tokens` tokens of prose. English runs near 0.75 words per token.
pub fn prompt_of(tokens: usize) -> String {
    let words = std::cmp::max(32, (tokens as f64 * 0.75) as usize);
    let mut out: Vec<&str> = Vec::with_capacity(words + words / 12);
    for i in 0..words {
        out.push(WORDS[(i * 7 + i / WORDS.len()) % WORDS.len()]);
        if i % 12 == 11 {
            out.push(".\n");
        }
    }
    out.join(" ")
}

/// One prompt for every candidate, sized to the smallest context so each does identical
/// work. Comparing configurations on different prompts compares nothing, and this is the
/// single lesson of a reading that said 2% against a 36-token prompt and 27% against a
/// real one.
pub fn prompt_for(candidates: &[Candidate]) -> String {
    let smallest = candidates.iter().map(|c| c.ctx).min().unwrap_or(4096);
    prompt_of((smallest / 2).clamp(256, 4096) as usize)
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Reading {
    pub prompt_tokens: f64,
    pub prompt_seconds: f64,
    pub gen_tokens: f64,
    pub gen_seconds: f64,
}

/// The server's own account of the request it just answered, which is more exact than
/// anything measured from outside it.
pub fn parse_timings(body: &str) -> Option<Reading> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    let timings = value.get("timings")?;
    let number = |key: &str| timings.get(key).and_then(serde_json::Value::as_f64);
    Some(Reading {
        prompt_tokens: number("prompt_n")?,
        prompt_seconds: number("prompt_ms")? / 1000.0,
        gen_tokens: number("predicted_n")?,
        gen_seconds: number("predicted_ms")? / 1000.0,
    })
}

/// Set from the UI thread and read by the ladder between steps.
#[derive(Clone, Default)]
pub struct Cancel(Arc<AtomicBool>);

impl Cancel {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    pub fn cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// A child that is killed when it goes out of scope, however the ladder leaves.
///
/// Cancelling mid-measurement, a candidate that will not load, and a panic all end the
/// same way: a `llama-server` holding tens of gigabytes must not outlive the function
/// that started it.
struct Server(Child);

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

impl Server {
    fn pid(&self) -> u32 {
        self.0.id()
    }
}

pub fn profile_for(base: &Profile, candidate: &Candidate) -> Profile {
    let mut profile = base.clone();
    profile.ctx = candidate.ctx;
    profile.cache_type_k = candidate.cache_k.clone();
    profile.cache_type_v = candidate.cache_v.clone();
    profile.port = PORT;
    profile
}

/// Launches one candidate, asks it the question, and reports what the server itself said.
pub fn measure(
    binary: &str,
    args: &[String],
    prompt: &str,
    cancel: &Cancel,
    on_pid: impl Fn(Option<u32>),
) -> Result<Reading, String> {
    let child = Command::new(binary)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("could not start {binary}: {e}"))?;
    let server = Server(child);
    on_pid(Some(server.pid()));

    let base = format!("http://127.0.0.1:{PORT}");
    let deadline = std::time::Instant::now() + HEALTH_TIMEOUT;
    loop {
        if cancel.cancelled() {
            on_pid(None);
            return Err("cancelled".into());
        }
        if std::time::Instant::now() > deadline {
            on_pid(None);
            return Err("timed out waiting for the server to become ready".into());
        }
        if ureq::get(&format!("{base}/health"))
            .timeout(Duration::from_secs(2))
            .call()
            .is_ok()
        {
            break;
        }
        thread::sleep(Duration::from_millis(500));
    }

    let body = serde_json::json!({
        "prompt": prompt,
        "n_predict": PREDICT,
        "stream": false,
        "cache_prompt": false
    })
    .to_string();

    let answered = ureq::post(&format!("{base}/completion"))
        .timeout(Duration::from_secs(600))
        .set("Content-Type", "application/json")
        .send_string(&body)
        .map_err(|e| e.to_string())?
        .into_string()
        .map_err(|e| e.to_string())?;
    on_pid(None);

    parse_timings(&answered).ok_or_else(|| "the server reported no timings".to_string())
}

/// One candidate's result. A failure is kept rather than dropped: "this did not load" is
/// as much of an answer as a speed, and the table has to be able to say so.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Outcome {
    pub candidate: Candidate,
    pub reading: Option<Reading>,
    pub error: Option<String>,
}

impl Outcome {
    pub fn gen_tps(&self) -> Option<f64> {
        let reading = self.reading?;
        if reading.gen_seconds <= 0.0 {
            return None;
        }
        Some(reading.gen_tokens / reading.gen_seconds)
    }
}

/// What the ladder has done so far. Announced as a whole on every change, the way the
/// runner announces its state: a screen that has to assemble progress from increments
/// gets it wrong the moment one is missed.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub running: bool,
    pub model_id: Option<String>,
    pub model_name: Option<String>,
    /// Roughly how large the shared prompt is, so the screen can say what was asked.
    pub prompt_words: Option<u64>,
    /// Every rung the ladder will try, named before any of them has run: the screen draws
    /// the ones still waiting, which cannot be assembled from finished rows.
    pub candidates: Vec<Candidate>,
    pub current: Option<Candidate>,
    pub done: usize,
    pub total: usize,
    pub rows: Vec<Outcome>,
    pub error: Option<String>,
    pub cancelled: bool,
}

impl Report {
    /// The fastest generation of everything that ran. What Tune is for.
    pub fn fastest(&self) -> Option<&Outcome> {
        self.rows
            .iter()
            .filter(|row| row.gen_tps().is_some())
            .max_by(|a, b| {
                a.gen_tps()
                    .unwrap_or(0.0)
                    .total_cmp(&b.gen_tps().unwrap_or(0.0))
            })
    }
}

pub struct Request {
    pub model_id: String,
    pub model_name: String,
    pub model_path: String,
    pub base: Profile,
    pub candidates: Vec<Candidate>,
    pub caps: Capabilities,
    pub llama_version: Option<String>,
}

pub struct Tuner {
    report: Arc<std::sync::Mutex<Report>>,
    cancel: std::sync::Mutex<Cancel>,
    /// The server being measured right now, so the orphan scan does not report Tune's own
    /// child as something the user has left behind.
    pid: Arc<std::sync::Mutex<Option<u32>>>,
    events: crate::runner::Events,
}

impl Tuner {
    pub fn new(events: crate::runner::Events) -> Self {
        Self {
            report: Arc::new(std::sync::Mutex::new(Report::default())),
            cancel: std::sync::Mutex::new(Cancel::default()),
            pid: Arc::new(std::sync::Mutex::new(None)),
            events,
        }
    }

    pub fn report(&self) -> Report {
        self.report.lock().expect("tune lock").clone()
    }

    pub fn live_pid(&self) -> Option<u32> {
        *self.pid.lock().expect("tune pid lock")
    }

    pub fn cancel(&self) {
        self.cancel.lock().expect("tune cancel lock").cancel();
    }

    pub fn start(&self, request: Request) -> Result<(), String> {
        if self.report.lock().expect("tune lock").running {
            return Err("a measurement is already running".into());
        }
        if request.candidates.is_empty() {
            return Err(
                "nothing to measure: no context this model supports fits the GPU, or the \
                 build did not report one"
                    .into(),
            );
        }
        if let Some(conflict) = crate::runner::inspect_port("127.0.0.1", PORT) {
            return Err(format!(
                "port {} is busy, and Tune measures on it — stop what is using it first",
                conflict.port
            ));
        }

        let cancel = Cancel::default();
        *self.cancel.lock().expect("tune cancel lock") = cancel.clone();

        let prompt = prompt_for(&request.candidates);
        {
            let mut report = self.report.lock().expect("tune lock");
            *report = Report {
                running: true,
                model_id: Some(request.model_id.clone()),
                model_name: Some(request.model_name.clone()),
                prompt_words: Some(prompt.split_whitespace().count() as u64),
                candidates: request.candidates.clone(),
                total: request.candidates.len(),
                ..Default::default()
            };
        }
        self.announce();

        let report = self.report.clone();
        let pid = self.pid.clone();
        let events = self.events.clone();
        thread::spawn(move || {
            ladder(request, prompt, cancel, report, pid, events);
        });
        Ok(())
    }

    fn announce(&self) {
        announce(&self.events, &self.report);
    }
}

fn announce(events: &crate::runner::Events, report: &Arc<std::sync::Mutex<Report>>) {
    let snapshot = report.lock().expect("tune lock").clone();
    if let Ok(payload) = serde_json::to_value(snapshot) {
        events.emit("tune:report", payload);
    }
}

fn ladder(
    request: Request,
    prompt: String,
    cancel: Cancel,
    report: Arc<std::sync::Mutex<Report>>,
    pid: Arc<std::sync::Mutex<Option<u32>>>,
    events: crate::runner::Events,
) {
    for candidate in &request.candidates {
        if cancel.cancelled() {
            break;
        }

        {
            let mut guard = report.lock().expect("tune lock");
            guard.current = Some(candidate.clone());
        }
        announce(&events, &report);

        let profile = profile_for(&request.base, candidate);
        let args = profile.args(&request.model_path, &request.caps);
        let registry = pid.clone();
        let measured = measure(&request.caps.binary, &args, &prompt, &cancel, move |live| {
            *registry.lock().expect("tune pid lock") = live;
        });
        *pid.lock().expect("tune pid lock") = None;

        let outcome = match measured {
            Ok(reading) => {
                record(&request, candidate, reading);
                Outcome {
                    candidate: candidate.clone(),
                    reading: Some(reading),
                    error: None,
                }
            }
            Err(error) => Outcome {
                candidate: candidate.clone(),
                reading: None,
                error: Some(error),
            },
        };

        {
            let mut guard = report.lock().expect("tune lock");
            guard.rows.push(outcome);
            guard.done += 1;
            guard.current = None;
        }
        announce(&events, &report);
    }

    {
        let mut guard = report.lock().expect("tune lock");
        guard.running = false;
        guard.current = None;
        guard.cancelled = cancel.cancelled();
    }
    announce(&events, &report);
}

/// Measured under control, and filed in the same place ordinary use is filed.
fn record(request: &Request, candidate: &Candidate, reading: Reading) {
    let profile = profile_for(&request.base, candidate);
    let record = crate::speeds::SpeedRecord {
        timestamp_secs: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        key: crate::speeds::SpeedKey::of(&request.model_id, &profile),
        llama_version: request.llama_version.clone(),
        source: crate::speeds::Source::Measured,
        prompt_tokens: reading.prompt_tokens,
        prompt_seconds: reading.prompt_seconds,
        gen_tokens: reading.gen_tokens,
        gen_seconds: reading.gen_seconds,
    };
    if !record.is_sound() {
        return;
    }
    let _ = crate::store::append_speed(&crate::store::speeds_path(), record);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probe::Device;
    use std::collections::BTreeSet;

    fn metadata(max_ctx: u64) -> GgufMetadata {
        GgufMetadata {
            gguf_version: 3,
            tensor_count: 0,
            architecture: "test".into(),
            name: None,
            size_label: None,
            context_length: Some(max_ctx),
            block_count: Some(40),
            embedding_length: Some(4096),
            head_count: Some(32),
            head_count_kv: Some(8),
            key_length: Some(128),
            value_length: Some(128),
            sliding_window: None,
            full_attention_interval: None,
            key_length_swa: None,
            value_length_swa: None,
            expert_count: None,
            file_type: None,
            has_chat_template: true,
        }
    }

    fn caps_with(devices: Vec<Device>) -> Capabilities {
        Capabilities {
            binary: "llama-server".into(),
            version: Some("b10360".into()),
            flags: BTreeSet::new(),
            flash_attn_takes_value: true,
            devices,
        }
    }

    fn gpu(total_mib: u64) -> Capabilities {
        caps_with(vec![Device {
            id: "MTL0".into(),
            name: "Apple M2 Pro".into(),
            total_mib,
            free_mib: total_mib,
        }])
    }

    #[test]
    fn the_arithmetics_own_pick_is_measured_first() {
        let picked = candidates(&metadata(262_144), 1_000_000_000, &gpu(8192));

        assert!(!picked.is_empty());
        let widest = picked[0].clone();
        assert!(
            picked.iter().all(|c| c.ctx <= widest.ctx),
            "the rule Tune exists to check goes first: {picked:?}"
        );
        assert!(
            picked.iter().any(|c| c.cache() != widest.cache()),
            "both cache types are measured, or the axis that beat the arithmetic on \
             Ornith is never tried: {picked:?}"
        );
    }

    /// Where both fit, the same context with the other cache is measured — the pair that
    /// disagreed by 27%. Where it does not fit, as on Ornith at 262,144, the ladder is
    /// three candidates rather than four and the comparison is made across contexts.
    #[test]
    fn the_same_context_with_the_other_cache_is_measured_when_it_fits() {
        let picked = candidates(&metadata(262_144), 1_000_000_000, &gpu(200_000));
        let widest = picked[0].clone();

        assert_eq!(widest.ctx, 262_144);
        assert_eq!(
            widest.cache(),
            "f16",
            "the more precise of two that both fit"
        );
        assert!(
            picked
                .iter()
                .any(|c| c.ctx == widest.ctx && c.cache() == "q8_0"),
            "{picked:?}"
        );
    }

    #[test]
    fn a_candidate_is_never_repeated() {
        let picked = candidates(&metadata(262_144), 1_000_000_000, &gpu(26_000));
        let mut seen = Vec::new();
        for candidate in &picked {
            assert!(!seen.contains(&candidate), "measured twice: {candidate:?}");
            seen.push(candidate);
        }
    }

    #[test]
    fn nothing_that_does_not_fit_is_measured() {
        let caps = gpu(8192);
        let md = metadata(262_144);
        let budget = budget_bytes(&caps).expect("budget");

        for candidate in candidates(&md, 1_000_000_000, &caps) {
            assert!(
                fits(&md, 1_000_000_000, candidate.ctx, candidate.cache(), budget),
                "{candidate:?} does not fit and would only measure a failure to load"
            );
        }
    }

    /// A build that cannot report its devices leaves the ceiling unknown, and measuring
    /// against a guessed one is the defect the memory panel already refuses.
    #[test]
    fn a_build_that_reports_no_devices_offers_nothing_to_measure() {
        assert!(candidates(&metadata(262_144), 1_000_000_000, &caps_with(Vec::new())).is_empty());
    }

    #[test]
    fn a_model_too_large_for_the_gpu_offers_nothing_to_measure() {
        assert!(candidates(&metadata(262_144), 40_000_000_000, &gpu(8192)).is_empty());
    }

    #[test]
    fn the_prompt_is_long_enough_to_measure_throughput() {
        let prompt = prompt_of(4096);
        let words = prompt.split_whitespace().count();
        assert!(
            (3000..3400).contains(&words),
            "0.75 words per token of 4096: {words}"
        );
        assert!(prompt.contains(".\n"), "sentences, not one run-on line");
        assert!(
            prompt.split_whitespace().collect::<BTreeSet<_>>().len() > 20,
            "a repeated line is not what the cache does with real text"
        );
    }

    /// The 36-token reading that said 2% where a real prompt said 27% is why this is
    /// floored rather than simply scaled.
    #[test]
    fn a_small_context_still_gets_a_prompt_worth_timing() {
        assert!(prompt_of(1).split_whitespace().count() >= 32);

        let tiny = vec![Candidate {
            ctx: 4096,
            cache_k: "f16".into(),
            cache_v: "f16".into(),
        }];
        assert!(prompt_for(&tiny).split_whitespace().count() >= 256);
    }

    #[test]
    fn every_candidate_is_asked_the_same_question() {
        let spread = vec![
            Candidate {
                ctx: 262_144,
                cache_k: "q8_0".into(),
                cache_v: "q8_0".into(),
            },
            Candidate {
                ctx: 8192,
                cache_k: "f16".into(),
                cache_v: "f16".into(),
            },
        ];
        let prompt = prompt_for(&spread);
        assert!(
            prompt.split_whitespace().count() < 8192,
            "sized to the smallest context, or the smallest candidate cannot hold it"
        );
    }

    #[test]
    fn timings_are_read_from_what_the_server_reported() {
        let body = r#"{"content":"one two","timings":{"cache_n":0,"predicted_ms":323.474,
            "predicted_n":48,"predicted_per_second":148.389,"prompt_ms":53.639,
            "prompt_n":6,"prompt_per_second":111.858}}"#;
        let reading = parse_timings(body).expect("timings");
        assert_eq!(reading.gen_tokens, 48.0);
        assert!((reading.gen_seconds - 0.323474).abs() < 1e-9);
        assert_eq!(reading.prompt_tokens, 6.0);

        assert!(parse_timings(r#"{"content":"no timings here"}"#).is_none());
        assert!(parse_timings("not json").is_none());
    }

    #[test]
    fn the_fastest_generation_wins_rather_than_the_widest_context() {
        let report = Report {
            rows: vec![
                Outcome {
                    candidate: Candidate {
                        ctx: 262_144,
                        cache_k: "q8_0".into(),
                        cache_v: "q8_0".into(),
                    },
                    reading: Some(Reading {
                        prompt_tokens: 3794.0,
                        prompt_seconds: 9.05,
                        gen_tokens: 64.0,
                        gen_seconds: 2.1,
                    }),
                    error: None,
                },
                Outcome {
                    candidate: Candidate {
                        ctx: 65_536,
                        cache_k: "f16".into(),
                        cache_v: "f16".into(),
                    },
                    reading: Some(Reading {
                        prompt_tokens: 3794.0,
                        prompt_seconds: 7.47,
                        gen_tokens: 64.0,
                        gen_seconds: 1.54,
                    }),
                    error: None,
                },
                Outcome {
                    candidate: Candidate {
                        ctx: 131_072,
                        cache_k: "f16".into(),
                        cache_v: "f16".into(),
                    },
                    reading: None,
                    error: Some("did not load".into()),
                },
            ],
            ..Default::default()
        };

        let fastest = report.fastest().expect("a winner");
        assert_eq!(
            fastest.candidate.ctx, 65_536,
            "the arithmetic's pick was the slowest of the three"
        );
    }
}
