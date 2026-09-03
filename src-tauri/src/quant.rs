//! Chooses which file in a repository to fetch.
//!
//! It decides what to download and never how to run it — [`crate::tune`] still owns that.
//! Arithmetic picking a launch is what Tune measured as choosing the slowest of three
//! candidates; arithmetic picking which file to fetch is a far weaker claim, and it is the
//! one this app has to make before a model exists on disk.

use serde::Serialize;

use crate::catalog::{parse_shard, quant_from_name};
use crate::hub::Entry;

/// Within this much of the largest that fits, quantisations are treated as
/// indistinguishable on size and the family decides instead.
const BAND: f64 = 0.95;

/// llama.cpp's own margin, not one this project invented: `--fit-target` defaults to
/// 1,024 MiB below the device working set, which is the room it keeps for everything that is
/// not weights. Charging the same margin means a pick that says it fits agrees with what the
/// binary would admit, and it costs nothing to justify — see
/// [`crate::estimate`] and the `--fit` note in the project's Knowledge.
const HEADROOM: u64 = 1024 * 1024 * 1024;

/// The default this ecosystem converged on, used only when there is no ceiling to measure
/// against — no `llama-server`, so no Metal working set.
const CONVENTIONAL: &str = "Q4_K_M";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    /// The quantisation as the repository spells it, or the file stem when it names none.
    pub label: String,
    /// One path, or a shard set in order. Each becomes a job in the existing queue.
    pub paths: Vec<String>,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Pick {
    pub candidate: Candidate,
    /// `None` when nothing could be measured against — see [`pick`].
    pub fits: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Family {
    K,
    I,
    Legacy,
    Full,
    Unknown,
}

/// Files that are not the model.
///
/// Every rule here is drawn from 961 real GGUF paths across the 40 most trending repositories,
/// read 2026-09-03, because the obvious rules are wrong. **`mtp` appears in 132 of those paths
/// and only 8 are drafters**: `…-mtp-q4_k_m.gguf` is a model built with MTP, and
/// `…-no-mtp.gguf` is one built without it. Excluding on the substring would have thrown away
/// 124 real quantisations. A drafter announces itself by living in an `mtp/` directory or by
/// leading its own name, never by being mentioned.
fn is_sidecar(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    let (directories, file) = match lower.rsplit_once('/') {
        Some((head, file)) => (head, file),
        None => ("", lower.as_str()),
    };

    if directories.split('/').any(|segment| segment == "mtp") {
        return true;
    }
    // An importance matrix is an artefact of quantising, not weights. It is small, so the
    // fallback that picks the smallest when nothing fits would otherwise reach for it first.
    if file.starts_with("mtp-")
        || file.starts_with("mmproj")
        || file.contains("imatrix")
        || file.contains("-draft")
        || file.contains("-encoder")
    {
        return true;
    }
    false
}

/// Splits a repository path into its directories and the file stem, so the shard suffix and
/// the quantisation can each be read off the part that carries them.
fn split_path(path: &str) -> (&str, &str) {
    let without_extension = path.strip_suffix(".gguf").unwrap_or(path);
    match without_extension.rsplit_once('/') {
        Some((directories, stem)) => (directories, stem),
        None => ("", without_extension),
    }
}

/// Reads the quantisation off the whole path, not just the file name: a repository as often
/// names it with a directory — `UD-Q6_K_XL/`, `BF16/` — and shards the files beneath it.
///
/// `quant_from_name` is [`crate::catalog`]'s, so a quantisation is spelled the same here as
/// in the Library. Two spellings of `UD-Q4_K_XL` on two screens is a defect nobody would
/// think to test for.
fn label_for(path: &str) -> String {
    let (directories, stem) = split_path(path);
    let base = parse_shard(stem).map_or(stem, |(base, _, _)| base);
    if let Some(quant) = quant_from_name(base) {
        return quant;
    }
    if let Some(quant) = directories.rsplit('/').find_map(quant_from_name) {
        return quant;
    }
    base.to_string()
}

fn family_of(label: &str) -> Family {
    let lower = label.to_ascii_lowercase();
    let lower = lower.strip_prefix("ud-").unwrap_or(&lower).to_string();
    if lower.starts_with("iq") {
        return Family::I;
    }
    if matches!(lower.as_str(), "f16" | "bf16" | "f32" | "fp16") {
        return Family::Full;
    }
    if lower.starts_with('q') {
        if lower.contains("_k") {
            return Family::K;
        }
        return Family::Legacy;
    }
    Family::Unknown
}

/// A quantisation while it is still being assembled: its shards arrive in whatever order the
/// tree listed them, and its size is the running sum of the parts seen so far.
struct Group {
    key: String,
    parts: Vec<(u32, String)>,
    size: u64,
}

/// Whether the weights alone clear the ceiling, once llama.cpp's own margin is charged.
/// `None` means there is no ceiling to measure against, which is not the same as `false`.
///
/// The one place this question is answered, so a row and the detail list beside it cannot
/// reach different verdicts about the same file.
pub fn fits(size: u64, ceiling: Option<u64>) -> Option<bool> {
    ceiling.map(|ceiling| size <= ceiling.saturating_sub(HEADROOM))
}

/// One entry per quantisation, shard sets summed and ordered, sidecars gone.
pub fn candidates(entries: &[Entry]) -> Vec<Candidate> {
    let mut groups: Vec<Group> = Vec::new();

    for entry in entries {
        if !entry.path.to_ascii_lowercase().ends_with(".gguf") || is_sidecar(&entry.path) {
            continue;
        }
        let (directories, stem) = split_path(&entry.path);
        let (base, index) = parse_shard(stem).map_or((stem, 0), |(base, index, _)| (base, index));
        let key = format!("{directories}/{base}").to_ascii_lowercase();
        match groups.iter_mut().find(|group| group.key == key) {
            Some(group) => {
                group.parts.push((index, entry.path.clone()));
                group.size += entry.size;
            }
            None => groups.push(Group {
                key,
                parts: vec![(index, entry.path.clone())],
                size: entry.size,
            }),
        }
    }

    groups
        .into_iter()
        .map(|mut group| {
            group
                .parts
                .sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));
            Candidate {
                label: label_for(&group.parts[0].1),
                paths: group.parts.into_iter().map(|(_, path)| path).collect(),
                size: group.size,
            }
        })
        .collect()
}

