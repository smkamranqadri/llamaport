use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::time::SystemTime;

use serde::Serialize;

/// A GUI app launched from Finder inherits a minimal PATH, so `which` alone will not
/// find a Homebrew install.
const FALLBACK_PATHS: [&str; 3] = [
    "/opt/homebrew/bin/llama-server",
    "/usr/local/bin/llama-server",
    "/usr/bin/llama-server",
];

/// A device llama.cpp will allocate on, as it reports itself. The totals here are the
/// only honest ceiling: on an M2 Pro the Metal working set is 25,559 MiB against 32,768
/// MiB of installed memory, and it is installed memory this app used to measure against.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub id: String,
    pub name: String,
    pub total_mib: u64,
    pub free_mib: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    pub binary: String,
    pub version: Option<String>,
    pub flags: BTreeSet<String>,
    pub flash_attn_takes_value: bool,
    /// Empty where the build cannot report them. Callers say the ceiling is unknown
    /// rather than falling back to installed memory, which is the defect being fixed.
    pub devices: Vec<Device>,
}

impl Capabilities {
    pub fn has(&self, flag: &str) -> bool {
        self.flags.contains(flag)
    }

    /// What a fully offloaded launch has to fit inside, or `None` where the build did
    /// not say. The largest reporting device: llama.cpp lists compute backends with no
    /// memory of their own beside the one that has it.
    pub fn device_budget_mib(&self) -> Option<u64> {
        self.devices
            .iter()
            .map(|d| d.total_mib)
            .filter(|t| *t > 0)
            .max()
    }
}

/// Parses `llama-server --list-devices`, whose lines read
/// `  MTL0: Apple M2 Pro (25559 MiB, 25558 MiB free)`. Anything that does not match that
/// shape is skipped rather than guessed at.
fn parse_devices(text: &str) -> Vec<Device> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        let Some((id, rest)) = line.split_once(':') else {
            continue;
        };
        let Some(open) = rest.rfind('(') else {
            continue;
        };
        let Some(close) = rest.rfind(')') else {
            continue;
        };
        if close < open {
            continue;
        }
        let name = rest[..open].trim();
        let inside = &rest[open + 1..close];
        let Some((total, free)) = inside.split_once(',') else {
            continue;
        };
        let mib = |s: &str| -> Option<u64> {
            s.split_whitespace()
                .next()
                .and_then(|n| n.parse::<u64>().ok())
        };
        let (Some(total_mib), Some(free_mib)) = (mib(total), mib(free)) else {
            continue;
        };
        if id.is_empty() || name.is_empty() {
            continue;
        }
        out.push(Device {
            id: id.trim().to_string(),
            name: name.to_string(),
            total_mib,
            free_mib,
        });
    }
    out
}

fn read_devices(binary: &Path) -> Vec<Device> {
    let Ok(output) = Command::new(binary).arg("--list-devices").output() else {
        return Vec::new();
    };
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    parse_devices(&text)
}

pub fn discover(configured: Option<&str>) -> Option<PathBuf> {
    if let Some(path) = configured.filter(|p| !p.trim().is_empty()) {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }

    if let Ok(output) = Command::new("which").arg("llama-server").output() {
        let found = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !found.is_empty() && Path::new(&found).is_file() {
            return Some(PathBuf::from(found));
        }
    }

    if let Ok(home) = std::env::var("HOME") {
        let local = PathBuf::from(home).join(".local/bin/llama-server");
        if local.is_file() {
            return Some(local);
        }
    }

    FALLBACK_PATHS
        .iter()
        .map(PathBuf::from)
        .find(|p| p.is_file())
}

fn collect_flags(help: &str) -> BTreeSet<String> {
    let mut flags = BTreeSet::new();
    let bytes: Vec<char> = help.chars().collect();
    let mut i = 0;

    while i + 2 < bytes.len() {
        if bytes[i] == '-' && bytes[i + 1] == '-' && bytes[i + 2].is_ascii_alphabetic() {
            let start = i;
            i += 2;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == '-') {
                i += 1;
            }
            let flag: String = bytes[start..i].iter().collect();
            flags.insert(flag.trim_end_matches('-').to_string());
            continue;
        }
        i += 1;
    }
    flags
}

