//! Reads the Hugging Face model index: a page of repositories, and one repository's files.
//!
//! Everything this returns is untrusted. A repository id comes back from the network and
//! is then spliced into the next URL, so `valid_repo_id` gates that splice; file names go
//! on to `downloads::file_name_for`, which stays the only thing that decides what may land
//! in the models directory.

use std::time::Duration;

use serde::{Deserialize, Serialize};

const HOST: &str = "https://huggingface.co";

/// Never `full=true`. **`expand=gguf` was refused here on a byte count and is now
/// included**, which is a reversal on purpose: the architecture exists nowhere else, and
/// without it a row cannot say whether a model is a mixture of experts. The cost was
/// re-measured rather than argued from the bytes — 0.82s to 1.38s over 24 rows, against a
/// page already spending 2.3s fetching trees. `expand=tags` comes with it and is small.
///
/// The byte figures, for the record: this set without `gguf` is 4,582 over 24 rows,
/// `full=true` is 78,022, and adding `gguf` is 271,439 — almost all of it `chat_template`,
/// which nothing here reads and nothing here can decline.
const EXPAND: [&str; 7] = [
    "downloads",
    "likes",
    "lastModified",
    "gated",
    "pipeline_tag",
    "gguf",
    "tags",
];

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
    /// A `rel="next"` URL from an earlier page, followed as given.
    pub cursor: Option<String>,
    /// Billions of parameters, inclusive. Filtered by the API rather than here: its figure
    /// survives the sidecar trap that makes `gguf.total` report 1.86B for a 27B model,
    /// checked 2026-09-03 on a repository that reads 1.86B and still lands in the 20–40B
    /// band.
    pub params: Option<(Option<u32>, Option<u32>)>,
}

/// A body, and where the next page is if there is one.
pub struct Fetched {
    pub body: String,
    /// From `Link: <…>; rel="next"`. The API paginates by opaque cursor rather than by
    /// offset, so this URL is followed verbatim and never rebuilt.
    pub next: Option<String>,
}

/// The client fetches through this rather than reaching for `ureq` itself, so the parsers
/// and everything built on them run against canned bodies with no network.
pub trait Transport: Send + Sync {
    fn get(&self, url: &str) -> Result<Fetched, String>;
}

