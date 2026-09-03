use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Mutex, OnceLock};

use crate::downloads::{DownloadJob, Options};
use crate::profile::Profile;
use crate::speeds::SpeedRecord;

/// Bumped whenever the shape changes. Absence means the original shape, which had no
/// version field at all.
pub const CURRENT_SCHEMA: u32 = 8;

/// What the window looks like: a palette and, for the built-in one, whether it follows
/// macOS or is pinned. Both are plain strings and neither is an enum, because a config is
/// untrusted input — a name written by a newer build must fall back to the default
/// palette on the screen, not fail the parse and take every other setting with it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Appearance {
    pub theme: String,
    pub mode: String,
    /// Whether the sidebar lets the desktop through. Stored rather than derived because
    /// the window is transparent either way — `transparent` is set at creation and cannot
    /// be changed after — so this is the only thing that decides whether anything shows.
    ///
    /// **Defaults to on, including for a config written before it existed.** A window that
    /// is transparent and paints over every pixel of itself is the effect not existing,
    /// and the first build shipped that way: the author saw no change and reported the
    /// feature broken. `#[derive(Default)]` would have made it `false`, so `Default` is
    /// written out rather than derived.
    pub translucent: bool,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            theme: String::new(),
            mode: String::new(),
            translucent: true,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    pub schema_version: u32,
    pub models_dir: Option<String>,
    pub llama_server_path: Option<String>,
    /// The settings each model was last launched with, so the form opens where it was
    /// left rather than at a generic default. Not a profile system: there is one entry
    /// per model, it is written by launching, and nothing merges.
    pub last_used: BTreeMap<String, Profile>,
    /// When each model was last launched, by the same identity `last_used` is keyed on.
    /// Deliberately *not* named `lastRun`: that key is retired below, and on real v1
    /// configs it held a map of exactly this shape, so serde would claim it and show a
    /// timestamp from a build several schemas old as if it were this one's.
    pub last_launched: BTreeMap<String, u64>,
    pub downloads: Options,
    /// Models the user has starred, by the same identity `last_used` is keyed on —
    /// `(size, hash of the leading bytes)`, which survives a rename or a directory move.
    /// An id naming no model on disk is kept: the file may be on a volume that is not
    /// mounted, and forgetting the star would lose it for good.
    pub favourites: BTreeSet<String>,
    /// Where a model that has never been launched opens its form. Deliberately *not*
    /// named `defaultProfile`: that key was retired in v3 and is stripped from `extra`
    /// below, and a real field of that name would be claimed by serde first — silently
    /// adopting launch settings written by a build two schemas old.
    pub launch_defaults: Option<Profile>,
    /// Absent until the user picks one, which is what lets the screen say the app is
    /// following macOS rather than showing a choice nobody made.
    pub appearance: Option<Appearance>,
    /// Keys written by a different version of the app. Captured and written back
    /// untouched so that running an older build cannot silently delete newer settings.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

fn home() -> PathBuf {
    std::env::var("HOME").map(PathBuf::from).unwrap_or_default()
}

/// The app was called llama-cpp-hub until it was renamed. Everything it keeps lives
/// under one directory named after it: this config, the runner pidfile, the last run log.
const LEGACY_DIR: &str = "llama-cpp-hub";

fn support_dir() -> PathBuf {
    home().join("Library").join("Application Support")
}

static CONFIG_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Points everything the app keeps at another directory. The suite drives the real
/// runner, which writes a pidfile and mirrors the run log through `config_dir`, so
/// without this it writes them into the directory the installed app is using — which is
/// how `last-run.log` came to hold test output. Taken once; the first caller wins.
pub fn use_config_dir(dir: PathBuf) {
    let _ = CONFIG_DIR.set(dir);
}

