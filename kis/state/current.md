# Current

```text
Branch:   main. `origin` now configured, nothing pushed — the repo is public and
          empty, so the first push publishes. Held until phase 3.
Task:     beta release phase 4, ship — in progress. `.dmg` built and verified;
          push, tag and release remain.
          [intent/release.md](../intent/release.md)
Mode:     Phase
Blocker:  none. The `.dmg` no longer needs Apple events: `CI=true` makes
          create-dmg skip the Finder styling.
Next:     phase 1 blockers, then 3 public face, then 4 ship.
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

Tray staleness, 2026-08-03:

- Found by the author using the app, not by the suite: stopping from the window
  left the menu bar advertising a running model. `Runner::stop` changed state
  without emitting, and the tray reads only the event stream.
- 145 tests. The fix was checked by removal — strip the emit and
  `reaches_ready_reports_telemetry_and_stops` fails — and confirmed in the built
  `.app` by the author, who watched the menu return to "No model running".
- The orphan path was checked for the same shape and does not have it: the tray
  label reads the runner's own state, and orphans never reached it.

Blockers, 2026-08-02:

- 144 tests, up 7. The guard's tests fail against a gutted `check_raw_args`; the
  two asserting that unrelated flags still pass survive it, as a permissive stub
  should.
- `--no-host` and `--reuse-port` were checked against the installed
  `llama-server --help` rather than assumed: both are real flags, neither is
  blocked, and neither `--host` nor `--port` has a short alias in that build.
- The window fix is confirmed by the author looking at it, not by this session.
  Screen recording and Apple events are both denied here, so window geometry
  could not be read. Five launches of the bundled `.app` each survived, which
  proves the process starts and nothing more.

Icon, 2026-08-02:

- Three marks were rendered at 256px and at a real 32px and looked at before any
  was offered, which is why two were reworked first: the llama read as a rabbit
  and the abstract mark read as a wifi/broadcast icon, wrong for an app that
  binds loopback only.
- `Llamaport.app/Contents/Resources/icon.icns` is byte-identical to
  `src-tauri/icons/icon.icns` by sha256, `CFBundleIconFile` points at it, and
  `Resources/` holds exactly one icon. No Tauri logo survives in the bundle.
- Not claimed: nobody has watched the icon appear in the Dock. What is proved is
  that the bundle carries the mark and macOS is pointed at it.

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
  sidebar all read Llamaport; Library lists all 9 models in the models directory;
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