/// `<https://…>; rel="next"`, and only that relation — the header also carries others.
fn next_link(header: &str) -> Option<String> {
    header.split(',').find_map(|part| {
        if !part.contains("rel=\"next\"") {
            return None;
        }
        let start = part.find('<')? + 1;
        let end = part[start..].find('>')? + start;
        Some(part[start..end].to_string())
    })
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
    fn get(&self, url: &str) -> Result<Fetched, String> {
        match self.agent.get(url).call() {
            Ok(response) => {
                let next = response.header("link").and_then(next_link);
                let body = response.into_string().map_err(|e| e.to_string())?;
                Ok(Fetched { body, next })
            }
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
    pub pipeline_tag: Option<String>,
    /// Read off one file in the repository. That file is sometimes a sidecar — the same
    /// trap that makes `gguf.total` report 1.86B for a 27B model — but a drafter shares
    /// its base architecture, so this survives where the parameter count does not.
    pub architecture: Option<String>,
    pub context_length: Option<u64>,
    /// **Two independent signals, and neither is close to complete on its own.** Measured
    /// over 300 GGUF repositories 2026-09-03: the uploader's `moe` tag is on 35, an
    /// architecture naming MoE covers 34, **only 13 carry both**, and the union is 56.
    /// The tag catches `deepseek4`, `laguna` and `qwen4exp`, whose architectures say
    /// nothing; the architecture catches `unsloth/Qwen3-Coder-30B-A3B-Instruct-GGUF`,
    /// which nobody tagged. Each alone misses about two in five.
    ///
    /// Still never a guess from the repository's name. A tag is a claim its uploader
    /// made and an architecture is a fact read out of the file; both are evidence, where
    /// `-A3B` in a title is only a naming habit. Certainty for a model already on disk
    /// stays [`crate::gguf::Metadata::is_moe`], which reads a real expert count.
    pub moe: bool,
}

fn says_moe(architecture: Option<&str>, tags: &[String]) -> bool {
    if tags.iter().any(|tag| tag.eq_ignore_ascii_case("moe")) {
        return true;
    }
    architecture.is_some_and(|arch| arch.to_ascii_lowercase().contains("moe"))
}

/// Whether `llama-server` could serve this at all.
///
/// **A denylist, and the direction matters.** Over 300 GGUF repositories sampled 2026-09-03
/// across all three sorts, 48 carry *no* pipeline tag at all — `unsloth/Qwen3.8-27B-GGUF`
/// among them — so keeping only the known-good tags would hide some of the best models on
/// the site. What is listed below is what is definitely not a language model: 51 of those
/// 300, one in six, and Discover offered every one of them until an ASR model was
/// downloaded and turned out to have no context length.
///
/// `image-text-to-text` stays: those are vision language models and the server runs them.
pub fn serves_text(pipeline_tag: Option<&str>) -> bool {
    !matches!(
        pipeline_tag,
        Some(
            "automatic-speech-recognition"
                | "audio-classification"
                | "feature-extraction"
                | "sentence-similarity"
                | "token-classification"
                | "reinforcement-learning"
                | "text-to-image"
                | "text-to-speech"
                | "text-to-video"
                | "image-to-image"
                | "image-to-video"
                | "image-text-to-video"
                | "any-to-any"
        )
    )
}

/// What the detail page states, and nothing it cannot back. Every field is optional
/// because every one of them is genuinely absent on some repository.
#[derive(Debug, Clone, Serialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Facts {
    pub id: String,
    pub downloads: u64,
    pub likes: u64,
    pub last_modified: Option<String>,
    pub gated: bool,
    pub license: Option<String>,
    /// Read off one file in the repository, so it is wrong when that file is a sidecar —
    /// a 27B repository whose first GGUF is an MTP drafter reports 1.86B.
    pub params: Option<u64>,
    pub architecture: Option<String>,
    pub context_length: Option<u64>,
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

/// A page of repositories and the cursor that continues it.
pub struct Page {
    pub repos: Vec<Repo>,
    pub next: Option<String>,
}

pub fn list(transport: &dyn Transport, query: &Query) -> Result<Page, String> {
    let fetched = transport.get(&listing_url(query))?;
    Ok(Page {
        repos: parse_listing(&fetched.body)?,
        next: fetched.next,
    })
}

pub fn tree(transport: &dyn Transport, repo: &str) -> Result<Vec<Entry>, String> {
    parse_tree(&transport.get(&tree_url(repo)?)?.body)
}

/// Everything the detail page states about a repository, in one call. `expand=gguf` is
/// affordable here and not in a listing: one repository's chat template is kilobytes, and
/// twenty-four of them were the 271 KB that kept it out of the browse call.
pub fn facts(transport: &dyn Transport, repo: &str) -> Result<Facts, String> {
    if !valid_repo_id(repo) {
        return Err(format!("{repo} is not a repository id"));
    }
    let url = format!(
        "{HOST}/api/models/{repo}?expand=downloads&expand=likes&expand=lastModified\
&expand=gated&expand=gguf&expand=cardData"
    );
    parse_facts(&transport.get(&url)?.body)
}

pub fn listing_url(query: &Query) -> String {
    // The cursor URL already carries the sort, the filter, the expands and the search it
    // was produced for. Rebuilding any of that is how a second page comes back sorted
    // differently from the first.
    if let Some(cursor) = query.cursor.as_deref() {
        return cursor.to_string();
    }
    let mut url = format!(
        "{HOST}/api/models?filter=gguf&sort={}&direction={DIRECTION}&limit={}",
        query.sort.as_param(),
        query.limit.clamp(1, 100)
    );
    for field in EXPAND {
        url.push_str("&expand=");
        url.push_str(field);
    }
    if let Some((min, max)) = query.params {
        let mut terms = Vec::new();
        if let Some(min) = min {
            terms.push(format!("min:{min}B"));
        }
        if let Some(max) = max {
            terms.push(format!("max:{max}B"));
        }
        if !terms.is_empty() {
            url.push_str("&num_parameters=");
            url.push_str(&encoded(&terms.join(",")));
        }
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
    #[serde(default)]
    pipeline_tag: Option<String>,
    #[serde(default)]
    gguf: Option<RawGguf>,
    #[serde(default)]
    tags: Vec<String>,
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
            pipeline_tag: repo.pipeline_tag,
            moe: says_moe(
                repo.gguf.as_ref().and_then(|g| g.architecture.as_deref()),
                &repo.tags,
            ),
            architecture: repo.gguf.as_ref().and_then(|g| g.architecture.clone()),
            context_length: repo.gguf.as_ref().and_then(|g| g.context_length),
        })
        .collect())
}

