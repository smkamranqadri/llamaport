# Current

```text
Branch:   main (clean, nothing pushed)
Task:     none in progress
Mode:     Standard
Blocker:  none
Next:     packaging and release. Fix README.md:6 before it
          ([intent/roadmap.md](../intent/roadmap.md)).
```

Run and verify commands: [knowledge/technical.md](../knowledge/technical.md).
Plans and decisions: [intent/roadmap.md](../intent/roadmap.md).

## Where the project actually is

The runner lists, launches, supervises and tests models. The downloader fetches
from Hugging Face in four ranged segments, survives a kill, resumes from its
sidecar, verifies sha256 and lands the file in the models directory. Its speed
limit and its time-remaining estimate are both reachable from the Downloads
screen, and a limit changed there reaches the transfer already running.

The app is now Llamaport, identifier `com.mkamran.llamaport`, config under
`Application Support/llamaport` ([intent/rename.md](../intent/rename.md)).

Discover was planned and then dropped ([intent/roadmap.md](../intent/roadmap.md)).

Everything is committed; `git log` is the record.

## Proof

The four commands were last run green after the rename, which is the whole tree:
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`,
`bun run build` — all exit 0, statuses captured directly rather than through a
pipe. 137 tests across 10 binaries.

Rename, 2026-08-02:

- The config directory move ran for real, in the real app, on the real config.
  A `tauri dev` from earlier in the session rebuilt on the edit and relaunched as
  `target/debug/llamaport` at 22:57; afterwards `Application Support/llamaport`
  held the existing schema-5 config with both `lastUsed` profiles and
  `benchmarks.json` at its original mtime, and `llama-cpp-hub` was gone.
  `store.rs:68` is the only rename that can move that directory and `lib.rs:526`
  its only caller, so nothing else could have done it.
- The adoption test was checked against a gutted `adopt_legacy_dir` and failed,
  so it detects the absence of what it claims to prove. The two tests covering
  the declining cases pass against the stub — they guard the clobber rule, not
  the move.
- UI, three screenshots of the running app. Title bar, menu bar and
  sidebar all read Llamaport; Library lists all 9 models at /Users/mkamran/models;
  Settings still resolves /opt/homebrew/bin/llama-server, version 10090, 321
  flags. The window opened at a usable size, which the roadmap's third risk says
  cannot be assumed.

Downloader, 2026-08-02:

- The live-rate test was checked against a gutted `set_rate_limit` and failed, so
  it detects the absence of what it claims to prove.
- Real transfer: 676 MB from Hugging Face killed mid-flight, resumed, sha256
  verified, landed in the models directory, offered in Library.
- UI, screenshots of one running 656 MB transfer. The observed rate
  followed the limit from unlimited to 1.0 to 1.5 MB/s on the same job with
  progress climbing, so the change reached a transfer already running. The
  estimate read "about 5 min left" against a smoothed rate rather than the last
  sample.
