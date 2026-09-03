//! The Hugging Face model index, live. Ignored by default: it needs the network.
//!
//! The inline tests prove the parsers against bodies this project wrote. Only this proves
//! the assumptions underneath them — that the four cheap `expand` fields are served, that
//! `gated` really is a bool or a string, that a tree carries `lfs` sizes, and that the
//! payload stays the size the client was designed around.
//!
//!   cargo test --manifest-path src-tauri/Cargo.toml --test real_hub -- --ignored --nocapture

use std::time::Duration;

use llamaport_lib::hub::{self, Http, Query, Sort, Transport};

fn http() -> Http {
    Http::new(Duration::from_secs(20))
}

fn query(sort: Sort) -> Query {
    Query {
        sort,
        search: None,
        limit: 24,
        cursor: None,
    }
}

#[test]
#[ignore = "reads the live Hugging Face index"]
fn every_sort_returns_a_full_page_of_usable_rows() {
    for sort in [Sort::Trending, Sort::Downloads, Sort::Likes] {
        let repos = hub::list(&http(), &query(sort)).expect("a listing").repos;
        assert_eq!(repos.len(), 24, "{sort:?} returned {} rows", repos.len());
        assert!(
            repos.iter().all(|repo| repo.id.contains('/')),
            "{sort:?} returned a row whose id survived validation but is not owner/name"
        );
        // The reason lastModified and createdAt are not offered: they sort, and they
        // return repos nobody has downloaded. Any sort this app ships must not.
        let dead = repos.iter().filter(|repo| repo.downloads == 0).count();
        assert!(dead <= 2, "{sort:?} returned {dead} rows with no downloads");
    }
}

#[test]
#[ignore = "reads the live Hugging Face index"]
fn the_three_sorts_are_not_the_same_list() {
    let of = |sort| {
        hub::list(&http(), &query(sort))
            .expect("a listing")
            .repos
            .into_iter()
            .map(|repo| repo.id)
            .collect::<Vec<_>>()
    };
    let trending = of(Sort::Trending);
    let downloads = of(Sort::Downloads);
    let likes = of(Sort::Likes);
    assert_ne!(trending, downloads);
    assert_ne!(trending, likes);
    assert_ne!(downloads, likes);
}

#[test]
#[ignore = "reads the live Hugging Face index"]
fn a_search_finds_a_named_model_and_the_term_survives_encoding() {
    let repos = hub::list(
        &http(),
        &Query {
            search: Some("qwen2.5 coder".into()),
            ..query(Sort::Downloads)
        },
    )
    .expect("a listing")
    .repos;
    assert!(!repos.is_empty(), "a two-word search returned nothing");
    assert!(
        repos
            .iter()
            .any(|repo| repo.id.to_lowercase().contains("qwen")),
        "the search term did not reach the API: {:?}",
        repos.iter().map(|r| &r.id).collect::<Vec<_>>()
    );
}

#[test]
#[ignore = "reads the live Hugging Face index"]
fn the_cheap_expand_set_is_what_the_client_was_sized_for() {
    let body = http()
        .get(&hub::listing_url(&query(Sort::Trending)))
        .expect("a listing body")
        .body;
    assert!(
        body.len() < 20_000,
        "24 rows came to {} bytes; the set was measured at 4,582 and the reason \
         expand=gguf is refused is that it costs 271,439",
        body.len()
    );
    assert!(
        !body.contains("chat_template"),
        "the listing is carrying chat templates, which nothing here reads"
    );
}

#[test]
#[ignore = "reads the live Hugging Face index"]
fn a_tree_carries_quants_in_subdirectories_with_lfs_sizes() {
    let entries = hub::tree(&http(), "unsloth/Qwen3.8-27B-GGUF").expect("a tree");

    let ggufs: Vec<_> = entries
        .iter()
        .filter(|entry| entry.path.ends_with(".gguf"))
        .collect();
    assert!(ggufs.len() > 5, "only {} GGUFs listed", ggufs.len());
    assert!(
        ggufs
            .iter()
            .all(|entry| entry.lfs && entry.size > 1_000_000),
        "a GGUF came back without an LFS size, which the transfer engine refuses"
    );
    // recursive=true is why these are here at all, and the reason the picker cannot
    // simply take the largest file: BF16/ holds shard halves, MTP/ holds a drafter.
    assert!(
        ggufs.iter().any(|entry| entry.path.contains('/')),
        "no quant came back from a subdirectory, so recursive=true stopped mattering"
    );
    assert!(
        entries.iter().all(|entry| entry.size > 0),
        "a directory survived the file filter"
    );
}

#[test]
#[ignore = "reads the live Hugging Face index"]
fn a_gated_repository_is_flagged_before_anything_tries_to_download_it() {
    // The trap this guards: the tree call answers 200 for a gated repo and resolve
    // answers 401, so a screen that ignores the flag shows a size and then fails.
    let repos = hub::list(
        &http(),
        &Query {
            limit: 100,
            ..query(Sort::Trending)
        },
    )
    .expect("a listing")
    .repos;
    let gated = repos.iter().filter(|repo| repo.gated).count();
    assert!(
        gated > 0,
        "no gated repo in 100 trending rows — either the flag stopped being served \
         or the expand field was dropped"
    );
}

/// The 32 GB machine this is developed on: the Metal working set llama.cpp reports, not the
/// installed memory, which is the distinction the whole project turns on.
const CEILING: u64 = 25_559 * 1024 * 1024;

