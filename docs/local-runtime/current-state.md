# Current state

## Where things stand

The runtime half of the app is built and cleaned up. **The downloader is not
built**, and is the next session's work.

Read before starting: [docs/downloader-spec.md](../downloader-spec.md), then
[decisions.md](decisions.md) — D17 and D20 changed how launching works, and D18
records why the downloader stayed outstanding for so long.

## What the app does

List GGUF models with real header metadata → launch one under `llama-server`
with the exact command visible → watch memory, throughput and logs → stop it.
Plus a model test that says whether a running server actually works.

Settings are not configured or named: the form opens with whatever that model was
last launched with, and a successful launch updates it.

## What it deliberately does not do

- Download models. Designed, unbuilt. The Downloads screen says so.
- Profiles, templates or saved presets (D19, D20).
- Benchmark history, agent configuration generation (D19).
- API keys or non-loopback binding (D16 — Phase 6 was skipped).

## Shape

```
4,569 lines Rust · 99 tests · 13 commands · 7 frontend files
```

Modules: `catalog`, `gguf`, `probe`, `profile`, `estimate`, `sysmem`, `safety`,
`runner`, `health`, `store`.

## Verification

```bash
bun run build                                     # tsc + vite
cargo test --manifest-path src-tauri/Cargo.toml   # 99 passing, 1 ignored
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
```

All four pass. The ignored test (`real_launch`) loads a real model; run it
deliberately with `-- --ignored --nocapture`.

## Known problems

1. **Calibration has no samples yet.** Estimates use the nominal figure, which
   over-predicts on Apple Silicon. Three clean start-to-stop cycles fit a ratio.
2. **MLA (`deepseek2`) KV is over-counted** — GLM-4.7-Flash's compressed latent
   is not per-head.
3. **Headroom thresholds are unvalidated** against daily use (2 GB red, 4 GB
   amber on a 32 GB machine).
4. **`tauri dev` orphans servers** on every Rust rebuild: it SIGKILLs the app, so
   the exit handler never stops the child. Orphan scanning finds them now, but
   the cause is development-mode only.
5. **`rawArgs` bypasses structured validation** — typing `--host 0.0.0.0` there
   would expose an unauthenticated server. Default is loopback (D16).

## Exact next step

Build the downloader, per [docs/downloader-spec.md](../downloader-spec.md).
Suggested order, each independently verifiable:

1. **Resolve** — follow redirects manually, capture the final CDN URL,
   `x-linked-size` and `x-linked-etag`, confirm `accept-ranges`. Strip
   `Authorization` on the cross-host redirect.
2. **Single-segment download with resume** — `.part` file plus `.part.json`
   sidecar, re-resolving the expiring URL on every resume. Test by killing the
   process mid-transfer.
3. **Segmented transfer** — 4–8 ranged GETs, positioned writes, one shared rate
   limit, stall detection.
4. **Queue and UI** — the Downloads screen that currently says "not built yet".

Copy the runner's `EventSink` pattern for progress reporting: it is what makes
the process lifecycle testable without a window, and the downloader needs the
same property.
