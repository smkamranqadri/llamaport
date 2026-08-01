use serde::{Deserialize, Serialize};

use crate::gguf::GgufMetadata;

/// Compute buffers and allocator slack, on top of weights and KV cache. Part of the
/// model's nominal footprint, which is a different quantity from its machine impact.
pub const DEFAULT_OVERHEAD_BYTES: u64 = 1_500_000_000;
const MIN_SAMPLES: usize = 3;
pub const MAX_SAMPLES: usize = 50;

/// Ratios outside this range are not believable and are more likely to be a machine
/// doing something else during the run than a real measurement.
const PLAUSIBLE_RESIDENCY: std::ops::RangeInclusive<f64> = 0.1..=2.0;

/// Bytes per cache element. Quantised types carry a scale per 32-element block.
pub fn bytes_per_element(cache_type: &str) -> f64 {
    match cache_type {
        "f32" => 4.0,
        "f16" | "bf16" => 2.0,
        "q8_0" => 34.0 / 32.0,
        "q5_1" => 24.0 / 32.0,
        "q5_0" => 22.0 / 32.0,
        "q4_1" => 20.0 / 32.0,
        "q4_0" | "iq4_nl" => 18.0 / 32.0,
        _ => 2.0,
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Estimate {
    pub weights_bytes: u64,
    pub kv_bytes: u64,
    pub overhead_bytes: u64,
    /// What the model nominally needs: weights + KV + overhead.
    pub total_bytes: u64,
    /// How much machine-wide used memory is expected to grow. Lower than nominal on
    /// Apple Silicon, where mmapped weights and Metal buffers are not all counted.
    pub machine_impact_bytes: u64,
    /// Fitted weights+KV residency, once enough runs have been observed.
    pub residency: Option<f64>,
    pub calibrated: bool,
}

/// K and V are summed separately rather than doubled: latent-attention architectures
/// such as deepseek2 size them differently.
pub fn kv_bytes(md: &GgufMetadata, ctx: u64, cache_k: &str, cache_v: &str) -> Option<u64> {
    let layers = md.block_count?;
    let kv_heads = md.head_count_kv?;
    let k_dim = md.head_dim()?;
    let v_dim = md.value_head_dim()?;

    let per_token =
        (k_dim as f64 * bytes_per_element(cache_k)) + (v_dim as f64 * bytes_per_element(cache_v));
    let total = layers as f64 * ctx as f64 * kv_heads as f64 * per_token;
    Some(total as u64)
}

pub fn estimate(
    md: &GgufMetadata,
    file_size: u64,
    ctx: u64,
    cache_k: &str,
    cache_v: &str,
    residency: Option<f64>,
) -> Option<Estimate> {
    let kv = kv_bytes(md, ctx, cache_k, cache_v)?;
    let total_bytes = file_size + kv + DEFAULT_OVERHEAD_BYTES;

    // Uncalibrated, the nominal figure is the conservative choice: on this platform it
    // over-predicts, and over-predicting is the safe direction.
    let machine_impact_bytes = match residency {
        Some(ratio) => (((file_size + kv) as f64) * ratio) as u64,
        None => total_bytes,
    };

    Some(Estimate {
        weights_bytes: file_size,
        kv_bytes: kv,
        overhead_bytes: DEFAULT_OVERHEAD_BYTES,
        total_bytes,
        machine_impact_bytes,
        residency,
        calibrated: residency.is_some(),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalibrationSample {
    pub model_id: String,
    pub ctx: u64,
    pub cache_type_k: String,
    pub cache_type_v: String,
    /// Weights + KV as predicted at launch time.
    pub predicted_base: u64,
    /// Peak machine-wide memory growth observed across the run. Per-process figures are
    /// useless here: with `-ngl all` on Apple Silicon the weights and KV cache sit in
    /// Metal buffers counted as wired memory, so the process reports a fraction of the
    /// real cost while RSS separately over-counts mmapped file pages.
    pub observed_total: u64,
}

/// Median ratio of observed machine growth to nominal weights+KV.
///
/// An additive overhead cannot work on Apple Silicon: measured on a 32 GB M2, loading a
/// 15.7 GB model with a 2.7 GB KV cache grew used memory by only ~10.6 GB, because
/// mmapped weight pages and Metal buffers are not all counted as used. The residual is
/// therefore reliably negative and an additive fit never accumulates a single usable
/// sample. A multiplicative residency absorbs the platform's accounting instead of
/// fighting it.
pub fn fit_residency(samples: &[CalibrationSample]) -> Option<f64> {
    let mut ratios: Vec<f64> = samples
        .iter()
        .filter(|s| s.predicted_base > 0 && s.observed_total > 0)
        .map(|s| s.observed_total as f64 / s.predicted_base as f64)
        .filter(|ratio| PLAUSIBLE_RESIDENCY.contains(ratio))
        .collect();

    if ratios.len() < MIN_SAMPLES {
        return None;
    }

    ratios.sort_by(|a, b| a.partial_cmp(b).expect("ratios are finite"));
    Some(ratios[ratios.len() / 2])
}

/// Bytes of KV cache per token of context, independent of how long the context is.
pub fn kv_bytes_per_token(md: &GgufMetadata, cache_k: &str, cache_v: &str) -> Option<f64> {
    let layers = md.block_count? as f64;
    let kv_heads = md.head_count_kv? as f64;
    let k_dim = md.head_dim()? as f64;
    let v_dim = md.value_head_dim()? as f64;

    Some(
        layers
            * kv_heads
            * ((k_dim * bytes_per_element(cache_k)) + (v_dim * bytes_per_element(cache_v))),
    )
}

/// The largest context whose predicted machine impact still fits in `budget_bytes`.
///
/// Deliberately reported as a range-finder rather than a limit: it depends on what else
/// is running right now, which is why the UI calls it an estimate for this machine and
/// not a property of the model.
pub fn context_within_budget(
    md: &GgufMetadata,
    file_size: u64,
    cache_k: &str,
    cache_v: &str,
    residency: Option<f64>,
    budget_bytes: i64,
    max_ctx: u64,
) -> Option<u64> {
    let per_token = kv_bytes_per_token(md, cache_k, cache_v)?;
    if per_token <= 0.0 {
        return None;
    }

    let scale = residency.unwrap_or(1.0);
    // Uncalibrated the nominal figure also carries a fixed overhead; calibrated, the
    // ratio absorbs it.
    let fixed = if residency.is_some() {
        file_size as f64 * scale
    } else {
        file_size as f64 + DEFAULT_OVERHEAD_BYTES as f64
    };

    let remaining = budget_bytes as f64 - fixed;
    if remaining <= 0.0 {
        return Some(0);
    }

    let tokens = (remaining / (per_token * scale)).floor();
    if tokens <= 0.0 {
        return Some(0);
    }
    Some((tokens as u64).min(max_ctx))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metadata(kv_heads: u64, k: u64, v: Option<u64>) -> GgufMetadata {
        GgufMetadata {
            gguf_version: 3,
            tensor_count: 0,
            architecture: "test".into(),
            name: None,
            size_label: None,
            context_length: Some(262144),
            block_count: Some(40),
            embedding_length: Some(4096),
            head_count: Some(32),
            head_count_kv: Some(kv_heads),
            key_length: Some(k),
            value_length: v,
            expert_count: None,
            file_type: None,
            has_chat_template: true,
        }
    }

    #[test]
    fn kv_scales_with_context() {
        let md = metadata(2, 128, None);
        let small = kv_bytes(&md, 8192, "q8_0", "q8_0").unwrap();
        let large = kv_bytes(&md, 16384, "q8_0", "q8_0").unwrap();
        assert_eq!(large, small * 2);
    }

    #[test]
    fn quantised_cache_is_smaller_than_f16() {
        let md = metadata(8, 128, None);
        let q8 = kv_bytes(&md, 65536, "q8_0", "q8_0").unwrap();
        let f16 = kv_bytes(&md, 65536, "f16", "f16").unwrap();
        assert!(q8 < f16);
        assert!((f16 as f64 / q8 as f64 - 1.882).abs() < 0.01);
    }

    #[test]
    fn asymmetric_key_and_value_widths_are_not_doubled() {
        let symmetric = kv_bytes(&metadata(1, 576, Some(576)), 4096, "f16", "f16").unwrap();
        let asymmetric = kv_bytes(&metadata(1, 576, Some(512)), 4096, "f16", "f16").unwrap();
        assert!(asymmetric < symmetric);

        let layers = 40u64;
        let ctx = 4096u64;
        let kv_heads = 1u64;
        let expected = layers * ctx * kv_heads * (576 * 2 + 512 * 2);
        assert_eq!(asymmetric, expected);
    }

    #[test]
    fn value_width_falls_back_to_key_width() {
        let md = metadata(4, 128, None);
        assert_eq!(md.value_head_dim(), Some(128));
    }

    #[test]
    fn estimate_sums_the_parts() {
        let md = metadata(8, 128, None);
        let e = estimate(&md, 20_000_000_000, 65536, "q8_0", "q8_0", None).unwrap();

        assert_eq!(e.weights_bytes, 20_000_000_000);
        assert_eq!(e.overhead_bytes, DEFAULT_OVERHEAD_BYTES);
        assert_eq!(
            e.total_bytes,
            e.weights_bytes + e.kv_bytes + e.overhead_bytes
        );
        assert!(!e.calibrated);
    }

    fn sample(predicted: u64, observed: u64) -> CalibrationSample {
        CalibrationSample {
            model_id: "m".into(),
            ctx: 65536,
            cache_type_k: "q8_0".into(),
            cache_type_v: "q8_0".into(),
            predicted_base: predicted,
            observed_total: observed,
        }
    }

    #[test]
    fn a_bigger_budget_allows_more_context() {
        let md = metadata(8, 128, None);
        let small = context_within_budget(
            &md,
            15_000_000_000,
            "q8_0",
            "q8_0",
            Some(0.6),
            20_000_000_000,
            262144,
        );
        let large = context_within_budget(
            &md,
            15_000_000_000,
            "q8_0",
            "q8_0",
            Some(0.6),
            28_000_000_000,
            262144,
        );
        assert!(large > small, "{large:?} should exceed {small:?}");
    }

    #[test]
    fn a_budget_below_the_weights_allows_no_context() {
        let md = metadata(8, 128, None);
        let found = context_within_budget(
            &md,
            20_000_000_000,
            "q8_0",
            "q8_0",
            Some(1.0),
            5_000_000_000,
            262144,
        );
        assert_eq!(found, Some(0), "weights alone do not fit");
    }

    #[test]
    fn the_model_maximum_is_never_exceeded() {
        let md = metadata(8, 128, None);
        let found = context_within_budget(
            &md,
            1_000_000,
            "q8_0",
            "q8_0",
            Some(0.5),
            900_000_000_000,
            32768,
        );
        assert_eq!(
            found,
            Some(32768),
            "hardware headroom cannot raise a model's ceiling"
        );
    }

    #[test]
    fn a_quantised_cache_buys_more_context_than_f16() {
        let md = metadata(8, 128, None);
        let budget = 25_000_000_000;
        let q8 = context_within_budget(
            &md,
            15_000_000_000,
            "q8_0",
            "q8_0",
            Some(0.6),
            budget,
            262144,
        );
        let f16 =
            context_within_budget(&md, 15_000_000_000, "f16", "f16", Some(0.6), budget, 262144);
        assert!(q8 > f16, "q8_0 {q8:?} should allow more than f16 {f16:?}");
    }

    #[test]
    fn per_token_cost_is_independent_of_context_length() {
        let md = metadata(2, 128, None);
        let per_token = kv_bytes_per_token(&md, "q8_0", "q8_0").expect("per token");
        let at_8k = kv_bytes(&md, 8192, "q8_0", "q8_0").expect("kv") as f64;
        assert!((per_token * 8192.0 - at_8k).abs() < 1.0);
    }

    #[test]
    fn calibration_needs_three_usable_samples() {
        assert_eq!(fit_residency(&[]), None);
        assert_eq!(fit_residency(&[sample(1000, 600), sample(1000, 700)]), None);
    }

    #[test]
    fn calibration_takes_the_median_ratio() {
        let samples = [sample(1000, 500), sample(1000, 700), sample(1000, 600)];
        assert_eq!(fit_residency(&samples), Some(0.6));
    }

    /// The case that made an additive fit useless: every observed run grows the machine
    /// by less than weights+KV, so every residual is negative. A ratio still fits.
    #[test]
    fn runs_that_grow_less_than_nominal_still_calibrate() {
        let samples = [
            sample(18_400, 10_600),
            sample(18_400, 10_400),
            sample(18_400, 10_800),
        ];
        let residency = fit_residency(&samples).expect("should calibrate");
        assert!((residency - 0.576).abs() < 0.01, "got {residency}");
    }

    #[test]
    fn implausible_ratios_are_discarded() {
        let samples = [
            sample(1000, 10),   // 0.01 — the machine was evicting, not measuring
            sample(1000, 5000), // 5.0  — something else was loading at the same time
            sample(1000, 600),
            sample(1000, 620),
            sample(1000, 640),
        ];
        let residency = fit_residency(&samples).expect("should calibrate");
        assert!((residency - 0.62).abs() < 0.001, "got {residency}");
    }
}
