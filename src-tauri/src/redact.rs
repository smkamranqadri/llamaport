//! Secrets that cannot leak by accident.
//!
//! `Redacted` has no `Display`, its `Debug` and `Serialize` emit a placeholder, and the
//! real value is only reachable through `expose()`. Formatting a struct that contains
//! one — into a log line, an event payload, a diagnostics bundle — therefore cannot
//! print the secret, and a reviewer can find every deliberate use by grepping for
//! `expose`.

use std::fmt;

use serde::{Serialize, Serializer};

pub const PLACEHOLDER: &str = "[redacted]";

/// Flags whose *following* argument is a secret.
const SECRET_VALUE_FLAGS: [&str; 3] = ["--api-key", "--api-key-file", "--ssl-key-file"];

#[derive(Clone, Default, PartialEq, Eq)]
pub struct Redacted(String);

impl Redacted {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// The only way to read the secret. Call sites should be few and obvious.
    pub fn expose(&self) -> &str {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Debug for Redacted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(PLACEHOLDER)
    }
}

impl Serialize for Redacted {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(PLACEHOLDER)
    }
}

/// Replaces secret argument values in a rendered command line or argv.
///
/// Handles both `--api-key VALUE` and `--api-key=VALUE`, because llama.cpp accepts both
/// and a diagnostics bundle must not depend on which one the user typed.
pub fn redact_args(args: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::with_capacity(args.len());
    let mut redact_next = false;

    for arg in args {
        if redact_next {
            out.push(PLACEHOLDER.to_string());
            redact_next = false;
            continue;
        }

        match arg.split_once('=') {
            Some((flag, _)) if SECRET_VALUE_FLAGS.contains(&flag) => {
                out.push(format!("{flag}={PLACEHOLDER}"));
            }
            _ => {
                redact_next = SECRET_VALUE_FLAGS.contains(&arg.as_str());
                out.push(arg.clone());
            }
        }
    }
    out
}

/// Header values that must never appear in logs or diagnostics.
pub fn redact_header(name: &str, value: &str) -> String {
    let sensitive = matches!(
        name.to_ascii_lowercase().as_str(),
        "authorization" | "x-api-key" | "cookie" | "set-cookie" | "proxy-authorization"
    );
    if sensitive {
        PLACEHOLDER.to_string()
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_never_prints_the_secret() {
        let secret = Redacted::new("sk-super-secret");
        assert_eq!(format!("{secret:?}"), PLACEHOLDER);
        assert!(!format!("{secret:?}").contains("sk-"));
    }

    #[test]
    fn serialising_never_emits_the_secret() {
        #[derive(Serialize)]
        struct Holder {
            key: Redacted,
        }

        let json = serde_json::to_string(&Holder {
            key: Redacted::new("sk-super-secret"),
        })
        .expect("serialise");

        assert!(!json.contains("sk-super-secret"), "{json}");
        assert!(json.contains(PLACEHOLDER));
    }

    #[test]
    fn nesting_inside_a_debugged_struct_stays_redacted() {
        #[derive(Debug)]
        #[allow(dead_code)]
        struct Config {
            host: String,
            key: Redacted,
        }

        let printed = format!(
            "{:?}",
            Config {
                host: "127.0.0.1".into(),
                key: Redacted::new("sk-super-secret"),
            }
        );
        assert!(!printed.contains("sk-super-secret"), "{printed}");
    }

    #[test]
    fn the_value_is_still_reachable_when_deliberately_asked_for() {
        assert_eq!(Redacted::new("sk-abc").expose(), "sk-abc");
    }

    #[test]
    fn separated_secret_arguments_are_redacted() {
        let args = vec![
            "-m".to_string(),
            "/models/a.gguf".to_string(),
            "--api-key".to_string(),
            "sk-secret".to_string(),
            "--port".to_string(),
            "8888".to_string(),
        ];
        let redacted = redact_args(&args);

        assert_eq!(redacted[3], PLACEHOLDER);
        assert_eq!(redacted[5], "8888", "ordinary values are untouched");
        assert!(!redacted.join(" ").contains("sk-secret"));
    }

    #[test]
    fn joined_secret_arguments_are_redacted() {
        let args = vec!["--api-key=sk-secret".to_string(), "--port=8888".to_string()];
        let redacted = redact_args(&args);

        assert_eq!(redacted[0], format!("--api-key={PLACEHOLDER}"));
        assert_eq!(redacted[1], "--port=8888");
    }

    #[test]
    fn a_trailing_secret_flag_does_not_panic() {
        let redacted = redact_args(&["--api-key".to_string()]);
        assert_eq!(redacted, vec!["--api-key".to_string()]);
    }

    #[test]
    fn sensitive_headers_are_redacted_case_insensitively() {
        assert_eq!(redact_header("Authorization", "Bearer sk-x"), PLACEHOLDER);
        assert_eq!(redact_header("authorization", "Bearer sk-x"), PLACEHOLDER);
        assert_eq!(redact_header("X-API-Key", "sk-x"), PLACEHOLDER);
        assert_eq!(
            redact_header("Content-Type", "application/json"),
            "application/json"
        );
    }
}
