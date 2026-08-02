# Llamaport

A macOS app for running local GGUF models. Lists what is in your models
directory, launches one under `llama-server` with the exact command visible, and
shows what it costs while it runs.

Fetches them too. Paste the URL of a `.gguf` on Hugging Face and it downloads
over four ranged connections, resumes from where it stopped after a kill or a
dropped connection, verifies sha256, and lands the file in your models
directory — see [docs/downloader-spec.md](docs/downloader-spec.md).

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
| [docs/runner-spec.md](docs/runner-spec.md) | Listing and running models |
| [docs/downloader-spec.md](docs/downloader-spec.md) | Downloading with resume |

## Notes

Settings are not saved as profiles. A model's form opens with whatever it was
last launched with, and a successful launch updates that.

Memory is reported, not predicted: exact weights and KV cache arithmetic before
launch, live system memory and swap while running, and no verdict in between.

The server binds to `127.0.0.1` and has no authentication, so anything that can
reach it can use it. Extra arguments are passed to `llama-server` verbatim, with
one exception: `--host` and `--port` are refused there, because the app owns both
and `llama-server` takes the last value it is given. Without that, a `--host
0.0.0.0` typed into extra arguments would put an unauthenticated server on the
network while the app went on reporting it as loopback.
