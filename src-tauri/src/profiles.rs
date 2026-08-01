//! Named launch profiles and the built-in workload templates.
//!
//! These extend the existing two-layer resolution (global default, then per-model
//! patch) rather than replacing it. A profile stores a *sparse* patch, not a full
//! configuration, so applying "Long Context" changes the context and cache types
//! without also stamping someone else's alias and port over the model's own.
//!
//! Built-in templates are defined in code and identified by stable ids. A stored entry
//! with the same id shadows the code definition, which is what makes a built-in
//! editable; resetting simply drops the stored entry. User profiles are never touched
//! by that operation.

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

pub fn is_built_in(id: &str) -> bool {
    built_ins().iter().any(|profile| profile.id == id)
}

/// Built-ins first (stored edits shadowing code defaults), then user profiles.
pub fn resolve_all(stored: &[NamedProfile]) -> Vec<NamedProfile> {
    let mut out: Vec<NamedProfile> = built_ins()
        .into_iter()
        .map(|default| {
            stored
                .iter()
                .find(|entry| entry.id == default.id)
                .cloned()
                .map(|mut edited| {
                    // A stored copy cannot promote itself out of being a built-in.
                    edited.built_in = true;
                    edited
                })
                .unwrap_or(default)
        })
        .collect();

    out.extend(
        stored
            .iter()
            .filter(|entry| !is_built_in(&entry.id))
            .cloned()
            .map(|mut entry| {
                entry.built_in = false;
                entry
            }),
    );
    out
}

/// Appends " copy", then " copy 2", " copy 3" … until nothing collides.
pub fn unique_name(base: &str, existing: &[NamedProfile]) -> String {
    let taken = |candidate: &str| {
        existing
            .iter()
            .any(|profile| profile.name.eq_ignore_ascii_case(candidate))
    };

    if !taken(base) {
        return base.to_string();
    }

    let first = format!("{base} copy");
    if !taken(&first) {
        return first;
    }

    (2..)
        .map(|n| format!("{base} copy {n}"))
        .find(|candidate| !taken(candidate))
        .expect("an unused name always exists")
}

/// Ids are derived from the name but must not collide with a built-in or each other.
pub fn new_id(name: &str, existing: &[NamedProfile]) -> String {
    let slug: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let slug = slug.trim_matches('-').to_string();
    let base = if slug.is_empty() {
        "profile".to_string()
    } else {
        format!("user-{slug}")
    };

    let taken = |candidate: &str| {
        is_built_in(candidate) || existing.iter().any(|profile| profile.id == candidate)
    };

    if !taken(&base) {
        return base;
    }
    (2..)
        .map(|n| format!("{base}-{n}"))
        .find(|candidate| !taken(candidate))
        .expect("an unused id always exists")
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

    fn user_profile(id: &str, name: &str) -> NamedProfile {
        NamedProfile {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            built_in: false,
            workload: Workload::Custom,
            model_id: None,
            api_key_ref: None,
            settings: ProfilePatch::default(),
        }
    }

    #[test]
    fn built_ins_are_always_present_even_with_nothing_stored() {
        let resolved = resolve_all(&[]);
        assert_eq!(resolved.len(), 4);
        assert!(resolved.iter().all(|p| p.built_in));
    }

    #[test]
    fn a_stored_edit_shadows_the_code_default() {
        let mut edited = built_ins()
            .into_iter()
            .find(|p| p.id == BALANCED)
            .expect("balanced");
        edited.settings.ctx = Some(12345);

        let resolved = resolve_all(&[edited]);
        let balanced = resolved
            .iter()
            .find(|p| p.id == BALANCED)
            .expect("balanced");

        assert_eq!(balanced.settings.ctx, Some(12345));
        assert!(balanced.built_in, "an edit must not make it a user profile");
        assert_eq!(resolved.len(), 4, "editing must not duplicate the template");
    }

    #[test]
    fn user_profiles_survive_alongside_built_ins() {
        let resolved = resolve_all(&[user_profile("user-mine", "Mine")]);
        assert_eq!(resolved.len(), 5);
        assert_eq!(resolved.iter().filter(|p| p.built_in).count(), 4);
        assert!(resolved.iter().any(|p| p.name == "Mine" && !p.built_in));
    }

    #[test]
    fn a_stored_entry_cannot_claim_to_be_built_in() {
        let mut impostor = user_profile("user-impostor", "Impostor");
        impostor.built_in = true;

        let resolved = resolve_all(&[impostor]);
        let found = resolved.iter().find(|p| p.id == "user-impostor").unwrap();
        assert!(!found.built_in);
    }

    #[test]
    fn duplicate_names_are_suffixed_rather_than_rejected() {
        let existing = vec![user_profile("a", "Balanced")];
        assert_eq!(unique_name("Balanced", &existing), "Balanced copy");

        let existing = vec![
            user_profile("a", "Balanced"),
            user_profile("b", "Balanced copy"),
        ];
        assert_eq!(unique_name("Balanced", &existing), "Balanced copy 2");

        assert_eq!(unique_name("Fresh", &existing), "Fresh");
    }

    #[test]
    fn name_collisions_ignore_case() {
        let existing = vec![user_profile("a", "balanced")];
        assert_eq!(unique_name("Balanced", &existing), "Balanced copy");
    }

    #[test]
    fn generated_ids_avoid_built_ins_and_each_other() {
        let id = new_id("My Setup!", &[]);
        assert_eq!(id, "user-my-setup");

        let existing = vec![user_profile("user-my-setup", "My Setup")];
        assert_eq!(new_id("My Setup", &existing), "user-my-setup-2");
    }

    #[test]
    fn an_unnameable_profile_still_gets_an_id() {
        assert_eq!(new_id("", &[]), "profile");
        assert_eq!(new_id("!!!", &[]), "profile");
    }
}
