# Discover

Planned 2026-09-03 from an artboard the author supplied, and **the third time
this has been planned**. The first two ended in
[roadmap.md](roadmap.md)'s "Decided against", the second with a note asking that
it not be planned again. This one ships, and the note's reason is what shaped
it rather than what stopped it.

The bar it has to clear is [direction.md](direction.md)'s: a search box over repo
ids is a worse browser tab, so what is in scope is **finding the best model for
this machine**, and if it cannot beat the tab it should not ship. What it beats
the tab with is the three things a tab cannot know — the real Metal working set,
what is already in the Library, and a downloader that resumes.

The artboard is titled "Discover — build later". The author chose to build it
now, so the README screenshots and the release wait a second time
([release.md](release.md)).

## What ships

A live screen over the Hugging Face API. Four single-select named lists; a
search box whose results get the same treatment as a browse row; rows carrying
name, author, the numbers the ranking is made of, the quant the app chose, and
its size against the ceiling; a Download that hands a resolved URL to the queue
that already exists.

## Decisions

**Live from the API, not a curated list.** The artboard's row prose — "Strong
all-rounder; the current community favourite at this size" — is editorial and no
API returns it. Shipping it would put the app's name on claims about models it
has never run, and a shipped list is stale the day after a release. Unsloth
reached the same place: their hub has a curated catalog for datasets and none
for models.

**The second line carries the numbers the ranking is made of** — downloads,
when it was updated, how many quants — because a list that calls itself popular
should show the popularity it sorted on. Nothing on the row is asserted that the
API did not return.

**The app picks the quant and names it.** One rule over the file list: the
largest that fits, preferring the K-quant family, with the choice printed in the
row rather than hidden behind the button. This is the substance of
[direction.md](direction.md) item 7 — fit and quantisation are part of the
answer. Unsloth's `gguf-variant-sort.ts` ranks the same way and supplies a
fallback this plan had not: within the tier that does not fit, **smallest
first**, since the least-bad is the only useful answer there.

The picker decides which file to fetch and **never how to run it**. Tune still
owns that. Arithmetic choosing a launch is what [tune.md](tune.md) measured as
picking the slowest of three; arithmetic choosing which file to download is a
different and much weaker claim.

**Four chips, all backed**: Fits this Mac, Small & fast, Most downloaded, Most
liked. The heading renames itself to the chip, so "Trending on Hugging Face" is
shown only when the list really is `trendingScore` — the artboard's "Popular
this week" is a window HF does not document.

**Coding and Chat are dropped, and this is measured rather than argued.** Over
the top 50 trending GGUF repos the `code` tag appears **0 times** and
`conversational` appears **46 times**, so one chip has nothing behind it and the
other removes four rows out of fifty. Backing Coding with `search=coder` is the
repo-id substring match this project has twice ruled makes a worse browser tab.
Unsloth offers neither: their capability filter is modality — Reasoning, Vision,
Audio, Embeddings, Image generation — which is an independent arrival at the
same answer by the largest app on this stack.

**Search ships.** `search=` composes with `sort=`, and every result goes through
the same quant pick, size and verdict as a browse row. The 2026-08-02 objection
was against a bare substring list, and this is not one. Pasting a link keeps
working and hands off to Downloads as it does today.

**No row says "fits".** The row prints the size against the ceiling — `8.5 GB of
26.8 GB` — and warns only when the weights alone exceed it, which is a bound
that can only be wrong in the safe direction. Every omitted term (KV, compute
buffers, projector) moves the real requirement up, so "will not fit" is sound
and "fits" is not available.

The evidence is Unsloth's, not this project's argument. Their `classifyGgufFit`
is the artboard's badge shipped: `size × 1.15 + 1 GB` against 97% of the card,
in five classes rather than two. Its own comments record that the badge and the
memory bar **on the same row** disagreed on **11 of 19 sizes**, that sharing a
constant took it to **8 of 19**, and that the residual eight are the estimator
itself — "a 18 to 19 GiB quant was badged `fits` beside a bar reporting an
overage", "a 20 GiB file on a full 24 GiB card at 1.0 read `fits` while
`_select_gpus` fell back to `--fit`". Their context allowance is a flat 1 GB "at
a typical 4K window"; this app runs 65,536 with an f16 cache, where the cache is
several GB. That is the author's own rule arriving from outside:
a sum says a launch is allowed, never that it is good.