#[derive(Deserialize)]
struct RawGguf {
    #[serde(default)]
    total: Option<u64>,
    #[serde(default)]
    architecture: Option<String>,
    #[serde(default)]
    context_length: Option<u64>,
}

#[derive(Deserialize)]
struct RawCard {
    #[serde(default)]
    license: Option<String>,
}

#[derive(Deserialize)]
struct RawFacts {
    id: String,
    #[serde(default)]
    downloads: u64,
    #[serde(default)]
    likes: u64,
    #[serde(default, rename = "lastModified")]
    last_modified: Option<String>,
    #[serde(default)]
    gated: Option<RawGated>,
    #[serde(default)]
    gguf: Option<RawGguf>,
    #[serde(default, rename = "cardData")]
    card: Option<RawCard>,
}

pub fn parse_facts(body: &str) -> Result<Facts, String> {
    let raw: RawFacts = serde_json::from_str(body).map_err(|e| e.to_string())?;
    if !valid_repo_id(&raw.id) {
        return Err(format!("{} is not a repository id", raw.id));
    }
    Ok(Facts {
        id: raw.id,
        downloads: raw.downloads,
        likes: raw.likes,
        last_modified: raw.last_modified,
        gated: raw.gated.is_some_and(|gated| gated.is_gated()),
        license: raw.card.and_then(|card| card.license),
        params: raw.gguf.as_ref().and_then(|gguf| gguf.total),
        architecture: raw.gguf.as_ref().and_then(|gguf| gguf.architecture.clone()),
        context_length: raw.gguf.as_ref().and_then(|gguf| gguf.context_length),
    })
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
        fn get(&self, _: &str) -> Result<Fetched, String> {
            Ok(Fetched {
                body: self.0.to_string(),
                next: None,
            })
        }
    }

    fn query(sort: Sort) -> Query {
        Query {
            sort,
            search: None,
            limit: 24,
            cursor: None,
            params: None,
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
        // expand=gguf is required, not refused: the architecture behind the MoE mark
        // comes from nowhere else. This assertion used to say the opposite, and it went
        // on passing for a commit in which the field was silently absent and every row
        // came back unmarked.
        assert!(url.contains("expand=gguf"), "{url}");
        assert!(url.contains("expand=tags"), "{url}");
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
    fn a_cursor_is_followed_verbatim_rather_than_rebuilt() {
        // The cursor already encodes the sort, the filter, the expands and the search it
        // was issued for. Rebuilding any of it is how page two comes back sorted unlike
        // page one.
        let cursor = "https://huggingface.co/api/models?filter=gguf&cursor=OPAQUE%3D%3D";
        let url = listing_url(&Query {
            cursor: Some(cursor.into()),
            search: Some("ignored".into()),
            ..query(Sort::Likes)
        });
        assert_eq!(url, cursor);
    }

    #[test]
    fn the_next_page_is_read_off_the_link_header() {
        let header = "<https://huggingface.co/api/models?cursor=ABC>; rel=\"next\"";
        assert_eq!(
            next_link(header).as_deref(),
            Some("https://huggingface.co/api/models?cursor=ABC")
        );
        assert_eq!(next_link("<https://x/prev>; rel=\"prev\""), None);
        assert_eq!(next_link("nonsense"), None);
    }

    #[test]
    fn facts_read_what_the_detail_page_states_and_nothing_more() {
        let facts = parse_facts(
            r#"{"id":"unsloth/Model-GGUF","downloads":9553042,"likes":3406,
                "lastModified":"2026-08-20T12:04:25.000Z","gated":false,
                "gguf":{"total":27320697856,"architecture":"qwen35","context_length":262144},
                "cardData":{"license":"apache-2.0"}}"#,
        )
        .expect("facts");
        assert_eq!(facts.params, Some(27_320_697_856));
        assert_eq!(facts.architecture.as_deref(), Some("qwen35"));
        assert_eq!(facts.context_length, Some(262_144));
        assert_eq!(facts.license.as_deref(), Some("apache-2.0"));
        assert!(!facts.gated);
    }

    #[test]
    fn facts_survive_a_repository_that_declares_almost_nothing() {
        let facts = parse_facts(r#"{"id":"a/bare"}"#).expect("facts");
        assert_eq!(facts.params, None);
        assert_eq!(facts.license, None);
        assert_eq!(facts.downloads, 0);
        assert!(parse_facts(r#"{"id":"../evil"}"#).is_err());
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

    fn tags(list: &[&str]) -> Vec<String> {
        list.iter().map(|tag| tag.to_string()).collect()
    }

    #[test]
    fn a_mixture_of_experts_is_read_from_two_signals_because_neither_is_complete() {
        // Architecture alone. Nobody tagged unsloth/Qwen3-Coder-30B-A3B-Instruct-GGUF.
        assert!(says_moe(Some("qwen3moe"), &[]));
        assert!(says_moe(Some("QWEN35MOE"), &[]));
        // Tag alone. qwen4exp is a 131B-A6B mixture of experts whose architecture is silent.
        assert!(says_moe(Some("qwen4exp"), &tags(&["gguf", "moe"])));
        assert!(says_moe(Some("deepseek4"), &tags(&["moe"])));
        assert!(says_moe(None, &tags(&["MoE"])));

        assert!(!says_moe(
            Some("qwen35"),
            &tags(&["gguf", "conversational"])
        ));
        assert!(!says_moe(None, &[]));
        // Not from the name: -A3B is a naming habit, not a claim anybody made.
        assert!(!says_moe(Some("llama"), &tags(&["Qwen3-Coder-30B-A3B"])));
    }

    #[test]
    fn a_parameter_band_is_asked_of_the_api_rather_than_filtered_here() {
        let banded = |params| {
            listing_url(&Query {
                params,
                ..query(Sort::Downloads)
            })
        };
        assert!(banded(Some((Some(20), Some(40)))).contains("num_parameters=min%3A20B%2Cmax%3A40B"));
        assert!(banded(Some((Some(40), None))).contains("num_parameters=min%3A40B"));
        assert!(banded(Some((None, Some(4)))).contains("num_parameters=max%3A4B"));
        assert!(!banded(None).contains("num_parameters"));
        assert!(!banded(Some((None, None))).contains("num_parameters"));
    }

    #[test]
    fn what_llama_server_cannot_serve_is_named_rather_than_guessed() {
        // The direction is the point: an unknown or missing tag is kept, because 48 of 300
        // sampled repositories carry none and some of those are the best models listed.
        assert!(serves_text(None));
        assert!(serves_text(Some("text-generation")));
        assert!(serves_text(Some("image-text-to-text")));
        assert!(serves_text(Some("something-invented-next-year")));

        assert!(!serves_text(Some("automatic-speech-recognition")));
        assert!(!serves_text(Some("feature-extraction")));
        assert!(!serves_text(Some("text-to-image")));
        assert!(!serves_text(Some("any-to-any")));
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
        let page = list(
            &Canned(r#"[{"id":"a/one","downloads":9,"likes":1,"gated":"auto"}]"#),
            &query(Sort::Trending),
        )
        .expect("a listing");
        assert_eq!(page.repos.len(), 1);
        assert!(page.repos[0].gated);

        let entries = tree(
            &Canned(r#"[{"type":"file","path":"a.gguf","size":7,"lfs":{"size":7}}]"#),
            "owner/name",
        )
        .expect("a tree");
        assert_eq!(entries.len(), 1);
    }
}
