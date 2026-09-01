//! Pointing pi at the model this app is serving.
//!
//! pi's `models.json` is hand-maintained and holds four providers this app knows nothing
//! about. So this module reads the whole document, replaces exactly one provider, and
//! writes the rest back unchanged. It never has an opinion about the rest of the file.

use std::fs;
use std::io::ErrorKind;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::Serialize;
use serde_json::{json, Value};

use crate::store;

/// The one provider this app owns. Everything else in that file is the user's.
pub const PROVIDER: &str = "llamaport";

/// `llama-server` ignores it and pi wants the field. Three of the five providers already
/// in the file carry a placeholder exactly like this one; nothing written here should
/// look like a credential.
const API_KEY: &str = "none";

/// What `local-llama` — the file's other llama.cpp provider — sets on both its models.
/// Not derivable from anything the server reports, so the convention already in the file
/// decides it rather than a number of ours.
const MAX_TOKENS: u64 = 8192;

/// What the running server is, in the terms pi needs. Every field comes from the runner:
/// the port it bound, the alias it was launched under, and the context the server itself
/// reports — not the one that was asked for, which `--fit` is free to overrule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Serving {
    pub alias: String,
    pub name: String,
    pub port: u16,
    pub ctx: u64,
}

/// One file's before and after, rendered for the panel to diff.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileChange {
    pub path: String,
    /// Absent when the file does not exist yet, or holds nothing of ours.
    pub before: Option<String>,
    pub after: String,
    pub creates_file: bool,
}

/// What stands in pi's two files, and what would replace it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub provider: FileChange,
    /// `enabledModels` in pi's settings. A provider alone is not selectable: pi will not
    /// offer a model until it is named here.
    pub enabled: FileChange,
    /// Entries of ours dropped because the provider no longer lists that model. Our
    /// provider holds exactly one, so every other `llamaport/` line is already dead.
    pub pruned: Vec<String>,
    /// Other providers already naming this port. A `baseUrl` is a declaration and not
    /// evidence that anything is bound there — but only one server can hold a port, so
    /// the model this app is serving will answer to those names too.
    pub sharing_port: Vec<String>,
    /// Seeded from the entry already in the file, so a second write of the same model
    /// needs no thought. Nothing in a GGUF states this reliably.
    pub reasoning: bool,
}

fn base_url(port: u16) -> String {
    format!("http://127.0.0.1:{port}/v1")
}

fn block(serving: &Serving, reasoning: bool) -> Value {
    json!({
        "baseUrl": base_url(serving.port),
        "api": "openai-completions",
        "apiKey": API_KEY,
        "compat": {
            "supportsDeveloperRole": false,
            "supportsReasoningEffort": false
        },
        "models": [{
            "id": serving.alias,
            "name": serving.name,
            "reasoning": reasoning,
            "input": ["text"],
            "contextWindow": serving.ctx,
            "maxTokens": MAX_TOKENS,
            "cost": { "input": 0, "output": 0, "cacheRead": 0, "cacheWrite": 0 }
        }]
    })
}

/// The document, or `None` when there is no file yet.
///
/// Missing is not a failure: the file has one top-level key and creating it is not a
/// guess. Unparseable is a failure and must stay one — overwriting a file we could not
/// read is how a hand-maintained config gets destroyed.
fn document(path: &Path) -> Result<Option<Value>, String> {
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("{} could not be read: {error}", path.display())),
    };

    let parsed: Value = serde_json::from_str(&raw).map_err(|error| {
        format!(
            "{} is not valid JSON ({error}). Nothing was written — fix the file by hand, \
             or the next write would take everything else in it with the fix.",
            path.display()
        )
    })?;

    if !parsed.get("providers").is_some_and(Value::is_object) {
        return Err(format!(
            "{} has no \"providers\" object, so this is not a pi model file. Nothing was \
             written.",
            path.display()
        ));
    }

    Ok(Some(parsed))
}

fn providers(document: &Value) -> Option<&serde_json::Map<String, Value>> {
    document.get("providers")?.as_object()
}

