use serde::Serialize;

use crate::gguf::GgufMetadata;

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

/// What a launch will allocate, computed from the file's own header.
///
/// Deliberately arithmetic and nothing more. An earlier version predicted total machine
/// impact by fitting a ratio from observed runs; four samples of the same model at the
/// same context produced ratios from 0.42 to 0.85, because how much of a model becomes
/// resident depends on what else is running, not on the model. A forecast that wrong is
/// worse than no forecast — these two numbers are exact.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Estimate {
    pub weights_bytes: u64,
    pub kv_bytes: u64,
    pub total_bytes: u64,
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
) -> Option<Estimate> {
    let kv = kv_bytes(md, ctx, cache_k, cache_v)?;
    Some(Estimate {
        weights_bytes: file_size,
        kv_bytes: kv,
        total_bytes: file_size + kv,
    })
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
    fn an_estimate_is_weights_plus_cache_and_nothing_invented() {
        let md = metadata(8, 128, None);
        let e = estimate(&md, 20_000_000_000, 65536, "q8_0", "q8_0").expect("estimate");
        assert_eq!(e.weights_bytes, 20_000_000_000);
        assert_eq!(e.total_bytes, e.weights_bytes + e.kv_bytes);
    }

    #[test]
    fn value_width_falls_back_to_key_width() {
        let md = metadata(4, 128, None);
        assert_eq!(md.value_head_dim(), Some(128));
    }
}
