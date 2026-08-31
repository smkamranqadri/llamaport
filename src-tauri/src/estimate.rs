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
/// worse than no forecast.
///
/// The weights figure is the file. The cache figure is the arithmetic below, which is
/// right about what it models — it does not claim to equal what the server allocates,
/// which rounds to whole cells and pads them.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Estimate {
    pub weights_bytes: u64,
    pub kv_bytes: u64,
    pub total_bytes: u64,
    /// True where a term was left out, so both figures are floors. Everything omitted
    /// can only add, which is what makes a floor safe to print: it can never say a model
    /// fits when it does not.
    pub bounded: bool,
    pub bound_note: Option<String>,
}

/// How many layers hold a cache sized to the whole context, and how many hold a
/// smaller one. A model naming no interval is all full attention, which is what a
/// plain attention model is.
fn layer_split(md: &GgufMetadata, layers: u64) -> Option<(u64, u64)> {
    let Some(interval) = md.full_attention_interval else {
        return Some((layers, 0));
    };
    if interval == 0 {
        return None;
    }
    let full = layers / interval;
    Some((full, layers - full))
}

fn per_token(k_dim: u64, v_dim: u64, cache_k: &str, cache_v: &str) -> f64 {
    (k_dim as f64 * bytes_per_element(cache_k)) + (v_dim as f64 * bytes_per_element(cache_v))
}

/// What the layers this file describes cannot account for. Carries the numbers it read,
/// so the screen says what this file declares rather than what such a file might.
fn bound_note(md: &GgufMetadata) -> Option<String> {
    let layers = md.block_count?;
    let interval = md.full_attention_interval?;
    let (_, uncounted) = layer_split(md, layers)?;
    if uncounted == 0 || md.sliding_window.is_some() {
        return None;
    }
    Some(format!(
        "One layer in {interval} does full attention and is counted here. The header \
         gives no window size for the other {uncounted}, so what those hold is left out \
         — the figures are floors, and the missing term can only add to them."
    ))
}

/// K and V are summed separately rather than doubled: latent-attention architectures
/// such as deepseek2 size them differently. Sliding-window layers are summed apart from
/// full-attention ones for the same reason one number cannot stand for both — they hold
/// a cache the context does not grow past the window.
///
/// Where the header names an interval but no window, the layers it does describe are
/// still counted and the result is marked a floor. Counting nothing there threw away a
/// figure the file gives.
pub fn kv_bytes(md: &GgufMetadata, ctx: u64, cache_k: &str, cache_v: &str) -> Option<u64> {
    Some(kv_terms(md, ctx, cache_k, cache_v)?.0)
}

fn kv_terms(md: &GgufMetadata, ctx: u64, cache_k: &str, cache_v: &str) -> Option<(u64, bool)> {
    let layers = md.block_count?;
    let kv_heads = md.head_count_kv?;
    let k_dim = md.head_dim()?;
    let v_dim = md.value_head_dim()?;

    let (full, sliding) = layer_split(md, layers)?;
    let mut total =
        full as f64 * ctx as f64 * kv_heads as f64 * per_token(k_dim, v_dim, cache_k, cache_v);

    if sliding == 0 {
        return Some((total as u64, false));
    }

    let Some(window) = md.sliding_window else {
        return Some((total as u64, true));
    };

    let swa_k = md.swa_head_dim()?;
    let swa_v = md.swa_value_head_dim()?;
    total += sliding as f64
        * window.min(ctx) as f64
        * kv_heads as f64
        * per_token(swa_k, swa_v, cache_k, cache_v);
    Some((total as u64, false))
}

