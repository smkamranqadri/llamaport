//! Reads the Hugging Face model index: a page of repositories, and one repository's files.
//!
//! Everything this returns is untrusted. A repository id comes back from the network and
//! is then spliced into the next URL, so `valid_repo_id` gates that splice; file names go
//! on to `downloads::file_name_for`, which stays the only thing that decides what may land
//! in the models directory.

use std::time::Duration;

use serde::{Deserialize, Serialize};

const HOST: &str = "https://huggingface.co";

/// Never `full=true`, and never `expand=gguf`. Measured 2026-09-03 over 24 rows: this set
/// costs 4,582 bytes, `full=true` costs 78,022, and adding `expand=gguf` costs 271,439 —
/// fifty-nine times, almost all of it `chat_template`, which nothing here reads.
const EXPAND: [&str; 4] = ["downloads", "likes", "lastModified", "gated"];

/// Only descending is ever asked for, which is as well: the API refuses `direction=1` on
/// `trendingScore` outright.
const DIRECTION: &str = "-1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Sort {
    Trending,
    Downloads,
    Likes,
}

impl Sort {
    fn as_param(self) -> &'static str {
        match self {
            Sort::Trending => "trendingScore",
            Sort::Downloads => "downloads",
            Sort::Likes => "likes",
        }
    }
}

/// `lastModified` and `createdAt` are absent on purpose. Both sort, and both return repos
/// with no downloads at all — the top three of each on 2026-09-03 — so neither is a list
/// anybody would want without a popularity floor this app does not have.
#[derive(Debug, Clone)]
pub struct Query {
    pub sort: Sort,
    pub search: Option<String>,
    pub limit: usize,
}

/// The client fetches through this rather than reaching for `ureq` itself, so the parsers
/// and everything built on them run against canned bodies with no network.
pub trait Transport: Send + Sync {
    fn get(&self, url: &str) -> Result<String, String>;
}

pub struct Http {
    agent: ureq::Agent,
}

impl Http {
    pub fn new(timeout: Duration) -> Self {
        Self {
            agent: ureq::AgentBuilder::new()
                .timeout_read(timeout)
                .timeout(timeout * 2)
                .build(),
        }
    }
}

