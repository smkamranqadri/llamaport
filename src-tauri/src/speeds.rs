//! What a model actually did, at the settings it did it under.
//!
//! A memory sum says a launch is allowed and never that it is good, so the app cannot
//! choose settings by arithmetic. This is where the evidence for choosing them is kept.

use serde::{Deserialize, Serialize};

use crate::profile::Profile;

/// Everything about a launch that can move the number. Alias, host and port are excluded
/// deliberately: they identify a server, not its speed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeedKey {
    pub model_id: String,
    pub ctx: u64,
    pub ngl: String,
    pub cache_type_k: String,
    pub cache_type_v: String,
    pub flash_attn: bool,
    pub parallel: u32,
    pub raw_args: Vec<String>,
}

impl SpeedKey {
    pub fn of(model_id: &str, profile: &Profile) -> Self {
        Self {
            model_id: model_id.to_string(),
            ctx: profile.ctx,
            ngl: profile.ngl.clone(),
            cache_type_k: profile.cache_type_k.clone(),
            cache_type_v: profile.cache_type_v.clone(),
            flash_attn: profile.flash_attn,
            parallel: profile.parallel,
            raw_args: profile.raw_args.clone(),
        }
    }
}

/// How the reading was obtained. Two rows from ordinary use did different work and can
/// only be compared with that in mind; two from a Tune run did identical work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Source {
    Observed,
    Measured,
}

/// One run's totals, not one poll's snapshot: an average over everything the run did.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeedRecord {
    pub timestamp_secs: u64,
    pub key: SpeedKey,
    /// The build that produced it, where it was known. Ranking is within one build: the
    /// binary is as capable of moving these numbers as any setting is.
    pub llama_version: Option<String>,
    pub source: Source,
    pub prompt_tokens: f64,
    pub prompt_seconds: f64,
    pub gen_tokens: f64,
    pub gen_seconds: f64,
}

impl SpeedRecord {
    pub fn prompt_tps(&self) -> Option<f64> {
        rate(self.prompt_tokens, self.prompt_seconds)
    }

    pub fn gen_tps(&self) -> Option<f64> {
        rate(self.gen_tokens, self.gen_seconds)
    }

    /// The config directory holds untrusted input, and every figure here is read back
    /// from it. A negative or non-finite one would not merely display wrongly; it would
    /// win a ranking.
    pub fn is_sound(&self) -> bool {
        [
            self.prompt_tokens,
            self.prompt_seconds,
            self.gen_tokens,
            self.gen_seconds,
        ]
        .iter()
        .all(|n| n.is_finite() && *n >= 0.0)
    }
}

fn rate(tokens: f64, seconds: f64) -> Option<f64> {
    if seconds <= 0.0 || !seconds.is_finite() || !tokens.is_finite() {
        return None;
    }
    Some(tokens / seconds)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> Profile {
        Profile {
            ctx: 65536,
            ngl: "all".into(),
            cache_type_k: "f16".into(),
            cache_type_v: "f16".into(),
            flash_attn: true,
            parallel: 1,
            ..Default::default()
        }
    }

    fn record() -> SpeedRecord {
        SpeedRecord {
            timestamp_secs: 1,
            key: SpeedKey::of("model", &profile()),
            llama_version: Some("b10360".into()),
            source: Source::Observed,
            prompt_tokens: 3794.0,
            prompt_seconds: 7.47,
            gen_tokens: 64.0,
            gen_seconds: 1.54,
        }
    }

    #[test]
    fn the_key_ignores_what_cannot_move_the_number() {
        let mut elsewhere = profile();
        elsewhere.alias = "another".into();
        elsewhere.host = "127.0.0.2".into();
        elsewhere.port = 9999;

        assert_eq!(
            SpeedKey::of("model", &profile()),
            SpeedKey::of("model", &elsewhere),
            "a port is where a server answers, not how fast it answers"
        );
    }

    #[test]
    fn the_key_separates_settings_that_can() {
        let mut wider = profile();
        wider.ctx = 262_144;
        assert_ne!(
            SpeedKey::of("model", &profile()),
            SpeedKey::of("model", &wider)
        );

        let mut quantised = profile();
        quantised.cache_type_k = "q8_0".into();
        assert_ne!(
            SpeedKey::of("model", &profile()),
            SpeedKey::of("model", &quantised)
        );

        let mut elsewhere = profile();
        elsewhere.raw_args = vec!["--threads".into(), "4".into()];
        assert_ne!(
            SpeedKey::of("model", &profile()),
            SpeedKey::of("model", &elsewhere)
        );
    }

    #[test]
    fn a_rate_needs_time_to_have_passed() {
        let mut instant = record();
        instant.gen_seconds = 0.0;
        assert_eq!(
            instant.gen_tps(),
            None,
            "dividing by no time at all is infinite tokens per second"
        );

        assert!(record()
            .gen_tps()
            .is_some_and(|tps| (tps - 41.5).abs() < 0.5));
    }

    #[test]
    fn figures_that_cannot_have_happened_are_not_sound() {
        assert!(record().is_sound());

        let mut backwards = record();
        backwards.gen_seconds = -1.0;
        assert!(
            !backwards.is_sound(),
            "a negative second wins every ranking"
        );

        let mut impossible = record();
        impossible.prompt_tokens = f64::INFINITY;
        assert!(!impossible.is_sound());
    }
}

