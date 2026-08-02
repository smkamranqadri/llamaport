# Downloader milestone

Design lives in [docs/downloader-spec.md](../../docs/downloader-spec.md). This
file holds only what the spec does not: the decisions taken, the phase split,
and what closes each phase.

Status: **approved 2026-08-02.** Phase 1 not started.

## Decisions

- **ureq + threads, not reqwest + tokio.** Enable ureq's default features for
  TLS, one thread per segment, positioned writes through `FileExt::write_at`.
  The codebase is blocking throughout and hands long work to `spawn_blocking`;
  8 sockets do not need an async runtime. `tokio` is already compiled in via
  tauri, but `reqwest` is not in the macOS build graph and would add ~40 crates.
- **Engine before Discover.** Discover sits on top of the engine, so building
  both at once means the first 21 GB transfer runs through two unproven layers.
- **No Hugging Face token this milestone.** Public repos only; a gated repo is
  detected on resolve and reported plainly. Most GGUF quants are public, and a
  released app should not keep a bearer token in plaintext. The spec's
  strip-`Authorization`-on-redirect rule stays dormant until this is revisited.
- **sha256 verification defaults to on**, in the background, before the rename,
  and is skippable.
- **Downloads land in the configured models directory.**

## Phase 1 — engine and Downloads screen

The engine per the spec, plus a Downloads screen driven by a pasted Hugging Face
URL: progress, speed, pause, resume, cancel. Default 4 segments.

Closes when:

- Resume survives a full process exit, restarting each segment from the sidecar.
- A 403 on an expired CDN signature re-resolves and continues, invisibly.
- A segment reporting zero bytes for 30 s while siblings progress is killed and
  the range reissued.
- The rate limit is shared across segments, not applied per segment.
- An etag mismatch discards and restarts; a 404 or a missing `accept-ranges`
  stops without retrying.
- Free disk is checked before enqueueing.
- The finished file lands in the models directory and appears in Library.
- The transfer beats a single connection.

Likely files: new `src-tauri/src/download.rs` and
`src-tauri/tests/download_resume.rs`; `lib.rs` commands and events; `store.rs`
settings with a schema bump to 5; `Cargo.toml` for ureq's TLS feature; new
`src/Downloads.tsx` replacing the placeholder at `App.tsx:163`; `api.ts`,
`types.ts`, `App.css`.

## Phase 2 — Discover

Hugging Face search, repo file listing, quant selection with sizes. Hands a URL
to the engine phase 1 proved.

## Phase 3 — polish and release-readiness

## Verification

`cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check`,
`bun run build` — then a real transfer. Tests alone do not close phase 1.

Agreed 2026-08-02: the proof is a small (~1 GB) public GGUF from a repo such as
bartowski or unsloth, killed mid-flight and resumed, verified against the real
Hugging Face CDN. A full 13-21 GB run is a separate decision to be asked for
explicitly, not assumed.

Use the tdd skill for the engine: the retry taxonomy and resume logic are cheap
to get wrong and expensive to discover at 97% of 21 GB.

## Risks and assumptions

- ureq 2.12's TLS feature name needs confirming before the engine is written.
  The loopback callers in `health.rs` and `runner.rs` are unaffected either way.
- `catalog.rs:131` filters on the `.gguf` extension, so `.part` and `.part.json`
  are ignored for free — but a completed download must trigger a rescan.
- Hugging Face may omit `x-linked-size` and `x-linked-etag` on non-LFS files.
- A 30 s stall threshold could false-positive on a very slow link.
