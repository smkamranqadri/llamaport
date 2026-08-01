# llama.cpp hub

A macOS app for running local GGUF models. Lists what is in your models
directory, launches one under `llama-server` with the exact command visible, and
shows what it costs while it runs.

Downloading models with resume is designed but **not built** — see
[docs/downloader-spec.md](docs/downloader-spec.md).

## Requires

macOS on Apple Silicon, `llama-server` on PATH or in a standard Homebrew
location, Rust, and bun.

## Run it

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

## Documents

| File | What it is |
| --- | --- |
| [DESIGN.md](DESIGN.md) | Architecture and what the runtime knows |
| [docs/downloader-spec.md](docs/downloader-spec.md) | The unbuilt half, specified |
| [docs/local-runtime/current-state.md](docs/local-runtime/current-state.md) | Where to resume |
| [docs/local-runtime/decisions.md](docs/local-runtime/decisions.md) | Every decision and what would reverse it |

## Notes

Settings are not saved as profiles. A model's form opens with whatever it was
last launched with, and a successful launch updates that.

The server binds to `127.0.0.1` and has no authentication. Do not change the host
without reading D16.
