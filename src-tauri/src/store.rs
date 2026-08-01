use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

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

pub fn load() -> Config {
    let Ok(raw) = fs::read_to_string(config_path()) else {
        return Config::default();
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

pub fn save(config: &Config) -> io::Result<()> {
    fs::create_dir_all(config_dir())?;
    let json = serde_json::to_string_pretty(config)?;
    fs::write(config_path(), json)
}

pub fn models_dir(config: &Config) -> PathBuf {
    match &config.models_dir {
        Some(dir) if !dir.trim().is_empty() => PathBuf::from(dir),
        _ => home().join("models"),
    }
}
