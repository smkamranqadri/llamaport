# Discover

Planned and built 2026-09-03, shipped in v0.7.0. This is the third time Discover
was planned; the first two attempts are recorded in [roadmap.md](roadmap.md)'s
"Decided against". The earlier objection was that a search over repository ids
is a worse browser tab. That objection shaped this build: every result carries
the quantisation this machine should take and its size against the real Metal
working set.

## Purpose

A search box over repository ids is no better than a browser tab. Discover has
to beat the tab with what a tab cannot know: the real Metal working set on
this machine, what is already in the Library, and a downloader that resumes.

## What ships

A live screen over the Hugging Face API. A sort control (Trending, Most
downloaded, Most liked, Smallest first) and filters (Fits this Mac, MoE, and a
parameter band) that combine with it. A search box whose results get the same
treatment as a browse row. A detail page listing every quantisation in a
repository. Download hands a resolved URL to the existing download queue. Load
more pages by cursor rather than offset.

## Decisions

- **Live from the API, not a curated list.** A shipped list is stale the day
  after a release, and editorial claims about a model the app has never run put
  the app's name behind them.
- **The row shows the numbers it was ranked on** (downloads, last update, quant
  count), so a list that calls itself popular shows the popularity it sorted
  on.
- **The app picks the quant and names it in the row.** The rule: the largest
  quantisation that fits, preferring the K-quant family, falling back to
  smallest-first below the ceiling when nothing fits.
- **The picker chooses which file to fetch, never how to run it.** Tune still
  owns launch settings; see [tune.md](tune.md).
- **Coding and Chat filters were dropped, and this was measured.** Over the top
  50 trending GGUF repositories, the `code` tag appears 0 times and
  `conversational` appears in 46 of 50. One filter had nothing behind it; the
  other removed almost nothing.
- **No row says "fits".** A row prints its size against the ceiling and warns
  only when the weights alone exceed it, since every omitted term (KV cache,
  compute buffers, projector) can only push the real requirement up. Unsloth's
  own fit badge disagreed with its own memory bar on the same row in the
  examples recorded in [knowledge/technical.md](../knowledge/technical.md),
  which is the evidence that a sum of bytes can say a launch is allowed but
  never that it is good.
- **Discover's verdict is deliberately weaker than the model screen's.** The
  model screen reads the downloaded file's header for layers and architecture;
  Discover has only a byte count before any file exists, so the same word must
  not describe both.
- **The client is Rust.** [knowledge/technical.md](../knowledge/technical.md)
  already rules that logic worth testing belongs in Rust, and keeping the HTTP
  call there too leaves one place that validates a URL.
- **No new path validator.** `downloads::file_name_for` already requires
  `https://`, an allowlisted host, `/resolve/` in the path, a `.gguf` suffix,
  and takes the last path segment. Discover builds URLs in the form that
  validator already accepts, so `file_name_for` stays the one choke point for
  every URL the app acts on.
- **With no ceiling, the picker returns `Q4_K_M`** (or the smallest available
  K-quant) and reports fit as unknown rather than false. Falling back to
  installed memory instead would repeat the measurement error described in
  [knowledge/technical.md](../knowledge/technical.md). Ruled kept 2026-09-04.
- **The picker charges 1,024 MiB below the working set** as margin for the KV
  cache, matching llama.cpp's own `--fit-target` default. The row still prints
  the raw working set, so that margin is not shown anywhere. Ruled kept
  2026-09-04.
- **The search box hands a pasted link straight to Downloads**, which already
  does that job, rather than building a second path to the same result.
- **The ceiling was removed from the row** because the size is decimal GB and
  the ceiling is binary GiB; the two cannot be compared by eye when printed
  side by side with the same unit label.
- **Small & fast is a sort, not a filter.** It orders the fitted set
  smallest-first rather than inventing a size threshold for "small".
- **A repository is marked MoE from the uploader's tag and the file's
  architecture together**, since either signal alone misses about two in five
  cases.
- **The parameter band filter runs at the API** (`num_parameters=min:X,max:Y`),
  before the file-tree fan-out, so a filtered-out row never costs a call.
- **`hub::serves_text` is a denylist**, because 48 of 300 sampled repositories
  carry no pipeline tag at all; an allowlist would hide some of the best
  models on the site.
- **The owner avatar is fetched in Rust**, cached one file per owner, with a
  generic fallback and no coloured initial. A Library row gets its owner from
  the download history by path, since a file on disk carries no publisher.

## Acceptance

- No row prints "fits". A row prints its size against the ceiling and warns
  only when the weights alone exceed it.
- A gated repository is marked from the listing's own flag, and its Download
  is refused before the 401, not after.
- The picker never returns an `mmproj` sidecar, an `MTP` drafter, or one shard
  of a set; a shard set becomes queued jobs summing to the size the row
  showed.
- With no `llama-server` found, the screen still lists and searches, and says
  why there is no size verdict.
- A search for a named model returns it ranked, sized, and picked.
- A download lands through the existing queue with no new path validator.

## Verified

Verified 2026-09-04: all four checks passed, including a mutation check on the
picker. `real_hub` tests against the live API are ignored by default, like the
other `real_*` suites. The author reviewed the screen in four rounds on
2026-09-03 and the owner avatars on 2026-09-04. An earlier MoE mark shipped
inert because a stale test asserted the old URL shape; the rule against
stale assertions is in [knowledge/technical.md](../knowledge/technical.md).

## Risks

- The ceiling is missing exactly when Discover matters most: on a first run
  with no probed binary, the app cannot size anything.
- `expand=gguf` reads architecture off one file in a repository, which can be
  the wrong file on a repository carrying sidecars.
- The Hugging Face API budget is unadvertised; no rate-limit headers come back
  unauthenticated.
- Everything from the API is untrusted input, and lands as a path and a URL
  the app then acts on.
