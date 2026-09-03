# Gaps

A comparison against two tools with an overlapping job, read from their source
on 2026-08-30. Not a plan: open items are not scheduled. Several reopen
decisions already made elsewhere; each says so where it appears.

## LlamaForge

[dadwritestech/LlamaForge](https://github.com/dadwritestech/LlamaForge) is a
Python dashboard driving llama.cpp's router mode with many registered models;
this app supervises one process.

1. Closed 2026-09-02. No search in the Library: the redesign added a search
   box matching display name and file name ([redesign.md](redesign.md)).
2. Open. No keyboard map. Theirs moves, expands, loads, unloads, saves and
   closes with single keys; this app has no key handlers.
3. Open. A failed launch here shows the raw log. Theirs matches the log tail
   against known errors (out of memory, bad flag, missing file) and returns
   one line naming the fix.
6. Closed 2026-09-03. In-app Hugging Face search shipped as part of Discover,
   making the same two calls theirs does ([discover.md](discover.md)). Their
   `?search=` is still a substring match over repo ids, which is why a search
   box alone would only be a worse browser tab.
7. Closed 2026-09-03. Shard sets are now collapsed into one row with a summed
   size, and sidecars such as `mmproj` projectors and drafters are listed
   separately, matching what their file listing already provided
   ([discover.md](discover.md)).
8. Closed 2026-09-03, by refusing it. Their fit rating predicts a regime and
   a speed figure from bits-per-weight and memory bandwidth, which is the
   forecast this project refuses to make. Unsloth's source later showed such a
   badge disagreeing with its own memory bar
   ([knowledge/technical.md](../knowledge/technical.md)). This app shows a
   file's size against the ceiling and warns only when the weights alone
   exceed it ([discover.md](discover.md)).
9. Closed 2026-09-01, shipped in v0.5.0. Their telemetry is polled and
   persisted per model for 30 days; this app now records what a run got at the
   settings it used ([tune.md](tune.md)).

Their downloader is weaker than this app's: one job at a time, unsegmented,
unthrottled, and it re-requests the original URL on resume, the exact failure
this project was built to fix.

## Unsloth

Read 2026-08-30 from [unslothai/unsloth](https://github.com/unslothai/unsloth).
`studio/src-tauri` is a Tauri app on the same stack running GGUF models
through llama.cpp, inside a much larger codebase that is also a training
framework and chat UI. Only the overlapping panel is compared here.

1. Open. No auto-update: an install on an old version has no way to learn a
   new one exists. Adopting it would reopen the standing decision against an
   auto-updater, which needs signing keys this project has not bought
   ([release.md](release.md)).
2. Closed in v0.4.0. Their launch leaves context and layer offload unset so
   llama.cpp's `--fit` can size them; this app now does the same, gated on a
   `--help` probe ([fitting.md](fitting.md)).
3. Closed in v0.4.0. A launch that cannot fit now gets an advisory warning,
   over the GPU limit or beyond what is free ([screen.md](screen.md)).
4. Closed in v0.4.0. The KV cache figure was wrong-high on sliding-window
   models and shown as exact; it is now sized against the layers that actually
   keep the full context ([figures.md](figures.md)).
5. Open, worth auditing. Their Apple memory budget takes the smaller of the
   device's working-set ceiling and available RAM, then a fraction of that, to
   cover the load time during which another app can take memory.
6. Open. One `.gguf` is the whole model here. They detect companion files by
   name and carry vision projectors, speculative drafters and imatrix files as
   one unit through download, inventory and launch.
7. Open. Their installer fetches pinned llama.cpp releases, verifies them
   against a published checksum manifest, and installs into a directory it
   owns. Contradicts the standing decision not to manage the llama.cpp
   installation, kept because the failure it removes is real: Homebrew hands
   the user whatever build it has today.
8. Open. They ship a notarized, signed build; this app ships `xattr`
   instructions instead. Already decided against on cost: signing needs a
   $99/yr Apple Developer account before anything says the app is wanted.

Their memory panel reaches a different settlement with the same question,
worth reading before touching this app's own:

- Closed in v0.4.0. A bounded figure is shown with a marked floor instead of
  withheld ([figures.md](figures.md)).
- Closed in v0.4.0. At most one warning is shown, the most actionable first
  ([screen.md](screen.md)).
- Open. "Fits the machine" and "fits what is free right now" are different
  questions; only the second should warn, since memory a pending load
  reclaims is mostly the outgoing model's own.
- Open. A verdict says how bad; a cause says what would fix it.
- Closed in v0.4.0. `formatBytes` divided by 1024 cubed while printing "GB";
  units now agree with their label ([figures.md](figures.md)).
- Open. Captions should break between bullets, not inside them, so a narrow
  window never orphans a word.