/// Whether the entry already in the file says this model reasons. A model that has never
/// been written gets `false`, which is the only answer available without asking.
fn reasoning_of(document: &Value, alias: &str) -> bool {
    providers(document)
        .and_then(|providers| providers.get(PROVIDER))
        .and_then(|provider| provider.get("models"))
        .and_then(Value::as_array)
        .and_then(|models| {
            models
                .iter()
                .find(|model| model.get("id").and_then(Value::as_str) == Some(alias))
        })
        .and_then(|model| model.get("reasoning"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

/// Providers other than ours whose `baseUrl` names the same port.
fn sharing_port(document: &Value, port: u16) -> Vec<String> {
    let url = base_url(port);
    let Some(providers) = providers(document) else {
        return Vec::new();
    };
    providers
        .iter()
        .filter(|(name, _)| name.as_str() != PROVIDER)
        .filter(|(_, provider)| {
            provider.get("baseUrl").and_then(Value::as_str) == Some(url.as_str())
        })
        .map(|(name, _)| name.clone())
        .collect()
}

fn render(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_default()
}

/// What a file holding API keys should be, and what pi's own files are. `write_atomic`
/// renames a fresh temporary into place, and a fresh file is not born with the mode of
/// the one it replaces — so without this, writing here would quietly publish the user's
/// keys to every account on the machine.
const PRIVATE: u32 = 0o600;

fn mode_of(path: &Path) -> u32 {
    fs::metadata(path)
        .map(|metadata| metadata.permissions().mode() & 0o777)
        .unwrap_or(PRIVATE)
}

/// The copy kept beside each file, overwritten on every confirm rather than stamped: that
/// directory already collects backups, and this should not add to the pile on every press.
fn backup_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".llamaport.bak");
    path.with_file_name(name)
}

/// The `enabledModels` list, and what it would become.
///
/// Ours is appended if it is not already there, and every other `llamaport/` entry is
/// dropped: the provider holds exactly one model, so any other line of ours names a model
/// it no longer lists. Entries belonging to other providers are never touched.
fn enabled_change(document: Option<&Value>, alias: &str) -> (Vec<String>, Vec<String>) {
    let ours = format!("{PROVIDER}/{alias}");
    let mine = format!("{PROVIDER}/");

    let current: Vec<String> = document
        .and_then(|document| document.get("enabledModels"))
        .and_then(Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();

    let pruned: Vec<String> = current
        .iter()
        .filter(|entry| entry.starts_with(&mine) && *entry != &ours)
        .cloned()
        .collect();

    let mut kept: Vec<String> = current
        .into_iter()
        .filter(|entry| !pruned.contains(entry))
        .collect();
    if !kept.contains(&ours) {
        kept.push(ours);
    }

    (kept, pruned)
}

fn render_enabled(entries: &[String]) -> String {
    render(&json!({ "enabledModels": entries }))
}

fn settings_document(path: &Path) -> Result<Option<Value>, String> {
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("{} could not be read: {error}", path.display())),
    };

    let parsed: Value = serde_json::from_str(&raw).map_err(|error| {
        format!(
            "{} is not valid JSON ({error}). Nothing was written to either file.",
            path.display()
        )
    })?;

    if !parsed.is_object() {
        return Err(format!(
            "{} is not a JSON object, so this is not pi's settings file. Nothing was \
             written.",
            path.display()
        ));
    }

    Ok(Some(parsed))
}

pub fn preview(models: &Path, settings: &Path, serving: &Serving) -> Result<Preview, String> {
    let document = document(models)?;
    let reasoning = document
        .as_ref()
        .map(|document| reasoning_of(document, &serving.alias))
        .unwrap_or(false);

    let before = document
        .as_ref()
        .and_then(providers)
        .and_then(|providers| providers.get(PROVIDER))
        .map(render);

    let sharing_port = document
        .as_ref()
        .map(|document| sharing_port(document, serving.port))
        .unwrap_or_default();

    let settings_document = settings_document(settings)?;
    let (kept, pruned) = enabled_change(settings_document.as_ref(), &serving.alias);
    let enabled_before = settings_document
        .as_ref()
        .and_then(|document| document.get("enabledModels"))
        .and_then(Value::as_array)
        .map(|entries| {
            let current: Vec<String> = entries
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect();
            render_enabled(&current)
        });

    Ok(Preview {
        provider: FileChange {
            path: models.display().to_string(),
            before,
            after: render(&block(serving, reasoning)),
            creates_file: document.is_none(),
        },
        enabled: FileChange {
            path: settings.display().to_string(),
            before: enabled_before,
            after: render_enabled(&kept),
            creates_file: settings_document.is_none(),
        },
        pruned,
        sharing_port,
        reasoning,
    })
}

