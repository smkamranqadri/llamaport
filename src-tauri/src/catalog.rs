use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::Serialize;
use sha2::{Digest, Sha256};
use sysinfo::Disks;

use crate::gguf::{self, GgufMetadata};

const IDENTITY_PREFIX_BYTES: usize = 4096;
const SHARD_SUFFIX_LEN: usize = "-00001-of-00002".len();

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShardInfo {
    pub total: u32,
    pub present: u32,
    pub missing: Vec<u32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelEntry {
    pub id: String,
    pub display_name: String,
    pub file_name: String,
    pub path: String,
    pub size_bytes: u64,
    pub modified_secs: Option<u64>,
    pub quant: Option<String>,
    pub shards: Option<ShardInfo>,
    pub metadata: Option<GgufMetadata>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirInfo {
    pub path: String,
    pub exists: bool,
    pub free_bytes: Option<u64>,
    pub total_bytes: Option<u64>,
}

/// Identity is `(size, hash of the leading bytes)`: free to compute during a scan that
/// already reads the header, and stable across renames and directory moves.
fn identity(path: &Path, size_bytes: u64) -> Option<String> {
    let mut file = File::open(path).ok()?;
    let mut buf = vec![0u8; IDENTITY_PREFIX_BYTES];
    let mut filled = 0;

    while filled < buf.len() {
        match file.read(&mut buf[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(_) => return None,
        }
    }

    let digest = Sha256::digest(&buf[..filled]);
    let hex: String = digest.iter().take(8).map(|b| format!("{b:02x}")).collect();
    Some(format!("{size_bytes:x}-{hex}"))
}

fn is_quant_token(token: &str) -> bool {
    if matches!(token, "F16" | "BF16" | "F32" | "F64") {
        return true;
    }
    let rest = token
        .strip_prefix("IQ")
        .or_else(|| token.strip_prefix('Q'))
        .or_else(|| token.strip_prefix("TQ"));
    rest.and_then(|r| r.chars().next())
        .is_some_and(|c| c.is_ascii_digit())
}

fn quant_from_name(name: &str) -> Option<String> {
    let tokens: Vec<&str> = name.split(['-', '.']).collect();
    for (i, token) in tokens.iter().enumerate() {
        let upper = token.to_ascii_uppercase();
        if !is_quant_token(&upper) {
            continue;
        }
        if i > 0 && tokens[i - 1].eq_ignore_ascii_case("UD") {
            return Some(format!("UD-{upper}"));
        }
        return Some(upper);
    }
    None
}

/// Splits `name-00001-of-00003` into its base name, index, and total.
fn parse_shard(stem: &str) -> Option<(&str, u32, u32)> {
    if stem.len() <= SHARD_SUFFIX_LEN {
        return None;
    }
    let split = stem.len() - SHARD_SUFFIX_LEN;
    if !stem.is_char_boundary(split) {
        return None;
    }
    let (base, suffix) = stem.split_at(split);
    if !suffix.is_ascii() || !suffix.starts_with('-') || &suffix[6..10] != "-of-" {
        return None;
    }
    let index: u32 = suffix[1..6].parse().ok()?;
    let total: u32 = suffix[10..].parse().ok()?;
    if index == 0 || total == 0 || index > total {
        return None;
    }
    Some((base, index, total))
}

struct RawFile {
    path: PathBuf,
    stem: String,
    size_bytes: u64,
    modified_secs: Option<u64>,
}

fn collect_gguf_files(dir: &Path) -> Vec<RawFile> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let is_gguf = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("gguf"));
        if !is_gguf {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        if !meta.is_file() {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };

        out.push(RawFile {
            stem: stem.to_string(),
            size_bytes: meta.len(),
            modified_secs: meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs()),
            path,
        });
    }
    out
}

