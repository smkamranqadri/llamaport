use serde::{Deserialize, Serialize};

use crate::probe::Capabilities;

pub const DEFAULT_PORT: u16 = 8888;
pub const DEFAULT_CTX: u64 = 65536;

/// The values one launch uses. Nothing persists these — the form starts from
/// `Profile::default()` each time and edits last as long as the page is open.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Profile {
    pub alias: String,
    pub host: String,
    pub port: u16,
    pub ctx: u64,
    pub ngl: String,
    pub parallel: u32,
    pub flash_attn: bool,
    pub cache_type_k: String,
    pub cache_type_v: String,
    pub jinja: bool,
    pub raw_args: Vec<String>,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            alias: String::new(),
            host: "127.0.0.1".to_string(),
            port: DEFAULT_PORT,
            ctx: DEFAULT_CTX,
            ngl: "all".to_string(),
            parallel: 1,
            flash_attn: true,
            cache_type_k: "q8_0".to_string(),
            cache_type_v: "q8_0".to_string(),
            jinja: true,
            raw_args: Vec::new(),
        }
    }
}

pub fn default_alias(display_name: &str) -> String {
    display_name
        .trim()
        .to_lowercase()
        .replace([' ', '_'], "-")
        .trim_matches('-')
        .to_string()
}

impl Profile {
    /// Renders argv gated on what the installed build actually accepts. `--metrics` is
    /// added silently because the telemetry view depends on it and it costs nothing.
    pub fn args(&self, model_path: &str, caps: &Capabilities) -> Vec<String> {
        let mut args: Vec<String> = vec!["-m".into(), model_path.into()];

        if !self.alias.trim().is_empty() {
            args.push("--alias".into());
            args.push(self.alias.clone());
        }

        args.push("--host".into());
        args.push(self.host.clone());
        args.push("--port".into());
        args.push(self.port.to_string());
        args.push("-c".into());
        args.push(self.ctx.to_string());
        args.push("-ngl".into());
        args.push(self.ngl.clone());
        args.push("-np".into());
        args.push(self.parallel.to_string());

        if caps.has("--flash-attn") {
            if caps.flash_attn_takes_value {
                args.push("--flash-attn".into());
                args.push(if self.flash_attn { "on" } else { "off" }.into());
            } else if self.flash_attn {
                args.push("--flash-attn".into());
            }
        }

        if caps.has("--cache-type-k") {
            args.push("--cache-type-k".into());
            args.push(self.cache_type_k.clone());
        }
        if caps.has("--cache-type-v") {
            args.push("--cache-type-v".into());
            args.push(self.cache_type_v.clone());
        }

        if self.jinja {
            if caps.has("--jinja") {
                args.push("--jinja".into());
            }
        } else if caps.has("--no-jinja") {
            args.push("--no-jinja".into());
        }

        if caps.has("--metrics") {
            args.push("--metrics".into());
        }

        args.extend(self.raw_args.iter().cloned());
        args
    }
}

pub fn render_command(binary: &str, args: &[String]) -> String {
    let mut parts = vec![shell_quote(binary)];
    parts.extend(args.iter().map(|a| shell_quote(a)));
    parts.join(" ")
}

fn shell_quote(value: &str) -> String {
    let safe = value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "-_./:=,".contains(c));
    if safe && !value.is_empty() {
        return value.to_string();
    }
    format!("'{}'", value.replace('\'', r"'\''"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn caps(flags: &[&str], flash_value: bool) -> Capabilities {
        Capabilities {
            binary: "llama-server".into(),
            version: None,
            flags: flags.iter().map(|f| f.to_string()).collect::<BTreeSet<_>>(),
            flash_attn_takes_value: flash_value,
        }
    }

    fn full_caps() -> Capabilities {
        caps(
            &[
                "--flash-attn",
                "--cache-type-k",
                "--cache-type-v",
                "--jinja",
                "--no-jinja",
                "--metrics",
            ],
            true,
        )
    }

    #[test]
    fn renders_the_expected_command() {
        let profile = Profile {
            alias: "qwen3.6-35b-a3b".into(),
            ..Profile::default()
        };
        let args = profile.args("/Users/me/models/Q.gguf", &full_caps());
        let rendered = render_command("llama-server", &args);

        assert!(rendered.contains("-m /Users/me/models/Q.gguf"));
        assert!(rendered.contains("--alias qwen3.6-35b-a3b"));
        assert!(rendered.contains("--host 127.0.0.1 --port 8888"));
        assert!(rendered.contains("-c 65536 -ngl all -np 1"));
        assert!(rendered.contains("--flash-attn on"));
        assert!(rendered.contains("--cache-type-k q8_0 --cache-type-v q8_0"));
        assert!(rendered.contains("--jinja"));
        assert!(rendered.contains("--metrics"));
    }

    #[test]
    fn omits_flags_the_build_does_not_support() {
        let profile = Profile::default();
        let args = profile.args("/m.gguf", &caps(&["--cache-type-k"], false));

        assert!(!args.iter().any(|a| a == "--flash-attn"));
        assert!(!args.iter().any(|a| a == "--metrics"));
        assert!(!args.iter().any(|a| a == "--cache-type-v"));
        assert!(args.iter().any(|a| a == "--cache-type-k"));
    }

    #[test]
    fn bare_flash_attn_is_a_switch_not_a_value() {
        let profile = Profile::default();
        let args = profile.args("/m.gguf", &caps(&["--flash-attn"], false));
        let index = args.iter().position(|a| a == "--flash-attn").unwrap();
        assert!(args.get(index + 1).is_none_or(|next| next != "on"));
    }

    #[test]
    fn disabling_jinja_uses_the_negative_flag() {
        let profile = Profile {
            jinja: false,
            ..Profile::default()
        };
        let args = profile.args("/m.gguf", &full_caps());
        assert!(args.iter().any(|a| a == "--no-jinja"));
        assert!(!args.iter().any(|a| a == "--jinja"));
    }

    #[test]
    fn quotes_paths_with_spaces() {
        let args = vec!["-m".to_string(), "/Users/me/my models/a.gguf".to_string()];
        assert_eq!(
            render_command("llama-server", &args),
            "llama-server -m '/Users/me/my models/a.gguf'"
        );
    }

    #[test]
    fn alias_defaults_from_display_name() {
        assert_eq!(default_alias("Qwen3.6-35B-A3B"), "qwen3.6-35b-a3b");
        assert_eq!(default_alias("Gemma 4 26B"), "gemma-4-26b");
    }
}