Discover's verdict is also **strictly weaker than the model screen's**. There,
the file is on disk and `estimate.rs` reads the header for layers and
architecture. Here the file does not exist and all the app has is a byte count.
The same word must not appear in both places.

**The client is Rust, where Unsloth's is not.** Theirs calls HF from the
frontend; their backend only ever sees a repo already chosen. Ours goes in Rust
because [knowledge/technical.md](../knowledge/technical.md) already rules that
there is no frontend test framework and logic worth testing belongs in Rust —
and the picker and the fit rule are the logic worth testing. Keeping the HTTP
there too leaves one place that validates a URL.

**No new path validator.** `downloads::file_name_for` already requires `https://`,
an allowlisted host, `/resolve/` in the path and a `.gguf` suffix, and takes the
**last** path segment — so a tree entry like `BF16/…-00001-of-00002.gguf` yields
a plain name and the traversal shape this project shipped once is already closed.
Discover constructs URLs in the form that validator accepts and adds nothing.
It stays the only choke point.

## Parcels

1. **The client.** List and file-tree calls behind a trait it owns, following
   `EventSink`/`ProgressSink` rather than adding a third idiom, so it is testable
   against a stand-in with no network.
2. **The picker.** A pure function over a file listing: exclude sidecars, collapse
   shard sets, rank by fit then size. Where the mutation check goes.
3. **The screen.** Chips, rows, search, and a defined state for figures that have
   not arrived.
4. **The download wiring.** Reuse `admit` and `file_name_for` unchanged; a shard
   set becomes queued jobs.
5. **The look.** The artboard rendered and put beside the app's own DOM before
   anything is handed to the author, per
   [knowledge/technical.md](../knowledge/technical.md) Verify.

## Acceptance

- The four chips return four different lists, each from the sort the chip names,
  and the heading matches the chip.
- No row prints "fits". A row prints its size against the ceiling and warns only
  when the weights alone exceed it.
- A gated repo is marked from the listing's own flag and its Download is refused
  **before** the 401, not after.
- The picker never returns an `mmproj` sidecar, an `MTP` drafter or one shard of
  a set; a shard set becomes queued jobs summing to the size the row showed.
- With no `llama-server` found, the screen still lists and searches, and says why
  there is no size verdict.
- A search for a named model returns it ranked, sized and picked.
- A download lands through the existing queue with no new path validator.

## Risks

- **The ceiling is missing exactly when Discover matters most.** It comes from
  `probe`'s `--list-devices`, and `lib.rs:342` already notes a first run may not
  have found the binary. Somebody with no binary and no models is the ideal
  Discover user and the one case the app cannot size. Needs a stated state, not
  a blank.
- **`expand=gguf` lies on repos carrying sidecars.**
  `HauhauCS/…-27B-…-MTP-GGUF` reports 1.86B parameters for a 27B model, because
  the field is read off one file and that repo's is the drafter. The same trap as
  the picker's, one layer earlier.
- **The API budget is unadvertised.** No rate-limit headers come back
  unauthenticated. A browse that fires a call per row on every chip press is how
  it would be found.
- **Everything from the API is untrusted input**, and lands as a path and a URL
  the app then acts on. Two live holes in this project came from forgetting that.
- **A security review before the release**, as v0.2.0 had. This adds a network
  client parsing third-party JSON into paths and URLs, which is the shape that
  review exists for.

## Verification

The four commands green. A mutation check on the picker: gut the sidecar and
shard exclusions and watch the tests fail. The client against a stand-in with no
network. Then the artboard rendered against the app's own DOM in headless Chrome
before the hand-over — and then the author looking at it, which is where 24 of
this project's 24 defects have come from.