/// **With a ceiling**: the largest that fits, with anything within [`BAND`] of it treated as
/// the same size and settled on family instead — a K-quant over an I-quant over a legacy one.
/// When nothing fits, the smallest, marked as not fitting: the least bad is the only useful
/// answer there.
///
/// **Without one**: no size claim is available at all, so this picks what the ecosystem
/// defaults to rather than inventing a ceiling out of installed memory — which is the
/// measurement error this whole project started from. The fit comes back `None` and the
/// screen has to say so.
pub fn pick(entries: &[Entry], ceiling: Option<u64>) -> Option<Pick> {
    let candidates = candidates(entries);
    if candidates.is_empty() {
        return None;
    }

    let Some(ceiling) = ceiling else {
        let conventional = candidates
            .iter()
            .find(|candidate| candidate.label.eq_ignore_ascii_case(CONVENTIONAL));
        let chosen = match conventional {
            Some(candidate) => candidate,
            None => candidates
                .iter()
                .filter(|candidate| family_of(&candidate.label) == Family::K)
                .min_by_key(|candidate| candidate.size)
                .unwrap_or(&candidates[0]),
        };
        return Some(Pick {
            candidate: chosen.clone(),
            fits: None,
        });
    };

    // Weights only, and that is the whole claim. What a cache costs cannot be known before
    // the file exists — there is no header to read — so this says a download is allowed and
    // never that a launch is good.
    let fitting: Vec<&Candidate> = candidates
        .iter()
        .filter(|candidate| fits(candidate.size, Some(ceiling)) == Some(true))
        .collect();

    if fitting.is_empty() {
        let smallest = candidates
            .iter()
            .min_by_key(|candidate| candidate.size)
            .expect("candidates is not empty");
        return Some(Pick {
            candidate: smallest.clone(),
            fits: Some(false),
        });
    }

    let largest = fitting
        .iter()
        .map(|candidate| candidate.size)
        .max()
        .expect("fitting is not empty");
    let floor = (largest as f64 * BAND) as u64;
    let chosen = fitting
        .into_iter()
        .filter(|candidate| candidate.size >= floor)
        .min_by(|left, right| {
            family_of(&left.label)
                .cmp(&family_of(&right.label))
                .then(right.size.cmp(&left.size))
                .then(left.label.cmp(&right.label))
        })
        .expect("at least the largest is within the band");

    Some(Pick {
        candidate: chosen.clone(),
        fits: Some(true),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str, size: u64) -> Entry {
        Entry {
            path: path.to_string(),
            size,
            lfs: true,
        }
    }

    const GB: u64 = 1024 * 1024 * 1024;

    #[test]
    fn a_drafter_is_recognised_by_where_it_lives_not_by_being_mentioned() {
        // The 132-against-8 case: every path below contains "mtp" and only two are drafters.
        assert!(is_sidecar("MTP/mtp-Qwen3.8-27B-Q4_0.gguf"));
        assert!(is_sidecar("mtp-rvn.gguf"));
        assert!(!is_sidecar("Qwen3.5-9B-neo-max-mtp-q4_k_m.gguf"));
        assert!(!is_sidecar("qwen3.8-27b-uncensored-ymq-xxs-no-mtp.gguf"));
        assert!(!is_sidecar(
            "huihui-qwen3.8-27b-abliterated-ud-iq2_s-mtp.gguf"
        ));
    }

    #[test]
    fn the_other_sidecars_are_excluded_too() {
        for path in [
            "mmproj-BF16.gguf",
            "mmproj-F16.gguf",
            "imatrix.gguf",
            "imatrix_unsloth.gguf",
            "tiel-coder-35b-a3b.imatrix.gguf",
            "qwen3.8-flash-next-uncensored-mtp-draft.gguf",
            "glm-5.3-flash-vision-encoder.gguf",
        ] {
            assert!(is_sidecar(path), "{path} was kept");
        }
    }

    #[test]
    fn a_shard_set_becomes_one_candidate_summing_its_parts_in_order() {
        let found = candidates(&[
            entry("BF16/Model-BF16-00002-of-00003.gguf", 2),
            entry("BF16/Model-BF16-00001-of-00003.gguf", 1),
            entry("BF16/Model-BF16-00003-of-00003.gguf", 4),
        ]);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].size, 7);
        assert_eq!(
            found[0].paths,
            [
                "BF16/Model-BF16-00001-of-00003.gguf",
                "BF16/Model-BF16-00002-of-00003.gguf",
                "BF16/Model-BF16-00003-of-00003.gguf",
            ]
        );
        assert_eq!(found[0].label, "BF16");
    }

    #[test]
    fn a_quantisation_named_by_its_directory_is_read_off_the_directory() {
        let found = candidates(&[
            entry("UD-Q6_K_XL/Model-00001-of-00002.gguf", 3),
            entry("UD-Q6_K_XL/Model-00002-of-00002.gguf", 4),
        ]);
        assert_eq!(found.len(), 1);
        assert_eq!(
            found[0].label, "UD-Q6_K_XL",
            "the Library spells this UD-Q6_K_XL and Discover must not spell it differently"
        );
        assert_eq!(found[0].size, 7);
    }

    #[test]
    fn a_file_that_names_no_quantisation_keeps_its_own_name() {
        let found = candidates(&[entry("spark-x2.5-4b.gguf", 9)]);
        assert_eq!(found[0].label, "spark-x2.5-4b");
    }

    #[test]
    fn llama_cpps_own_margin_is_kept_below_the_ceiling() {
        // 19.5 GB is under a 20 GB working set and over it once --fit-target's 1,024 MiB
        // is charged, so this is the case the margin exists for.
        let pick = pick(
            &[
                entry("Model-Q5_K_M.gguf", 19 * GB + GB / 2),
                entry("Model-Q4_K_M.gguf", 17 * GB),
            ],
            Some(20 * GB),
        )
        .expect("a pick");
        assert_eq!(pick.candidate.label, "Q4_K_M");
    }

    #[test]
    fn the_largest_that_fits_is_chosen_and_a_near_tie_goes_to_the_k_quant() {
        let pick = pick(
            &[
                entry("Model-Q4_0.gguf", 17 * GB + 100),
                entry("Model-IQ4_NL.gguf", 17 * GB + 50),
                entry("Model-Q4_K_M.gguf", 17 * GB),
                entry("Model-Q2_K.gguf", 11 * GB),
                entry("Model-BF16.gguf", 50 * GB),
            ],
            Some(20 * GB),
        )
        .expect("a pick");
        assert_eq!(pick.candidate.label, "Q4_K_M");
        assert_eq!(pick.fits, Some(true));
    }

    #[test]
    fn outside_the_band_size_wins_over_family() {
        let pick = pick(
            &[
                entry("Model-Q4_K_M.gguf", 10 * GB),
                entry("Model-Q8_0.gguf", 18 * GB),
            ],
            Some(20 * GB),
        )
        .expect("a pick");
        assert_eq!(pick.candidate.label, "Q8_0");
    }

    #[test]
    fn when_nothing_fits_the_smallest_is_offered_and_marked() {
        let pick = pick(
            &[
                entry("Model-Q8_0.gguf", 40 * GB),
                entry("Model-Q4_K_M.gguf", 30 * GB),
            ],
            Some(20 * GB),
        )
        .expect("a pick");
        assert_eq!(pick.candidate.label, "Q4_K_M");
        assert_eq!(pick.fits, Some(false));
    }

    #[test]
    fn with_no_ceiling_the_convention_is_chosen_and_the_fit_is_not_claimed() {
        let pick = pick(
            &[
                entry("Model-BF16.gguf", 50 * GB),
                entry("Model-Q4_K_M.gguf", 17 * GB),
                entry("Model-Q2_K.gguf", 11 * GB),
            ],
            None,
        )
        .expect("a pick");
        assert_eq!(pick.candidate.label, "Q4_K_M");
        assert_eq!(
            pick.fits, None,
            "no llama-server means no ceiling, and installed memory is not one"
        );
    }

    #[test]
    fn a_repository_of_nothing_but_sidecars_yields_no_pick() {
        assert!(pick(
            &[entry("mmproj-BF16.gguf", 1), entry("imatrix.gguf", 2)],
            Some(GB)
        )
        .is_none());
    }

    #[test]
    fn a_sidecar_is_never_the_answer_when_nothing_fits() {
        // The failure this prevents: an imatrix is a few MB, so the smallest-first fallback
        // would reach for it before any real quantisation.
        let pick = pick(
            &[
                entry("imatrix.gguf", 1),
                entry("MTP/mtp-Model-Q4_0.gguf", 2),
                entry("Model-Q4_K_M.gguf", 30 * GB),
            ],
            Some(GB),
        )
        .expect("a pick");
        assert_eq!(pick.candidate.label, "Q4_K_M");
        assert_eq!(pick.fits, Some(false));
    }
}
