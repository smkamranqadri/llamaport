# Technical

Tauri 2 desktop app. Rust backend, React 19 + TypeScript frontend built by Vite,
bun as the package manager. macOS on Apple Silicon only.

## Layout

```text
src-tauri/src/
  lib.rs      tauri commands, AppState, tray, window; the seam to the UI
  catalog.rs  scan the models directory, group shard sets
  gguf.rs     GGUF header parser
  estimate.rs weights and KV-cache arithmetic
  probe.rs    discover llama-server, probe its accepted flags
  runner.rs   spawn, supervise, telemetry, orphan detection
  health.rs   the ordered model test
  store.rs    the single JSON config under Application Support
  sysmem.rs   machine memory readings via libc
  profile.rs  launch settings -> argv
src/
  App.tsx, Library.tsx, ModelDetail.tsx, ProfileForm.tsx,
  SettingsScreen.tsx, HealthPanel.tsx, Memory.tsx, api.ts, types.ts
src-tauri/tests/   integration tests, incl. ones that need a real model present
```

The runner reports through an `EventSink` trait rather than calling Tauri
directly, which is what makes spawn -> Ready -> telemetry -> stop testable
without a window.

## Run

```bash
bun install
bun run tauri dev
```

## Verify

```bash
bun run build
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
```

## Constraints

- HTTP today is `ureq` with `default-features = false`, so **there is no TLS**.
  Every current request is plain HTTP to loopback. Reaching `huggingface.co`
  needs an HTTP client decision first.
- Rust edition 2021; deps are deliberately few (`serde`, `sha2`, `sysinfo`,
  `ureq`, `libc`).
- No comments that narrate what code does; the codebase keeps them for
  non-obvious why only.
