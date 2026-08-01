//! Connecting coding agents to the running server.
//!
//! Strictly read-and-generate. This module never writes to a Pi file, and inspection
//! returns *structural facts only* — provider names, base URLs, whether a key is set —
//! never a value that could be a credential. `models.json` holds API keys for cloud
//! providers, so returning its contents would leak secrets into event payloads and logs
//! that have nothing to do with llama.cpp.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Pi reserves headroom for output below the server's context size; mirroring that
/// avoids advertising a window the server cannot actually serve in one turn.
const CONTEXT_RESERVE: u64 = 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Connection {
    pub base_url: String,
    pub openai_url: String,
    pub alias: String,
    pub host: String,
    pub port: u16,
    pub loopback_only: bool,
}

pub fn connection(host: &str, port: u16, alias: &str) -> Connection {
    Connection {
        base_url: format!("http://{host}:{port}"),
        openai_url: format!("http://{host}:{port}/v1"),
        alias: alias.to_string(),
        host: host.to_string(),
        port,
        loopback_only: is_loopback(host),
    }
}

pub fn is_loopback(host: &str) -> bool {
    matches!(host, "127.0.0.1" | "::1" | "localhost" | "0:0:0:0:0:0:0:1")
}

// ---------------------------------------------------------------- Pi inspection

