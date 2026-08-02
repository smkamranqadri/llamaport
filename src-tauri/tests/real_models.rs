//! Runs against whatever GGUF files are actually in the models directory.
//! Skips rather than fails when there are none, so CI and fresh clones stay green.

use std::collections::HashSet;
use std::path::PathBuf;

use llamaport_lib::catalog::{self, ModelEntry};

fn models_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(home).join("models")
}

fn scan_local() -> Option<Vec<ModelEntry>> {
    let dir = models_dir();
    if !dir.is_dir() {
        eprintln!("skipping: {} does not exist", dir.display());
        return None;
    }
    let entries = catalog::scan(&dir);
    if entries.is_empty() {
        eprintln!("skipping: no GGUF files in {}", dir.display());
        return None;
    }
    Some(entries)
}

#[test]
fn every_local_model_parses() {
    let Some(entries) = scan_local() else { return };

    for entry in &entries {
        assert!(
            entry.error.is_none(),
            "{} failed to parse: {:?}",
            entry.file_name,
            entry.error
        );

        let md = entry.metadata.as_ref().expect("metadata present");
        assert!(
            !md.architecture.is_empty() && md.architecture != "unknown",
            "{} has no architecture",
            entry.file_name
        );
        assert!(
            md.context_length.is_some(),
            "{} has no context length",
            entry.file_name
        );
        assert!(
            md.block_count.is_some(),
            "{} has no block count",
            entry.file_name
        );
        assert!(
            md.head_dim().is_some(),
            "{} cannot derive head dim",
            entry.file_name
        );
        assert!(entry.size_bytes > 0, "{} is empty", entry.file_name);

        println!(
            "{:<46} {:<12} ctx {:>7}  layers {:>3}  kv heads {:>3}  head dim {:>4}  moe {:<5} template {}",
            entry.file_name,
            md.architecture,
            md.context_length.unwrap_or(0),
            md.block_count.unwrap_or(0),
            md.head_count_kv.unwrap_or(0),
            md.head_dim().unwrap_or(0),
            md.is_moe(),
            md.has_chat_template,
        );
    }
}

#[test]
fn quant_is_extracted_for_every_local_model() {
    let Some(entries) = scan_local() else { return };

    for entry in &entries {
        assert!(
            entry.quant.is_some(),
            "{} has no detectable quant",
            entry.file_name
        );
    }
}

#[test]
fn identities_are_unique_and_stable() {
    let Some(first) = scan_local() else { return };
    let second = catalog::scan(&models_dir());

    let unique: HashSet<&str> = first.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(unique.len(), first.len(), "model ids collide");

    for (a, b) in first.iter().zip(second.iter()) {
        assert_eq!(a.id, b.id, "id for {} changed between scans", a.file_name);
    }
}
