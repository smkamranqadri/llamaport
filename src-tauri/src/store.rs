use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use std::collections::BTreeMap;

use crate::estimate::{CalibrationSample, MAX_SAMPLES};
use crate::profile::Profile;

/// Bumped whenever the shape changes. Absence means the original shape, which had no
/// version field at all.
pub const CURRENT_SCHEMA: u32 = 3;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    pub schema_version: u32,
    pub models_dir: Option<String>,
    pub llama_server_path: Option<String>,
    pub calibration: Vec<CalibrationSample>,
    /// The settings each model was last launched with, so the form opens where it was
    /// left rather than at a generic default. Not a profile system: there is one entry
    /// per model, it is written by launching, and nothing merges.
    pub last_used: BTreeMap<String, Profile>,
    /// Keys written by a different version of the app. Captured and written back
    /// untouched so that running an older build cannot silently delete newer settings.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Config {
    pub fn record_sample(&mut self, sample: CalibrationSample) {
        self.calibration.push(sample);
        if self.calibration.len() > MAX_SAMPLES {
            let excess = self.calibration.len() - MAX_SAMPLES;
            self.calibration.drain(0..excess);
        }
    }
}

fn home() -> PathBuf {
    std::env::var("HOME").map(PathBuf::from).unwrap_or_default()
}

pub fn config_dir() -> PathBuf {
    home()
        .join("Library")
        .join("Application Support")
        .join("llama-cpp-hub")
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

/// Brings a config forward to the current schema.
///
/// Deliberately additive: it never removes or rewrites a field it does not understand,
/// and unknown keys ride along in `extra`. Running an older build after a newer one must
/// not cost the user data.
pub fn migrate(mut config: Config) -> Config {
    if config.schema_version >= CURRENT_SCHEMA {
        return config;
    }

    // v3 drops the profile system's storage. These keys reach `extra` now that no
    // field claims them, and would otherwise be carried forward forever as the
    // unknown-key rule intends — correct for a key from a *newer* build, wrong for one
    // this build deliberately removed.
    for retired in ["defaultProfile", "overrides", "lastRun", "profiles"] {
        config.extra.remove(retired);
    }

    config.schema_version = CURRENT_SCHEMA;
    config
}

pub fn load_from(path: &Path) -> Config {
    let Ok(raw) = fs::read_to_string(path) else {
        return Config {
            schema_version: CURRENT_SCHEMA,
            ..Default::default()
        };
    };
    let parsed: Config = serde_json::from_str(&raw).unwrap_or_default();
    migrate(parsed)
}

/// Writes through a temporary file and renames, so an interrupted write cannot leave a
/// truncated file behind. Rename within a directory is atomic on APFS.
pub fn write_atomic(path: &Path, contents: &str) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, contents)?;
    fs::rename(&temporary, path)
}

pub fn save_to(path: &Path, config: &Config) -> io::Result<()> {
    write_atomic(path, &serde_json::to_string_pretty(config)?)
}

pub fn load() -> Config {
    load_from(&config_path())
}

pub fn save(config: &Config) -> io::Result<()> {
    save_to(&config_path(), config)
}