/// Serialises the read-modify-write. `write_atomic`'s temporary is one name per
/// destination rather than per writer, so two writers race on it and the loser's rename
/// fails — and here the loser would also have read a document the winner has replaced.
static WRITING: Mutex<()> = Mutex::new(());

fn write_json(path: &Path, document: &Value, existed: bool) -> Result<(), String> {
    let mode = match existed {
        true => mode_of(path),
        false => PRIVATE,
    };

    if existed {
        fs::copy(path, backup_path(path))
            .map_err(|error| format!("could not back up {}: {error}", path.display()))?;
    }

    let mut rendered = render(document);
    rendered.push('\n');
    store::write_atomic(path, &rendered)
        .map_err(|error| format!("could not write {}: {error}", path.display()))?;
    fs::set_permissions(path, fs::Permissions::from_mode(mode))
        .map_err(|error| format!("could not restore the mode of {}: {error}", path.display()))
}

/// Replaces our provider, names our model in `enabledModels`, and writes everything else
/// in both files back untouched.
///
/// Both files are read and checked before either is written: an unparseable settings file
/// used to be discovered after `models.json` had already changed, which left pi holding a
/// provider it would not offer. Re-reads inside the lock rather than trusting what the
/// preview saw — the user may have edited either file while the panel was open.
pub fn apply(
    models: &Path,
    settings: &Path,
    serving: &Serving,
    reasoning: bool,
) -> Result<Preview, String> {
    let _guard = WRITING
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let existing = document(models)?;
    let existing_settings = settings_document(settings)?;

    let mut document = existing
        .clone()
        .unwrap_or_else(|| json!({ "providers": {} }));
    let providers = document
        .get_mut("providers")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| format!("{} has no \"providers\" object", models.display()))?;
    providers.insert(PROVIDER.to_string(), block(serving, reasoning));

    let mut settings_json = existing_settings.clone().unwrap_or_else(|| json!({}));
    let (kept, pruned) = enabled_change(existing_settings.as_ref(), &serving.alias);
    settings_json
        .as_object_mut()
        .ok_or_else(|| format!("{} is not a JSON object", settings.display()))?
        .insert("enabledModels".into(), json!(kept));

    write_json(models, &document, existing.is_some())?;
    write_json(settings, &settings_json, existing_settings.is_some())?;

    // What was removed is only visible before the write; reading it back finds the list
    // already clean and would report having pruned nothing.
    Ok(Preview {
        pruned,
        ..preview(models, settings, serving)?
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn serving() -> Serving {
        Serving {
            alias: "qwen3.6-35b-a3b".into(),
            name: "Qwen3.6 35B-A3B".into(),
            port: 8080,
            ctx: 65536,
        }
    }

    fn settings_beside(models: &Path) -> PathBuf {
        models.with_file_name("settings.json")
    }

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "llamaport-pi-{}-{}-{name}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|since| since.as_nanos())
                .unwrap_or_default()
        ));
        fs::create_dir_all(&dir).expect("temp dir");
        dir.join("models.json")
    }

    /// The shape of the author's own file: five providers, ours absent, not in
    /// alphabetical order.
    fn hand_written() -> String {
        let document = json!({
            "providers": {
                "ollama": { "baseUrl": "http://127.0.0.1:11434/v1", "api": "openai-completions", "apiKey": "secret-one", "models": [] },
                "local-llama": { "baseUrl": "http://127.0.0.1:8888/v1", "api": "openai-completions", "apiKey": "secret-two", "models": [{ "id": "qwen3.6-35b-a3b", "reasoning": true }] },
                "unsloth": { "baseUrl": "http://127.0.0.1:8888/v1", "api": "openai-completions", "apiKey": "secret-three", "models": [] },
                "mlx-lm": { "baseUrl": "http://127.0.0.1:8080/v1", "api": "openai-completions", "apiKey": "secret-four", "models": [] },
                "omlx": { "baseUrl": "http://127.0.0.1:8080/v1", "api": "openai-completions", "apiKey": "secret-five", "authHeader": true, "models": [] }
            }
        });
        let mut rendered = serde_json::to_string_pretty(&document).expect("render");
        rendered.push('\n');
        rendered
    }

    #[test]
    fn writes_only_our_provider() {
        let path = temp("only-ours");
        fs::write(&path, hand_written()).expect("seed");

        apply(&path, &settings_beside(&path), &serving(), false).expect("apply");

        let after: Value =
            serde_json::from_str(&fs::read_to_string(&path).expect("read")).expect("parse");
        let before: Value = serde_json::from_str(&hand_written()).expect("parse seed");

        for (name, provider) in providers(&before).expect("providers") {
            assert_eq!(
                providers(&after).expect("providers").get(name),
                Some(provider),
                "{name} survived unchanged"
            );
        }
        assert!(providers(&after).expect("providers").contains_key(PROVIDER));
    }

    #[test]
    fn other_providers_keep_their_keys() {
        let path = temp("keys");
        fs::write(&path, hand_written()).expect("seed");

        apply(&path, &settings_beside(&path), &serving(), false).expect("apply");

        let raw = fs::read_to_string(&path).expect("read");
        for key in [
            "secret-one",
            "secret-two",
            "secret-three",
            "secret-four",
            "secret-five",
        ] {
            assert!(raw.contains(key), "{key} survived");
        }
    }

    #[test]
    fn provider_order_survives() {
        let path = temp("order");
        fs::write(&path, hand_written()).expect("seed");

        apply(&path, &settings_beside(&path), &serving(), false).expect("apply");

        let raw = fs::read_to_string(&path).expect("read");
        let order: Vec<usize> = ["ollama", "local-llama", "unsloth", "mlx-lm", "omlx"]
            .iter()
            .map(|name| raw.find(&format!("\"{name}\"")).expect("provider present"))
            .collect();
        let mut sorted = order.clone();
        sorted.sort_unstable();
        assert_eq!(order, sorted, "hand-written order is not reshuffled");
    }

    #[test]
    fn unparseable_is_refused_and_left_alone() {
        let path = temp("broken");
        let broken = "{ \"providers\": { oops }";
        fs::write(&path, broken).expect("seed");

        let error = apply(&path, &settings_beside(&path), &serving(), false).expect_err("refused");

        assert!(
            error.contains("not valid JSON"),
            "says what is wrong: {error}"
        );
        assert_eq!(fs::read_to_string(&path).expect("read"), broken);
        assert!(
            !backup_path(&path).exists(),
            "no backup of a file we never wrote"
        );
    }

    #[test]
    fn a_file_without_providers_is_refused() {
        let path = temp("not-pi");
        fs::write(&path, "{ \"models\": [] }").expect("seed");

        let error = apply(&path, &settings_beside(&path), &serving(), false).expect_err("refused");

        assert!(error.contains("providers"), "{error}");
    }

    #[test]
    fn missing_file_is_created_with_one_key() {
        let path = temp("created");

        apply(&path, &settings_beside(&path), &serving(), true).expect("apply");

        let created: Value =
            serde_json::from_str(&fs::read_to_string(&path).expect("read")).expect("parse");
        assert_eq!(
            created
                .as_object()
                .expect("object")
                .keys()
                .collect::<Vec<_>>(),
            vec!["providers"]
        );
        assert!(!backup_path(&path).exists(), "nothing to back up");
    }

    #[test]
    fn the_entry_carries_what_the_server_reported() {
        let path = temp("entry");

        apply(&path, &settings_beside(&path), &serving(), true).expect("apply");

        let document: Value =
            serde_json::from_str(&fs::read_to_string(&path).expect("read")).expect("parse");
        let provider = &document["providers"][PROVIDER];
        assert_eq!(provider["baseUrl"], "http://127.0.0.1:8080/v1");
        let model = &provider["models"][0];
        assert_eq!(model["id"], "qwen3.6-35b-a3b");
        assert_eq!(model["contextWindow"], 65536);
        assert_eq!(model["reasoning"], true);
        assert_eq!(model["maxTokens"], MAX_TOKENS);
    }

    #[test]
    fn one_model_replaces_the_last() {
        let path = temp("replaces");
        apply(&path, &settings_beside(&path), &serving(), false).expect("first");

        let second = Serving {
            alias: "devstral-small".into(),
            name: "Devstral Small".into(),
            port: 8081,
            ctx: 32768,
        };
        apply(&path, &settings_beside(&path), &second, false).expect("second");

        let document: Value =
            serde_json::from_str(&fs::read_to_string(&path).expect("read")).expect("parse");
        let models = document["providers"][PROVIDER]["models"]
            .as_array()
            .expect("models");
        assert_eq!(models.len(), 1, "one entry, not a growing list");
        assert_eq!(models[0]["id"], "devstral-small");
        assert_eq!(
            document["providers"][PROVIDER]["baseUrl"],
            "http://127.0.0.1:8081/v1"
        );
    }

    #[test]
    fn reasoning_is_remembered_for_the_same_model() {
        let path = temp("remembered");
        apply(&path, &settings_beside(&path), &serving(), true).expect("apply");

        let seen = preview(&path, &settings_beside(&path), &serving()).expect("preview");

        assert!(seen.reasoning, "seeded from what is already written");
        assert!(
            !preview(
                &path,
                &settings_beside(&path),
                &Serving {
                    alias: "other".into(),
                    ..serving()
                }
            )
            .expect("preview")
            .reasoning
        );
    }

    #[test]
    fn the_shared_port_is_named() {
        let path = temp("sharing");
        fs::write(&path, hand_written()).expect("seed");

        let seen = preview(&path, &settings_beside(&path), &serving()).expect("preview");

        let mut named = seen.sharing_port.clone();
        named.sort();
        assert_eq!(named, vec!["mlx-lm".to_string(), "omlx".to_string()]);
    }

    #[test]
    fn a_free_port_names_nobody() {
        let path = temp("free");
        fs::write(&path, hand_written()).expect("seed");

        let seen = preview(
            &path,
            &settings_beside(&path),
            &Serving {
                port: 9099,
                ..serving()
            },
        )
        .expect("preview");

        assert!(seen.sharing_port.is_empty());
    }

    #[test]
    fn the_previous_file_is_kept_beside_it() {
        let path = temp("backup");
        fs::write(&path, hand_written()).expect("seed");

        apply(&path, &settings_beside(&path), &serving(), false).expect("apply");

        assert_eq!(
            fs::read_to_string(backup_path(&path)).expect("backup"),
            hand_written()
        );
    }

    #[test]
    fn a_private_file_stays_private() {
        let path = temp("mode");
        fs::write(&path, hand_written()).expect("seed");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).expect("chmod");

        apply(&path, &settings_beside(&path), &serving(), false).expect("apply");

        assert_eq!(
            mode_of(&path),
            0o600,
            "the mode of a file holding keys survives"
        );
    }

    #[test]
    fn a_created_file_is_private() {
        let path = temp("created-mode");

        apply(&path, &settings_beside(&path), &serving(), false).expect("apply");

        assert_eq!(mode_of(&path), PRIVATE);
    }

    #[test]
    fn preview_of_a_missing_file_says_it_would_create_one() {
        let path = temp("absent");

        let seen = preview(&path, &settings_beside(&path), &serving()).expect("preview");

        assert!(seen.provider.creates_file);
        assert!(seen.provider.before.is_none());
    }

    /// The shape of the author's own settings: a default model and provider, other
    /// people's enabled entries, and two of ours from earlier writes.
    fn hand_written_settings() -> String {
        let document = json!({
            "defaultModel": "mlx-community--Qwen3.6-35B-A3B-4bit",
            "defaultProvider": "omlx",
            "theme": "dark",
            "enabledModels": [
                "local-llama/qwen3.6-35b-a3b",
                "llamaport/qwen2.5-0.5b-instruct",
                "omlx/mlx-community--Qwen3.6-35B-A3B-4bit",
                "llamaport/qwen2.5-1.5b-instruct"
            ]
        });
        let mut rendered = serde_json::to_string_pretty(&document).expect("render");
        rendered.push('\n');
        rendered
    }

    fn enabled_in(path: &Path) -> Vec<String> {
        let document: Value =
            serde_json::from_str(&fs::read_to_string(path).expect("read")).expect("parse");
        document["enabledModels"]
            .as_array()
            .expect("enabledModels")
            .iter()
            .map(|entry| entry.as_str().expect("string").to_string())
            .collect()
    }

    #[test]
    fn the_model_is_named_in_enabled_models() {
        let path = temp("enabled");
        let settings = settings_beside(&path);
        fs::write(&settings, hand_written_settings()).expect("seed");

        apply(&path, &settings, &serving(), false).expect("apply");

        assert!(enabled_in(&settings).contains(&"llamaport/qwen3.6-35b-a3b".to_string()));
    }

    #[test]
    fn other_peoples_entries_and_settings_survive() {
        let path = temp("survive");
        let settings = settings_beside(&path);
        fs::write(&settings, hand_written_settings()).expect("seed");

        apply(&path, &settings, &serving(), false).expect("apply");

        let entries = enabled_in(&settings);
        assert!(entries.contains(&"local-llama/qwen3.6-35b-a3b".to_string()));
        assert!(entries.contains(&"omlx/mlx-community--Qwen3.6-35B-A3B-4bit".to_string()));

        let raw = fs::read_to_string(&settings).expect("read");
        assert!(raw.contains("\"defaultProvider\": \"omlx\""));
        assert!(raw.contains("\"theme\": \"dark\""));
    }

    #[test]
    fn our_dead_entries_are_pruned_and_others_never_are() {
        let path = temp("prune");
        let settings = settings_beside(&path);
        fs::write(&settings, hand_written_settings()).expect("seed");

        let seen = apply(&path, &settings, &serving(), false).expect("apply");

        let entries = enabled_in(&settings);
        let ours: Vec<&String> = entries
            .iter()
            .filter(|entry| entry.starts_with("llamaport/"))
            .collect();
        assert_eq!(ours.len(), 1, "one of ours, matching the one model we list");
        assert_eq!(ours[0], "llamaport/qwen3.6-35b-a3b");

        let mut pruned = seen.pruned.clone();
        pruned.sort();
        assert_eq!(
            pruned,
            vec![
                "llamaport/qwen2.5-0.5b-instruct".to_string(),
                "llamaport/qwen2.5-1.5b-instruct".to_string()
            ]
        );
    }

    #[test]
    fn writing_the_same_model_twice_does_not_double_it() {
        let path = temp("twice");
        let settings = settings_beside(&path);

        apply(&path, &settings, &serving(), false).expect("first");
        apply(&path, &settings, &serving(), false).expect("second");

        let entries = enabled_in(&settings);
        assert_eq!(
            entries
                .iter()
                .filter(|entry| *entry == "llamaport/qwen3.6-35b-a3b")
                .count(),
            1
        );
    }

    #[test]
    fn an_unparseable_settings_file_leaves_models_untouched() {
        let path = temp("half");
        let settings = settings_beside(&path);
        fs::write(&path, hand_written()).expect("seed models");
        fs::write(&settings, "{ oops").expect("seed settings");

        let error = apply(&path, &settings, &serving(), false).expect_err("refused");

        assert!(error.contains("not valid JSON"), "{error}");
        assert_eq!(
            fs::read_to_string(&path).expect("read"),
            hand_written(),
            "models.json is not written when settings cannot be read"
        );
    }

    #[test]
    fn a_private_settings_file_stays_private() {
        let path = temp("settings-mode");
        let settings = settings_beside(&path);
        fs::write(&settings, hand_written_settings()).expect("seed");
        fs::set_permissions(&settings, fs::Permissions::from_mode(0o600)).expect("chmod");

        apply(&path, &settings, &serving(), false).expect("apply");

        assert_eq!(mode_of(&settings), 0o600);
    }

    #[test]
    fn the_previous_settings_are_kept_beside_them() {
        let path = temp("settings-backup");
        let settings = settings_beside(&path);
        fs::write(&settings, hand_written_settings()).expect("seed");

        apply(&path, &settings, &serving(), false).expect("apply");

        assert_eq!(
            fs::read_to_string(backup_path(&settings)).expect("backup"),
            hand_written_settings()
        );
    }
}
