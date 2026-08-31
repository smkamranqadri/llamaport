//! Tune against the real machine: the file on disk, the installed build, and the ceiling
//! that build reports. `tools/fits.py` is the oracle, and a disagreement about a real
//! model is a finding rather than a nuisance — the same relationship `estimate.rs` has
//! with it.
//!
//! Skips rather than fails where the model or the binary is missing, so a fresh clone
//! stays green. The ladder itself is ignored by default: it launches a 21 GB model
//! several times.
//!
//!     cargo test --test real_tune -- --ignored --nocapture

use std::path::PathBuf;

use llamaport_lib::catalog::{self, ModelEntry};
use llamaport_lib::probe::{self, Capabilities};
use llamaport_lib::profile::Profile;
use llamaport_lib::tune::{self, Candidate};

mod common;

/// The model this project's measurements were taken against.
const ORNITH: &str = "ornith-1.0-35b";

fn models_dir() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_default()).join("models")
}

fn machine(name: &str) -> Option<(ModelEntry, Capabilities)> {
    let dir = models_dir();
    if !dir.is_dir() {
        eprintln!("skipping: {} does not exist", dir.display());
        return None;
    }
    let Some(model) = catalog::scan(&dir)
        .into_iter()
        .find(|m| m.file_name.contains(name) && m.error.is_none())
    else {
        eprintln!("skipping: no model matching {name}");
        return None;
    };
    let Some(binary) = probe::discover(None) else {
        eprintln!("skipping: llama-server not found");
        return None;
    };
    let caps = probe::probe(&binary).expect("probe");
    if caps.device_budget_mib().is_none() {
        eprintln!("skipping: this build reports no devices");
        return None;
    }
    Some((model, caps))
}

fn rung(ctx: u64, cache: &str) -> Candidate {
    Candidate {
        ctx,
        cache_k: cache.into(),
        cache_v: cache.into(),
    }
}

/// What `tools/fits.py` picks for this file on this machine, read off its output on
/// 2026-08-31: 262,144 with a quantised cache is the widest that fits, a full-precision
/// cache at that context does not fit at all, and the two comparisons left are a small
/// and a middle rung.
#[test]
fn the_candidates_agree_with_the_oracle() {
    let Some((model, caps)) = machine(ORNITH) else {
        return;
    };
    let metadata = model.metadata.as_ref().expect("metadata");

    let picked = tune::candidates(metadata, model.size_bytes, &caps);
    println!("{} on {}", model.file_name, caps.binary);
    for candidate in &picked {
        println!("  {:>9} ctx / {}", candidate.ctx, candidate.cache_k);
    }

    assert_eq!(
        picked,
        vec![
            rung(262_144, "q8_0"),
            rung(8192, "f16"),
            rung(65_536, "f16"),
        ],
        "the Rust and the script disagree about a real file, and one of them is wrong"
    );
}

/// The whole argument for Tune, run for real: arithmetic picks the first of these and
/// measurement picks the last.
#[test]
#[ignore]
fn the_ladder_reproduces_the_oracles_ordering() {
    let Some((model, caps)) = machine(ORNITH) else {
        return;
    };
    common::isolate_config_dir();
    let metadata = model.metadata.as_ref().expect("metadata");
    let picked = tune::candidates(metadata, model.size_bytes, &caps);

    let base = Profile {
        alias: "tune".into(),
        ..Profile::default()
    };
    let prompt = tune::prompt_for(&picked);
    println!(
        "same prompt for each, {} words\n",
        prompt.split_whitespace().count()
    );

    let cancel = tune::Cancel::default();
    let mut rows = Vec::new();
    for candidate in &picked {
        let profile = tune::profile_for(&base, candidate);
        let args = profile.args(&model.path, &caps);
        print!("  {:>9} ctx / {:<5} ...", candidate.ctx, candidate.cache_k);
        let reading = tune::measure(&caps.binary, &args, &prompt, &cancel, |_| {})
            .unwrap_or_else(|e| panic!("{candidate:?}: {e}"));
        let gen = reading.gen_tokens / reading.gen_seconds;
        let prompt_tps = reading.prompt_tokens / reading.prompt_seconds;
        println!(
            " {gen:6.1} tok/s generation · {prompt_tps:7.1} prompt ({} tokens)",
            reading.prompt_tokens
        );
        rows.push((candidate.clone(), gen));
    }

    let (fastest, best) = rows
        .iter()
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .expect("a winner");
    let (widest, widest_tps) = rows.first().expect("the arithmetic's pick");
    println!(
        "\n  fastest        : {:>9} ctx / {}  at {best:.1} tok/s",
        fastest.ctx, fastest.cache_k
    );
    println!(
        "  the arithmetic : {:>9} ctx / {}  at {widest_tps:.1} tok/s",
        widest.ctx, widest.cache_k
    );

    assert_eq!(
        fastest.ctx, 65_536,
        "measurement picked something other than what fits.py measured"
    );
    assert_eq!(fastest.cache_k, "f16");
    assert!(
        best > widest_tps,
        "the whole argument for Tune is that the widest that fits is not the fastest"
    );
}
