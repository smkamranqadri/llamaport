# Gaps

What comparable tools do that Llamaport does not, read off their source on
2026-08-30. Two so far, each under its own heading.

**This is an observation list, not a plan.** Items still open are not scheduled.
Several contradict decisions this project has already argued, said plainly where
they appear rather than buried, so adopting one means reopening the decision on
purpose.

**Closed items keep their entry, marked.** What another tool does is worth
remembering after we have done it too, and the reason it was worth doing is in
the entry. Four closed in v0.4.0, one in v0.5.0, and LlamaForge's 6, 7 and 8
closed together on 2026-09-03 when Discover was built.

---

# LlamaForge

[dadwritestech/LlamaForge](https://github.com/dadwritestech/LlamaForge) — a Python
dashboard that drives llama.cpp's *router* mode with many models registered in
`models.ini`, where this app supervises one process. Most of its surface is
already ruled out in [knowledge/project.md](../knowledge/project.md); what is
below is the part that is not.

## Cheap, and nothing decided stands against them

1. **CLOSED 2026-09-02, noticed 2026-09-04.** *No search in the Library.* The
   redesign's Library (`5727e8a`) carries a search box matching display name and
   file name; this note stayed open for two days after. No `/` shortcut, which
   belongs with item 2.

2. **No keyboard map.** Theirs: `↑↓`/`kj` to move, `Enter` to expand, `L`/`U` to
   load and unload, `S` to save, `Esc` to close. This app has no key handlers.

3. **A failed launch shows the log, not the reason.** `backend/diag.py` runs an
   ordered regex table over the log tail — OOM, CUDA, context too large, unknown
   flag, missing file, unsupported architecture — and returns one error line plus
   a fix naming the value it means ("lower n-gpu-layers from 99"). Pure string
   work over text we already capture in `crashTail`. The nearest thing here is
   the reader scrolling.

9. **Live telemetry is never written down.** `stats.py` polls the router's
   `/metrics` every 5s, diffs the Prometheus token counters, attributes the delta
   to the loaded model and persists per-model and daily buckets to `stats.json`,
   30 days kept. This app scrapes the same endpoint for the running view and
   discards it, so nothing survives the process it was measured from.

   **Closed 2026-09-01**, shipped in v0.5.0 as parcels 1 and 2 of
   [tune.md](tune.md) — the only item here that has been. What it grew into is
   not a stats screen: a record keyed on the settings a run used, so the app can
   say what a model got and what a change to its settings did.

## The three that reopened Discover — all CLOSED 2026-09-03

6, 7 and 8 are in-app Hugging Face search, its result badges, and a fit rating.
They were written as a record of what a shipped version looks like rather than
an argument to build one, because [the roadmap's "Decided
against"](roadmap.md#decided-against) asked that Discover not be planned a third
time. **It was planned a third time and built** ([discover.md](discover.md)),
and this section is the closest thing to a design document it had. Item 8 is the
one that mattered most and it was read as a warning rather than a recipe — see
below.

6. **CLOSED 2026-09-03.** *Search.* Both calls are what this app now makes, with
   `trendingScore` added and `lastModified` deliberately not offered — it sorts,
   and it returns repositories nobody has downloaded.
   `hub.py` is two unauthenticated calls:
   `GET /api/models?filter=gguf&search=…&sort=downloads|likes|lastModified` then
   `/api/models/{repo}/tree/main` for files and sizes. Note this does not answer
   the objection that killed Discover here — `?search=` is still the substring
   match over repo ids that made it a worse browser tab.

7. **CLOSED 2026-09-03.** Everything here is now done, and none of it was free:
   collapsing a shard set needs the five-digit suffix stripped and the parts
   summed, and the sidecars are not only `mmproj` — an `mtp/` drafter and an
   imatrix live in the same listing, and `mtp` in a *name* usually means a model
   built with MTP rather than a drafter.
   *What the tree call gives for free*: shard sets collapsed
   (`-00001-of-00005` into one row with a summed size), `mmproj` sidecars listed
   separately, a `gated` flag from the list API, and INSTALLED from the local
   catalog. A multi-file fetch would be a queue of jobs this app already has.

8. **CLOSED 2026-09-03 by refusing it.** This entry called the vendored physics
   core "the forecast rule verbatim" and said the only compatible version is
   arithmetic shown with no verdict attached. That is exactly what shipped: the
   row prints its size against the ceiling and says something only when the
   weights alone are over. Reading Unsloth's own source later put numbers on the
   warning — their badge disagreed with their memory bar on 8 of 19 sizes
   ([knowledge/technical.md](../knowledge/technical.md)).
   *A fit rating before the download.* Cheap version is `size * 1.15 <= VRAM`
   → FITS / TIGHT / CPU OFFLOAD. Theirs is now a vendored physics core
   (`vramwise`) using bits-per-weight, MoE active-vs-total params and memory
   bandwidth to predict a regime and a tok/s figure. **This is the forecast rule
   verbatim** — a number wrong by 2x gets believed — so the only version
   compatible with this project is the arithmetic `estimate.rs` already does,
   shown against installed memory with no verdict attached.

## Where their version is weaker

Worth knowing, because it says the gap is breadth rather than depth: their
downloader is one job at a time, unsegmented, unthrottled, and re-requests the
original URL on resume — the exact failure this project exists to fix.

---

# Unsloth

Read 2026-08-30 from [unslothai/unsloth](https://github.com/unslothai/unsloth).
A different kind of neighbour: `studio/src-tauri` is a Tauri desktop app
shipping a signed macOS `.dmg` that runs GGUF models through llama.cpp — our
stack and our job — inside roughly 900k lines that are also a training
framework, a chat UI, an agent bridge, an MCP host and a diffusion runner. Only
the panel that runs one GGUF overlaps. What follows is that overlap, and unlike
the LlamaForge list most of it is correctness rather than features.

## Uncontested

1. **No auto-update.** Someone on v0.2.0 has no way to learn v0.4.0 exists;
   every install we have ever made is stranded on whatever it was. They run
   Tauri's updater against `releases/latest/download/latest.json`, and
   `desktop_update_policy.rs` splits in-app update from manual — Linux packages
   get a release-page link instead of a download.

2. **CLOSED in v0.4.0** ([fitting.md](fitting.md)) — *The launch always named
   `-c` and `-ngl all`, which turned off a feature that is on by default.* Sharpened 2026-08-31 by reading the installed build
   rather than their source: `--fit` adjusts *unset* arguments to fit device
   memory and defaults to `on`, so this is not "a feature we could adopt" but
   "a feature this app disables on every launch"
   ([knowledge/technical.md](../knowledge/technical.md) carries the
   measurements). Unsloth's auto-layers mode omits `-c` for exactly this reason.
   Letting the Context and GPU-layers fields sit *unset* is the app declining to
   guess, which is the forecast rule rather than an exception to it. Gated on a
   `--help` probe like every other flag, since an older build has neither.

   It also corroborates item 4 below on real hardware. `Qwen3.6-35B` runs at its
   full 262,144 context here; the estimate this app shipped until today charged
   40 layers a full cache and claimed 39.52 GB against 34.36 GB installed, which
   reads as "will not fit" for a model llama.cpp was running at maximum context.
   The attention layers alone come to 23.41 GB.

   This is the largest item in this file and wants its own planning pass, not a
   bolt-on to something else.

3. **CLOSED in v0.4.0** ([screen.md](screen.md)) — *A launch that cannot fit got
   no warning.* The panel's verdict is now the worse of two questions, over the
   GPU limit or beyond what is free, and it is advisory exactly as theirs is.
   The original entry follows because its construction is why the warning is
   safe.* `_launch_host_shortfall_message`
   prices weights alone against free VRAM plus available RAM, and is explicitly
   a lower bound: KV, projector, drafter and compute buffers are all left out,
   every omitted term moves the figure down, and so no missing term can turn a
   quiet load into a warned one. Advisory only — it logs and launches anyway.
   The construction is what keeps it from being a forecast.

## Correctness

4. **CLOSED in v0.4.0** ([figures.md](figures.md)) — *The KV figure was
   wrong-high on sliding-window models and called exact.* The entry as first
   written follows. `estimate.rs` charged every layer a full-context cache. Gemma-family
   models do not hold one. They resolve the SWA period from the HF config's
   `layer_types` (falling back to a transformers config object), persist it so
   it is one fetch per repo, and size the cache against the layers that actually
   keep the whole context. This is the item worth arguing for first: an exact
   number that is wrong is worse than the forecast we refuse to make.

5. **`sysmem.rs` is worth auditing against their Apple reasoning.**
   `_apple_metal_memory_budget_bytes` takes the smaller of MLX's
   `max_recommended_working_set_size` and `psutil.available`, then a fraction of
   that, for two documented reasons. Both a device ceiling and total RAM
   describe the machine rather than the moment — on a 16 GB Mac the budget came
   out around 9 GB whether idle or nearly full. And macOS `free` omits the
   reclaimable inactive queue, so it reads far below what a new allocation can
   actually get; `available` adds it back. The fraction then covers the seconds
   `llama-server` spends loading, during which another app can take memory.

6. **One `.gguf` is the whole model to us.** They detect companions by filename
   and wire the flags: `mmproj-*` projectors for vision, MTP and EAGLE drafters
   for speculative decoding, imatrix files. A repo's shard set, projector and
   drafter travel as one unit through download, inventory and launch.

7. **`brew install llama.cpp` is our entire story for the binary.**
   `install_llama_prebuilt.py` fetches pinned `ggml-org/llama.cpp` release
   assets per platform and backend, verifies them against a published sha256
   manifest, and installs into a directory it owns, with a file lock and
   separate exit codes for busy, no-space and backend-unavailable. **This
   contradicts the standing decision not to manage the llama.cpp installation**,
   and is recorded because the failure it removes is real: Homebrew hands the
   user whatever build it has today, and the `--help` probe is the only thing
   between that and a mismatch.

8. **Unsigned.** They ship a hardened runtime with an entitlements plist, a
   notarized `.dmg`, trusted signing on Windows and a VirusTotal step. We ship
   `xattr -dr` instructions. Already decided — $99/yr before anything says the
   app is wanted — and listed only so the comparison is honest.

## What their UI does that ours does not

Their memory panel answers the same question ours does and reaches a different
settlement with it. Worth reading before touching `Memory.tsx` or `ModelDetail`.

- **CLOSED in v0.4.0** ([figures.md](figures.md)) — *A bounded figure is shown
  with a `≥`, not withheld.* Where the header
  cannot size the cache, they print the floor, mark it, and say which term is
  missing and that the cache is the one that grows fastest with context. Our
  `estimate()` returns `Option` and the screen falls silent. Marked-and-shown is
  compatible with the forecast rule and tells the reader more than nothing does.
- **CLOSED in v0.4.0** ([screen.md](screen.md)) — *At most one note, most
  actionable first.* An unsizable cache outranks any
  verdict drawn from the figures, because it says the figures are incomplete.
- **"Fits the machine" and "fits what is free right now" are different
  questions, and the second only ever warns** — the memory a pending load
  reclaims is mostly the outgoing model's own, which cannot be attributed from
  the panel. Apple Silicon is their single-pool case, where the pool decides the
  wording and which figures are compared rather than whether the warning exists;
  they shipped a version where that arm was dead code and Apple machines could
  only ever be told "exceeds".
- **A verdict says how bad, a cause says what would fix it** — `context`
  against `irreducible`, since a shorter context recovers one and nothing
  recovers the other.
- **CLOSED in v0.4.0** ([figures.md](figures.md)) — *`formatBytes` divided by
  1024³ and printed "GB", and `formatRate` on all three units.* Unsloth fixed exactly this and put a number
  on it: every figure overstated by 7.4% against its label. Ours shows a
  16.45 GiB file as "16.5 GB" where Finder, which is decimal, says 17.66 GB.
  Either divide by 1000³ or write GiB; the two must not disagree.
- **Captions break between bullets, not inside them.** `glueNoteItems` puts a
  non-breaking space inside each item of a `·`-separated caption and glues each
  bullet to the item that follows, so a narrow window never orphans a bullet or
  strands one word on its own line.
