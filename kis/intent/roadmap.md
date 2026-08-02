# Roadmap

Goal: a releasable macOS app that both runs and downloads local GGUF models.

## Order

1. **Downloader milestone** — current work, three phases: the engine and a
   Downloads screen, then Discover, then polish. Scope, decisions and acceptance
   live in [downloader.md](downloader.md).
2. **Packaging and release** — signing, notarization, a bundled `.app`, install
   docs. Nothing beyond Tauri defaults exists today.

Discover was originally sequenced after packaging. It now sits inside the
downloader milestone as phase 2, because it is only useful on top of the engine.

## Risks

- `rawArgs` is passed to `llama-server` verbatim, so `--host 0.0.0.0` typed there
  exposes an unauthenticated server to the network. Acceptable for a personal
  tool; a release blocker. Decide before step 2.
- The runner spec has no "known gaps" section, but `README.md:48` sends readers
  to one. Fix the reference or write the section.
