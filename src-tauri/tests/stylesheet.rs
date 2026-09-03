//! Guards the one defect class this stylesheet has actually shipped.
//!
//! The redesign painted `.tune-head`, `.tune-note` and `.tune-row.is-unranked` with
//! `--muted` and `.tune-row.is-fastest` with `--surface-raised`, neither of which App.css
//! has ever defined, so three greys rendered as body text and a highlight as nothing. CSS
//! fails silently here: an undefined custom property is not an error, it is an inherited
//! colour that looks almost right. Nothing else in this project can catch that — there is
//! no frontend test framework — so it is caught here.
//!
//! Writing `--muted` a second time is what prompted this, four months after the first.

use std::collections::HashSet;
use std::path::PathBuf;

fn stylesheet() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../src/App.css");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// Every `--name:` declaration, wherever it sits. Matched on the colon rather than on the
/// line start, so a palette written inline counts the same as one written a line at a time.
fn defined(css: &str) -> HashSet<String> {
    let bytes = css.as_bytes();
    let mut names = HashSet::new();
    let mut at = 0;
    while let Some(found) = css[at..].find("--") {
        let start = at + found;
        let before = start.checked_sub(1).map(|i| bytes[i]);
        // A `--` inside `var(--x)` is a read, not a declaration.
        if before.is_some_and(|b| b.is_ascii_alphanumeric() || b == b'(' || b == b'-') {
            at = start + 2;
            continue;
        }
        let rest = &css[start..];
        let end = rest
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
            .unwrap_or(rest.len());
        if rest[end..].starts_with(':') {
            names.insert(rest[..end].to_string());
        }
        at = start + end.max(2);
    }
    names
}

/// Reads with no fallback, which are the only ones that fail silently. `var(--warn, #d29922)`
/// paints the fallback and is a deliberate fixed colour — this project has three of them,
/// recorded in its own notes as ambers and greens no palette moves.
fn read_without_fallback(css: &str) -> HashSet<String> {
    let mut names = HashSet::new();
    let mut rest = css;
    while let Some(at) = rest.find("var(--") {
        let after = &rest[at + 4..];
        let end = after
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
            .unwrap_or(after.len());
        if after[end..].starts_with(')') {
            names.insert(after[..end].to_string());
        }
        rest = &after[end..];
    }
    names
}

#[test]
fn every_custom_property_the_stylesheet_reads_is_one_it_defines() {
    let css = stylesheet();
    let defined = defined(&css);
    let mut missing: Vec<String> = read_without_fallback(&css)
        .into_iter()
        .filter(|name| !defined.contains(name))
        .collect();
    missing.sort();

    assert!(
        missing.is_empty(),
        "App.css reads custom properties it never defines, which CSS will not complain \
         about and the screen will not obviously show: {missing:?}"
    );
}

#[test]
fn the_check_would_notice_a_property_that_is_only_read() {
    let css = ":root { --real: red; }\n.a { color: var(--real); background: var(--imaginary); }";
    let defined = defined(css);
    assert!(
        defined.contains("--real"),
        "a declaration was missed: {defined:?}"
    );
    let read = read_without_fallback(css);
    let missing: Vec<&String> = read
        .iter()
        .filter(|name| !defined.contains(*name))
        .collect();
    assert_eq!(missing, [&"--imaginary".to_string()]);
}

#[test]
fn a_read_with_a_fallback_is_not_a_defect() {
    // CSS paints the fallback, so nothing renders wrong. Three of this project's fixed
    // ambers and greens are written exactly this way on purpose.
    let css = ".a { color: var(--nowhere, #d29922); }";
    assert!(read_without_fallback(css).is_empty());
}
