use std::collections::{BTreeMap, BTreeSet};
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
    /// Set by `arrange`, which is the only thing that knows the config. False on a raw
    /// scan rather than absent, so the screen never has to reason about a missing field.
    pub favourite: bool,
    /// Also set by `arrange`. `None` for a model that has never been launched, which is
    /// what sends the row back to `modified_secs` for something to show.
    pub last_launched_secs: Option<u64>,
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
        favourite: false,
        last_launched_secs: None,
    }
}

pub fn scan(dir: &Path) -> Vec<ModelEntry> {
    let files = collect_gguf_files(dir);

    let mut singles: Vec<RawFile> = Vec::new();
    let mut groups: BTreeMap<String, (u32, Vec<(u32, RawFile)>)> = BTreeMap::new();

    for file in files {
        match parse_shard(&file.stem) {
            Some((base, index, total)) => {
                let group = groups
                    .entry(base.to_string())
                    .or_insert((total, Vec::new()));
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

/// Favourites to the top, and within each group the most recent activity first.
///
/// A re-sort, which the alphabetical order this replaced was written to avoid. The
/// condition on that is `recency`: the list may reorder because the value it reorders on
/// is on the row, so the movement has a visible reason. If that cell ever goes, this
/// sorting goes with it.
///
/// A starred id that names no model here is kept, not dropped — the file may be on a disk
/// that is not mounted, and forgetting the star would be losing it for good.
pub fn arrange(
    entries: Vec<ModelEntry>,
    favourites: &BTreeSet<String>,
    last_launched: &BTreeMap<String, u64>,
) -> Vec<ModelEntry> {
    let (mut starred, mut rest): (Vec<_>, Vec<_>) = entries
        .into_iter()
        .map(|mut entry| {
            entry.favourite = favourites.contains(&entry.id);
            entry.last_launched_secs = last_launched.get(&entry.id).copied();
            entry
        })
        .partition(|entry| entry.favourite);

    by_recency(&mut starred);
    by_recency(&mut rest);
    [starred, rest].concat()
}

/// What the row shows in its last cell, and so the only thing it may be sorted on.
fn recency(entry: &ModelEntry) -> Option<u64> {
    entry.last_launched_secs.or(entry.modified_secs)
}

/// Newest first, a model with no date at all last, and a stable sort so that models
/// sharing a date keep the alphabetical order the scan gave them.
fn by_recency(entries: &mut [ModelEntry]) {
    entries.sort_by_key(|entry| std::cmp::Reverse(recency(entry)));
}

/// Whether the file may be taken out from under whatever is using it. A running server
/// reads tensors on demand, so deleting its model leaves it serving until it needs a page
/// that is no longer there.
pub fn deletable(entry: &ModelEntry, running: Option<&str>) -> Result<(), String> {
    if running == Some(entry.id.as_str()) {
        return Err(format!(
            "{} is running — stop it before deleting it",
            entry.display_name
        ));
    }
    Ok(())
}

/// Every file one catalog entry is made of.
///
/// A shard set is one model across several filenames and only its first part is recorded
/// on the entry, so the rest are found the way the scan grouped them: by base name. What
/// is on disk decides, not what the set declares — an incomplete set still deletes the
/// parts it does have.
pub fn files_of(entry: &ModelEntry) -> Vec<PathBuf> {
    let path = PathBuf::from(&entry.path);
    if entry.shards.is_none() {
        return vec![path];
    }

    let base = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .and_then(parse_shard)
        .map(|(base, _, _)| base.to_string());
    let (Some(base), Some(dir)) = (base, path.parent()) else {
        return vec![path];
    };

    let mut parts: Vec<PathBuf> = collect_gguf_files(dir)
        .into_iter()
        .filter(|file| parse_shard(&file.stem).is_some_and(|(found, _, _)| found == base))
        .map(|file| file.path)
        .collect();
    parts.sort();
    if parts.is_empty() {
        return vec![path];
    }
    parts
}

/// Moves files to the Trash, so a mistaken delete of a 20 GB quant is recoverable.
///
/// Through JavaScript for Automation rather than Finder: `osascript -l JavaScript` reaches
/// `NSFileManager.trashItemAtURL` over the ObjC bridge, which asks Finder for nothing and
/// so raises no Automation prompt — the thing that made the obvious `tell application
/// "Finder" to delete` unattractive on an unsigned app. Shelling out matches how the app
/// already reveals a file, and adds no dependency.
///
/// All or nothing is not on offer: each file is its own call, and a failure part-way
/// through leaves the rest where they are and says which one stopped it.
pub fn trash(paths: &[PathBuf]) -> Result<(), String> {
    for path in paths {
        let script = format!(
            r#"ObjC.import("Foundation");
               var error = Ref();
               var url = $.NSURL.fileURLWithPath({});
               $.NSFileManager.defaultManager
                   .trashItemAtURLResultingItemURLError(url, null, error)
                 ? "ok"
                 : "failed: " + error[0].localizedDescription.js"#,
            json_string(&path.to_string_lossy())
        );

        let output = std::process::Command::new("osascript")
            .arg("-l")
            .arg("JavaScript")
            .arg("-e")
            .arg(&script)
            .output()
            .map_err(|e| e.to_string())?;

        let said = String::from_utf8_lossy(&output.stdout);
        if !output.status.success() || said.trim() != "ok" {
            let detail = match said.trim() {
                "" => String::from_utf8_lossy(&output.stderr).trim().to_string(),
                reported => reported.to_string(),
            };
            return Err(format!(
                "could not move {} to the Trash: {detail}",
                path.file_name().unwrap_or_default().to_string_lossy()
            ));
        }
    }
    Ok(())
}

/// A path reaches the script as a JavaScript literal, so it has to be escaped as one — a
/// model named with a quote would otherwise end the string and run as code.
fn json_string(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_string())
}

/// Available and total bytes on the volume holding `dir`, picking the deepest matching
/// mount point so a nested volume wins over `/`.
pub fn disk_space(dir: &Path) -> Option<(u64, u64)> {
    let disks = Disks::new_with_refreshed_list();
    disks
        .list()
        .iter()
        .filter(|d| dir.starts_with(d.mount_point()))
        .max_by_key(|d| d.mount_point().as_os_str().len())
        .map(|d| (d.available_space(), d.total_space()))
}

pub fn dir_info(dir: &Path) -> DirInfo {
    let space = disk_space(dir);

    DirInfo {
        path: dir.to_string_lossy().into_owned(),
        exists: dir.is_dir(),
        free_bytes: space.map(|(available, _)| available),
        total_bytes: space.map(|(_, total)| total),
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

    fn stub(name: &str) -> ModelEntry {
        ModelEntry {
            id: format!("id-{name}"),
            display_name: name.to_string(),
            file_name: format!("{name}.gguf"),
            path: format!("/models/{name}.gguf"),
            size_bytes: 1,
            modified_secs: None,
            quant: None,
            shards: None,
            metadata: None,
            error: None,
            favourite: false,
            last_launched_secs: None,
        }
    }

    fn dated(name: &str, modified_secs: Option<u64>) -> ModelEntry {
        ModelEntry {
            modified_secs,
            ..stub(name)
        }
    }

    fn launches(pairs: &[(&str, u64)]) -> BTreeMap<String, u64> {
        pairs
            .iter()
            .map(|(name, secs)| (format!("id-{name}"), *secs))
            .collect()
    }

    fn named(entries: &[ModelEntry]) -> Vec<String> {
        entries.iter().map(|e| e.display_name.clone()).collect()
    }

    fn touch(dir: &Path, name: &str) {
        fs::write(dir.join(name), b"gguf").expect("seed file");
    }

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("llama-hub-catalog-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("scratch dir");
        dir
    }

    /// A shard set is one model wearing several filenames. Deleting the entry the Library
    /// shows must take all of them: leaving the other parts behind wastes the disk space
    /// the delete was for, and leaves an incomplete set the Library then complains about.
    #[test]
    fn a_delete_takes_every_part_of_a_shard_set_and_nothing_else() {
        let dir = scratch("files-of");
        touch(&dir, "Big-Q4_K_M-00001-of-00003.gguf");
        touch(&dir, "Big-Q4_K_M-00002-of-00003.gguf");
        touch(&dir, "Big-Q4_K_M-00003-of-00003.gguf");
        touch(&dir, "Small-Q4_K_M.gguf");
        touch(&dir, "Unrelated-Q8_0-00001-of-00002.gguf");

        let entries = scan(&dir);
        let big = entries
            .iter()
            .find(|e| e.shards.is_some())
            .expect("the shard set is one entry");

        let mut taken = files_of(big);
        taken.sort();
        assert_eq!(
            taken,
            vec![
                dir.join("Big-Q4_K_M-00001-of-00003.gguf"),
                dir.join("Big-Q4_K_M-00002-of-00003.gguf"),
                dir.join("Big-Q4_K_M-00003-of-00003.gguf"),
            ],
            "a shard set goes as a unit, and takes nothing that is not part of it"
        );

        let small = entries
            .iter()
            .find(|e| e.file_name == "Small-Q4_K_M.gguf")
            .expect("the single file is an entry");
        assert_eq!(files_of(small), vec![dir.join("Small-Q4_K_M.gguf")]);

        let _ = fs::remove_dir_all(&dir);
    }

    /// A set with a part missing is still a set. Deleting it should clear what is there
    /// rather than refuse because of what is not.
    #[test]
    fn a_delete_takes_the_parts_an_incomplete_set_does_have() {
        let dir = scratch("files-of-incomplete");
        touch(&dir, "Big-Q4_K_M-00001-of-00003.gguf");
        touch(&dir, "Big-Q4_K_M-00003-of-00003.gguf");

        let entries = scan(&dir);
        let mut taken = files_of(&entries[0]);
        taken.sort();
        assert_eq!(
            taken,
            vec![
                dir.join("Big-Q4_K_M-00001-of-00003.gguf"),
                dir.join("Big-Q4_K_M-00003-of-00003.gguf"),
            ]
        );

        let _ = fs::remove_dir_all(&dir);
    }

    /// Favourites are how a library of thirty models stays usable, so they go to the top
    /// — and only that. Nothing here has a date of any kind, so the sort has nothing to
    /// say and the alphabetical order the scan produced has to survive it.
    #[test]
    fn favourites_sort_above_everything_and_undated_models_do_not_move() {
        let entries: Vec<ModelEntry> = ["alpha", "bravo", "charlie", "delta"]
            .iter()
            .map(|name| stub(name))
            .collect();

        let none = BTreeSet::new();
        let never = BTreeMap::new();
        assert_eq!(
            named(&arrange(entries.clone(), &none, &never)),
            ["alpha", "bravo", "charlie", "delta"],
            "with no favourites and no dates the order is the one the scan produced"
        );

        let starred: BTreeSet<String> = ["id-delta".to_string(), "id-bravo".to_string()]
            .into_iter()
            .collect();
        let ordered = arrange(entries.clone(), &starred, &never);
        assert_eq!(
            named(&ordered),
            ["bravo", "delta", "alpha", "charlie"],
            "favourites first, each group still alphabetical"
        );
        assert_eq!(
            ordered.iter().map(|e| e.favourite).collect::<Vec<_>>(),
            [true, true, false, false],
            "the screen draws the star from the entry, so it has to be marked"
        );

        // A favourite for a model that is no longer in the directory is not an error: the
        // file may come back, and dropping the star for a model on a detached disk would
        // lose it for good.
        let stale: BTreeSet<String> = ["id-gone".to_string()].into_iter().collect();
        assert_eq!(
            named(&arrange(entries, &stale, &never)),
            ["alpha", "bravo", "charlie", "delta"]
        );
    }

    /// The whole point of the column: what you have been running is at the top. A model
    /// that has never been launched falls back to when its file arrived, so a fresh
    /// download is near the top too — it is still the most recent thing that happened to
    /// it — and a model with no date at all goes last rather than first.
    #[test]
    fn the_list_orders_on_the_same_value_the_row_shows() {
        let entries = vec![
            dated("alpha", Some(100)),
            dated("bravo", Some(900)),
            dated("charlie", Some(200)),
            stub("delta"),
        ];

        let none = BTreeSet::new();
        assert_eq!(
            named(&arrange(
                entries.clone(),
                &none,
                &launches(&[("alpha", 500)])
            )),
            ["bravo", "alpha", "charlie", "delta"],
            "bravo's file is newer than alpha's launch; delta has neither date"
        );

        let ordered = arrange(entries.clone(), &none, &launches(&[("alpha", 1000)]));
        assert_eq!(
            named(&ordered),
            ["alpha", "bravo", "charlie", "delta"],
            "launching alpha moves it above a file newer than it"
        );
        assert_eq!(
            ordered[0].last_launched_secs,
            Some(1000),
            "the row cannot show what it was not given"
        );
        assert_eq!(
            ordered[1].last_launched_secs, None,
            "and a model never launched has to be distinguishable from one that was"
        );

        let starred: BTreeSet<String> = ["id-charlie".to_string()].into_iter().collect();
        assert_eq!(
            named(&arrange(entries, &starred, &launches(&[("alpha", 1000)]))),
            ["charlie", "alpha", "bravo", "delta"],
            "a favourite stays above a model launched more recently"
        );
    }

    /// Deleting the file out from under a running server is how you get a model that
    /// serves until it needs to read a tensor it no longer has.
    #[test]
    fn the_model_the_runner_is_running_cannot_be_deleted() {
        let model = stub("alpha");

        assert!(deletable(&model, Some("id-alpha")).is_err());
        let refused = deletable(&model, Some("id-alpha")).expect_err("refused");
        assert!(refused.contains("running"), "{refused}");

        assert!(
            deletable(&model, Some("id-other")).is_ok(),
            "a different model running is no reason to refuse"
        );
        assert!(deletable(&model, None).is_ok());
    }

    /// Really moves real files, because a delete that silently does nothing is the worst
    /// possible outcome and no stand-in would catch it. The odd name is the injection
    /// case: the path reaches an interpreter, so a quote in a model name must not end the
    /// string it is inside.
    #[test]
    fn trashing_takes_real_files_including_awkwardly_named_ones() {
        let dir = scratch("trash");
        let plain = dir.join("Plain-Q4_K_M.gguf");
        let awkward = dir.join(r#"Odd "quoted" \name.gguf"#);
        fs::write(&plain, b"gguf").expect("seed");
        fs::write(&awkward, b"gguf").expect("seed");

        trash(&[plain.clone(), awkward.clone()]).expect("trashed");

        assert!(!plain.exists(), "the file is still there");
        assert!(!awkward.exists(), "the awkward name defeated the delete");

        assert!(
            trash(&[dir.join("never-existed.gguf")]).is_err(),
            "a delete that finds nothing must say so rather than report success"
        );

        let _ = fs::remove_dir_all(&dir);
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
