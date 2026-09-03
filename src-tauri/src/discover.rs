//! Assembles the Discover screen: one listing call, then a file tree per repository.
//!
//! The listing is cheap and the trees are not — sizes exist nowhere else, so a row cannot
//! say what it would download until one has been fetched. Twenty-four of them run at 2.3
//! seconds across six lanes and 13.7 seconds in a queue, measured 2026-09-03, which is why
//! this fans out rather than looping.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;

use crate::hub::{self, Entry, Facts, Http, Query, Sort};
use crate::quant::{self, Candidate, Pick};
use crate::store;

const LANES: usize = 6;
const TIMEOUT: Duration = Duration::from_secs(20);
pub const PAGE: usize = 24;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Row {
    pub id: String,
    pub owner: String,
    pub name: String,
    pub downloads: u64,
    pub likes: u64,
    pub last_modified: Option<String>,
    pub gated: bool,
    pub architecture: Option<String>,
    /// True only where the architecture names it; see [`crate::hub::Repo::moe`].
    pub moe: bool,
    /// Absent for a gated repository, one holding no quantisation this app can take, and
    /// one whose tree could not be read. `note` says which.
    pub pick: Option<Pick>,
    pub quants: usize,
    pub note: Option<String>,
}

/// Trees, keyed on repository. The three sorts overlap heavily and the chips re-filter the
/// same rows, so without this every chip press pays the two and a half seconds again.
///
/// It holds the tree rather than the pick on purpose: a pick is only true against a
/// ceiling, and the ceiling appears the moment `llama-server` is found.
#[derive(Default)]
pub struct Trees {
    seen: Mutex<HashMap<String, Vec<Entry>>>,
    /// Owner to picture, `None` remembered as firmly as a hit. An owner with no avatar
    /// would otherwise be asked for again on every page they appear on, and the ones
    /// without pictures are exactly the ones that publish many repositories.
    avatars: Mutex<HashMap<String, Option<String>>>,
}

impl Trees {
    fn known(&self, repo: &str) -> Option<Vec<Entry>> {
        self.seen.lock().expect("trees lock").get(repo).cloned()
    }

    fn remember(&self, repo: &str, entries: Vec<Entry>) {
        self.seen
            .lock()
            .expect("trees lock")
            .insert(repo.to_string(), entries);
    }

    /// One lookup per owner, ever. Fifteen distinct owners appear in a page of twenty-four
    /// and the same handful publish most of what trends, so this is nearly always a hit —
    /// in memory within a session, and on disk across them.
    ///
    /// **A picture is worth keeping and never worth waiting for.** Anything that goes wrong
    /// reading or writing the cache is ignored: the answer is still correct, it just costs
    /// a request.
    pub fn avatar(&self, owner: &str) -> Option<String> {
        if let Some(known) = self.avatars.lock().expect("avatars lock").get(owner) {
            return known.clone();
        }
        let path = cache_path(owner);
        if let Some(stored) = path.as_deref().and_then(cached) {
            let found = (!stored.is_empty()).then_some(stored);
            self.avatars
                .lock()
                .expect("avatars lock")
                .insert(owner.to_string(), found.clone());
            return found;
        }

        let found = hub::avatar(&Http::new(TIMEOUT), owner);
        if let Some(path) = path {
            store_cached(&path, found.as_deref().unwrap_or(""));
        }
        self.avatars
            .lock()
            .expect("avatars lock")
            .insert(owner.to_string(), found.clone());
        found
    }
}

/// A page of rows and the cursor that continues it. `next` is `None` at the end of the
/// listing, which is what turns Load more off.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    pub rows: Vec<Row>,
    pub next: Option<String>,
}

/// One repository in full: the facts, and every quantisation with its own verdict.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Detail {
    pub facts: Facts,
    pub owner: String,
    pub name: String,
    pub quants: Vec<Offer>,
    pub note: Option<String>,
}

/// A quantisation as the detail page offers it: what it is, what it costs, and whether the
/// weights alone clear this machine. Never a claim that a launch will be good.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Offer {
    pub candidate: Candidate,
    pub fits: Option<bool>,
    /// What the app would have taken on its own, so the list says which one that was.
    pub picked: bool,
}