/// Recent builds changed `--flash-attn` from a bare switch to one taking on/off/auto.
fn flash_attn_takes_value(help: &str) -> bool {
    help.lines()
        .filter(|line| line.contains("--flash-attn"))
        .any(|line| line.contains("[on") || line.contains("on|off"))
}

fn read_version(binary: &Path) -> Option<String> {
    let output = Command::new(binary).arg("--version").output().ok()?;
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    text.lines()
        .find(|line| line.trim_start().starts_with("version:"))
        .map(|line| {
            line.trim_start()
                .trim_start_matches("version:")
                .trim()
                .to_string()
        })
}

const NOT_FOUND: &str = "llama-server was not found on PATH or in the usual locations. \
     Install it with `brew install llama.cpp`, or set its path in Settings.";

#[derive(Debug, Clone, PartialEq, Eq)]
struct Stamp {
    modified: SystemTime,
    len: u64,
}

fn stamp(binary: &Path) -> Option<Stamp> {
    let meta = std::fs::metadata(binary).ok()?;
    Some(Stamp {
        modified: meta.modified().ok()?,
        len: meta.len(),
    })
}

struct Probed {
    stamp: Option<Stamp>,
    result: Result<Capabilities, String>,
}

/// The probe, kept until the binary it was read off changes. A Homebrew upgrade replaces
/// the file in place, and flags probed off the old one would otherwise stand until the
/// app was relaunched.
#[derive(Default)]
pub struct Cache(Mutex<Option<Probed>>);

impl Cache {
    pub fn get(&self, configured: Option<&str>) -> Result<Capabilities, String> {
        let mut cached = self.0.lock().expect("caps lock");
        if let Some(probed) = cached.as_ref() {
            let unchanged = match (&probed.result, &probed.stamp) {
                (Ok(caps), Some(seen)) => stamp(Path::new(&caps.binary)).as_ref() == Some(seen),
                _ => true,
            };
            if unchanged {
                return probed.result.clone();
            }
        }

        // Stamped before the probe runs: a file replaced during it would otherwise be
        // remembered under the old flags for good.
        let (stamp, result) = match discover(configured) {
            Some(binary) => (stamp(&binary), probe(&binary)),
            None => (None, Err(NOT_FOUND.into())),
        };
        *cached = Some(Probed {
            stamp,
            result: result.clone(),
        });
        result
    }

    pub fn forget(&self) {
        *self.0.lock().expect("caps lock") = None;
    }
}

