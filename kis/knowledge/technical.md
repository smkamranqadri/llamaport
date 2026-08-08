# Technical

Tauri 2 desktop app. Rust backend, React 19 + TypeScript frontend built by Vite,
bun as the package manager. macOS only, either architecture.

**Nothing in the source is ARM-only** — there is no `cfg(target_arch)` anywhere,
and the shipped `.dmg` being `aarch64` was a fact about the build machine rather
than about the code. What is Darwin-bound is `sysmem.rs` entire (`sysctl` names,
`proc_pid_rusage`), the Trash through `NSFileManager`, `probe.rs`'s Homebrew
fallbacks, `~/Library/Application Support` and `open -R`. That list is what a
Linux port would cost, and it is why the answer is no rather than not yet.

## Layout

```text
src-tauri/src/
  lib.rs       tauri commands, AppState, tray, window; the seam to the UI
  catalog.rs   scan the models directory, group shard sets, disk space
  gguf.rs      GGUF header parser
  estimate.rs  weights and KV-cache arithmetic
  probe.rs     discover llama-server, probe its accepted flags
  runner.rs    spawn, supervise, telemetry, orphan detection
  health.rs    the ordered model test
  download.rs  the transfer engine: resolve, segments, resume, verify
  downloads.rs the job manager the commands drive; admission and settling
  store.rs     the single JSON config under Application Support
  sysmem.rs    machine memory readings via libc
  profile.rs   launch settings -> argv
src/
  App.tsx, Library.tsx, ModelDetail.tsx, ProfileForm.tsx, SettingsScreen.tsx,
  HealthPanel.tsx, Memory.tsx, Downloads.tsx, api.ts, types.ts, format.ts
src-tauri/tests/   integration tests; real_* need a model, a binary or the network
```

Both long-running subsystems report through a trait they own rather than calling
Tauri: the runner through `EventSink`, the downloader through `ProgressSink`.
That is what makes spawn -> Ready -> telemetry -> stop, and resolve -> transfer
-> verify, testable against a stand-in with no window. Follow the pattern rather
than adding a third idiom.

Returning new state to a caller is not the same as announcing it. A command hands
its snapshot to the window, but the tray has no caller and learns everything from
the event stream, so a state change that only returns leaves the menu bar stale.
Assert on what was emitted, not only on the snapshot: the snapshot is right in
exactly the case this gets wrong.

Config is one JSON file at schema 7, every field `#[serde(default)]`, with
unknown keys preserved through a load/save round-trip. `migrate` strips keys the
app deliberately retired, so a new field must never reuse a retired name — serde
would claim it first and adopt settings from a build several schemas old.

Finished downloads live beside it in `downloads.json`, written atomically on
state transitions and never on a progress tick. A file of its own because a
transfer settles often and the config holds the models directory, the
llama-server path and every remembered launch — an unreadable history should cost
the user their history and nothing else.

It holds more than the history: **whatever nothing else on disk remembers.** A
transfer that moved bytes is described by the sidecar beside its `.part`, which
cannot go stale while it runs, and that account is left to own it. A queued row
has no `.part` and no sidecar, and neither does the Paused row a queued one comes
back as — so both are written here, and `only_record_of` is where that is
decided. Writing only the queued state made the queue survive one restart and not
the next.

Everything the app keeps lives in `~/Library/Application Support/llamaport`:
that config, `downloads.json`, the runner pidfile, the last run log.
`store::adopt_legacy_dir`
takes over the directory left under the old `llama-cpp-hub` name, once, as the
first statement in `setup` — before the pidfile is read or the config is loaded,
both of which are in that same block.

## Run

```bash
bun install
bun run tauri dev
```

## Verify

Also in the README. Both copies are deliberate: the session anchor loads this
one, contributors read that one. Not a duplication to clean up.

```bash
bun run build
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
```

`cargo` is not on the PATH of a non-interactive shell here — it lives in
`~/.cargo/bin`, which only a login shell picks up. Without it all three cargo
commands exit **127**, which is "command not found" wearing the shape of a
failing check. Export it first: `export PATH="$HOME/.cargo/bin:$PATH"`.

## Constraints

- HTTP is `ureq` with `default-features = false, features = ["tls"]`, which is
  rustls plus webpki-roots. Blocking, one thread per connection — there is no
  async runtime and nothing needs one.
- Rust edition 2021; deps are deliberately few (`serde`, `sha2`, `sysinfo`,
  `ureq`, `libc`).
- No comments that narrate what code does; the codebase keeps them for
  non-obvious why only.
