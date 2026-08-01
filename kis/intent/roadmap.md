# Roadmap

Goal: a releasable macOS app that both runs and downloads local GGUF models.

## Order

1. **Downloader** — build [docs/downloader-spec.md](../../docs/downloader-spec.md).
   Current work. The Downloads screen is a placeholder wired into `App.tsx`.
2. **Packaging and release** — signing, notarization, a bundled `.app`, install
   docs. Nothing beyond Tauri defaults exists today.
3. **Discover** — browsing Hugging Face for models. Placeholder screen; useless
   without the downloader, so it comes after.

## Downloader — done means

- A 13-21 GB file downloads faster than one connection, and resumes after both a
  pause and a full process exit.
- An expiring CDN signature is re-resolved mid-transfer and never surfaces as a
  failure.
- A stalled segment is detected and reissued rather than sitting at 97%.
- `Failed` is resumable from the sidecar.
- Proof is a real transfer, not a unit test alone.

## Open decisions

- **HTTP client.** `ureq` is compiled without TLS and is blocking; the downloader
  needs HTTPS and concurrent ranged segments. Choose before writing the engine.
- **Verification pass.** Whether sha256-against-etag runs by default or on
  request — it costs a minute or two on 21 GB.

## Risks

- `rawArgs` is passed to `llama-server` verbatim, so `--host 0.0.0.0` typed there
  exposes an unauthenticated server to the network. Acceptable for a personal
  tool; a release blocker. Decide before step 2.
- The runner spec has no "known gaps" section, but `README.md:48` sends readers
  to one. Fix the reference or write the section.