pub fn probe(binary: &Path) -> Result<Capabilities, String> {
    let output = Command::new(binary)
        .arg("--help")
        .output()
        .map_err(|e| format!("could not run {}: {e}", binary.display()))?;

    let help = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let flags = collect_flags(&help);
    if flags.is_empty() {
        return Err(format!(
            "{} produced no recognisable help",
            binary.display()
        ));
    }

    // Only asked for where the build says it can answer: an older one would spend a
    // process spawn to print an error.
    let mut devices = Vec::new();
    if flags.contains("--list-devices") {
        devices = read_devices(binary);
    }

    Ok(Capabilities {
        binary: binary.to_string_lossy().into_owned(),
        version: read_version(binary),
        flash_attn_takes_value: flash_attn_takes_value(&help),
        devices,
        flags,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const HELP_SAMPLE: &str = "\
-c,    --ctx-size N                     size of the prompt context
-fa,   --flash-attn [on|off|auto]       set Flash Attention use
-ctk,  --cache-type-k TYPE              KV cache data type for K
-ngl,  --gpu-layers, --n-gpu-layers N   max. number of layers
--metrics                               enable prometheus compatible metrics endpoint
--jinja, --no-jinja                     whether to use jinja template engine
";

    #[test]
    fn collects_long_flags() {
        let flags = collect_flags(HELP_SAMPLE);
        assert!(flags.contains("--ctx-size"));
        assert!(flags.contains("--flash-attn"));
        assert!(flags.contains("--cache-type-k"));
        assert!(flags.contains("--n-gpu-layers"));
        assert!(flags.contains("--metrics"));
        assert!(flags.contains("--no-jinja"));
    }

    const DEVICES_SAMPLE: &str = "\
Available devices:
  BLAS: Accelerate (0 MiB, 0 MiB free)
  MTL0: Apple M2 Pro (25559 MiB, 25558 MiB free)
";

    #[test]
    fn reads_the_ceiling_the_build_reports() {
        let devices = parse_devices(DEVICES_SAMPLE);
        assert_eq!(devices.len(), 2);
        assert_eq!(
            devices[1],
            Device {
                id: "MTL0".into(),
                name: "Apple M2 Pro".into(),
                total_mib: 25559,
                free_mib: 25558,
            }
        );
    }

    /// A compute backend with no memory of its own must not become the ceiling.
    #[test]
    fn the_budget_is_the_device_that_has_memory() {
        let caps = Capabilities {
            binary: "llama-server".into(),
            version: None,
            flags: BTreeSet::new(),
            flash_attn_takes_value: false,
            devices: parse_devices(DEVICES_SAMPLE),
        };
        assert_eq!(caps.device_budget_mib(), Some(25559));
    }

    /// Where every device reports no memory of its own, the ceiling is unknown rather
    /// than zero. `Some(0)` would read on screen as "nothing fits", which is a different
    /// and worse lie than "unknown".
    #[test]
    fn devices_that_all_report_nothing_leave_the_budget_unknown() {
        let caps = Capabilities {
            binary: "llama-server".into(),
            version: None,
            flags: BTreeSet::new(),
            flash_attn_takes_value: false,
            devices: parse_devices("  BLAS: Accelerate (0 MiB, 0 MiB free)\n"),
        };
        assert_eq!(caps.devices.len(), 1, "the device was parsed");
        assert_eq!(caps.device_budget_mib(), None);
    }

    /// The defect this exists to fix: a build that cannot report devices must leave the
    /// ceiling unknown, never fall back to installed memory.
    #[test]
    fn a_build_that_cannot_report_devices_has_no_budget() {
        let caps = Capabilities {
            binary: "llama-server".into(),
            version: None,
            flags: BTreeSet::new(),
            flash_attn_takes_value: false,
            devices: Vec::new(),
        };
        assert_eq!(caps.device_budget_mib(), None);
    }

    #[test]
    fn lines_that_are_not_devices_are_skipped_rather_than_guessed_at() {
        let devices = parse_devices(
            "Available devices:\n  ggml_metal_init: picking default device\n               note: something (unparseable) here\n  MTL0: Apple M2 Pro (100 MiB, 50 MiB free)\n",
        );
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].total_mib, 100);
    }

    #[test]
    fn a_replaced_binary_is_probed_again_and_an_unchanged_one_is_not() {
        use std::os::unix::fs::PermissionsExt;

        let dir = std::env::temp_dir().join(format!("llamaport-probe-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        let binary = dir.join("llama-server");
        let calls = dir.join("calls");
        let script = |flags: &str| {
            format!(
                "#!/bin/sh\necho x >> '{}'\necho '{flags}'\n",
                calls.display()
            )
        };
        let runs = || {
            std::fs::read_to_string(&calls)
                .map(|text| text.lines().count())
                .unwrap_or(0)
        };

        std::fs::write(&binary, script("--ctx-size N")).expect("script");
        std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755)).expect("mode");
        let configured = binary.to_str();

        let cache = Cache::default();
        let first = cache.get(configured).expect("a probe");
        assert!(first.has("--ctx-size"));
        let after_first = runs();
        assert!(after_first > 0, "the script was run");

        let again = cache.get(configured).expect("a probe");
        assert!(again.has("--ctx-size"));
        assert_eq!(runs(), after_first, "an unchanged binary was probed again");

        std::fs::write(&binary, script("--fit on")).expect("script");
        let replaced = cache.get(configured).expect("a probe");
        assert!(replaced.has("--fit"), "{:?}", replaced.flags);
        assert!(!replaced.has("--ctx-size"), "the old probe was served");
        assert!(runs() > after_first, "the replaced binary was not probed");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn detects_valued_flash_attn() {
        assert!(flash_attn_takes_value(HELP_SAMPLE));
        assert!(!flash_attn_takes_value(
            "-fa,   --flash-attn                     enable Flash Attention"
        ));
    }
}
