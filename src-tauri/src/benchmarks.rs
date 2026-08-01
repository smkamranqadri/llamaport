//! Benchmark history.
//!
//! Stored in `benchmarks.json` beside the config rather than inside it: history grows
//! without bound and every profile edit would otherwise rewrite it, putting settings at
//! risk for the sake of a log. Same technology, no new dependency, separate blast radius.
//!
//! The point of the whole feature is a like-for-like comparison of quantisations on one
//! machine, so every row records the settings that produced it, not just the result.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::health::Verdict;
use crate::store;

const CURRENT_SCHEMA: u32 = 1;
/// History is a log, not a database. Old rows fall off rather than growing forever.
const MAX_RECORDS: usize = 500;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BenchmarkRecord {
    pub id: String,
    pub timestamp_secs: u64,

    pub model_file: String,
    pub model_size_bytes: u64,
    pub architecture: Option<String>,
    pub quantisation: Option<String>,

    pub profile_name: Option<String>,
    pub ctx: u64,
    pub cache_type_k: String,
    pub cache_type_v: String,
    pub ngl: String,
    pub parallel: u32,
    pub llama_version: Option<String>,

    pub time_to_first_token_ms: Option<u64>,
    pub prompt_tokens: Option<u64>,
    pub prompt_tps: Option<f64>,
    pub generated_tokens: Option<u64>,
    pub gen_tps: Option<f64>,

    pub peak_process_bytes: Option<u64>,
    pub peak_swap_bytes: Option<u64>,

    pub test_duration_ms: u64,
    pub verdict: Verdict,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Benchmarks {
    pub schema_version: u32,
    pub records: Vec<BenchmarkRecord>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Sort {
    #[default]
    Timestamp,
    GenTps,
    PromptTps,
    TimeToFirstToken,
    PeakMemory,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Query {
    pub model_file: Option<String>,
    pub quantisation: Option<String>,
    pub sort: Sort,
    pub descending: bool,
}

pub fn path() -> PathBuf {
    store::config_dir().join("benchmarks.json")
}

pub fn load_from(path: &Path) -> Benchmarks {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return Benchmarks {
            schema_version: CURRENT_SCHEMA,
            records: Vec::new(),
        };
    };
    let mut parsed: Benchmarks = serde_json::from_str(&raw).unwrap_or_default();
    parsed.schema_version = CURRENT_SCHEMA;
    parsed
}

pub fn save_to(path: &Path, benchmarks: &Benchmarks) -> std::io::Result<()> {
    store::write_atomic(path, &serde_json::to_string_pretty(benchmarks)?)
}

pub fn add(benchmarks: &mut Benchmarks, record: BenchmarkRecord) {
    benchmarks.records.push(record);
    if benchmarks.records.len() > MAX_RECORDS {
        let excess = benchmarks.records.len() - MAX_RECORDS;
        benchmarks.records.drain(0..excess);
    }
}

pub fn delete(benchmarks: &mut Benchmarks, id: &str) -> bool {
    let before = benchmarks.records.len();
    benchmarks.records.retain(|record| record.id != id);
    benchmarks.records.len() != before
}

pub fn set_note(benchmarks: &mut Benchmarks, id: &str, note: Option<String>) -> bool {
    match benchmarks.records.iter_mut().find(|record| record.id == id) {
        Some(record) => {
            record.note = note.filter(|text| !text.trim().is_empty());
            true
        }
        None => false,
    }
}

/// Filters then sorts. Rows missing the sort key sink to the bottom regardless of
/// direction — a missing measurement is not a fast one or a slow one.
pub fn query(records: &[BenchmarkRecord], query: &Query) -> Vec<BenchmarkRecord> {
    let mut out: Vec<BenchmarkRecord> = records
        .iter()
        .filter(|record| {
            query
                .model_file
                .as_ref()
                .is_none_or(|wanted| &record.model_file == wanted)
        })
        .filter(|record| {
            query
                .quantisation
                .as_ref()
                .is_none_or(|wanted| record.quantisation.as_ref() == Some(wanted))
        })
        .cloned()
        .collect();

    let key = |record: &BenchmarkRecord| -> Option<f64> {
        match query.sort {
            Sort::Timestamp => Some(record.timestamp_secs as f64),
            Sort::GenTps => record.gen_tps,
            Sort::PromptTps => record.prompt_tps,
            Sort::TimeToFirstToken => record.time_to_first_token_ms.map(|v| v as f64),
            Sort::PeakMemory => record.peak_process_bytes.map(|v| v as f64),
        }
    };

    out.sort_by(|a, b| match (key(a), key(b)) {
        (Some(a), Some(b)) => {
            let ordering = a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal);
            if query.descending {
                ordering.reverse()
            } else {
                ordering
            }
        }
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });
    out
}

const CSV_COLUMNS: [&str; 21] = [
    "timestamp",
    "model_file",
    "model_size_bytes",
    "architecture",
    "quantisation",
    "profile_name",
    "ctx",
    "cache_type_k",
    "cache_type_v",
    "ngl",
    "parallel",
    "llama_version",
    "time_to_first_token_ms",
    "prompt_tokens",
    "prompt_tps",
    "generated_tokens",
    "gen_tps",
    "peak_process_bytes",
    "peak_swap_bytes",
    "test_duration_ms",
    "verdict",
];

fn escape(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

fn optional<T: ToString>(value: &Option<T>) -> String {
    value.as_ref().map(ToString::to_string).unwrap_or_default()
}

pub fn to_csv(records: &[BenchmarkRecord]) -> String {
    let mut out = String::new();
    out.push_str(&CSV_COLUMNS.join(","));
    out.push('\n');

    for record in records {
        let verdict = serde_json::to_value(record.verdict)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_default();

        let fields = [
            record.timestamp_secs.to_string(),
            record.model_file.clone(),
            record.model_size_bytes.to_string(),
            optional(&record.architecture),
            optional(&record.quantisation),
            optional(&record.profile_name),
            record.ctx.to_string(),
            record.cache_type_k.clone(),
            record.cache_type_v.clone(),
            record.ngl.clone(),
            record.parallel.to_string(),
            optional(&record.llama_version),
            optional(&record.time_to_first_token_ms),
            optional(&record.prompt_tokens),
            optional(&record.prompt_tps),
            optional(&record.generated_tokens),
            optional(&record.gen_tps),
            optional(&record.peak_process_bytes),
            optional(&record.peak_swap_bytes),
            record.test_duration_ms.to_string(),
            verdict,
        ];

        out.push_str(
            &fields
                .iter()
                .map(|field| escape(field))
                .collect::<Vec<_>>()
                .join(","),
        );
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(id: &str, quant: &str, gen_tps: Option<f64>) -> BenchmarkRecord {
        BenchmarkRecord {
            id: id.into(),
            timestamp_secs: 1_000,
            model_file: "Qwen3.6-35B-A3B-UD-Q3_K_XL.gguf".into(),
            model_size_bytes: 16_000_000_000,
            architecture: Some("qwen35moe".into()),
            quantisation: Some(quant.into()),
            profile_name: Some("Balanced".into()),
            ctx: 65536,
            cache_type_k: "q8_0".into(),
            cache_type_v: "q8_0".into(),
            ngl: "all".into(),
            parallel: 1,
            llama_version: Some("10090 (7347430f4)".into()),
            time_to_first_token_ms: Some(300),
            prompt_tokens: Some(12),
            prompt_tps: Some(300.0),
            generated_tokens: Some(16),
            gen_tps,
            peak_process_bytes: Some(1_300_000_000),
            peak_swap_bytes: Some(2_600_000_000),
            test_duration_ms: 1_500,
            verdict: Verdict::Passed,
            note: None,
        }
    }

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("llama-hub-bench-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch");
        dir.join("benchmarks.json")
    }

    #[test]
    fn records_round_trip_through_disk() {
        let path = scratch("roundtrip");
        let mut benchmarks = Benchmarks::default();
        add(&mut benchmarks, record("a", "UD-Q3_K_XL", Some(15.5)));

        save_to(&path, &benchmarks).expect("save");
        let loaded = load_from(&path);

        assert_eq!(loaded.records.len(), 1);
        assert_eq!(loaded.records[0].gen_tps, Some(15.5));
        assert_eq!(
            loaded.records[0].llama_version.as_deref(),
            Some("10090 (7347430f4)")
        );
        assert_eq!(loaded.schema_version, CURRENT_SCHEMA);
    }

    #[test]
    fn a_missing_file_is_an_empty_history_not_an_error() {
        let loaded = load_from(Path::new("/nonexistent/benchmarks.json"));
        assert!(loaded.records.is_empty());
    }

    #[test]
    fn history_is_capped_and_drops_the_oldest() {
        let mut benchmarks = Benchmarks::default();
        for n in 0..(MAX_RECORDS + 10) {
            add(&mut benchmarks, record(&n.to_string(), "Q4_K_M", Some(1.0)));
        }
        assert_eq!(benchmarks.records.len(), MAX_RECORDS);
        assert_eq!(benchmarks.records[0].id, "10", "oldest rows fall off first");
    }

    #[test]
    fn deleting_reports_whether_anything_was_removed() {
        let mut benchmarks = Benchmarks::default();
        add(&mut benchmarks, record("a", "Q4_K_M", Some(1.0)));

        assert!(delete(&mut benchmarks, "a"));
        assert!(benchmarks.records.is_empty());
        assert!(
            !delete(&mut benchmarks, "a"),
            "deleting twice is not a success"
        );
    }

    #[test]
    fn a_note_can_be_set_and_cleared() {
        let mut benchmarks = Benchmarks::default();
        add(&mut benchmarks, record("a", "Q4_K_M", Some(1.0)));

        assert!(set_note(&mut benchmarks, "a", Some("felt slow".into())));
        assert_eq!(benchmarks.records[0].note.as_deref(), Some("felt slow"));

        set_note(&mut benchmarks, "a", Some("   ".into()));
        assert_eq!(
            benchmarks.records[0].note, None,
            "blank clears rather than stores"
        );
        assert!(!set_note(&mut benchmarks, "missing", None));
    }

    #[test]
    fn filtering_by_quantisation_is_what_makes_q3_vs_q4_comparable() {
        let records = vec![
            record("a", "UD-Q3_K_XL", Some(15.5)),
            record("b", "UD-Q4_K_M", Some(12.2)),
            record("c", "UD-Q3_K_XL", Some(16.0)),
        ];

        let found = query(
            &records,
            &Query {
                quantisation: Some("UD-Q3_K_XL".into()),
                ..Default::default()
            },
        );
        assert_eq!(found.len(), 2);
        assert!(found
            .iter()
            .all(|r| r.quantisation.as_deref() == Some("UD-Q3_K_XL")));
    }

    #[test]
    fn sorting_by_generation_speed_orders_both_ways() {
        let records = vec![
            record("slow", "Q4_K_M", Some(12.2)),
            record("fast", "Q3_K_XL", Some(16.0)),
        ];

        let ascending = query(
            &records,
            &Query {
                sort: Sort::GenTps,
                ..Default::default()
            },
        );
        assert_eq!(ascending[0].id, "slow");

        let descending = query(
            &records,
            &Query {
                sort: Sort::GenTps,
                descending: true,
                ..Default::default()
            },
        );
        assert_eq!(descending[0].id, "fast");
    }

    #[test]
    fn rows_without_the_sort_key_sink_regardless_of_direction() {
        let records = vec![
            record("measured", "Q4_K_M", Some(12.2)),
            record("unmeasured", "Q4_K_M", None),
        ];

        for descending in [false, true] {
            let sorted = query(
                &records,
                &Query {
                    sort: Sort::GenTps,
                    descending,
                    ..Default::default()
                },
            );
            assert_eq!(
                sorted.last().expect("rows").id,
                "unmeasured",
                "a missing measurement is neither fast nor slow (descending={descending})"
            );
        }
    }

    #[test]
    fn csv_has_a_header_and_one_line_per_record() {
        let csv = to_csv(&[record("a", "Q4_K_M", Some(15.5))]);
        let lines: Vec<&str> = csv.trim_end().split('\n').collect();

        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with("timestamp,model_file"));
        assert!(lines[1].contains("Qwen3.6-35B-A3B-UD-Q3_K_XL.gguf"));
        assert!(lines[1].contains("15.5"));
        assert!(lines[1].ends_with("passed"));
    }

    #[test]
    fn csv_escapes_separators_and_quotes_in_free_text() {
        let mut row = record("a", "Q4_K_M", Some(1.0));
        row.model_file = "weird,name \"quoted\".gguf".into();

        let csv = to_csv(&[row]);
        assert!(csv.contains("\"weird,name \"\"quoted\"\".gguf\""), "{csv}");
        assert_eq!(
            csv.trim_end().split('\n').count(),
            2,
            "no stray line breaks"
        );
    }

    #[test]
    fn missing_measurements_become_empty_csv_cells_not_zeroes() {
        let mut row = record("a", "Q4_K_M", None);
        row.time_to_first_token_ms = None;
        row.peak_swap_bytes = None;

        let csv = to_csv(&[row]);
        let data = csv.trim_end().split('\n').nth(1).expect("row");
        assert!(
            data.contains(",,"),
            "unmeasured fields must be blank: {data}"
        );
        assert!(!data.contains(",0,"), "a blank must not become a zero");
    }
}