/// What the reader has narrowed the listing to. Kept together because both are decided
/// before a single tree is fetched, which is what makes them cheap: a filtered-out row
/// never costs a call.
#[derive(Debug, Clone, Default)]
pub struct Narrow {
    /// Billions of parameters, inclusive, asked of the API.
    pub params: Option<(Option<u32>, Option<u32>)>,
    /// Applied here rather than by the API. `filter=gguf,moe` would return only the
    /// tagged ones and drop 21 of the 56 a 300-repository sample found.
    pub only_moe: bool,
}

pub fn browse(
    trees: &Trees,
    sort: Sort,
    search: Option<String>,
    cursor: Option<String>,
    narrow: &Narrow,
    ceiling: Option<u64>,
) -> Result<Page, String> {
    let http = Http::new(TIMEOUT);
    let page = hub::list(
        &http,
        &Query {
            sort,
            search,
            limit: PAGE,
            cursor,
            params: narrow.params,
        },
    )?;
    // An ASR model, an embedding model and an image generator are all GGUF files this app
    // can download and `llama-server` cannot serve. One of them was downloaded before this
    // existed and it had no context length.
    let repos: Vec<hub::Repo> = page
        .repos
        .into_iter()
        .filter(|repo| hub::serves_text(repo.pipeline_tag.as_deref()))
        .filter(|repo| !narrow.only_moe || repo.moe)
        .collect();

    // A gated repository is skipped rather than fetched. Its tree answers 200 and its
    // files answer 401, so a size read here would sit above a Download that cannot work.
    let wanted: Vec<String> = repos
        .iter()
        .filter(|repo| !repo.gated && trees.known(&repo.id).is_none())
        .map(|repo| repo.id.clone())
        .collect();

    let fetched = fan_out(&http, &wanted);
    for (repo, entries) in fetched {
        if let Ok(entries) = entries {
            trees.remember(&repo, entries);
        }
    }

    Ok(Page {
        rows: repos
            .into_iter()
            .map(|repo| row(trees, repo, ceiling))
            .collect(),
        next: page.next,
    })
}

/// The detail page. One facts call, and the tree it very likely already has from the row
/// the reader clicked.
pub fn detail(trees: &Trees, repo: &str, ceiling: Option<u64>) -> Result<Detail, String> {
    let http = Http::new(TIMEOUT);
    let facts = hub::facts(&http, repo)?;

    let (owner, name) = match facts.id.split_once('/') {
        Some((owner, name)) => (owner.to_string(), name.to_string()),
        None => (String::new(), facts.id.clone()),
    };

    if facts.gated {
        return Ok(Detail {
            facts,
            owner,
            name,
            quants: Vec::new(),
            note: Some("gated on Hugging Face — accept its terms there first".into()),
        });
    }

    let entries = match trees.known(repo) {
        Some(entries) => entries,
        None => {
            let entries = hub::tree(&http, repo)?;
            trees.remember(repo, entries.clone());
            entries
        }
    };

    let quants = offers(&entries, ceiling);
    let note = quants
        .is_empty()
        .then(|| "no quantisation this app can download".to_string());

    Ok(Detail {
        facts,
        owner,
        name,
        quants,
        note,
    })
}

/// How long a picture is kept before it is asked for again. Long, because an avatar
/// changing is cosmetic and nobody can tell from the row that it has.
const AVATAR_TTL: Duration = Duration::from_secs(30 * 24 * 60 * 60);

/// Whether a cached picture of this age is still worth using. Its own function so the rule
/// can be checked without waiting a month or reaching for a crate to forge a timestamp.
fn fresh(age: Duration) -> bool {
    age <= AVATAR_TTL
}

/// An owner's cached picture, `Some("")` for a remembered miss, `None` for absent or aged
/// out. The caller cannot tell the last two apart and does not need to: both mean ask.
fn cached(path: &Path) -> Option<String> {
    let age = path.metadata().ok()?.modified().ok()?.elapsed().ok()?;
    if !fresh(age) {
        return None;
    }
    std::fs::read_to_string(path).ok()
}

fn store_cached(path: &Path, uri: &str) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, uri);
}

/// Where an owner's picture is kept, or `None` if the name is not one file name.
///
/// **Checked on the name, never on the joined path.** `Path::starts_with` is lexical, so
/// `avatars/..` starts with `avatars` and a containment test passes for a name that climbs
/// straight out of the directory. This project has shipped that mistake once already, in
/// `restore` rebuilding a destination from an untrusted file name.
///
/// `hub` refuses such an owner before the lookup too, but the write is what would act on
/// it, so it is checked where it is used rather than trusted from two calls away.
fn cache_path(owner: &str) -> Option<PathBuf> {
    hub::valid_segment(owner).then(|| store::avatar_path(owner))
}

