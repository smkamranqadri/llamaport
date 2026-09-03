# Roadmap

Goal: a releasable macOS app that both runs and downloads local GGUF models.

## Where this stands

The runner lists, launches, supervises and tests models
([docs/runner-spec.md](../../docs/runner-spec.md)). The downloader fetches from
Hugging Face with resume, a queue and a rate limit
([downloader.md](downloader.md)). Both are done.

Since 2026-08-31 the app sizes a launch against real ceilings: it reads the GPU
working set from the build instead of assuming installed memory, and can leave
context and layer offload unset for llama.cpp to fit
([fitting.md](fitting.md), [screen.md](screen.md)).

Since 2026-09-01 the app measures. Every run that serves a request records what
it got, at the settings that got it, and a ladder can measure on request
([tune.md](tune.md)).

Since 2026-09-02 one click writes pi's provider and enables the model it should
reach ([pi.md](pi.md)).

Discover, a live search over Hugging Face, shipped 2026-09-03
([discover.md](discover.md)), and the UI was redesigned around the same date
([redesign.md](redesign.md)).

## Order

1. **Rename to Llamaport.** Done 2026-08-02, before packaging. [rename.md](rename.md).
2. **First beta release.** Shipped 2026-08-03, unsigned, MIT, `v0.1.0` as a
   GitHub pre-release. Signing and notarization were left out: no Developer ID,
   and one costs $99/yr before anything says the app is wanted.
   [release.md](release.md).
3. **Persistence.** Done 2026-08-03. Downloads survive a restart, the Library
   gets favourites and delete, and a launch keeps its defaults.
   [persistence.md](persistence.md).
4. **v0.2.0.** Shipped 2026-08-03, with three security fixes the release review
   turned up. [release.md](release.md).
5. **Download queue.** Done 2026-08-04. A second download request waits instead
   of being refused; one transfer still runs at a time. Proved by a four-deep
   queue draining about 48 GB unattended, which also turned up a path traversal
   live in v0.2.0 and fixed in v0.2.1. [downloader.md](downloader.md).
6. **Last used.** Done 2026-08-08. The Library now sorts by when a model was
   last run, not only when it landed on disk, which took the config to
   schema 7. [last-used.md](last-used.md).
7. **Distribution.** Done 2026-08-08. The release was made findable: a
   description and topics where there had been none, a social preview, a demo
   clip, and submissions to four lists. [release.md](release.md).
8. **No feature work.** Issue 1 was fixed 2026-08-08 and shipped in v0.3.0 (see
   Risks). No launch channel is open: r/LocalLLaMA needs karma this account
   does not have. One check is still owed: no Intel Mac has run the universal
   build.
9. **Figures.** Done 2026-08-31, shipped in v0.4.0. Fixed two wrong numbers: the
   KV cache term over-counted by roughly fourfold on sliding-window models, and
   `formatBytes` divided by 1024 cubed while labelling the result GB.
   [figures.md](figures.md).
10. **Fitting.** Done 2026-08-31, shipped in v0.4.0. The app stopped naming `-c`
    and `-ngl` on every launch, so llama.cpp's own `--fit` can size them,
    offered only when a `--help` probe confirms the build supports it.
    [fitting.md](fitting.md).
11. **Screen.** Done 2026-08-31, shipped in v0.4.0. Reads the GPU ceiling
    instead of comparing against installed RAM, turns four lines of prose into
    four figures, and moves seven rarely used settings behind Advanced.
    [screen.md](screen.md).
12. **Tune.** Built 2026-08-31, shipped in v0.5.0. The app measures a model's
    real speed instead of guessing from arithmetic, and states an opinion only
    once it has. [tune.md](tune.md).
13. **The pi button.** Built 2026-09-02, shipped in v0.6.0. Writes the provider
    and enables the model in one step, after showing a diff and waiting for
    confirmation, since the file pi keeps is not the app's. [pi.md](pi.md).
14. **Redesign.** Planned 2026-09-02, shipped 2026-09-03. Moves the UI to a
    sidebar-plus-pane layout and collapses the launch form behind three named
    choices. [redesign.md](redesign.md).
15. **Appearance.** Shipped 2026-09-03. A Mode control and five palettes over a
    stylesheet that derives its surfaces from seven anchors instead of listing
    fifteen values twice. [appearance.md](appearance.md).
16. **MoE launches.** Written up 2026-09-02, open, blocked on a measurement.
    Every launch is fully offloaded, so a mixture-of-experts model costs its
    whole working set even though only a fraction of its parameters run per
    token; `llama-server`'s `--n-cpu-moe` is not yet used. [moe.md](moe.md).
17. **Discover.** Planned and built 2026-09-03. A live screen over the Hugging
    Face API: a sort control with filters, a search that applies the same
    quantisation and fit logic as a browse row, and downloads that go through
    the existing queue. [discover.md](discover.md).

## Gaps

A comparison against LlamaForge and Unsloth, read from their source on
2026-08-30, lists what those tools do that this one does not
([gaps.md](gaps.md)). It is not a plan, and most items listed there have since
closed. What still stands: a keyboard map, a failed launch that shows the log
instead of the reason, and auto-update.

## Decided against

Benchmarks were removed 2026-08-01, commit `31031b2`, which deleted
`benchmarks.rs`, the Benchmarks and Connect screens, the benchmark half of
`health.rs`, and the profile CRUD surface. The reason was scope discipline:
roughly a third of the app served features never part of the goal, while the
resumable downloader, half of that goal, remained unbuilt. `benchmarks.json`
was left on disk and still holds records from that day.
[tune.md](tune.md) is that feature returning, asked for again by the author as
the app's user.

Discover was dropped 2026-08-02, before any code was written, for two reasons:
Hugging Face's `?search=` is a substring match over repo ids and would make a
worse browser tab, and pasting a URL into Downloads already closed the loop
this project set out to close. It was built 2026-09-03, as step 17, once both
reasons were answered: every result carries the quantisation this machine
should take and its size against the real ceiling, which a browser tab cannot
show. [discover.md](discover.md).

## Risks

- `rawArgs` is passed to `llama-server` verbatim, so a value like
  `--host 0.0.0.0` typed there would expose an unauthenticated server to the
  network. Decided: `rawArgs` may not set what the app owns.
- Sightings of an unusable window at launch closed 2026-09-04. They occurred
  only when the app was started from a non-interactive shell during agent work,
  and never on a launch by the author, dev or bundled.
- The Dock icon failing to reopen the main window was fixed 2026-08-08, shipped
  in v0.3.0, and confirmed by hand on the installed v0.5.0.
- The README's reference to a "known gaps" section that did not exist has been
  resolved; no such reference remains.