/// Floors under what counts as a reading worth ranking.
///
/// Not measured thresholds — chosen from the one case this project has: a 36-token
/// prompt put `q8_0` within 2% of `f16`, and a 3,794-token prompt put it 27% behind. A
/// run that generated a handful of tokens after a six-token prompt measured startup.
pub const MIN_PROMPT_TOKENS: f64 = 256.0;
pub const MIN_GEN_TOKENS: f64 = 64.0;

/// How close two readings have to be before calling one faster is noise.
///
/// Read off the disagreement rather than picked: across three ladders on one machine the
/// same two full-precision rungs came out 65,536 ahead by 2.8%, 8,192 ahead by 6.3%, and
/// level at 0.2%. Any threshold below 6.3% crowns whichever ran last. Ten per cent is the
/// round number above that, and it is still far inside the 20-27% those two both beat the
/// quantised full context by — which is the gap that has never once changed sign.
pub const NOISE: f64 = 0.10;

impl SpeedRecord {
    /// Whether the run did enough work for its rate to mean anything.
    pub fn worth_ranking(&self) -> bool {
        self.is_sound()
            && self.prompt_tokens >= MIN_PROMPT_TOKENS
            && self.gen_tokens >= MIN_GEN_TOKENS
            && self.gen_tps().is_some()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Confidence {
    /// Nothing has done enough work to be worth ranking.
    NeverMeasured,
    /// Ordinary use only. Those runs did different work, so the ordering can be wrong.
    Observed,
    /// A ladder, where every candidate answered the same question.
    Tuned,
}

/// One setting, and the best it has been seen to do.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeedRow {
    pub key: SpeedKey,
    pub source: Source,
    pub gen_tps: Option<f64>,
    pub prompt_tps: Option<f64>,
    /// The workload behind the figure, shown beside it: a rate is only comparable to
    /// another rate that did the same amount of work.
    pub prompt_tokens: f64,
    pub gen_tokens: f64,
    pub timestamp_secs: u64,
    pub llama_version: Option<String>,
    /// Measured on a different build from the one installed now, so not ranked against
    /// rows from this one. The binary moves these numbers as much as any setting does.
    pub stale: bool,
    pub ranked: bool,
    pub runs: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub rows: Vec<SpeedRow>,
    pub confidence: Confidence,
    /// The settings to use, where anything has earned the right to say so.
    pub suggestion: Option<SpeedKey>,
    pub suggested_tps: Option<f64>,
    /// Settings measured within `NOISE` of the suggestion, including it. More than one
    /// means the suggestion is the widest context among equals rather than a winner.
    pub tied: usize,
    /// What the suggestion beats, and by how much, where that gap is outside noise.
    pub beats: Option<SpeedKey>,
    pub beats_by_percent: Option<f64>,
}

/// The best each setting has been seen to do, ranked where the runs earned it.
///
/// Rows are grouped by settings and build, and the fastest of each group kept: a slow run
/// says the machine was busy, not that the setting is bad, and the question here is what a
/// setting can do.
pub fn summarise(records: &[SpeedRecord], build: Option<&str>) -> Summary {
    let mut grouped: Vec<SpeedRow> = Vec::new();
    for record in records {
        if !record.is_sound() {
            continue;
        }
        let stale = match (build, record.llama_version.as_deref()) {
            (Some(current), Some(theirs)) => current != theirs,
            _ => false,
        };
        let candidate = SpeedRow {
            key: record.key.clone(),
            source: record.source,
            gen_tps: record.gen_tps(),
            prompt_tps: record.prompt_tps(),
            prompt_tokens: record.prompt_tokens,
            gen_tokens: record.gen_tokens,
            timestamp_secs: record.timestamp_secs,
            llama_version: record.llama_version.clone(),
            stale,
            ranked: record.worth_ranking() && !stale,
            runs: 1,
        };

        // Grouped by settings *and* build: the same context on a different binary is a
        // different reading, and merging them hides the older one behind a number it did
        // not produce.
        match grouped
            .iter_mut()
            .find(|row| row.key == candidate.key && row.llama_version == candidate.llama_version)
        {
            Some(existing) => {
                existing.runs += 1;
                let better = candidate.ranked && !existing.ranked
                    || candidate.ranked == existing.ranked
                        && candidate.gen_tps.unwrap_or(0.0) > existing.gen_tps.unwrap_or(0.0);
                if better {
                    let runs = existing.runs;
                    *existing = candidate;
                    existing.runs = runs;
                }
            }
            None => grouped.push(candidate),
        }
    }

    grouped.sort_by(|a, b| {
        b.ranked.cmp(&a.ranked).then(
            b.gen_tps
                .unwrap_or(0.0)
                .total_cmp(&a.gen_tps.unwrap_or(0.0)),
        )
    });

    let ranked: Vec<&SpeedRow> = grouped.iter().filter(|row| row.ranked).collect();
    let Some(fastest) = ranked.first() else {
        return Summary {
            rows: grouped,
            confidence: Confidence::NeverMeasured,
            suggestion: None,
            suggested_tps: None,
            tied: 0,
            beats: None,
            beats_by_percent: None,
        };
    };

    let best = fastest.gen_tps.unwrap_or(0.0);
    // Everything a measurement cannot tell apart. Among those the widest context wins:
    // context costs nothing when the speed is the same, and picking by a 3% difference
    // produces a different answer every run.
    let tied: Vec<&&SpeedRow> = ranked
        .iter()
        .filter(|row| row.gen_tps.unwrap_or(0.0) >= best * (1.0 - NOISE))
        .collect();
    let chosen = tied
        .iter()
        .max_by_key(|row| row.key.ctx)
        .expect("the fastest is tied with itself");

    let outside_noise = ranked
        .iter()
        .find(|row| row.gen_tps.unwrap_or(0.0) < best * (1.0 - NOISE));

    let compared: Vec<&&SpeedRow> = ranked
        .iter()
        .filter(|row| row.source == Source::Measured)
        .collect();
    let mut confidence = Confidence::Observed;
    if compared.len() >= 2 && chosen.source == Source::Measured {
        confidence = Confidence::Tuned;
    }

    Summary {
        rows: grouped.clone(),
        confidence,
        suggestion: Some(chosen.key.clone()),
        suggested_tps: chosen.gen_tps,
        tied: tied.len(),
        beats: outside_noise.map(|row| row.key.clone()),
        beats_by_percent: outside_noise.and_then(|row| {
            let slower = row.gen_tps?;
            if slower <= 0.0 {
                return None;
            }
            Some((chosen.gen_tps? / slower - 1.0) * 100.0)
        }),
    }
}

#[cfg(test)]
mod summary_tests {
    use super::*;