- The shell here is **zsh**, so `$PIPESTATUS` is not the array you want —
  bash's name for it. It expands to nothing, `echo "x: ${PIPESTATUS[0]}"` prints
  an empty status, and the command reads as though it reported success. zsh's own
  is `$pipestatus` (lowercase). The rule below is the reliable answer either way.
- Capture a command's exit status directly. `cmd | tail -3; echo $?` reports
  `tail`'s status, which reported a failing clippy as clean twice in one session.
- **A new test is not trusted until the code it covers has been gutted and the
  test watched to fail.** Used on every phase since the downloader, and it has
  paid twice: two tests that passed for the wrong reason, and a guard nothing
  would have noticed the loss of. A green suite is not the claim; a suite that
  fails when the code is wrong is.
- **Read the file the app actually wrote.** Four defects in this project were
  found by looking — at the screen, or at `downloads.json` between runs — and
  none of them by the suite. Anything that persists state earns this check.
- macOS substitutes an em dash for `--` inside the webview's text fields, so a
  flag typed into any field is corrupted before it reaches Rust and
  `llama-server` rejects it. A field that takes flags undoes the substitution on
  input. Only a token's leading dash can be one — substitution needs two
  hyphens, so hyphens inside `--cache-type-k` are the user's own.
- A field whose value is a parsed list rendered back as text cannot be typed
  into: the separator is dropped the instant it is typed. Such a field keeps its
  own text and re-seeds from props only when they disagree.
- `on_window_event` fires for every window, so any handler there must check
  `window.label()`. The close-hides-instead rule is the main window's alone.
- A `.part` and its sidecar are named from the destination — `{dest}.part` and
  `{dest}.part.json` — so a partial is found by scanning the models directory for
  the sidecar suffix and stripping it. `catalog::scan` filters on the `.gguf`
  extension and will never see either.
- The models directory and the config directory hold **untrusted input**. A `.part`,
  a `.part.json` and `downloads.json` are files anything with write access can
  create, and each of them names something the app then acts on — a URL to fetch,
  a path to write, a path to delete. Every one is re-validated on the way in.
  Two live holes came from forgetting this.
- A `.part` is opened with `O_NOFOLLOW` (`custom_flags`), never a plain `open`.
  `lstat`-then-open is two syscalls with a window between them, which is not a
  guard against a link planted on purpose.
- `admit` is not the only gate a transfer passes. Anything that starts one —
  today `start` and `resume` — validates the URL itself. Resume once did not, and
  that asymmetry was the whole vulnerability.
- A restored row's **file name** is validated too, not only its path. `path` is
  rebuilt as `models_dir.join(file_name)`, and `join("../evil")` lands outside
  the directory the rebuild was meant to confine it to. Rebuilding a path from an
  untrusted name is not a check.
- One transfer at a time is enforced by **queueing**, not by refusing. The
  invariant is one line — nothing `Active` and something `Queued` means the head
  of the queue starts — and every path a transfer can settle on runs it, on the
  finishing transfer's own thread after the jobs lock is released. Promoting from
  inside the settle path takes the same mutex and deadlocks.
- A queued job carries the `Options` it was admitted on, because it starts on a
  thread with no caller to ask. The rate limit is the one term that stays live:
  `set_rate_limit` rewrites it on waiting rows as well as on the running one.
- What `admit` checked can be hours stale by the time a queued job starts. The
  destination is re-checked at the moment of promotion, and a file that landed
  meanwhile fails that row rather than being written over.
- `AppState::save_config` takes the config lock itself, and a `std::sync::Mutex` is
  not reentrant. Anything that edits the config must drop its guard — a scoped
  block — before saving. Calling it while holding the guard deadlocks that path
  outright, and the paths that edit the config are launching and settling.
- **`Ready` is announced on every telemetry tick, not once.** The `runner:state`
  stream carries the current state, so a listener that acts on Ready acts dozens
  of times per run. Anything it does must be idempotent or guarded against the
  repeat; `store::stamp_if_newer` is the guard, keyed on `started_secs`, which
  moves only when a process is spawned.
- A retired key must be checked for *shape*, not only for name. `lastRun` on real
  v1 configs is a map of model id to timestamp — the shape a launch-time field
  wants — so naming that field `lastRun` would have serde adopt five-year-old
  values as this build's. Proved by naming it that and watching the test fail.
- There is no frontend test framework. Every test is Rust, in `src-tauri/tests/`
  or an inline `#[cfg(test)]` module; TypeScript is covered by `tsc` and by
  looking at the screen. Logic worth testing belongs in Rust until that changes.
