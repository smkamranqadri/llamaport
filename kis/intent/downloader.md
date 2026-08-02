# Downloader milestone

Design lives in [docs/downloader-spec.md](../../docs/downloader-spec.md). This
file holds only what the spec does not: the decisions taken, what was carried
rather than met, and the limits found in the building.

Live status is in [state/current.md](../state/current.md), not here.

## Decisions

- **ureq + threads, not reqwest + tokio.** Enable ureq's default features for
  TLS, one thread per segment, positioned writes through `FileExt::write_at`.
  The codebase is blocking throughout and hands long work to `spawn_blocking`;
  8 sockets do not need an async runtime. `tokio` is already compiled in via
  tauri, but `reqwest` is not in the macOS build graph and would add ~40 crates.
- **The rate limit is live.** `Control` carries it and the bucket re-reads it on
  every charge, so a limit changed while a download runs applies to that
  download. The alternative — fixing it in the `Spec` at the start — was
  rejected: a limit is set while watching the transfer it is meant for.
- **The floor on a limit is app policy, not engine mechanism.** `normalized_rate`
  in `downloads.rs` bounds what may be asked for; the engine honours whatever it
  is handed, which is what lets a test drive it at a byte a second.
- **No Hugging Face token.** Public repos only; a gated repo is
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

## Verification

`cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check`,
`bun run build` — then a real transfer. Tests alone have never been enough to
close work on this subsystem, and were not enough for the speed limit either:
what proved that was watching a running download change rate.

Agreed 2026-08-02: the proof is a small (~1 GB) public GGUF from a repo such as
bartowski or unsloth, killed mid-flight and resumed, verified against the real
Hugging Face CDN. A full 13-21 GB run is a separate decision to be asked for
explicitly, not assumed.

Use the tdd skill for the engine: the retry taxonomy and resume logic are cheap
to get wrong and expensive to discover at 97% of 21 GB.

## Known limits of the engine

Found while planning Discover, which was then dropped ([roadmap.md](roadmap.md)).
They are facts about the engine and outlive that plan.

- Hugging Face omits `x-linked-size` and `x-linked-etag` on non-LFS files, and
  the engine refuses a file with no declared size. In a repo tree an `lfs` object
  on the entry is exactly the condition under which those headers exist.
- A quant too big for one file ships as `{name}-00001-of-00003.gguf`. The engine
  takes one file per job and refuses a second rather than queueing, so a split
  set has to be fetched a part at a time.
- Pause is not a state: cancel leaves the `.part` and sidecar, and starting the
  same URL again resumes. Add a real Paused state only if the UI wants a pause
  button.
- One at a time is enforced by refusing, not queueing, so there is no Queued
  state. Any "download all" would need one.
- `resolution_against_a_silent_server_is_bounded_by_its_timeouts` guards less
  than its name claims: it pins that *some* timeout bounds resolution, not which
  one — removing either the read timeout or the overall timeout alone still
  passes. Tighten it if that code is touched.