fn pi_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(home).join(".pi").join("agent")
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalProvider {
    pub name: String,
    pub base_url: Option<String>,
    pub api: Option<String>,
    /// Whether a key is configured — never the key.
    pub has_api_key: bool,
    pub model_ids: Vec<String>,
    /// Extra keys this Pi version uses, so a generated entry can mirror them instead of
    /// assuming a schema.
    pub extra_keys: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PiInspection {
    pub settings_found: bool,
    pub models_found: bool,
    pub default_provider: Option<String>,
    pub default_model: Option<String>,
    pub provider_names: Vec<String>,
    /// The provider already pointing at a loopback address, if any.
    pub local_provider: Option<LocalProvider>,
    pub notes: Vec<String>,
}

fn read_json(path: &Path) -> Option<Value> {
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

/// Reads shape, never content. Invoked only when the user asks for it.
pub fn inspect_pi() -> PiInspection {
    inspect_pi_in(&pi_dir())
}

pub fn inspect_pi_in(dir: &Path) -> PiInspection {
    let settings = read_json(&dir.join("settings.json"));
    let models = read_json(&dir.join("models.json"));

    let mut inspection = PiInspection {
        settings_found: settings.is_some(),
        models_found: models.is_some(),
        ..Default::default()
    };

    if let Some(settings) = &settings {
        inspection.default_provider = settings
            .get("defaultProvider")
            .and_then(Value::as_str)
            .map(String::from);
        inspection.default_model = settings
            .get("defaultModel")
            .and_then(Value::as_str)
            .map(String::from);
    }

    let Some(models) = models else {
        inspection
            .notes
            .push("No models.json found; the preview uses a documented default shape.".into());
        return inspection;
    };

    let Some(providers) = models.get("providers").and_then(Value::as_object) else {
        inspection
            .notes
            .push("models.json has no providers section.".into());
        return inspection;
    };

    inspection.provider_names = providers.keys().cloned().collect();

    for (name, provider) in providers {
        let base_url = provider.get("baseUrl").and_then(Value::as_str);
        let points_local = base_url.is_some_and(|url| {
            url.contains("127.0.0.1") || url.contains("localhost") || url.contains("[::1]")
        });
        if !points_local {
            continue;
        }

        let model_ids = provider
            .get("models")
            .and_then(Value::as_array)
            .map(|models| {
                models
                    .iter()
                    .filter_map(|m| m.get("id").and_then(Value::as_str).map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let known = ["baseUrl", "api", "apiKey", "models"];
        let extra_keys = provider
            .as_object()
            .map(|object| {
                object
                    .keys()
                    .filter(|key| !known.contains(&key.as_str()))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();

        // Prefer a provider that also serves llama.cpp's default port over, say, Ollama.
        let better = base_url.is_some_and(|url| !url.contains("11434"));
        if inspection.local_provider.is_none() || better {
            inspection.local_provider = Some(LocalProvider {
                name: name.clone(),
                base_url: base_url.map(String::from),
                api: provider
                    .get("api")
                    .and_then(Value::as_str)
                    .map(String::from),
                has_api_key: provider
                    .get("apiKey")
                    .and_then(Value::as_str)
                    .is_some_and(|key| !key.is_empty()),
                model_ids,
                extra_keys,
            });
        }
    }

    inspection
}

// ---------------------------------------------------------------- Preview

pub struct PreviewInput<'a> {
    pub provider_name: String,
    pub connection: &'a Connection,
    pub display_name: String,
    pub context_tokens: u64,
    pub reasoning: bool,
    pub existing: Option<&'a LocalProvider>,
}

/// Builds a provider entry for `models.json`.
///
/// When an existing local provider was found, its `api` value and extra keys are mirrored
/// so the result matches the installed Pi version rather than a schema this app invented.
/// The API key is always a placeholder: echoing the user's real key into a preview they
/// will copy, paste and possibly share is exactly the leak `Redacted` exists to prevent.
pub fn pi_provider_preview(input: &PreviewInput) -> String {
    let api = input
        .existing
        .and_then(|existing| existing.api.clone())
        .unwrap_or_else(|| "openai-completions".to_string());

    let mut provider = serde_json::Map::new();
    provider.insert("baseUrl".into(), json!(input.connection.openai_url));
    provider.insert("api".into(), json!(api));
    provider.insert(
        "apiKey".into(),
        json!(if input.existing.is_some_and(|e| e.has_api_key) {
            "<keep your existing value>"
        } else {
            "not-needed-for-local"
        }),
    );

    if input
        .existing
        .is_some_and(|existing| existing.extra_keys.iter().any(|key| key == "compat"))
    {
        provider.insert(
            "compat".into(),
            json!({
                "supportsDeveloperRole": false,
                "supportsReasoningEffort": false,
            }),
        );
    }

    provider.insert(
        "models".into(),
        json!([{
            "id": input.connection.alias,
            "name": input.display_name,
            "reasoning": input.reasoning,
            "input": ["text"],
            "contextWindow": input.context_tokens.saturating_sub(CONTEXT_RESERVE),
            "cost": { "input": 0, "output": 0, "cacheRead": 0, "cacheWrite": 0 },
        }]),
    );

    let document = json!({
        "providers": { input.provider_name.clone(): Value::Object(provider) }
    });
    serde_json::to_string_pretty(&document).unwrap_or_default()
}

/// The settings.json fragment that selects the model. Shown separately because it is a
/// different file, and merging the two would encourage pasting into the wrong one.
pub fn pi_settings_preview(provider_name: &str, alias: &str) -> String {
    serde_json::to_string_pretty(&json!({
        "defaultProvider": provider_name,
        "defaultModel": alias,
        "enabledModels": [format!("{provider_name}/{alias}")],
    }))
    .unwrap_or_default()
}

// ---------------------------------------------------------------- Applications

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedApp {
    pub name: String,
    pub path: String,
}

const CANDIDATES: [&str; 4] = ["Picot", "Visual Studio Code", "Cursor", "iTerm"];

pub fn detect_apps() -> Vec<DetectedApp> {
    let home = std::env::var("HOME").unwrap_or_default();
    let roots = [
        PathBuf::from("/Applications"),
        PathBuf::from(home).join("Applications"),
    ];

    let mut found = Vec::new();
    for name in CANDIDATES {
        for root in &roots {
            let path = root.join(format!("{name}.app"));
            if path.exists() {
                found.push(DetectedApp {
                    name: name.to_string(),
                    path: path.to_string_lossy().into_owned(),
                });
                break;
            }
        }
    }
    found
}

pub fn pi_sessions_dir() -> Option<String> {
    let dir = pi_dir().join("sessions");
    dir.is_dir().then(|| dir.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unique per call, not per name: several tests share `seeded()`, and cargo runs
    /// them in parallel, so a directory keyed only by name lets one test delete another's
    /// fixture mid-read.
    fn scratch(name: &str) -> PathBuf {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let unique = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "llama-hub-pi-{name}-{}-{unique}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch");
        dir
    }

    /// Mirrors the real file's shape, including a cloud provider whose key must not leak.
    const MODELS: &str = r#"{
      "providers": {
        "openai": {
          "api": "openai-completions",
          "apiKey": "sk-super-secret-cloud-key",
          "baseUrl": "https://api.openai.com/v1",
          "models": [{ "id": "gpt-x" }]
        },
        "ollama": {
          "api": "openai-completions",
          "apiKey": "ollama",
          "baseUrl": "http://127.0.0.1:11434/v1",
          "models": [{ "id": "qwen3.6:latest" }]
        },
        "local-llama": {
          "baseUrl": "http://127.0.0.1:8888/v1",
          "api": "openai-completions",
          "apiKey": "sk-local",
          "compat": { "supportsDeveloperRole": false },
          "models": [{ "id": "qwen3.6-35b-a3b", "contextWindow": 64512 }]
        }
      }
    }"#;

    fn seeded() -> PathBuf {
        let dir = scratch("inspect");
        std::fs::write(
            dir.join("settings.json"),
            r#"{"defaultProvider":"local-llama","defaultModel":"qwen3.6-35b-a3b"}"#,
        )
        .expect("settings");
        std::fs::write(dir.join("models.json"), MODELS).expect("models");
        dir
    }

    #[test]
    fn inspection_reports_structure_not_contents() {
        let inspection = inspect_pi_in(&seeded());
        let serialised = serde_json::to_string(&inspection).expect("serialise");

        assert!(
            !serialised.contains("sk-super-secret-cloud-key"),
            "a cloud provider key leaked: {serialised}"
        );
        assert!(!serialised.contains("sk-local"), "the local key leaked");
        assert!(serialised.contains("hasApiKey"));
    }

    #[test]
    fn inspection_finds_the_local_provider_and_ignores_cloud_ones() {
        let inspection = inspect_pi_in(&seeded());
        let local = inspection.local_provider.expect("local provider");

        assert_eq!(local.name, "local-llama");
        assert_eq!(local.base_url.as_deref(), Some("http://127.0.0.1:8888/v1"));
        assert!(local.has_api_key);
        assert_eq!(local.model_ids, vec!["qwen3.6-35b-a3b"]);
        assert!(local.extra_keys.contains(&"compat".to_string()));
    }

    #[test]
    fn a_llama_provider_is_preferred_over_ollama() {
        let inspection = inspect_pi_in(&seeded());
        assert_eq!(
            inspection.local_provider.expect("provider").name,
            "local-llama",
            "ollama is also loopback but is not the server this app runs"
        );
    }

    #[test]
    fn settings_values_are_read_when_present() {
        let inspection = inspect_pi_in(&seeded());
        assert_eq!(inspection.default_provider.as_deref(), Some("local-llama"));
        assert_eq!(inspection.default_model.as_deref(), Some("qwen3.6-35b-a3b"));
        assert_eq!(inspection.provider_names.len(), 3);
    }

    #[test]
    fn a_missing_pi_installation_is_reported_not_fabricated() {
        let inspection = inspect_pi_in(&scratch("empty"));
        assert!(!inspection.settings_found);
        assert!(!inspection.models_found);
        assert!(inspection.local_provider.is_none());
        assert!(!inspection.notes.is_empty());
    }

    fn preview_with(existing: Option<&LocalProvider>) -> String {
        let connection = connection("127.0.0.1", 8890, "qwen3.6-35b-a3b");
        pi_provider_preview(&PreviewInput {
            provider_name: "local-llama".into(),
            connection: &connection,
            display_name: "Qwen3.6 35B-A3B Q4_K_M".into(),
            context_tokens: 65536,
            reasoning: true,
            existing,
        })
    }

    #[test]
    fn the_preview_points_at_the_running_port() {
        let preview = preview_with(None);
        assert!(preview.contains("http://127.0.0.1:8890/v1"), "{preview}");
        assert!(preview.contains("\"qwen3.6-35b-a3b\""));
    }

    #[test]
    fn the_preview_never_contains_a_real_key() {
        let inspection = inspect_pi_in(&seeded());
        let existing = inspection.local_provider.expect("provider");
        let preview = preview_with(Some(&existing));

        assert!(!preview.contains("sk-local"), "{preview}");
        assert!(preview.contains("<keep your existing value>"));
    }

    #[test]
    fn the_preview_mirrors_keys_the_installed_version_uses() {
        let inspection = inspect_pi_in(&seeded());
        let existing = inspection.local_provider.expect("provider");

        assert!(preview_with(Some(&existing)).contains("compat"));
        assert!(
            !preview_with(None).contains("compat"),
            "a key this Pi version may not know must not be invented"
        );
    }

    #[test]
    fn the_advertised_window_leaves_room_for_output() {
        let preview = preview_with(None);
        assert!(
            preview.contains("64512"),
            "expected 65536 minus reserve: {preview}"
        );
    }

    #[test]
    fn the_settings_fragment_uses_the_provider_slash_model_form() {
        let fragment = pi_settings_preview("local-llama", "qwen3.6-35b-a3b");
        assert!(
            fragment.contains("\"local-llama/qwen3.6-35b-a3b\""),
            "{fragment}"
        );
    }

    #[test]
    fn loopback_detection_covers_the_usual_spellings() {
        for host in ["127.0.0.1", "::1", "localhost"] {
            assert!(is_loopback(host), "{host}");
        }
        for host in ["0.0.0.0", "192.168.1.10", "example.com"] {
            assert!(!is_loopback(host), "{host}");
        }
    }
}
