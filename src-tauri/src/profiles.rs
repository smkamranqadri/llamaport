//! Named launch profiles and the built-in workload templates.
//!
//! These extend the existing two-layer resolution (global default, then per-model
//! patch) rather than replacing it. A profile stores a *sparse* patch, not a full
//! configuration, so applying "Long Context" changes the context and cache types
//! without also stamping someone else's alias and port over the model's own.
//!
//! Templates are defined in code and applied to the launch form. There is deliberately
//! no create/rename/duplicate/delete surface: per-model overrides already persist what a
//! user changes, and a second profile system on top of that was scope the app did not
//! need.

use serde::{Deserialize, Serialize};

use crate::profile::ProfilePatch;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Workload {
    QualityCoding,
    Balanced,
    LongContext,
    Lightweight,
    #[default]
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NamedProfile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub built_in: bool,
    #[serde(default)]
    pub workload: Workload,
    /// Binds the profile to one model. `None` means it applies to any.
    #[serde(default)]
    pub model_id: Option<String>,
    /// Names a credential held elsewhere. Never the key itself.
    #[serde(default)]
    pub api_key_ref: Option<String>,
    pub settings: ProfilePatch,
}

pub const QUALITY_CODING: &str = "builtin-quality-coding";
pub const BALANCED: &str = "builtin-balanced";
pub const LONG_CONTEXT: &str = "builtin-long-context";
pub const LIGHTWEIGHT: &str = "builtin-lightweight";

/// Starting points sized for a 32 GB Apple Silicon machine that is also running an
/// editor, a browser and a coding agent. They are templates, not correct answers — the
/// UI says so, and every value is editable.
pub fn built_ins() -> Vec<NamedProfile> {
    vec![
        NamedProfile {
            id: QUALITY_CODING.into(),
            name: "Quality Coding".into(),
            description: "Q4-class weights with a conservative context. Favours \
                          reliability over headroom for long conversations."
                .into(),
            built_in: true,
            workload: Workload::QualityCoding,
            model_id: None,
            api_key_ref: None,
            settings: ProfilePatch {
                ctx: Some(32768),
                cache_type_k: Some("q8_0".into()),
                cache_type_v: Some("q8_0".into()),
                flash_attn: Some(true),
                parallel: Some(1),
                ..Default::default()
            },
        },
        NamedProfile {
            id: BALANCED.into(),
            name: "Balanced".into(),
            description: "Moderate context for Q3 or Q4 weights, leaving room for an \
                          editor and browser alongside."
                .into(),
            built_in: true,
            workload: Workload::Balanced,
            model_id: None,
            api_key_ref: None,
            settings: ProfilePatch {
                ctx: Some(65536),
                cache_type_k: Some("q8_0".into()),
                cache_type_v: Some("q8_0".into()),
                flash_attn: Some(true),
                parallel: Some(1),
                ..Default::default()
            },
        },
        NamedProfile {
            id: LONG_CONTEXT.into(),
            name: "Long Context".into(),
            description: "Large context with a quantised V cache to pay for it. Intended \
                          for memory-efficient weights; check the memory warning before \
                          launching."
                .into(),
            built_in: true,
            workload: Workload::LongContext,
            model_id: None,
            api_key_ref: None,
            settings: ProfilePatch {
                ctx: Some(131072),
                // K stays wider than V deliberately: attention is more sensitive to key
                // precision, and quantised V requires flash attention.
                cache_type_k: Some("q8_0".into()),
                cache_type_v: Some("q4_0".into()),
                flash_attn: Some(true),
                parallel: Some(1),
                ..Default::default()
            },
        },
        NamedProfile {
            id: LIGHTWEIGHT.into(),
            name: "Lightweight".into(),
            description: "Small context and minimal footprint, for smaller models or for \
                          leaving the machine free."
                .into(),
            built_in: true,
            workload: Workload::Lightweight,
            model_id: None,
            api_key_ref: None,
            settings: ProfilePatch {
                ctx: Some(8192),
                cache_type_k: Some("q8_0".into()),
                cache_type_v: Some("q8_0".into()),
                flash_attn: Some(true),
                parallel: Some(1),
                ..Default::default()
            },
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_templates_ship_with_distinct_ids_and_names() {
        let templates = built_ins();
        assert_eq!(templates.len(), 4);

        let mut ids: Vec<&str> = templates.iter().map(|p| p.id.as_str()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), 4);

        assert!(templates.iter().all(|p| p.built_in));
        assert!(templates.iter().all(|p| !p.description.is_empty()));
    }

    #[test]
    fn templates_are_sparse_so_they_do_not_stamp_over_alias_or_port() {
        for template in built_ins() {
            assert!(template.settings.alias.is_none(), "{}", template.name);
            assert!(template.settings.port.is_none(), "{}", template.name);
            assert!(template.settings.host.is_none(), "{}", template.name);
        }
    }

    #[test]
    fn long_context_asks_for_more_context_than_quality_coding() {
        let templates = built_ins();
        let ctx = |id: &str| {
            templates
                .iter()
                .find(|p| p.id == id)
                .and_then(|p| p.settings.ctx)
                .expect("template sets a context")
        };
        assert!(ctx(LONG_CONTEXT) > ctx(BALANCED));
        assert!(ctx(BALANCED) > ctx(QUALITY_CODING));
        assert!(ctx(QUALITY_CODING) > ctx(LIGHTWEIGHT));
    }
}
