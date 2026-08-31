use serde::{Deserialize, Serialize};

use crate::probe::Capabilities;

pub const DEFAULT_PORT: u16 = 8888;

/// What a context falls back to where llama.cpp cannot size one itself.
pub const DEFAULT_CTX: u64 = 65536;

/// Context 0 means: pass no `-c` and let the server fit one to memory. Not an invented
/// sentinel — llama.cpp spells "loaded from model" as 0 in its own `--help`, and its
/// fitter adjusts the arguments a launch leaves unset. Do not convert this to an
/// `Option`; it would cost a config schema version and buy the same meaning.
pub const AUTO_CTX: u64 = 0;

/// Likewise for layer offload, which is llama.cpp's own default and not this app's.
pub const AUTO_NGL: &str = "auto";

/// The values one launch uses. Where a form opens on them is decided by `seed`: what a
/// model was last launched with, else the configured defaults, else these.
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
            ctx: AUTO_CTX,
            ngl: AUTO_NGL.to_string(),
            parallel: 1,
            flash_attn: true,
            cache_type_k: "q8_0".to_string(),
            cache_type_v: "q8_0".to_string(),
            jinja: true,
            raw_args: Vec::new(),
        }
    }
}

/// Flags the app owns, and the field that owns each. `--host` and `--port` are the ones
/// that must not move: it binds loopback deliberately and tracks the port to find the
/// server again, so a raw argument setting either would leave the app supervising an
/// address it does not know about. `--alias` is here because the form already has the
/// field, so a second one is a mistake rather than a choice.
const OWNED_FLAGS: [(&str, &str); 3] =
    [("--alias", "Alias"), ("--host", "Host"), ("--port", "Port")];

