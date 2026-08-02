# Current

```text
Branch:   main (uncommitted: KIS, docs, engine, command layer, frontend)
Task:     none in progress — the live speed limit is done and proved
Mode:     Standard, tdd for the engine
Blocker:  none
Next:     nothing planned. Roadmap says packaging and release.
```

## Speed limit — done 2026-08-02

The engine has taken a rate limit since phase 1 and `set_download_options` has
always existed. Neither was reachable: `api.ts` had no wrapper and `Settings` in
`types.ts` did not declare the `downloads` field Rust was already sending.

Decided 2026-08-02: the limit applies to a transfer already in flight, not only
the next one. That is an engine change, so tdd.

What was built:

1. `Control` carries the live rate. It was already the shared handle the manager
   and the engine both hold, and already passed to `Bucket::charge`, so no new
   plumbing was needed. `Spec::rate_limit` was removed rather than left as a
   second source for the same number.
2. `Bucket` is always constructed and reads the rate from `Control` on every
   charge, so a change lands within one `CHARGE_SLICE` (250 ms). An unlimited
   transfer leaves on the atomic load, before the mutex.
3. The 64 KB/s floor stays in `downloads.rs` as app policy, behind
   `normalized_rate`. The engine honours what it is told — an engine test sets
   1 byte/s on purpose.
4. Frontend: `DownloadOptions` in `types.ts`, `setDownloadOptions` in `api.ts`,
   a Speed limit panel on the Downloads screen. The field is MB/s at 1024²,
   matching `formatRate`, so a limit typed as 10 does not read back as 9.5.

Run and verify commands: [knowledge/technical.md](../knowledge/technical.md).
Plan and decisions: [intent/downloader.md](../intent/downloader.md).

## Where the project actually is

Both halves of the original goal work. The runner lists, launches, supervises and
tests models. The downloader fetches from Hugging Face in four ranged segments,
survives a kill, resumes from its sidecar, verifies sha256 and lands the file in
the models directory.

Discover was planned and then dropped ([intent/roadmap.md](../intent/roadmap.md)).
Its nav placeholder is gone from `App.tsx`.

Everything before today is committed; `git log` is the record.

## Proof

- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `bun run build` — all exit 0, statuses captured directly rather than through a
  pipe.
- `cargo test` — 134 passing across 10 test binaries, exit 0. Two of those are
  new: one drives a real transfer at a byte a second, lifts the limit from
  another thread, and requires it to finish; one drives the manager and requires
  a change to reach a running job, the floor included.
- The manager test was checked against a gutted `set_rate_limit` and failed, so
  it detects the absence of what it claims to prove.
- Real transfer, 2026-08-02: 676 MB from Hugging Face killed mid-flight, resumed,
  sha256 verified, landed in the models directory, offered in Library. Predates
  the speed limit work.
- UI, 2026-08-02, screenshots of the running app during one 656 MB transfer.
  Field empty, hint "No limit", observed 19 KB/s at 0%. Field `1`, hint
  "Limited to 1.0 MB/s", observed 1.0 MB/s at 3%. Field `1.5`, hint "Limited to
  1.5 MB/s", observed 1.5 MB/s at 4%. One job throughout, progress climbing, so
  the change reached a transfer already running rather than the next one. The
  same screenshots show admission refusing a file already in the models
  directory and the Download button held while a transfer is active.