impl Transport for Http {
    fn get(&self, url: &str) -> Result<String, String> {
        match self.agent.get(url).call() {
            Ok(response) => response.into_string().map_err(|e| e.to_string()),
            // 429 is the only status worth naming: the budget is unadvertised — no
            // rate-limit headers come back unauthenticated — so hitting it is the only
            // way anyone learns it is there.
            Err(ureq::Error::Status(429, _)) => {
                Err("Hugging Face is rate limiting this app — try again shortly".into())
            }
            Err(ureq::Error::Status(status, _)) => Err(format!("Hugging Face returned {status}")),
            Err(e) => Err(e.to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Repo {
    pub id: String,
    pub downloads: u64,
    pub likes: u64,
    pub last_modified: Option<String>,
    /// True for `"auto"` and `"manual"` alike. The distinction is about how access is
    /// granted, and this app has no token either way.
    pub gated: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub path: String,
    pub size: u64,
    /// Whether the entry is stored in LFS, which is the condition under which the
    /// transfer's `x-linked-size` and `x-linked-etag` headers exist at all — and the
    /// engine refuses a file with no declared size.
    pub lfs: bool,
}

pub fn list(transport: &dyn Transport, query: &Query) -> Result<Vec<Repo>, String> {
    parse_listing(&transport.get(&listing_url(query))?)
}

pub fn tree(transport: &dyn Transport, repo: &str) -> Result<Vec<Entry>, String> {
    parse_tree(&transport.get(&tree_url(repo)?)?)
}

pub fn listing_url(query: &Query) -> String {
    let mut url = format!(
        "{HOST}/api/models?filter=gguf&sort={}&direction={DIRECTION}&limit={}",
        query.sort.as_param(),
        query.limit.clamp(1, 100)
    );
    for field in EXPAND {
        url.push_str("&expand=");
        url.push_str(field);
    }
    if let Some(search) = query.search.as_deref().map(str::trim) {
        if !search.is_empty() {
            url.push_str("&search=");
            url.push_str(&encoded(search));
        }
    }
    url
}

/// `recursive=true` is not optional: quants live under `BF16/` and `MTP/` as readily as at
/// the root, and a non-recursive listing reports those as directories and stops.
pub fn tree_url(repo: &str) -> Result<String, String> {
    if !valid_repo_id(repo) {
        return Err(format!("{repo} is not a repository id"));
    }
    Ok(format!("{HOST}/api/models/{repo}/tree/main?recursive=true"))
}

/// The download URL for one file, in the shape `downloads::file_name_for` already accepts:
/// an allowlisted host, `/resolve/` in the path, and a name it takes from the last segment.
/// Nothing here is a second validator — that one stays the only gate.
pub fn download_url(repo: &str, path: &str) -> Result<String, String> {
    if !valid_repo_id(repo) {
        return Err(format!("{repo} is not a repository id"));
    }
    if !valid_tree_path(path) {
        return Err(format!("{path} is not a file in a repository"));
    }
    Ok(format!("{HOST}/{repo}/resolve/main/{path}"))
}

/// Owner and name, and nothing that could steer the URL it is spliced into. A `?` would
/// append a parameter, a `..` would climb, and either arrives from the network for free.
fn valid_repo_id(repo: &str) -> bool {
    let Some((owner, name)) = repo.split_once('/') else {
        return false;
    };
    [owner, name].iter().all(|segment| {
        !segment.is_empty()
            && *segment != "."
            && *segment != ".."
            && segment
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    })
}

/// A tree path may carry directories — `BF16/model-00001-of-00002.gguf` — so slashes are
/// allowed and traversal is not.
fn valid_tree_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && path.split('/').all(|segment| {
            !segment.is_empty()
                && segment != "."
                && segment != ".."
                && segment
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
        })
}

fn encoded(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// `false`, `"auto"` or `"manual"` — one field with two JSON types, so it is read as either.
#[derive(Deserialize)]
#[serde(untagged)]
enum RawGated {
    Flag(bool),
    Mode(String),
}

impl RawGated {
    fn is_gated(&self) -> bool {
        match self {
            RawGated::Flag(flag) => *flag,
            RawGated::Mode(mode) => !mode.eq_ignore_ascii_case("false"),
        }
    }
}

#[derive(Deserialize)]
struct RawRepo {
    id: String,
    #[serde(default)]
    downloads: u64,
    #[serde(default)]
    likes: u64,
    #[serde(default, rename = "lastModified")]
    last_modified: Option<String>,
    #[serde(default)]
    gated: Option<RawGated>,
}

#[derive(Deserialize)]
struct RawLfs {
    size: u64,
}

#[derive(Deserialize)]
struct RawEntry {
    #[serde(rename = "type")]
    kind: String,
    path: String,
    #[serde(default)]
    size: u64,
    #[serde(default)]
    lfs: Option<RawLfs>,
}

/// A repository whose id is not one is dropped rather than failing the page: one hostile or
/// malformed row should not cost the reader the other twenty-three.
pub fn parse_listing(body: &str) -> Result<Vec<Repo>, String> {
    let raw: Vec<RawRepo> = serde_json::from_str(body).map_err(|e| e.to_string())?;
    Ok(raw
        .into_iter()
        .filter(|repo| valid_repo_id(&repo.id))
        .map(|repo| Repo {
            id: repo.id,
            downloads: repo.downloads,
            likes: repo.likes,
            last_modified: repo.last_modified,
            gated: repo.gated.is_some_and(|gated| gated.is_gated()),
        })
        .collect())
}

/// Files only, and only those with a name this app could act on. Directories carry a size
/// of zero and would otherwise read as empty files.
pub fn parse_tree(body: &str) -> Result<Vec<Entry>, String> {
    let raw: Vec<RawEntry> = serde_json::from_str(body).map_err(|e| e.to_string())?;
    Ok(raw
        .into_iter()
        .filter(|entry| entry.kind == "file" && valid_tree_path(&entry.path))
        .map(|entry| Entry {
            size: entry.lfs.as_ref().map_or(entry.size, |lfs| lfs.size),
            lfs: entry.lfs.is_some(),
            path: entry.path,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Canned(&'static str);

    impl Transport for Canned {
        fn get(&self, _: &str) -> Result<String, String> {
            Ok(self.0.to_string())
        }
    }

    fn query(sort: Sort) -> Query {
        Query {
            sort,
            search: None,
            limit: 24,
        }
    }

    #[test]
    fn a_listing_url_asks_for_the_four_cheap_fields_and_never_the_expensive_one() {
        let url = listing_url(&query(Sort::Trending));
        assert!(url.contains("sort=trendingScore"), "{url}");
        assert!(url.contains("direction=-1"), "{url}");
        assert!(url.contains("filter=gguf"), "{url}");
        for field in EXPAND {
            assert!(url.contains(&format!("expand={field}")), "{url}");
        }
        assert!(!url.contains("expand=gguf"), "{url}");
        assert!(!url.contains("full=true"), "{url}");
    }

    #[test]
    fn a_search_term_is_encoded_rather_than_pasted() {
        let url = listing_url(&Query {
            search: Some("qwen coder&limit=1".into()),
            ..query(Sort::Downloads)
        });
        assert!(url.contains("search=qwen%20coder%26limit%3D1"), "{url}");
        assert_eq!(url.matches("limit=").count(), 1, "{url}");
    }

    #[test]
    fn a_blank_search_is_left_off_rather_than_sent_empty() {
        let url = listing_url(&Query {
            search: Some("   ".into()),
            ..query(Sort::Likes)
        });
        assert!(!url.contains("search="), "{url}");
    }

    #[test]
    fn a_repository_id_cannot_steer_the_url_it_is_spliced_into() {
        for hostile in [
            "../../api/models",
            "owner/../../etc",
            "owner/name?limit=1",
            "owner",
            "owner/name/extra",
            "/name",
            "owner/",
            "own er/name",
        ] {
            assert!(tree_url(hostile).is_err(), "{hostile} was accepted");
            assert!(download_url(hostile, "a.gguf").is_err(), "{hostile}");
        }
        assert!(tree_url("unsloth/Qwen3.8-27B-GGUF").is_ok());
    }

    #[test]
    fn a_download_url_is_built_in_the_shape_the_existing_validator_accepts() {
        let url = download_url("unsloth/Qwen3.8-27B-GGUF", "BF16/model-00001-of-00002.gguf")
            .expect("a well-formed pair");
        assert_eq!(
            url,
            "https://huggingface.co/unsloth/Qwen3.8-27B-GGUF/resolve/main/BF16/model-00001-of-00002.gguf"
        );
        assert_eq!(
            crate::downloads::file_name_for(&url),
            Ok("model-00001-of-00002.gguf".to_string()),
            "the one gate on what may land in the models directory has to accept this"
        );
    }

    #[test]
    fn a_file_path_that_climbs_is_refused() {
        for hostile in ["../evil.gguf", "a/../../evil.gguf", "/etc/passwd", ""] {
            assert!(download_url("owner/name", hostile).is_err(), "{hostile}");
        }
    }

    #[test]
    fn gated_reads_the_three_shapes_the_api_actually_returns() {
        let repos = parse_listing(
            r#"[{"id":"a/one","gated":false},
                {"id":"a/two","gated":"auto"},
                {"id":"a/three","gated":"manual"},
                {"id":"a/four"}]"#,
        )
        .expect("a listing");
        assert_eq!(
            repos.iter().map(|r| r.gated).collect::<Vec<_>>(),
            [false, true, true, false]
        );
    }

    #[test]
    fn a_listing_drops_a_hostile_row_rather_than_failing_the_page() {
        let repos = parse_listing(
            r#"[{"id":"a/one","downloads":5,"likes":2,"lastModified":"2026-09-01T00:00:00.000Z"},
                {"id":"../../evil"},
                {"id":"b/two"}]"#,
        )
        .expect("a listing");
        assert_eq!(
            repos.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
            ["a/one", "b/two"]
        );
        assert_eq!(repos[0].downloads, 5);
        assert_eq!(repos[0].likes, 2);
        assert_eq!(
            repos[0].last_modified.as_deref(),
            Some("2026-09-01T00:00:00.000Z")
        );
    }

    #[test]
    fn a_tree_keeps_files_drops_directories_and_prefers_the_lfs_size() {
        let entries = parse_tree(
            r#"[{"type":"directory","path":"BF16","size":0},
                {"type":"file","path":"model-Q4_K_M.gguf","size":136,
                 "lfs":{"size":8500000000}},
                {"type":"file","path":".gitattributes","size":4175},
                {"type":"file","path":"../escape.gguf","size":1,"lfs":{"size":1}}]"#,
        )
        .expect("a tree");
        assert_eq!(
            entries
                .iter()
                .map(|e| (e.path.as_str(), e.size, e.lfs))
                .collect::<Vec<_>>(),
            [
                ("model-Q4_K_M.gguf", 8_500_000_000, true),
                (".gitattributes", 4175, false),
            ]
        );
    }

    #[test]
    fn a_body_that_is_not_a_listing_is_an_error_rather_than_an_empty_page() {
        assert!(parse_listing(r#"{"error":"Invalid sort parameter"}"#).is_err());
        assert!(parse_tree("not json at all").is_err());
    }

    #[test]
    fn the_client_composes_a_fetch_and_a_parse() {
        let repos = list(
            &Canned(r#"[{"id":"a/one","downloads":9,"likes":1,"gated":"auto"}]"#),
            &query(Sort::Trending),
        )
        .expect("a listing");
        assert_eq!(repos.len(), 1);
        assert!(repos[0].gated);

        let entries = tree(
            &Canned(r#"[{"type":"file","path":"a.gguf","size":7,"lfs":{"size":7}}]"#),
            "owner/name",
        )
        .expect("a tree");
        assert_eq!(entries.len(), 1);
    }
}