#[test]
#[ignore = "reads the live Hugging Face index"]
fn the_picker_never_returns_a_sidecar_or_a_lone_shard_from_a_real_repository() {
    // Chosen because each carries a different trap: BF16 shard halves and an MTP drafter
    // directory, quantisations named by their directory, and mmproj projectors.
    for repo in [
        "unsloth/Qwen3.8-27B-GGUF",
        "unsloth/Qwen3-Coder-30B-A3B-Instruct-GGUF",
        "unsloth/Qwen3.8-Flash-Next-GGUF",
    ] {
        let entries = hub::tree(&http(), repo).expect("a tree");
        let candidates = llamaport_lib::quant::candidates(&entries);
        assert!(!candidates.is_empty(), "{repo} produced no candidates");

        for candidate in &candidates {
            let lower = candidate.label.to_ascii_lowercase();
            assert!(
                !lower.contains("mmproj") && !lower.contains("imatrix"),
                "{repo} offered the sidecar {}",
                candidate.label
            );
            for path in &candidate.paths {
                assert!(
                    !path.to_ascii_lowercase().starts_with("mtp/"),
                    "{repo} offered the drafter {path}"
                );
            }
        }

        let pick = llamaport_lib::quant::pick(&entries, Some(CEILING)).expect("a pick");
        // Not every repository has something that fits — Flash-Next is a 176B model and
        // none of its quantisations comes near a 25 GB working set. What must hold is that
        // the answer is honest either way: a fitting pick is under the ceiling, and a
        // non-fitting one is the smallest on offer rather than an arbitrary overage.
        match pick.fits {
            Some(true) => assert!(
                pick.candidate.size <= CEILING,
                "{repo} said {} fits at {} bytes",
                pick.candidate.label,
                pick.candidate.size
            ),
            Some(false) => {
                let smallest = candidates
                    .iter()
                    .map(|candidate| candidate.size)
                    .min()
                    .expect("candidates is not empty");
                assert_eq!(
                    pick.candidate.size, smallest,
                    "{repo} fits nothing and offered {} rather than the smallest",
                    pick.candidate.label
                );
            }
            None => panic!("{repo} was given a ceiling and did not use it"),
        }
        // A shard set is all of its parts or none of them: half a quantisation is a file
        // llama.cpp cannot open.
        assert!(
            !pick.candidate.paths.is_empty(),
            "{repo} picked a candidate with no files"
        );
        let sharded = pick.candidate.paths.len() > 1;
        assert_eq!(
            sharded,
            pick.candidate.paths[0].contains("-of-"),
            "{repo} picked one part of a shard set: {:?}",
            pick.candidate.paths
        );
        println!(
            "{repo}: {} at {:.1} GB across {} file(s), fits={:?}, from {} candidates",
            pick.candidate.label,
            pick.candidate.size as f64 / 1e9,
            pick.candidate.paths.len(),
            pick.fits,
            candidates.len()
        );
    }
}

#[test]
#[ignore = "reads the live Hugging Face index"]
fn a_second_page_follows_the_cursor_and_repeats_nothing() {
    let first = hub::list(&http(), &query(Sort::Trending)).expect("page one");
    let cursor = first.next.expect("page one carried no next link");
    let second = hub::list(
        &http(),
        &Query {
            cursor: Some(cursor),
            ..query(Sort::Trending)
        },
    )
    .expect("page two");

    assert_eq!(second.repos.len(), 24, "page two came back short");
    let seen: Vec<&String> = first.repos.iter().map(|repo| &repo.id).collect();
    let repeated = second
        .repos
        .iter()
        .filter(|repo| seen.contains(&&repo.id))
        .count();
    assert_eq!(repeated, 0, "page two repeated {repeated} of page one");
}

#[test]
#[ignore = "reads the live Hugging Face index"]
fn the_facts_call_carries_what_the_detail_page_states() {
    let facts = hub::facts(&http(), "unsloth/Qwen3.8-27B-GGUF").expect("facts");
    assert_eq!(facts.id, "unsloth/Qwen3.8-27B-GGUF");
    assert!(facts.downloads > 0 && facts.likes > 0);
    assert!(facts.params.is_some(), "no parameter count");
    assert!(facts.architecture.is_some(), "no architecture");
    assert!(facts.context_length.is_some(), "no trained context");
    assert!(facts.license.is_some(), "no licence");
}

#[test]
#[ignore = "reads the live Hugging Face index"]
fn the_index_really_does_serve_models_llama_server_cannot_run() {
    // The reason Discover filters at all. If this ever comes back empty, either Hugging
    // Face stopped tagging these or the denylist has drifted from what it names.
    let repos = hub::list(
        &http(),
        &Query {
            limit: 100,
            ..query(Sort::Downloads)
        },
    )
    .expect("a listing")
    .repos;

    let refused: Vec<&str> = repos
        .iter()
        .filter(|repo| !hub::serves_text(repo.pipeline_tag.as_deref()))
        .map(|repo| repo.id.as_str())
        .collect();
    assert!(
        !refused.is_empty(),
        "100 of the most downloaded GGUF repositories and not one is a non-text model"
    );

    // And the direction: a repository with no tag is kept, because many of the best carry
    // none. Nothing here may depend on the tag being present.
    let untagged = repos
        .iter()
        .filter(|repo| repo.pipeline_tag.is_none())
        .count();
    assert!(
        untagged > 0,
        "no untagged repository in 100, so the keep-what-is-unknown rule is untested here"
    );
    println!(
        "refused {} of 100, {untagged} carry no tag at all",
        refused.len()
    );
}