    fn at(ctx: u64, cache: &str) -> SpeedKey {
        SpeedKey::of(
            "ornith",
            &Profile {
                ctx,
                ngl: "all".into(),
                cache_type_k: cache.into(),
                cache_type_v: cache.into(),
                flash_attn: true,
                parallel: 1,
                ..Default::default()
            },
        )
    }

    /// Generation tokens per second, expressed the way a run reports it.
    fn run(ctx: u64, cache: &str, source: Source, gen_seconds: f64) -> SpeedRecord {
        SpeedRecord {
            timestamp_secs: ctx,
            key: at(ctx, cache),
            llama_version: Some("10360 (48d22e295)".into()),
            source,
            prompt_tokens: 3812.0,
            prompt_seconds: 8.1,
            gen_tokens: 64.0,
            gen_seconds,
        }
    }

    /// The ladder as it actually came out through the app on 2026-08-31.
    fn ladder() -> Vec<SpeedRecord> {
        vec![
            run(262_144, "q8_0", Source::Measured, 2.129934),
            run(8192, "f16", Source::Measured, 1.644783),
            run(65_536, "f16", Source::Measured, 1.747966),
        ]
    }

    fn build() -> Option<&'static str> {
        Some("10360 (48d22e295)")
    }

    /// The finding this rule exists for: those two rungs swapped places between runs, so
    /// the widest of them is taken rather than the fastest of them.
    #[test]
    fn the_widest_of_two_readings_measurement_cannot_tell_apart_is_suggested() {
        let summary = summarise(&ladder(), build());

        assert_eq!(summary.tied, 2);
        assert_eq!(
            summary.suggestion.expect("a suggestion").ctx,
            65_536,
            "8,192 was the faster of the two and the difference was noise; context is \
             free when the speed is the same"
        );
    }

