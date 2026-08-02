# Downloader milestone

Design lives in [docs/downloader-spec.md](../../docs/downloader-spec.md). This
file holds only what the spec does not: the decisions taken, the phase split,
and what closes each phase.

Live status is in [state/current.md](../state/current.md), not here.

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

## Phase 1 — engine and Downloads screen — DONE 2026-08-02

Every closing condition met except one, proved by a real 676 MB transfer that was
killed, resumed, verified and landed in the models directory.

Two carried forward rather than met:

- **"Beats a single connection" was never measured.** The observed rate varied
  90 KB/s to 1.4 MB/s and the line, not the engine, looked like the limit. It
  needs a like-for-like comparison against `curl` before anyone claims it.
- **Stall detection ignores the spec's sibling-progress condition.** Any segment
  silent past `stall_after` is reissued, bounded by the 5-attempt limit, rather
  than only one silent while siblings move. Deliberate and accepted.

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

## Carried into phase 2

- Hugging Face omits `x-linked-size` and `x-linked-etag` on non-LFS files. The
  engine refuses a file with no declared size, so Discover must not offer one.
- Pause is not a state: cancel leaves the `.part` and sidecar, and starting the
  same URL again resumes. Add a real Paused state only if the UI wants a pause
  button.
- One at a time is enforced by refusing, not queueing, so there is no Queued
  state. Discover offering a "download all" would need one.