pub fn estimate(
    md: &GgufMetadata,
    file_size: u64,
    ctx: u64,
    cache_k: &str,
    cache_v: &str,
) -> Option<Estimate> {
    // Auto: no context has been chosen, so there is nothing to size a cache against.
    // The weights are still exact, and they are a floor for the whole.
    if ctx == crate::profile::AUTO_CTX {
        return Some(Estimate {
            weights_bytes: file_size,
            kv_bytes: 0,
            total_bytes: file_size,
            bounded: true,
            bound_note: Some(
                "No context has been chosen. The server fits one to memory when it \
                 starts and sizes the cache with it, and this screen says what it chose \
                 once it is running."
                    .to_string(),
            ),
        });
    }

    let (kv_bytes, bounded) = kv_terms(md, ctx, cache_k, cache_v)?;
    let mut note = None;
    if bounded {
        note = bound_note(md);
    }
    Some(Estimate {
        weights_bytes: file_size,
        kv_bytes,
        total_bytes: file_size + kv_bytes,
        bounded,
        bound_note: note,
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
            sliding_window: None,
            full_attention_interval: None,
            key_length_swa: None,
            value_length_swa: None,
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
        assert!(!e.bounded);
        assert!(e.bound_note.is_none());
    }

    #[test]
    fn a_model_naming_no_interval_is_all_full_attention() {
        let md = metadata(8, 128, None);
        let layers = md.block_count.unwrap();
        let ctx = 4096u64;
        let expected = layers * ctx * 8 * (128 * 2 + 128 * 2);
        assert_eq!(kv_bytes(&md, ctx, "f16", "f16"), Some(expected));
    }

    #[test]
    fn sliding_window_layers_are_sized_to_the_window() {
        let mut md = metadata(8, 128, None);
        md.full_attention_interval = Some(4);
        md.sliding_window = Some(1024);

        let ctx = 65536u64;
        let per_layer_token = 128 * 2 + 128 * 2;
        let full = 10u64;
        let sliding = 30u64;
        let expected = (full * ctx + sliding * 1024) * 8 * per_layer_token;

        assert_eq!(kv_bytes(&md, ctx, "f16", "f16"), Some(expected));

        let all_full = metadata(8, 128, None);
        let charged_flat = kv_bytes(&all_full, ctx, "f16", "f16").unwrap();
        assert!(
            charged_flat > expected * 3,
            "the flat sum over-counts by more than three times"
        );
    }

    #[test]
    fn a_window_wider_than_the_context_holds_only_the_context() {
        let mut md = metadata(8, 128, None);
        md.full_attention_interval = Some(4);
        md.sliding_window = Some(200_000);

        let ctx = 4096u64;
        let flat = metadata(8, 128, None);
        assert_eq!(
            kv_bytes(&md, ctx, "f16", "f16"),
            kv_bytes(&flat, ctx, "f16", "f16"),
            "no layer holds more cells than the context has"
        );
    }

    #[test]
    fn sliding_layers_use_their_own_widths_where_the_header_carries_them() {
        let mut narrow = metadata(8, 128, None);
        narrow.full_attention_interval = Some(4);
        narrow.sliding_window = Some(1024);
        narrow.key_length_swa = Some(64);
        narrow.value_length_swa = Some(64);

        let mut wide = narrow.clone();
        wide.key_length_swa = None;
        wide.value_length_swa = None;

        let ctx = 65536u64;
        assert!(
            kv_bytes(&narrow, ctx, "f16", "f16") < kv_bytes(&wide, ctx, "f16", "f16"),
            "narrower sliding-window heads hold less than the full-attention ones"
        );
    }

    #[test]
    fn a_pattern_without_a_window_counts_what_it_can_and_marks_a_floor() {
        let mut md = metadata(8, 128, None);
        md.full_attention_interval = Some(4);

        let ctx = 65536u64;
        let e = estimate(&md, 20_000_000_000, ctx, "f16", "f16").expect("a floor, not nothing");

        // Ten of the forty layers do full attention; only those are counted.
        let counted = 10 * ctx * 8 * (128 * 2 + 128 * 2);
        assert_eq!(e.kv_bytes, counted);
        assert_eq!(e.total_bytes, 20_000_000_000 + counted);
        assert!(e.bounded);

        let why = e.bound_note.expect("a reason, not a blank");
        assert!(
            why.contains("layer in 4"),
            "names the interval it read: {why}"
        );
        assert!(why.contains("other 30"), "names what it left out: {why}");
    }

    #[test]
    fn the_floor_is_below_what_charging_every_layer_would_claim() {
        let mut md = metadata(8, 128, None);
        md.full_attention_interval = Some(4);
        let flat = metadata(8, 128, None);

        let ctx = 65536u64;
        let floor = kv_bytes(&md, ctx, "f16", "f16").unwrap();
        let charged_flat = kv_bytes(&flat, ctx, "f16", "f16").unwrap();

        assert!(floor < charged_flat, "a floor is not the old over-count");
        assert_eq!(
            charged_flat / floor,
            4,
            "forty layers charged where ten are counted"
        );
    }

    #[test]
    fn an_interval_of_one_is_every_layer_full() {
        let mut md = metadata(8, 128, None);
        md.full_attention_interval = Some(1);
        let flat = metadata(8, 128, None);
        assert_eq!(
            kv_bytes(&md, 4096, "f16", "f16"),
            kv_bytes(&flat, 4096, "f16", "f16")
        );
    }

    #[test]
    fn auto_prices_the_weights_and_says_the_cache_is_not_chosen_yet() {
        let md = metadata(8, 128, None);
        let e = estimate(&md, 20_000_000_000, crate::profile::AUTO_CTX, "f16", "f16")
            .expect("weights are known whatever the context");

        assert_eq!(e.weights_bytes, 20_000_000_000);
        assert_eq!(e.kv_bytes, 0, "no context, so no cache is priced");
        assert_eq!(e.total_bytes, 20_000_000_000);
        assert!(e.bounded, "weights alone are a floor for the whole");

        let why = e.bound_note.expect("a reason");
        assert!(why.contains("No context has been chosen"), "{why}");
    }

    #[test]
    fn a_header_with_no_attention_dimensions_yields_nothing() {
        let mut md = metadata(8, 128, None);
        md.head_count_kv = None;
        assert!(estimate(&md, 20_000_000_000, 4096, "f16", "f16").is_none());
    }

    #[test]
    fn value_width_falls_back_to_key_width() {
        let md = metadata(4, 128, None);
        assert_eq!(md.value_head_dim(), Some(128));
    }
}
