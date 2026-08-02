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

Config is one JSON file at schema 5, every field `#[serde(default)]`, with
unknown keys preserved through a load/save round-trip.

Everything the app keeps lives in `~/Library/Application Support/llamaport`:
that config, the runner pidfile, the last run log. `store::adopt_legacy_dir`
takes over the directory left under the old `llama-cpp-hub` name, once, as the
first statement in `setup` — before the pidfile is read or the config is loaded,
both of which are in that same block.

## Run

```bash
bun install
bun run tauri dev
```

## Release

`origin` is https://github.com/smkamranqadri/llamaport. Builds are unsigned:
there is no Apple Developer ID, so a downloaded build is quarantined and refused
until the user opens System Settings and allows it.

```bash
CI=true bun run tauri build
```

`CI=true` is not optional. Without it `bundle_dmg.sh` drives Finder through Apple
events to style the disk image window, which fails outright without Automation
permission and takes the whole build down with it. Setting it skips only the
cosmetic layout; the `/Applications` symlink and volume icon are unaffected.

## Verify

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
- Leaving the app for a URL needs no Rust and no capability edit: `opener:default`
  already grants `allow-default-urls`, which covers http. `reveal_path` is a
  hand-rolled command only because it needs `open -R` semantics.
- There is no frontend test framework. Every test is Rust, in `src-tauri/tests/`
  or an inline `#[cfg(test)]` module; TypeScript is covered by `tsc` and by
  looking at the screen. Logic worth testing belongs in Rust until that changes.
