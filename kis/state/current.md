# Current

```text
Branch:   main
Task:     Downloader phase 1 — engine and Downloads screen
Mode:     Phase
Status:   working end to end against real Hugging Face; nothing committed yet
Blocker:  none
Next:     commit phase 1, then Discover (phase 2)
```

Proven end to end in the app by the user on 2026-08-02, against the real Hugging Face
CDN, with the Qwen2.5-0.5B q8_0 file (675,710,816 bytes):

- Transfer ran to 99% with a live rate that varied 90 KB/s to 1.4 MB/s, so the rate is
  a real delta and not a cumulative average.
- 644 MB matched the advertised size, so `x-linked-size` is read off the 302.
- Cancel followed by resume worked — the failure that started this project.
- The one-at-a-time guard refused a second start while one was in flight.
- Verifying ran as its own phase with its own progress, 5 MB then 497 MB of 644 MB at
  21.9 MB/s, so a 21 GB file will not look frozen there.
- sha256 passed: the job reached Complete and the file landed in the models directory
  rather than being discarded, which is what a mismatch would have done.
- Finished row offers "Show in Library"; free space fell 125.9 GB to 125.4 GB.

Phase 1 acceptance is met. The only criterion never measured is "beats a single
connection" — the observed rate was variable and the line, not the engine, looked like
the limit.

Verification plan: `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check`,
`bun run build`, then real transfers per [intent/downloader.md](../intent/downloader.md).

Tested at one seam by the user's decision: `download()` only, driven against a
hand-rolled stand-in server. Stall timeout and backoff are inputs to `download()`
rather than constants, so tests provoke them in milliseconds.

## Ledger

- ureq TLS confirmed: `features = ["tls"]` on 2.12.1, `cargo check` clean.
- Engine core done: manual redirect resolution, 4-way segmented ranged transfer with
  positioned writes, sidecar resume, cancellation, expired-signature re-resolve.
  5 tests at the `download()` seam.
- Retry taxonomy done: 5xx and transport errors back off exponentially over 5 attempts,
  4xx stops dead, 403 past a redirect re-signs without spending an attempt, and a
  non-2xx at resolve time fails before any transfer starts. 7 tests at the seam.
- Stall detection done via a read timeout of `stall_after` on every ranged request, so
  a silent socket errors and is reissued from `completed`. A clean end of stream short
  of the range asked for is now also transient rather than success — it was renaming a
  file with a hole in it.
- Deviation from the spec: the spec kills a segment silent for 30s **while sibling
  segments progress**. The sibling condition is not implemented; any segment silent
  past `stall_after` is reissued, bounded by the 5-attempt retry limit. Cheaper, and
  on a dead link the attempt limit parks the transfer anyway.
- Shared token bucket done: one budget for the whole transfer, charged after each read
  so short reads are not over-charged. Per-segment budgets would multiply the cap by
  the segment count.
- Segment workers now take a single `Transfer` context struct instead of eight loose
  arguments, which is what clippy's `too_many_arguments` was pointing at.
- Verification done: sha256 streamed over the `.part` and compared with
  `x-linked-etag` when `verify` is set, and the partial plus sidecar are discarded on a
  mismatch rather than left to be resumed. Only etags that look like a sha256 are
  compared — a non-LFS etag is an opaque validator and would fail every time.
- Etag comparison on resume: a changed etag means upstream was replaced, so the
  partial is deleted and every segment refetched from zero.
- Free-disk check done, before a byte moves, counting bytes already in the `.part` as
  room the transfer no longer needs. Reuses `catalog::disk_space`, extracted from
  `dir_info` rather than duplicated.
- A server without `accept-ranges` is now refused outright, per the spec: no ranges
  means no resume, and an unresumable 20 GB transfer is a trap. `fetch_whole` survives
  only for a server that supports ranges but declares no size.
- A three-round review/judge/fix workflow (12 agents) found 40 issues, of which the
  judge confirmed 15 actionable; all were fixed with tests, and the engine suite grew
  from 14 to 26 tests. Rounds converged 10 -> 3 -> 2.
- Both minor findings fixed and independently verified: a failed mid-transfer re-sign is
  now transient rather than terminal, and the misleading silent-server test was renamed
  to what it actually guards. Caveat recorded by the verifier: that test pins "some
  timeout bounds resolution", not which one — either timeout alone keeps it passing.
- Progress reporting done: a `ProgressSink` trait owned by the download module with no
  Tauri dependency, mirroring `runner.rs`'s `EventSink`. Rate is a delta between
  samples and is `None` on the first report rather than a fabricated zero. Emission is
  throttled (default 500ms) on its own thread reading an atomic, so no sink call
  happens under a lock and workers never block on the UI. Resume opens at the resumed
  byte count; verification reports as it hashes.
- Still open in phase 1: progress reporting, Tauri commands, Downloads screen,
  real-transfer proof.

- Partially fixed a pre-existing flake in `runner_lifecycle`: `free_port()` bound port
  0, closed the listener and returned the number, so a parallel test could be handed
  the same port and the launch was refused. Replaced with a `Port` reservation held
  until launch plus a `HANDOVER` mutex. Confirmed pre-existing by reproducing it with
  all downloader changes stashed.
  INCOMPLETE: the mutex is process-local and `cargo test` runs each test binary as its
  own process, so `download_engine`'s stand-in servers — which bind ephemeral ports
  continuously — can still take the port between release and launch. Still reproduces
  under load. A cross-binary fix cannot be a mutex; retrying the reserve/release/launch
  when start reports the port occupied would preserve what the test discriminates.

## Proof

- `cargo fmt --check` — passed
- `cargo clippy --all-targets -- -D warnings` — passed
- `bun run build` — passed
- `cargo test` — download engine 34/34, whole suite exit 0, verified independently
  after each workflow rather than taken from the agents' reports.

Capture the exit status directly, never after a pipe: `cmd | tail -3; echo $?` reports
tail's status, which reported a failing clippy as clean twice.

Plan, decisions and closing conditions: [intent/downloader.md](../intent/downloader.md).
Design: [docs/downloader-spec.md](../../docs/downloader-spec.md).

## Where the project actually is

The runner half is built and working: catalog, launch, supervision, telemetry,
memory reporting, the model test, tray and window behaviour. The downloader is
specified and entirely unbuilt. The Downloads and Discover screens render
"Not built yet" placeholders.

## Proof

None yet for this task. The verify commands in
[knowledge/technical.md](../knowledge/technical.md) have not been run this
session, so treat the working tree as unverified until they are.
