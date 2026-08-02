# Current

```text
Branch:   main (clean, nothing pushed)
Task:     none in progress — downloader phase 1 closed and committed
Mode:     Phase
Blocker:  none
Next:     plan phase 2, Discover — see intent/downloader.md
```

Run and verify commands: [knowledge/technical.md](../knowledge/technical.md).
Plan and decisions: [intent/downloader.md](../intent/downloader.md).

## Where the project actually is

Both halves of the original goal now work. The runner lists, launches, supervises
and tests models. The downloader fetches from Hugging Face in four ranged
segments, survives a kill, resumes from its sidecar, verifies sha256 and lands the
file in the models directory. Discover is still a "Not built yet" placeholder.

Six commits, `c5bf0b8..dbc04de`: engine, command layer, Downloads screen, two test
fixes, KIS.

## Proof

- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `bun run build` — all exit 0.
- `cargo test` — 132 across 8 binaries, exit 0. Verified independently after each
  agent workflow rather than taken from the agents' reports.
- Real transfer, 2026-08-02: 676 MB from Hugging Face killed mid-flight, resumed,
  sha256 verified, landed in the models directory, offered in Library.

## Open, not blocking

- **"Beats a single connection" is unmeasured** — the one phase-1 criterion with
  no evidence either way.
- **The window can start unusable.** Twice in one session the app came up with no
  window at all, once at 60x60. `show_main_window` is meant to prevent exactly
  that and does not reliably. Predates the downloader.
- **A test whose guard is weaker than its name.**
  `resolution_against_a_silent_server_is_bounded_by_its_timeouts` pins that *some*
  timeout bounds resolution, not which — either one alone keeps it passing.
