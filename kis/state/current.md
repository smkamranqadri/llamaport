# Current

```text
Branch:   main (clean, nothing pushed)
Task:     none in progress
Mode:     Fast
Blocker:  none
Next:     nothing planned. Roadmap says packaging and release.
```

Run and verify commands: [knowledge/technical.md](../knowledge/technical.md).
Plans and decisions: [intent/roadmap.md](../intent/roadmap.md).

## Where the project actually is

The runner lists, launches, supervises and tests models. The downloader fetches
from Hugging Face in four ranged segments, survives a kill, resumes from its
sidecar, verifies sha256 and lands the file in the models directory. Its speed
limit and its time-remaining estimate are both reachable from the Downloads
screen, and a limit changed there reaches the transfer already running.

Discover was planned and then dropped ([intent/roadmap.md](../intent/roadmap.md)).

Everything is committed; `git log` is the record.

## Proof

- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test`, `bun run build` — all exit 0, statuses captured directly rather
  than through a pipe. 134 tests across 10 binaries.
- The live-rate test was checked against a gutted `set_rate_limit` and failed, so
  it detects the absence of what it claims to prove.
- Real transfer, 2026-08-02: 676 MB from Hugging Face killed mid-flight, resumed,
  sha256 verified, landed in the models directory, offered in Library.
- UI, 2026-08-02, screenshots of one running 656 MB transfer. The observed rate
  followed the limit from unlimited to 1.0 to 1.5 MB/s on the same job with
  progress climbing, so the change reached a transfer already running. The
  estimate read "about 5 min left" against a smoothed rate rather than the last
  sample.