/// Every quantisation with its own verdict, largest first — which is how a repository's
/// own file list reads and how a reader scans for the best they can take. The one the app
/// would have taken on its own is marked rather than moved, so the ordering stays by size.
fn offers(entries: &[Entry], ceiling: Option<u64>) -> Vec<Offer> {
    let taken = quant::pick(entries, ceiling).map(|pick| pick.candidate.label);
    let mut quants: Vec<Offer> = quant::candidates(entries)
        .into_iter()
        .map(|candidate| Offer {
            fits: quant::fits(candidate.size, ceiling),
            picked: taken.as_deref() == Some(candidate.label.as_str()),
            candidate,
        })
        .collect();
    quants.sort_by_key(|offer| std::cmp::Reverse(offer.candidate.size));
    quants
}

/// One repository's tree, or why it could not be read. A failure is per row rather than per
/// page: one unreadable repository should not cost the reader the other twenty-three.
type Fetched = (String, Result<Vec<Entry>, String>);

/// Six lanes over one shared index. `ureq` is blocking and there is no async runtime here,
/// so concurrency is threads — which is what every other long call in this app uses.
fn fan_out(http: &Http, repos: &[String]) -> Vec<Fetched> {
    if repos.is_empty() {
        return Vec::new();
    }
    let next = AtomicUsize::new(0);
    let out: Mutex<Vec<Fetched>> = Mutex::new(Vec::new());

    std::thread::scope(|scope| {
        for _ in 0..LANES.min(repos.len()) {
            scope.spawn(|| loop {
                let index = next.fetch_add(1, Ordering::Relaxed);
                let Some(repo) = repos.get(index) else {
                    return;
                };
                let entries = hub::tree(http, repo);
                out.lock()
                    .expect("fan-out lock")
                    .push((repo.clone(), entries));
            });
        }
    });

    out.into_inner().expect("fan-out lock")
}

