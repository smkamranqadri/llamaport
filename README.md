# Llamaport

A small macOS app for running and downloading local GGUF models. It wraps
`llama-server`: lists what is in your models folder, launches one with the exact
command visible, shows what it costs while it runs, and fetches new models from
Hugging Face with resume that survives a kill.

Apple Silicon only. This is an unsigned beta — see [Install](#install).

![The Library screen, listing local GGUF models with quantisation, context length,
size and modification date, and marking mixture-of-experts
files](docs/library.png)

## Who it is for

People already running llama.cpp locally who are tired of two specific things:
retyping the same `llama-server` command with three values changed, and watching
`curl -C -` fail to resume a 20 GB download. If neither of those bothers you, you
do not need this.

## What it does

**Runs one model at a time.** Reads GGUF headers directly, so the list shows real
quant, context length and MoE structure rather than guesses from the filename.
The launch command is shown in full before you press start, and a model's form
opens with whatever it was last launched with successfully.

**Reports memory, never forecasts it.** Exact weights and KV-cache arithmetic
before launch; live system memory and swap while running; no verdict in between.
A forecast wrong by 2x gets believed, so it does not make one.

**Downloads with working resume.** Paste the URL of a `.gguf` on Hugging Face. It
transfers over four ranged connections, re-resolves the CDN redirect on every
resume — which is exactly where `curl -C -` fails — verifies sha256, and only
then moves the file into your models folder. Kill it, reboot, come back: it picks
up where it stopped. A server that will not serve ranges is refused outright,
because an unresumable 20 GB transfer is a trap, not a convenience.

## Requirements

- macOS on Apple Silicon
- `llama-server`, which Llamaport finds on `PATH` or in the usual Homebrew
  locations:

```bash
brew install llama.cpp
```

Llamaport does not bundle or manage llama.cpp. If it cannot find `llama-server`
it says so and lets you point at one in Settings.

## Install

Download the `.dmg` from [Releases](../../releases), open it, and drag Llamaport
to Applications.

**The first launch will be blocked.** This build is not signed with an Apple
Developer certificate, so macOS quarantines it and refuses to open it. That is
expected, and it is the price of a beta that costs nothing to publish. To open it
anyway:

1. Try to open Llamaport. macOS refuses and shows a warning.
2. Open **System Settings → Privacy & Security**, scroll to **Security**.
3. There is now a line about Llamaport with an **Open Anyway** button. Click it,
   then confirm.

Or, if you would rather do it in one line:

```bash
xattr -dr com.apple.quarantine /Applications/Llamaport.app
```

Only run that on software you meant to download. If being unsigned is a problem
for you, build from source instead — the result is identical and locally signed.

## Build from source

Requires Rust and [bun](https://bun.sh).

```bash
bun install
CI=true bun run tauri build
```

`CI=true` is not optional here. Without it the disk image step drives Finder
through Apple events to style the window, which fails unless your terminal has
been granted Automation permission — and takes the whole build down with it.
Setting it skips only the cosmetic layout; the app, the `/Applications` symlink
and the volume icon are unaffected. Drop it if you want the styled window and
have granted the permission.

For development:

```bash
bun run tauri dev
```

## What it does not do

- **No chat UI.** `llama-server` already ships one; Llamaport opens it.
- **No Windows or Linux.** macOS on Apple Silicon only.
- **No profiles or presets.** One remembered setup per model, written by a
  successful launch.
- **One model at a time.** A busy port refuses the launch rather than moving to
  another one.
- **No Hugging Face token.** Public repos only; a gated repo is reported plainly.

## Security

The server binds to `127.0.0.1` and has no authentication, so anything that can
reach it can use it. Extra arguments go to `llama-server` verbatim, with one
exception: `--host` and `--port` are refused there. The app owns both, and
`llama-server` takes the last value it is given, so a `--host 0.0.0.0` typed into
extra arguments would put an unauthenticated server on your network while the app
went on reporting it as loopback. For the same reason the app appends its own
`--host` and `--port` last, where they win regardless.

## Documents

| File | What it is |
| --- | --- |
| [docs/runner-spec.md](docs/runner-spec.md) | Listing and running models |
| [docs/downloader-spec.md](docs/downloader-spec.md) | Downloading with resume |

Design notes and the working history live in [`kis/`](kis/).

## Verify a checkout

```bash
bun run build
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
```

## Licence

MIT — see [LICENSE](LICENSE).
