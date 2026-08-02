# Current

```text
Branch:   main, 7 ahead of origin, nothing uncommitted. `v0.1.0` is tagged at
          `05c3a21`, so everything past it is unreleased and unpushed.
Task:     none in progress. The Web UI window is done and committed.
Mode:     Fast
Blocker:  none. One thing is unproved rather than blocking: the README's
          "Open Anyway" steps have never met a real Gatekeeper prompt.
Next:     download the `.dmg` from the release page in a browser and follow the
          README's Install section as a stranger would. Decide separately
          whether the seven unpushed commits want a `v0.1.1`.
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

A ready model offers **Web UI**, which opens `llama-server`'s own interface in a
second app window titled "llama.cpp — Web UI". An explicit stop closes it; a
Reload does not, because the server returns on the same port. The app still has
no chat of its own and is not getting one
([knowledge/project.md](../knowledge/project.md)).

**v0.1.0 is published**, unsigned, as a GitHub pre-release with the `.dmg`
attached: https://github.com/smkamranqadri/llamaport/releases/tag/v0.1.0. The tag
sits at `05c3a21`; `main` has moved past it and is not pushed, so the released
build and the tree are not the same thing.

Discover was planned and then dropped ([intent/roadmap.md](../intent/roadmap.md)).

Everything is committed; `git log` is the record.

## Proof

The four commands were last run green after the rename, which is the whole tree:
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`,
`bun run build` — all exit 0, statuses captured directly rather than through a
pipe. 137 tests across 10 binaries.

Release, 2026-08-03:

- The `.dmg` was built with `CI=true`, mounted, and checked: `Llamaport.app` at
  0.1.0, the `/Applications` symlink present, and `icon.icns` byte-identical to
  the source by sha256.
- The published asset was downloaded back from GitHub and is byte-identical to
  what was built, mounts, and holds the same app.
- Not proved, and the plan asked for it: the download was `curl`, which sets no
  quarantine attribute — checked, there is none on the file. So the Gatekeeper
  wall was never hit and the README's "Open Anyway" steps remain untested. That
  needs a browser download by a human.

Web UI window, 2026-08-03:

- Works, confirmed by the author against a running model: the window opens and
  the conversation persists across sessions. Both the browser version it replaced
  and this one were checked the same way — by the author looking, since screen
  recording is denied to the agent. Four commands green: build, fmt, clippy,
  test, 145 tests, each status captured directly.
- How llama.cpp's UI serves and stores was measured, not assumed, and the
  findings are in [knowledge/project.md](../knowledge/project.md) because they
  are stable facts rather than session history.
- Two stale UI claims were found and removed while checking the port. The
  pre-launch notice promised a fall-forward to the next free port; `spawn_run`
  refuses a busy port outright ([runner.rs](../../src-tauri/src/runner.rs)),
  which is what `knowledge/project.md` already recorded. A second notice, "was
  busy — listening on", was unreachable: `requested_port` was only ever set
  equal to `port`. That field is now deleted from Rust and TypeScript.
- Not covered, decided rather than missed: a **crash** leaves the window open on
  a dead server. Closing is driven from the two explicit stops, because `start`
  stops before it spawns and a listener on Idle would tear the window down on
  every Reload.
- Untested: `--no-webui` in `rawArgs` would leave the window on a 404. Left
  unguarded — `Capabilities.flags` lists the flag whether or not the UI is
  compiled in, so a probe would answer a different question.

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