fn build_entry(
    primary: &RawFile,
    display_stem: &str,
    size_bytes: u64,
    shards: Option<ShardInfo>,
) -> ModelEntry {
    let parsed = gguf::read_metadata(&primary.path);
    let (metadata, error) = match parsed {
        Ok(md) => (Some(md), None),
        Err(e) => (None, Some(e.to_string())),
    };

    let display_name = metadata
        .as_ref()
        .and_then(|m| m.name.clone())
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| display_stem.to_string());

    ModelEntry {
        id: identity(&primary.path, size_bytes)
            .unwrap_or_else(|| primary.path.to_string_lossy().into_owned()),
        display_name,
        file_name: primary
            .path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        path: primary.path.to_string_lossy().into_owned(),
        size_bytes,
        modified_secs: primary.modified_secs,
        quant: quant_from_name(display_stem),
        shards,
        metadata,
        error,
    }
}

pub fn scan(dir: &Path) -> Vec<ModelEntry> {
    let files = collect_gguf_files(dir);

    let mut singles: Vec<RawFile> = Vec::new();
    let mut groups: BTreeMap<String, (u32, Vec<(u32, RawFile)>)> = BTreeMap::new();

    for file in files {
        match parse_shard(&file.stem) {
            Some((base, index, total)) => {
                let group = groups.entry(base.to_string()).or_insert((total, Vec::new()));
                group.0 = group.0.max(total);
                group.1.push((index, file));
            }
            None => singles.push(file),
        }
    }

    let mut entries: Vec<ModelEntry> = singles
        .iter()
        .map(|f| build_entry(f, &f.stem, f.size_bytes, None))
        .collect();

    for (base, (total, mut parts)) in groups {
        parts.sort_by_key(|(index, _)| *index);
        let present: Vec<u32> = parts.iter().map(|(index, _)| *index).collect();
        let missing: Vec<u32> = (1..=total).filter(|i| !present.contains(i)).collect();
        let size_bytes = parts.iter().map(|(_, f)| f.size_bytes).sum();

        let Some((_, primary)) = parts.first() else {
            continue;
        };
        let shards = ShardInfo {
            total,
            present: parts.len() as u32,
            missing,
        };
        entries.push(build_entry(primary, &base, size_bytes, Some(shards)));
    }

    entries.sort_by_key(|e| e.display_name.to_lowercase());
    entries
}

pub fn dir_info(dir: &Path) -> DirInfo {
    let disks = Disks::new_with_refreshed_list();
    let best = disks
        .list()
        .iter()
        .filter(|d| dir.starts_with(d.mount_point()))
        .max_by_key(|d| d.mount_point().as_os_str().len());

    DirInfo {
        path: dir.to_string_lossy().into_owned(),
        exists: dir.is_dir(),
        free_bytes: best.map(|d| d.available_space()),
        total_bytes: best.map(|d| d.total_space()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_quant_from_real_filenames() {
        let cases = [
            ("Qwen3.6-35B-A3B-UD-Q3_K_XL", Some("UD-Q3_K_XL")),
            ("gemma-4-26B-A4B-it-UD-Q4_K_M", Some("UD-Q4_K_M")),
            ("Devstral-Small-2-24B-Instruct-2512-Q4_K_M", Some("Q4_K_M")),
            ("GLM-4.7-Flash-Q4_K_M", Some("Q4_K_M")),
            ("some-model-IQ2_XXS", Some("IQ2_XXS")),
            ("some-model-BF16", Some("BF16")),
            ("model-without-a-quant", None),
        ];

        for (input, expected) in cases {
            assert_eq!(
                quant_from_name(input).as_deref(),
                expected,
                "quant for {input}"
            );
        }
    }

    #[test]
    fn model_names_are_not_mistaken_for_quants() {
        assert!(!is_quant_token("QWEN3"));
        assert!(is_quant_token("Q4_K_M"));
    }

    #[test]
    fn splits_shard_filenames() {
        assert_eq!(
            parse_shard("Big-Model-Q4_K_M-00002-of-00005"),
            Some(("Big-Model-Q4_K_M", 2, 5))
        );
        assert_eq!(parse_shard("Plain-Model-Q4_K_M"), None);
        assert_eq!(parse_shard("Model-00002-of-00001"), None);
    }
}
