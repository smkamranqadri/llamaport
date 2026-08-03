# Technical

Tauri 2 desktop app. Rust backend, React 19 + TypeScript frontend built by Vite,
bun as the package manager. macOS on Apple Silicon only.

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

Config is one JSON file at schema 6, every field `#[serde(default)]`, with
unknown keys preserved through a load/save round-trip. `migrate` strips keys the
app deliberately retired, so a new field must never reuse a retired name — serde
would claim it first and adopt settings from a build several schemas old.

Finished downloads live beside it in `downloads.json`, written atomically on
state transitions and never on a progress tick. A file of its own because a
transfer settles often and the config holds the models directory, the
llama-server path and every remembered launch — an unreadable history should cost
the user their history and nothing else.

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

## Constraints

- HTTP is `ureq` with `default-features = false, features = ["tls"]`, which is
  rustls plus webpki-roots. Blocking, one thread per connection — there is no
  async runtime and nothing needs one.
- Rust edition 2021; deps are deliberately few (`serde`, `sha2`, `sysinfo`,
  `ureq`, `libc`).
- No comments that narrate what code does; the codebase keeps them for
  non-obvious why only.
- Capture a command's exit status directly. `cmd | tail -3; echo $?` reports
  `tail`'s status, which reported a failing clippy as clean twice in one session.
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
- There is no frontend test framework. Every test is Rust, in `src-tauri/tests/`
  or an inline `#[cfg(test)]` module; TypeScript is covered by `tsc` and by
  looking at the screen. Logic worth testing belongs in Rust until that changes.
