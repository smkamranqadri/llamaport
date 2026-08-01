use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::estimate::{CalibrationSample, MAX_SAMPLES};
use crate::profile::{Profile, ProfilePatch};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    pub models_dir: Option<String>,
    pub llama_server_path: Option<String>,
    pub default_profile: Profile,
    pub overrides: BTreeMap<String, ProfilePatch>,
    pub calibration: Vec<CalibrationSample>,
    pub last_run: BTreeMap<String, u64>,
    /// Keys written by a different version of the app. Captured and written back
    /// untouched so that running an older build cannot silently delete newer settings.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Config {
    pub fn patch_for(&self, model_id: &str) -> ProfilePatch {
        self.overrides.get(model_id).cloned().unwrap_or_default()
    }

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

pub fn load_from(path: &Path) -> Config {
    let Ok(raw) = fs::read_to_string(path) else {
        return Config::default();
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

/// Writes through a temporary file and renames, so an interrupted write cannot leave a
/// truncated config behind. Rename within a directory is atomic on APFS.
pub fn save_to(path: &Path, config: &Config) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let temporary = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(config)?;
    fs::write(&temporary, json)?;
    fs::rename(&temporary, path)
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
        config.default_profile.ctx = 32768;
        config.last_run.insert("abc".into(), 42);

        save_to(&path, &config).expect("save");
        let loaded = load_from(&path);

        assert_eq!(loaded.models_dir.as_deref(), Some("/models"));
        assert_eq!(loaded.default_profile.ctx, 32768);
        assert_eq!(loaded.last_run.get("abc"), Some(&42));
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
              "overrides": {},
              "calibration": [],
              "lastRun": { "3ec121be0-2395d7127ff460da": 1785592595 }
            }"#,
        )
        .expect("seed");

        let loaded = load_from(&path);
        assert_eq!(loaded.default_profile.port, 8888);
        assert_eq!(loaded.default_profile.ctx, 65536);
        assert_eq!(loaded.last_run.len(), 1);
        assert!(loaded.extra.is_empty());
    }
}