pub fn models_dir(config: &Config) -> PathBuf {
    match &config.models_dir {
        Some(dir) if !dir.trim().is_empty() => PathBuf::from(dir),
        _ => home().join("models"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("llama-hub-store-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("scratch dir");
        dir.join("config.json")
    }

    #[test]
    fn round_trips_known_fields() {
        let path = scratch("known");
        let mut config = Config {
            models_dir: Some("/models".into()),
            ..Default::default()
        };
        config.last_used.insert(
            "abc".into(),
            Profile {
                ctx: 32768,
                ..Default::default()
            },
        );

        save_to(&path, &config).expect("save");
        let loaded = load_from(&path);

        assert_eq!(loaded.models_dir.as_deref(), Some("/models"));
        assert_eq!(loaded.last_used.get("abc").map(|p| p.ctx), Some(32768));
    }

    #[test]
    fn preserves_keys_written_by_a_newer_version() {
        let path = scratch("unknown");
        fs::write(
            &path,
            r#"{
              "modelsDir": "/models",
              "benchmarksEnabled": true,
              "futureSection": { "nested": [1, 2, 3] }
            }"#,
        )
        .expect("seed");

        let loaded = load_from(&path);
        assert_eq!(loaded.models_dir.as_deref(), Some("/models"));
        assert!(loaded.extra.contains_key("benchmarksEnabled"));

        save_to(&path, &loaded).expect("save");
        let reloaded: Value =
            serde_json::from_str(&fs::read_to_string(&path).expect("read")).expect("json");

        assert_eq!(reloaded["benchmarksEnabled"], Value::Bool(true));
        assert_eq!(reloaded["futureSection"]["nested"][2], Value::from(3));
    }

    #[test]
    fn malformed_config_falls_back_to_defaults_without_panicking() {
        let path = scratch("malformed");
        fs::write(&path, "{ this is not json").expect("seed");
        let loaded = load_from(&path);
        assert!(loaded.models_dir.is_none());
    }

    #[test]
    fn save_leaves_no_temporary_file_behind() {
        let path = scratch("tmp");
        save_to(&path, &Config::default()).expect("save");

        let leftovers: Vec<_> = fs::read_dir(path.parent().expect("dir"))
            .expect("read dir")
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|name| name.ends_with(".tmp"))
            .collect();

        assert!(leftovers.is_empty(), "left behind: {leftovers:?}");
    }

    /// The exact config this machine had before schema v2 existed.
    const V1_CONFIG: &str = r#"{
      "modelsDir": null,
      "llamaServerPath": null,
      "overrides": { "abc-123": { "ctx": 32768 } },
      "calibration": [],
      "lastRun": { "3ec121be0-2395d7127ff460da": 1785592595 }
    }"#;

    #[test]
    fn v1_is_migrated_without_losing_anything() {
        let path = scratch("migrate");
        fs::write(&path, V1_CONFIG).expect("seed");

        let loaded = load_from(&path);

        assert_eq!(loaded.schema_version, CURRENT_SCHEMA);
        assert!(loaded.last_used.is_empty());
        assert!(
            loaded.extra.is_empty(),
            "a v1 config has nothing this build does not understand"
        );
    }

    #[test]
    fn the_settings_a_model_last_launched_with_round_trip() {
        let path = scratch("last-used");
        let mut config = Config::default();
        config.last_used.insert(
            "model-1".into(),
            Profile {
                ctx: 32768,
                cache_type_v: "q4_0".into(),
                ..Default::default()
            },
        );

        save_to(&path, &config).expect("save");
        let loaded = load_from(&path);
        let remembered = loaded.last_used.get("model-1").expect("remembered");

        assert_eq!(remembered.ctx, 32768);
        assert_eq!(remembered.cache_type_v, "q4_0");
        assert_eq!(remembered.port, 8888, "unset fields still fall back");
    }

    #[test]
    fn a_partial_profile_does_not_take_the_whole_config_down() {
        let path = scratch("partial-profile");
        fs::write(
            &path,
            r#"{
              "modelsDir": "/models",
              "lastUsed": { "abc": { "port": 9000 } }
            }"#,
        )
        .expect("seed");

        let loaded = load_from(&path);

        assert_eq!(
            loaded.last_used.get("abc").map(|p| p.port),
            Some(9000),
            "the stated field wins"
        );
        assert_eq!(
            loaded.last_used.get("abc").map(|p| p.ctx),
            Some(65536),
            "the rest fall back"
        );
        assert_eq!(loaded.models_dir.as_deref(), Some("/models"));
    }

    #[test]
    fn retired_keys_are_dropped_while_unknown_ones_are_kept() {
        let path = scratch("retired");
        fs::write(
            &path,
            r#"{
              "defaultProfile": { "port": 8888 },
              "overrides": { "abc": {} },
              "lastRun": { "abc": 1 },
              "profiles": [],
              "somethingFromANewerBuild": true
            }"#,
        )
        .expect("seed");

        let loaded = load_from(&path);
        for retired in ["defaultProfile", "overrides", "lastRun", "profiles"] {
            assert!(
                !loaded.extra.contains_key(retired),
                "{retired} should be gone"
            );
        }
        assert!(
            loaded.extra.contains_key("somethingFromANewerBuild"),
            "a key this build simply does not know must still survive"
        );
    }

    #[test]
    fn migration_is_idempotent() {
        let path = scratch("idempotent");
        fs::write(&path, V1_CONFIG).expect("seed");

        let once = load_from(&path);
        save_to(&path, &once).expect("save");
        let twice = load_from(&path);

        assert_eq!(twice.schema_version, CURRENT_SCHEMA);
        assert_eq!(twice.models_dir, once.models_dir);
    }

    #[test]
    fn migration_preserves_keys_from_a_newer_build() {
        let path = scratch("migrate-unknown");
        fs::write(
            &path,
            r#"{
              "somethingFromTheFuture": { "keep": "me" }
            }"#,
        )
        .expect("seed");

        let loaded = load_from(&path);
        assert_eq!(loaded.schema_version, CURRENT_SCHEMA);

        save_to(&path, &loaded).expect("save");
        let raw: Value =
            serde_json::from_str(&fs::read_to_string(&path).expect("read")).expect("json");
        assert_eq!(raw["somethingFromTheFuture"]["keep"], Value::from("me"));
    }

    #[test]
    fn an_existing_v1_config_still_loads() {
        let path = scratch("v1");
        fs::write(
            &path,
            r#"{
              "modelsDir": null,
              "llamaServerPath": null,
              "defaultProfile": {
                "alias": "", "host": "127.0.0.1", "port": 8888, "ctx": 65536,
                "ngl": "all", "parallel": 1, "flashAttn": true,
                "cacheTypeK": "q8_0", "cacheTypeV": "q8_0", "jinja": true, "rawArgs": []
              },
              "calibration": []
            }"#,
        )
        .expect("seed");

        let loaded = load_from(&path);
        assert_eq!(loaded.models_dir, None);
        assert!(loaded.extra.is_empty());
    }
}