    #[test]
    fn the_gap_that_is_real_is_the_one_reported() {
        let summary = summarise(&ladder(), build());

        assert_eq!(summary.beats.expect("something slower").ctx, 262_144);
        let by = summary.beats_by_percent.expect("a margin");
        assert!(
            (20.0..30.0).contains(&by),
            "the quantised full context has never once been within noise: {by}"
        );
        assert_eq!(summary.confidence, Confidence::Tuned);
    }

    /// The row the real server actually wrote on a six-token prompt.
    #[test]
    fn a_warm_up_is_kept_and_never_ranked() {
        let mut warm_up = run(8192, "f16", Source::Observed, 0.323);
        warm_up.prompt_tokens = 6.0;
        warm_up.gen_tokens = 48.0;

        let summary = summarise(&[warm_up], build());
        assert_eq!(summary.rows.len(), 1, "it is history either way");
        assert!(!summary.rows[0].ranked);
        assert_eq!(summary.confidence, Confidence::NeverMeasured);
        assert!(summary.suggestion.is_none());
    }

    #[test]
    fn ordinary_use_can_suggest_but_not_claim_to_have_compared() {
        let summary = summarise(
            &[
                run(65_536, "f16", Source::Observed, 1.7),
                run(262_144, "q8_0", Source::Observed, 2.2),
            ],
            build(),
        );

        assert_eq!(summary.suggestion.expect("a suggestion").ctx, 65_536);
        assert_eq!(
            summary.confidence,
            Confidence::Observed,
            "those two runs did different work, whatever the ordering says"
        );
    }

    /// The binary moves these numbers as much as any setting does.
    #[test]
    fn a_reading_from_another_build_is_shown_and_not_ranked() {
        let mut older = run(8192, "f16", Source::Measured, 0.5);
        older.llama_version = Some("10090 (7347430f4)".into());

        let mut records = ladder();
        records.push(older);
        let summary = summarise(&records, build());

        let stale = summary
            .rows
            .iter()
            .find(|row| row.stale)
            .expect("the older build's row");
        assert!(!stale.ranked);
        assert_eq!(
            summary.suggestion.expect("a suggestion").ctx,
            65_536,
            "a fast reading from a build that is gone must not win"
        );
    }

    #[test]
    fn a_setting_is_represented_by_the_best_it_has_done() {
        let summary = summarise(
            &[
                run(65_536, "f16", Source::Observed, 3.0),
                run(65_536, "f16", Source::Observed, 1.6),
            ],
            build(),
        );

        assert_eq!(summary.rows.len(), 1, "one setting, one row");
        assert_eq!(summary.rows[0].runs, 2);
        assert!(
            summary.rows[0].gen_tps.expect("a rate") > 39.0,
            "a slow run says the machine was busy, not that the setting is bad"
        );
    }

    #[test]
    fn nothing_measured_is_not_an_opinion() {
        let summary = summarise(&[], build());
        assert_eq!(summary.confidence, Confidence::NeverMeasured);
        assert!(summary.suggestion.is_none());
        assert!(summary.rows.is_empty());
    }
}