fn row(trees: &Trees, repo: hub::Repo, ceiling: Option<u64>) -> Row {
    let (owner, name) = match repo.id.split_once('/') {
        Some((owner, name)) => (owner.to_string(), name.to_string()),
        None => (String::new(), repo.id.clone()),
    };

    let mut pick = None;
    let mut quants = 0;
    let mut note = None;

    if repo.gated {
        note = Some("gated on Hugging Face — accept its terms there first".into());
    } else {
        match trees.known(&repo.id) {
            Some(entries) => {
                quants = quant::candidates(&entries).len();
                pick = quant::pick(&entries, ceiling);
                if pick.is_none() {
                    note = Some("no quantisation this app can download".into());
                }
            }
            None => note = Some("could not read this repository's files".into()),
        }
    }

    Row {
        id: repo.id,
        owner,
        name,
        downloads: repo.downloads,
        likes: repo.likes,
        last_modified: repo.last_modified,
        gated: repo.gated,
        architecture: repo.architecture,
        moe: repo.moe,
        pick,
        quants,
        note,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str, size: u64) -> Entry {
        Entry {
            path: path.to_string(),
            size,
            lfs: true,
        }
    }

    fn repo(id: &str, gated: bool) -> hub::Repo {
        hub::Repo {
            id: id.to_string(),
            downloads: 10,
            likes: 2,
            last_modified: None,
            gated,
            pipeline_tag: None,
            architecture: Some("qwen3moe".into()),
            context_length: None,
            moe: true,
        }
    }

    #[test]
    fn a_cached_picture_round_trips_and_a_miss_is_kept_as_firmly_as_a_hit() {
        let dir = std::env::temp_dir().join(format!("llamaport-avatar-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let hit = dir.join("unsloth");
        let miss = dir.join("nobody");

        assert_eq!(cached(&hit), None, "nothing is cached yet");
        store_cached(&hit, "data:image/webp;base64,AAAA");
        store_cached(&miss, "");

        assert_eq!(cached(&hit).as_deref(), Some("data:image/webp;base64,AAAA"));
        assert_eq!(
            cached(&miss).as_deref(),
            Some(""),
            "a remembered miss must be distinguishable from never having asked"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_picture_is_kept_for_a_month_and_no_longer() {
        assert!(fresh(Duration::from_secs(0)));
        assert!(fresh(AVATAR_TTL));
        assert!(!fresh(AVATAR_TTL + Duration::from_secs(1)));
    }

    #[test]
    fn an_owner_cannot_write_outside_the_directory_the_app_owns() {
        assert!(cache_path("unsloth").is_some());
        for hostile in ["..", "../../evil", "a/b"] {
            assert!(cache_path(hostile).is_none(), "{hostile} was accepted");
        }
    }

    #[test]
    fn a_gated_repository_says_so_and_offers_nothing_to_download() {
        let row = row(&Trees::default(), repo("owner/gated", true), Some(1 << 40));
        assert!(row.pick.is_none(), "a gated row offered a download");
        assert_eq!(row.quants, 0);
        assert!(row.note.is_some_and(|note| note.contains("gated")));
    }

    #[test]
    fn a_repository_whose_tree_never_arrived_says_that_rather_than_showing_nothing() {
        let row = row(
            &Trees::default(),
            repo("owner/missing", false),
            Some(1 << 40),
        );
        assert!(row.pick.is_none());
        assert_eq!(
            row.note.as_deref(),
            Some("could not read this repository's files")
        );
    }

    #[test]
    fn a_row_carries_the_pick_and_the_number_of_quantisations_behind_it() {
        let trees = Trees::default();
        trees.remember(
            "owner/model",
            vec![
                entry("Model-Q4_K_M.gguf", 8_500_000_000),
                entry("Model-Q8_0.gguf", 17_000_000_000),
                entry("mmproj-F16.gguf", 500_000_000),
            ],
        );
        let row = row(&trees, repo("owner/model", false), Some(40_000_000_000));
        assert_eq!(row.owner, "owner");
        assert_eq!(row.name, "model");
        assert_eq!(row.quants, 2, "the projector is not a quantisation");
        assert!(row.moe, "the architecture said moe and the row dropped it");
        assert_eq!(row.architecture.as_deref(), Some("qwen3moe"));
        let pick = row.pick.expect("a pick");
        assert_eq!(pick.candidate.label, "Q8_0");
        assert_eq!(pick.fits, Some(true));
        assert!(row.note.is_none());
    }

    #[test]
    fn the_detail_list_marks_the_one_the_app_would_have_taken_without_reordering() {
        let quants = offers(
            &[
                entry("Model-Q8_0.gguf", 30_000_000_000),
                entry("Model-Q4_K_M.gguf", 8_500_000_000),
                entry("Model-Q6_K.gguf", 17_000_000_000),
                entry("mmproj-F16.gguf", 500_000_000),
            ],
            Some(20_000_000_000),
        );
        assert_eq!(
            quants
                .iter()
                .map(|offer| (offer.candidate.label.as_str(), offer.fits, offer.picked))
                .collect::<Vec<_>>(),
            [
                ("Q8_0", Some(false), false),
                ("Q6_K", Some(true), true),
                ("Q4_K_M", Some(true), false),
            ],
            "largest first, the projector gone, and exactly one marked"
        );
    }

    #[test]
    fn with_no_ceiling_the_detail_list_claims_no_verdict_at_all() {
        let quants = offers(
            &[
                entry("Model-Q8_0.gguf", 30_000_000_000),
                entry("Model-Q4_K_M.gguf", 8_500_000_000),
            ],
            None,
        );
        assert!(quants.iter().all(|offer| offer.fits.is_none()));
        assert_eq!(quants.iter().filter(|offer| offer.picked).count(), 1);
    }

    #[test]
    fn the_ceiling_is_applied_at_read_time_so_finding_llama_server_changes_the_answer() {
        let trees = Trees::default();
        trees.remember(
            "owner/model",
            vec![
                entry("Model-Q4_K_M.gguf", 8_500_000_000),
                entry("Model-Q8_0.gguf", 17_000_000_000),
            ],
        );
        let cached = row(&trees, repo("owner/model", false), None);
        assert_eq!(cached.pick.expect("a pick").fits, None);

        let measured = row(&trees, repo("owner/model", false), Some(12_000_000_000));
        let measured = measured.pick.expect("a pick");
        assert_eq!(measured.candidate.label, "Q4_K_M");
        assert_eq!(measured.fits, Some(true));
    }
}
