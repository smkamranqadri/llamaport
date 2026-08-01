use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

/// A GUI app launched from Finder inherits a minimal PATH, so `which` alone will not
/// find a Homebrew install.
const FALLBACK_PATHS: [&str; 3] = [
    "/opt/homebrew/bin/llama-server",
    "/usr/local/bin/llama-server",
    "/usr/bin/llama-server",
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    pub binary: String,
    pub version: Option<String>,
    pub flags: BTreeSet<String>,
    pub flash_attn_takes_value: bool,
}

impl Capabilities {
    pub fn has(&self, flag: &str) -> bool {
        self.flags.contains(flag)
    }
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
        .map(|line| line.trim_start().trim_start_matches("version:").trim().to_string())
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
        return Err(format!("{} produced no recognisable help", binary.display()));
    }

    Ok(Capabilities {
        binary: binary.to_string_lossy().into_owned(),
        version: read_version(binary),
        flash_attn_takes_value: flash_attn_takes_value(&help),
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

    #[test]
    fn detects_valued_flash_attn() {
        assert!(flash_attn_takes_value(HELP_SAMPLE));
        assert!(!flash_attn_takes_value(
            "-fa,   --flash-attn                     enable Flash Attention"
        ));
    }
}
