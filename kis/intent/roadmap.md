# Roadmap

Goal: a releasable macOS app that both runs and downloads local GGUF models.

## Subsystems

Two, each with its own spec, module and screen:

- **Runner** — done. [docs/runner-spec.md](../../docs/runner-spec.md).
- **Downloader** — engine and Downloads screen done.
  [downloader.md](downloader.md).

## Order

1. ~~**Rename to Llamaport**~~ — done 2026-08-02, before packaging, because the
   bundle identifier is free to change until something is signed and step 2 signs
   it. [rename.md](rename.md).
2. **Packaging and release** — signing, notarization, a bundled `.app`, install
   docs. Nothing beyond Tauri defaults exists today. Not planned yet, and the
   three risks below are the reason to plan it rather than start it.

## Decided against

**Discover — dropped 2026-08-02, after being planned.** An in-app Hugging Face
browser: search, repo file listing, quant selection. Planned in full, then cut
before any code was written.

Two reasons, and they should stop this being planned a third time. Hugging Face's
`?search=` is a substring match over repo ids ranked by download count, so an
in-app search would have been a worse version of the browser tab that is already
open. And pasting a URL into Downloads already closes the loop the project set
out to close: `curl` not resuming was the problem, not finding the file.

Its research was not wasted: what it established about the engine's limits is in
[downloader.md](downloader.md) under "Known limits of the engine".

## Risks

- `rawArgs` is passed to `llama-server` verbatim, so `--host 0.0.0.0` typed there
  exposes an unauthenticated server to the network. Acceptable for a personal
  tool; a release blocker. Decide before step 2.
- The runner spec has no "known gaps" section, but `README.md:50` sends readers
  to one. Fix the reference or write the section.
- The app can start with an unusable window. Observed twice in one session: once
  with no window at all and the Window menu empty, once at 60x60.
  `show_main_window` asserts a usable frame and does not reliably achieve it.
  Predates the downloader, and is a release blocker of its own.