/// Which settings a form opens on, most specific first: what is being edited, then what
/// this model was last launched with, then the defaults, then the built-in ones.
///
/// Whole profiles rather than a merge. Taking the context from one and the cache types
/// from another produces a combination nobody chose and the screen cannot explain.
pub fn seed(
    draft: Option<Profile>,
    remembered: Option<Profile>,
    defaults: Option<Profile>,
) -> Profile {
    draft.or(remembered).or(defaults).unwrap_or_default()
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
    pub fn check_raw_args(&self) -> Result<(), String> {
        for arg in &self.raw_args {
            let flag = arg.trim().split('=').next().unwrap_or("");
            for (owned, field) in OWNED_FLAGS {
                if flag == owned {
                    return Err(format!(
                        "{owned} belongs to the {field} field, not to extra arguments. \
                         llama-server takes the last value it is given, so a second one \
                         here only makes the launch disagree with what the form shows."
                    ));
                }
            }
        }
        Ok(())
    }

    /// Renders argv gated on what the installed build actually accepts. `--metrics` is
    /// added silently because the telemetry view depends on it and it costs nothing.
    pub fn args(&self, model_path: &str, caps: &Capabilities) -> Vec<String> {
        let mut args: Vec<String> = vec!["-m".into(), model_path.into()];

        if !self.alias.trim().is_empty() {
            args.push("--alias".into());
            args.push(self.alias.clone());
        }

        // A build without the fitter is given the explicit values this app used before
        // it existed. Omitting them there fits nothing: llama.cpp falls back to the
        // model's whole trained context — 262,144 tokens on files already on disk — and
        // allocates against it with nothing to stop it.
        let fits = caps.has("--fit");

        if self.ctx != AUTO_CTX {
            args.push("-c".into());
            args.push(self.ctx.to_string());
        } else if !fits {
            args.push("-c".into());
            args.push(DEFAULT_CTX.to_string());
        }

        if self.ngl != AUTO_NGL {
            args.push("-ngl".into());
            args.push(self.ngl.clone());
        } else if !fits {
            args.push("-ngl".into());
            args.push("all".into());
        }

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

        // Last, so they win. `check_raw_args` already refuses these, but that is a
        // blocklist against an upstream that adds flags faster than this app tracks them,
        // and where the server binds is the one thing that must not depend on it.
        args.push("--host".into());
        args.push(self.host.clone());
        args.push("--port".into());
        args.push(self.port.to_string());

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
    /// The form opens on the most specific settings there are. Defaults are a starting
    /// point for a model nobody has launched yet — they never overrule what a model was
    /// actually launched with, which is the whole reason `last_used` exists.
    #[test]
    fn a_form_opens_on_the_most_specific_settings_there_are() {
        let defaults = Profile {
            ctx: 8192,
            ngl: "24".into(),
            parallel: 4,
            ..Default::default()
        };
        let remembered = Profile {
            ctx: 32768,
            ..Default::default()
        };
        let draft = Profile {
            ctx: 4096,
            ..Default::default()
        };

        assert_eq!(
            seed(None, None, None).ctx,
            AUTO_CTX,
            "with nothing stored a model fits itself to memory rather than to a guess"
        );
        assert_eq!(
            seed(None, None, Some(defaults.clone())).ctx,
            8192,
            "a model never launched opens on the defaults"
        );
        assert_eq!(
            seed(None, Some(remembered.clone()), Some(defaults.clone())).ctx,
            32768,
            "a model that has been launched opens on its own last launch, not the defaults"
        );
        assert_eq!(
            seed(
                Some(draft.clone()),
                Some(remembered.clone()),
                Some(defaults.clone())
            )
            .ctx,
            4096,
            "what the user is editing right now beats everything stored"
        );

        // Whole profiles, not a merge. Half of one set of settings and half of another is
        // a combination nobody chose and nobody can see.
        let seeded = seed(None, None, Some(defaults));
        assert_eq!(seeded.ngl, "24");
        assert_eq!(seeded.parallel, 4);
        let launched = seed(None, Some(remembered), None);
        assert_eq!(
            launched.ngl, AUTO_NGL,
            "an unset field falls back within its own profile, not across to another"
        );
    }

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

    /// The installed build. `full_caps` deliberately lacks `--fit`, so it stands for a
    /// build from before the fitter existed.
    fn fitting_caps() -> Capabilities {
        let mut c = full_caps();
        c.flags.insert("--fit".to_string());
        c
    }

    #[test]
    fn auto_omits_the_flags_the_fitter_sizes() {
        let profile = Profile {
            ctx: AUTO_CTX,
            ngl: AUTO_NGL.into(),
            ..Default::default()
        };
        let args = profile.args("/m.gguf", &fitting_caps());

        assert!(!args.iter().any(|a| a == "-c"), "no context: {args:?}");
        assert!(!args.iter().any(|a| a == "-ngl"), "no offload: {args:?}");
    }

    /// The one that matters. Omitting `-c` on a build with no fitter does not fit
    /// anything — llama.cpp takes the model's whole trained context, 262,144 tokens on
    /// files already on this disk, and allocates against it unchecked.
    #[test]
    fn auto_falls_back_to_explicit_values_where_the_build_cannot_fit() {
        let profile = Profile {
            ctx: AUTO_CTX,
            ngl: AUTO_NGL.into(),
            ..Default::default()
        };
        let args = profile.args("/m.gguf", &full_caps());

        let ctx = args
            .iter()
            .position(|a| a == "-c")
            .expect("a context is passed");
        assert_eq!(args[ctx + 1], DEFAULT_CTX.to_string());
        let ngl = args
            .iter()
            .position(|a| a == "-ngl")
            .expect("an offload is passed");
        assert_eq!(args[ngl + 1], "all");
    }

    #[test]
    fn an_explicit_choice_is_passed_whatever_the_build_can_do() {
        let profile = Profile {
            ctx: 32768,
            ngl: "24".into(),
            ..Default::default()
        };
        for caps in [full_caps(), fitting_caps()] {
            let args = profile.args("/m.gguf", &caps);
            let ctx = args.iter().position(|a| a == "-c").expect("-c");
            assert_eq!(args[ctx + 1], "32768");
            let ngl = args.iter().position(|a| a == "-ngl").expect("-ngl");
            assert_eq!(args[ngl + 1], "24");
        }
    }

    #[test]
    fn a_profile_written_without_a_context_reads_as_auto() {
        let profile: Profile = serde_json::from_str("{}").expect("an empty profile");
        assert_eq!(profile.ctx, AUTO_CTX);
        assert_eq!(profile.ngl, AUTO_NGL);
    }

    #[test]
    fn a_stored_number_is_not_turned_into_auto() {
        let profile: Profile =
            serde_json::from_str(r#"{"ctx":65536,"ngl":"all"}"#).expect("a stored profile");
        assert_eq!(profile.ctx, 65536);
        assert_eq!(profile.ngl, "all");
    }

    #[test]
    fn renders_the_expected_command() {
        let profile = Profile {
            alias: "qwen3.6-35b-a3b".into(),
            ..Profile::default()
        };
        // `full_caps` has no fitter, so a default profile renders the fallback values.
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

        // The same profile on the installed build: the two flags are simply absent, and
        // what is shown is what will run.
        let fitted = render_command(
            "llama-server",
            &profile.args("/Users/me/models/Q.gguf", &fitting_caps()),
        );
        assert!(!fitted.contains("-c "), "no context in {fitted}");
        assert!(!fitted.contains("-ngl "), "no offload in {fitted}");
        assert!(fitted.contains("-np 1"));
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

    fn with_raw(args: &[&str]) -> Profile {
        Profile {
            raw_args: args.iter().map(|a| a.to_string()).collect(),
            ..Profile::default()
        }
    }

    #[test]
    fn raw_args_may_not_set_what_the_app_owns() {
        for arg in ["--alias", "--host", "--port"] {
            let message = with_raw(&[arg, "0.0.0.0"])
                .check_raw_args()
                .expect_err("must be refused");
            assert!(message.contains(arg), "names the flag: {message}");
        }

        let host = with_raw(&["--host", "0.0.0.0"])
            .check_raw_args()
            .expect_err("must be refused");
        assert!(
            host.contains("Host"),
            "names the field that owns it: {host}"
        );

        let alias = with_raw(&["--alias", "qwen"])
            .check_raw_args()
            .expect_err("the form already has an Alias field");
        assert!(
            alias.contains("Alias"),
            "names the field that owns it: {alias}"
        );
    }

    #[test]
    fn the_equals_form_is_refused_too() {
        let message = with_raw(&["--host=0.0.0.0"])
            .check_raw_args()
            .expect_err("--host=value is the same flag");
        assert!(message.contains("--host"));
    }

    #[test]
    fn the_app_binding_wins_even_if_a_raw_arg_slips_past_the_guard() {
        let profile = with_raw(&["--host", "0.0.0.0", "--port", "1234"]);
        let args = profile.args("/m.gguf", &full_caps());

        let last_host = args.iter().rposition(|a| a == "--host").expect("--host");
        let last_port = args.iter().rposition(|a| a == "--port").expect("--port");
        assert_eq!(
            args.get(last_host + 1).map(String::as_str),
            Some("127.0.0.1")
        );
        assert_eq!(args.get(last_port + 1).map(String::as_str), Some("8888"));
    }

    #[test]
    fn a_leading_space_does_not_smuggle_an_owned_flag_past() {
        with_raw(&[" --host", "0.0.0.0"])
            .check_raw_args()
            .expect_err("whitespace is not a different flag");
    }

    #[test]
    fn flags_that_merely_start_the_same_are_not_blocked() {
        with_raw(&["--no-host", "--reuse-port"])
            .check_raw_args()
            .expect("real llama-server flags that are not the ones the app owns");
    }

    #[test]
    fn unrelated_raw_args_still_pass() {
        with_raw(&["--threads", "8", "--mlock", "--no-warmup"])
            .check_raw_args()
            .expect("nothing the app owns");
    }

    #[test]
    fn a_value_that_merely_looks_like_an_owned_flag_passes() {
        with_raw(&["--chat-template", "--port"])
            .check_raw_args()
            .expect_err("a bare --port is refused wherever it sits");
        with_raw(&["--chat-template", "host--port"])
            .check_raw_args()
            .expect("substrings are not flags");
    }

    #[test]
    fn alias_defaults_from_display_name() {
        assert_eq!(default_alias("Qwen3.6-35B-A3B"), "qwen3.6-35b-a3b");
        assert_eq!(default_alias("Gemma 4 26B"), "gemma-4-26b");
    }
}