pub fn config_dir() -> PathBuf {
    if let Some(dir) = CONFIG_DIR.get() {
        return dir.clone();
    }
    support_dir().join("llamaport")
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

static PI_MODELS_PATH: OnceLock<PathBuf> = OnceLock::new();

/// Points the pi integration at another file. This one is not the app's to lose: it is
/// hand-maintained, it holds four providers this app knows nothing about, and a suite
/// that reached it would be editing the user's tools. Taken once; the first caller wins.
pub fn use_pi_models_path(path: PathBuf) {
    let _ = PI_MODELS_PATH.set(path);
}

pub fn pi_models_path() -> PathBuf {
    if let Some(path) = PI_MODELS_PATH.get() {
        return path.clone();
    }
    pi_dir().join("models.json")
}

static PI_SETTINGS_PATH: OnceLock<PathBuf> = OnceLock::new();

/// The second pi file the app writes, and the more sensitive one: it holds the user's
/// default model and provider. Same reason for the override as the one above.
pub fn use_pi_settings_path(path: PathBuf) {
    let _ = PI_SETTINGS_PATH.set(path);
}

pub fn pi_settings_path() -> PathBuf {
    if let Some(path) = PI_SETTINGS_PATH.get() {
        return path.clone();
    }
    pi_dir().join("settings.json")
}

fn pi_dir() -> PathBuf {
    home().join(".pi").join("agent")
}

/// One small file per owner, rather than one map every writer has to rewrite. Avatars are
/// fetched from several threads at once, and `write_atomic` names its temporary after the
/// destination — one name per file, not per writer — so a shared file would have those
/// threads racing on it. A file each removes the question.
///
/// A miss is stored as an empty file and is as worth keeping as a hit: the owners with no
/// picture are the ones publishing the most repositories, so they are asked for most often.
pub fn avatar_path(owner: &str) -> PathBuf {
    config_dir().join("avatars").join(owner)
}

/// A file of its own rather than a section of the config.
///
/// A transfer settles often and the config holds the models directory, the llama-server
/// path and every remembered launch. Keeping the churn out of it means a history that
/// cannot be read costs the user nothing but their history.
pub fn history_path() -> PathBuf {
    config_dir().join("downloads.json")
}

/// What previous runs finished. Unreadable or malformed is an empty history rather than a
/// failure: it is a record of what already happened, and nothing depends on it.
pub fn load_history(path: &Path) -> Vec<DownloadJob> {
    let Ok(raw) = fs::read_to_string(path) else {
        return Vec::new();
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

pub fn save_history(path: &Path, jobs: &[DownloadJob]) -> io::Result<()> {
    write_atomic(path, &serde_json::to_string_pretty(jobs)?)
}

/// A file of its own for the same reason `downloads.json` is one: it grows on every run,
/// and a history that cannot be read should cost the user their history and nothing else.
pub fn speeds_path() -> PathBuf {
    config_dir().join("speeds.json")
}

/// What models have actually done. A row whose figures are not finite and non-negative is
/// dropped rather than ranked: this file sits in a directory anything with write access
/// can write to, and a negative second would win every comparison it entered.
pub fn load_speeds(path: &Path) -> Vec<SpeedRecord> {
    let Ok(raw) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let records: Vec<SpeedRecord> = serde_json::from_str(&raw).unwrap_or_default();
    records.into_iter().filter(SpeedRecord::is_sound).collect()
}

/// Serialises the read-modify-write below. Two runs settling in the same instant both
/// read the file before either wrote it, and the second rename silently discarded the
/// first row — seen once in five runs of the suite. Held only here: `load_speeds` is
/// called from inside this function and a `std::sync::Mutex` is not reentrant.
static APPENDING: Mutex<()> = Mutex::new(());

/// Never trimmed, and never rewritten in place: a run adds to what is there. A cap would
/// be a number nobody has evidence for.
pub fn append_speed(path: &Path, record: SpeedRecord) -> io::Result<()> {
    let _guard = APPENDING.lock().unwrap_or_else(|e| e.into_inner());
    let mut records = load_speeds(path);
    records.push(record);
    write_atomic(path, &serde_json::to_string_pretty(&records)?)
}

/// Takes over a directory left under an older name, once.
///
/// Declines when the current directory already exists: an older build run afterwards
/// recreates the legacy name, and adopting it a second time would throw away everything
/// written since. Must run before anything reads that directory.
pub fn adopt_legacy_dir(legacy: &Path, current: &Path) -> io::Result<bool> {
    if current.exists() || !legacy.is_dir() {
        return Ok(false);
    }
    if let Some(parent) = current.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::rename(legacy, current)?;
    Ok(true)
}

pub fn adopt_legacy_config_dir() -> io::Result<bool> {
    adopt_legacy_dir(&support_dir().join(LEGACY_DIR), &config_dir())
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
    for retired in [
        "defaultProfile",
        "overrides",
        "lastRun",
        "profiles",
        "calibration",
    ] {
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

/// Records that a model ran, once per run, and says whether anything changed.
///
/// The caller is a listener on a state stream that re-announces Ready on every telemetry
/// tick, so it sees one run many times and cannot tell the repeats apart. `started_secs`
/// can: it moves only when a process is spawned. A stamp already at or past it therefore
/// belongs to the run being announced, and there is nothing left to write.
///
/// That same value is what gets stored, rather than the moment Ready arrived — a few
/// seconds earlier, and it makes the guard exactly the comparison it writes.
pub fn stamp_if_newer(
    stamps: &mut BTreeMap<String, u64>,
    model_id: &str,
    started_secs: u64,
) -> bool {
    if stamps
        .get(model_id)
        .is_some_and(|stamped| *stamped >= started_secs)
    {
        return false;
    }
    stamps.insert(model_id.to_string(), started_secs);
    true
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

    fn scratch_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("llama-hub-store-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("scratch dir");
        dir
    }

    fn scratch(name: &str) -> PathBuf {
        scratch_dir(name).join("config.json")
    }

    fn speed_record(ctx: u64) -> SpeedRecord {
        SpeedRecord {
            timestamp_secs: ctx,
            key: crate::speeds::SpeedKey::of(
                "model",
                &Profile {
                    ctx,
                    ..Default::default()
                },
            ),
            llama_version: Some("b10360".into()),
            source: crate::speeds::Source::Observed,
            prompt_tokens: 3794.0,
            prompt_seconds: 7.47,
            gen_tokens: 64.0,
            gen_seconds: 1.54,
        }
    }

    #[test]
    fn two_runs_at_different_settings_are_two_rows() {
        let path = scratch_dir("speeds-two").join("speeds.json");
        append_speed(&path, speed_record(65_536)).expect("first");
        append_speed(&path, speed_record(262_144)).expect("second");

        let loaded = load_speeds(&path);
        assert_eq!(loaded.len(), 2, "a second run must not overwrite the first");
        assert_eq!(loaded[0].key.ctx, 65_536);
        assert_eq!(loaded[1].key.ctx, 262_144);
    }

    #[test]
    fn a_row_whose_figures_cannot_have_happened_is_dropped() {
        let path = scratch_dir("speeds-unsound").join("speeds.json");
        append_speed(&path, speed_record(65_536)).expect("sound");

        let mut planted = load_speeds(&path);
        let mut bad = speed_record(8192);
        bad.gen_seconds = -1.0;
        planted.push(bad);
        write_atomic(
            &path,
            &serde_json::to_string_pretty(&planted).expect("json"),
        )
        .expect("plant");

        let loaded = load_speeds(&path);
        assert_eq!(loaded.len(), 1, "the sound row must survive its neighbour");
        assert_eq!(loaded[0].key.ctx, 65_536);
    }

    #[test]
    fn rows_appended_at_once_do_not_overwrite_each_other() {
        let path = scratch_dir("speeds-at-once").join("speeds.json");
        let writers: Vec<_> = (0..8)
            .map(|writer| {
                let path = path.clone();
                std::thread::spawn(move || {
                    for run in 0..8 {
                        append_speed(&path, speed_record(writer * 100 + run)).expect("append");
                    }
                })
            })
            .collect();
        for writer in writers {
            writer.join().expect("writer");
        }

        assert_eq!(
            load_speeds(&path).len(),
            64,
            "an append that read the file before its neighbour wrote discards a run"
        );
    }

    #[test]
    fn an_unreadable_speeds_file_is_an_empty_history() {
        let dir = scratch_dir("speeds-unreadable");
        assert!(load_speeds(&dir.join("nothing-here.json")).is_empty());

        let path = dir.join("speeds.json");
        fs::write(&path, "{ not json").expect("write");
        assert!(load_speeds(&path).is_empty());
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
            Some(crate::profile::AUTO_CTX),
            "the rest fall back, and the fallback for a context is now Auto"
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
        for retired in [
            "defaultProfile",
            "overrides",
            "lastRun",
            "profiles",
            "calibration",
        ] {
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
    fn a_config_without_download_settings_gets_the_defaults() {
        let path = scratch("downloads-absent");
        fs::write(&path, r#"{ "schemaVersion": 4, "modelsDir": "/models" }"#).expect("seed");

        let loaded = load_from(&path);

        assert_eq!(loaded.schema_version, CURRENT_SCHEMA);
        assert_eq!(loaded.downloads.segments, 4);
        assert_eq!(loaded.downloads.rate_limit, None);
        assert!(loaded.downloads.verify);
    }

    #[test]
    fn download_settings_round_trip_and_fall_back_field_by_field() {
        let path = scratch("downloads");
        let config = Config {
            downloads: Options {
                segments: 8,
                rate_limit: Some(10_000_000),
                verify: false,
            },
            ..Default::default()
        };

        save_to(&path, &config).expect("save");
        let loaded = load_from(&path);
        assert_eq!(loaded.downloads.segments, 8);
        assert_eq!(loaded.downloads.rate_limit, Some(10_000_000));
        assert!(!loaded.downloads.verify);

        fs::write(&path, r#"{ "downloads": { "segments": 6 } }"#).expect("seed");
        let partial = load_from(&path);
        assert_eq!(partial.downloads.segments, 6);
        assert!(partial.downloads.verify, "the rest fall back");
    }

    #[test]
    fn favourites_round_trip_and_survive_a_config_that_predates_them() {
        let path = scratch("favourites");
        let config = Config {
            favourites: ["abc".to_string(), "def".to_string()].into_iter().collect(),
            ..Default::default()
        };

        save_to(&path, &config).expect("save");
        let loaded = load_from(&path);
        assert!(loaded.favourites.contains("abc") && loaded.favourites.contains("def"));

        fs::write(&path, r#"{ "schemaVersion": 5, "modelsDir": "/models" }"#).expect("seed");
        let older = load_from(&path);
        assert_eq!(older.schema_version, CURRENT_SCHEMA);
        assert!(
            older.favourites.is_empty(),
            "a config written before favourites existed simply has none"
        );
        assert_eq!(older.models_dir.as_deref(), Some("/models"));
    }

    #[test]
    fn launch_defaults_round_trip_and_never_come_from_the_retired_key() {
        let path = scratch("launch-defaults");
        let config = Config {
            launch_defaults: Some(Profile {
                ctx: 8192,
                ngl: "24".into(),
                ..Default::default()
            }),
            ..Default::default()
        };

        save_to(&path, &config).expect("save");
        let loaded = load_from(&path);
        let defaults = loaded.launch_defaults.expect("remembered");
        assert_eq!(defaults.ctx, 8192);
        assert_eq!(defaults.ngl, "24");

        // The trap. v1 configs on real machines carry a `defaultProfile` from the profile
        // system v3 removed. It must stay retired rather than being read back in under a
        // new name and quietly deciding how every unlaunched model launches.
        fs::write(
            &path,
            r#"{
              "defaultProfile": { "port": 9999, "ctx": 4096, "ngl": "0" },
              "modelsDir": "/models"
            }"#,
        )
        .expect("seed");

        let migrated = load_from(&path);
        assert!(
            migrated.launch_defaults.is_none(),
            "a retired key was adopted as the new one"
        );
        assert!(!migrated.extra.contains_key("defaultProfile"));
        assert_eq!(migrated.models_dir.as_deref(), Some("/models"));
    }

    #[test]
    fn a_config_from_before_the_sidebar_could_be_translucent_gets_the_effect() {
        // Deliberately not the other way round. The first build defaulted this off, so a
        // window that was transparent painted over every pixel of itself and the effect
        // may as well not have shipped. An absent key means the config predates the
        // setting, not that anybody turned it off.
        let path = scratch("appearance-old");
        fs::write(
            &path,
            r#"{"schemaVersion":8,"appearance":{"theme":"nous","mode":"dark"}}"#,
        )
        .expect("write");
        let loaded = load_from(&path).appearance.expect("remembered");
        assert_eq!(loaded.theme, "nous");
        assert!(loaded.translucent);
    }

    #[test]
    fn turning_it_off_survives_a_round_trip() {
        // The off state has to be storable, or the default would overwrite the choice on
        // every load and the toggle would spring back.
        let path = scratch("appearance-off");
        let config = Config {
            appearance: Some(Appearance {
                theme: "llamaport".into(),
                mode: "system".into(),
                translucent: false,
            }),
            ..Default::default()
        };
        save_to(&path, &config).expect("save");
        assert!(!load_from(&path).appearance.expect("remembered").translucent);
    }

    #[test]
    fn an_appearance_survives_a_round_trip_and_an_unknown_name_is_kept() {
        let path = scratch("appearance");
        let config = Config {
            appearance: Some(Appearance {
                theme: "nous".into(),
                mode: "dark".into(),
                translucent: true,
            }),
            ..Default::default()
        };

        save_to(&path, &config).expect("save");
        let loaded = load_from(&path).appearance.expect("remembered");
        assert_eq!(loaded.theme, "nous");
        assert_eq!(loaded.mode, "dark");
        assert!(loaded.translucent);

        // A palette a newer build named. Keeping it is the whole reason these are strings
        // rather than enums: an unreadable name must cost the screen its highlight, not
        // cost the user their models directory.
        fs::write(
            &path,
            r#"{
              "schemaVersion": 8,
              "modelsDir": "/models",
              "appearance": { "theme": "sunrise-from-a-later-build", "mode": "light" }
            }"#,
        )
        .expect("seed");

        let newer = load_from(&path);
        assert_eq!(
            newer.appearance.expect("kept").theme,
            "sunrise-from-a-later-build"
        );
        assert_eq!(newer.models_dir.as_deref(), Some("/models"));
    }

    /// Ready is announced on every telemetry tick, so a listener acting on it acts dozens
    /// of times per run. Without the guard that is a config write per tick, and a launch
    /// time that creeps forward while the model sits there doing nothing.
    #[test]
    fn a_run_is_stamped_once_however_often_ready_is_announced() {
        let mut stamps = BTreeMap::new();

        assert!(stamp_if_newer(&mut stamps, "abc", 1000));
        for _ in 0..20 {
            assert!(
                !stamp_if_newer(&mut stamps, "abc", 1000),
                "the same run was written twice"
            );
        }
        assert_eq!(stamps.get("abc"), Some(&1000));

        assert!(
            stamp_if_newer(&mut stamps, "abc", 2000),
            "a later run is a new fact"
        );
        assert_eq!(stamps.get("abc"), Some(&2000));

        assert!(
            !stamp_if_newer(&mut stamps, "abc", 1500),
            "a stale announcement must not drag the time backwards"
        );
        assert_eq!(stamps.get("abc"), Some(&2000));

        assert!(stamp_if_newer(&mut stamps, "def", 500));
        assert_eq!(stamps.get("abc"), Some(&2000), "one model, one entry");
    }

    #[test]
    fn launch_times_round_trip_and_never_come_from_the_retired_key() {
        let path = scratch("last-launched");
        let mut config = Config::default();
        config.last_launched.insert("abc".into(), 1785592595);

        save_to(&path, &config).expect("save");
        let loaded = load_from(&path);
        assert_eq!(loaded.last_launched.get("abc"), Some(&1785592595));

        // The trap. `lastRun` on a real v1 config is a map of model id to timestamp —
        // the same shape as this field — so serde would take it without complaint and
        // date every row from a build several schemas old.
        fs::write(
            &path,
            r#"{
              "lastRun": { "3ec121be0-2395d7127ff460da": 1785592595 },
              "modelsDir": "/models"
            }"#,
        )
        .expect("seed");

        let migrated = load_from(&path);
        assert!(
            migrated.last_launched.is_empty(),
            "a retired key was adopted as the new one"
        );
        assert!(!migrated.extra.contains_key("lastRun"));
        assert_eq!(migrated.models_dir.as_deref(), Some("/models"));

        fs::write(&path, r#"{ "schemaVersion": 6, "modelsDir": "/models" }"#).expect("seed");
        let older = load_from(&path);
        assert_eq!(older.schema_version, CURRENT_SCHEMA);
        assert!(
            older.last_launched.is_empty(),
            "a config written before launch times existed simply has none"
        );
        assert_eq!(older.models_dir.as_deref(), Some("/models"));
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
    fn a_directory_left_under_the_old_name_is_adopted_whole() {
        let root = scratch_dir("adopt");
        let legacy = root.join("llama-cpp-hub");
        let current = root.join("llamaport");
        fs::create_dir_all(&legacy).expect("legacy dir");
        save_to(
            &legacy.join("config.json"),
            &Config {
                models_dir: Some("/models".into()),
                ..Default::default()
            },
        )
        .expect("seed config");
        fs::write(legacy.join("runner.pid"), "4242").expect("seed pidfile");

        assert!(adopt_legacy_dir(&legacy, &current).expect("adopt"));

        assert_eq!(
            load_from(&current.join("config.json"))
                .models_dir
                .as_deref(),
            Some("/models")
        );
        assert_eq!(
            fs::read_to_string(current.join("runner.pid")).expect("pidfile"),
            "4242",
            "the whole directory moves, not just the config"
        );
        assert!(!legacy.exists(), "nothing is left under the old name");
    }

    #[test]
    fn adopting_never_clobbers_a_directory_that_already_exists() {
        let root = scratch_dir("adopt-existing");
        let legacy = root.join("llama-cpp-hub");
        let current = root.join("llamaport");
        fs::create_dir_all(&legacy).expect("legacy dir");
        fs::create_dir_all(&current).expect("current dir");
        fs::write(legacy.join("config.json"), r#"{ "modelsDir": "/stale" }"#).expect("seed legacy");
        fs::write(current.join("config.json"), r#"{ "modelsDir": "/live" }"#)
            .expect("seed current");

        assert!(!adopt_legacy_dir(&legacy, &current).expect("adopt"));

        assert_eq!(
            load_from(&current.join("config.json"))
                .models_dir
                .as_deref(),
            Some("/live"),
            "an older build recreating the old name must not win"
        );
        assert!(legacy.exists(), "and the old directory is left alone");
    }

    #[test]
    fn there_being_nothing_to_adopt_is_not_a_failure() {
        let root = scratch_dir("adopt-absent");
        let adopted = adopt_legacy_dir(&root.join("llama-cpp-hub"), &root.join("llamaport"))
            .expect("adopt must not error");
        assert!(!adopted);
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
