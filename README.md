# Llamaport

A small macOS app for running and downloading local GGUF models. It wraps
`llama-server`: lists what is in your models folder, launches one with the exact
command visible, shows what it costs while it runs, and fetches new models from
Hugging Face with resume that survives a kill.

macOS, Apple Silicon or Intel. This is an unsigned beta — see
[Install](#install).

![Opening a model from the Library shows its quantisation, parameters, context and
the launch settings it will use. Run starts llama-server and the page turns into
a live view of KV cache, memory and token rates. Test model then reports every
check passing — process, port, health, model list, alias, chat completion and
streaming — with generation at 285 tokens per second](docs/launch.gif)

## Who it is for

People already running llama.cpp locally who are tired of two specific things:
retyping the same `llama-server` command with three values changed, and watching
`curl -C -` fail to resume a 20 GB download. If neither of those bothers you, you
do not need this.

## What it does

**Runs one model at a time.** Reads GGUF headers directly, so the list shows real
quant, context length and MoE structure rather than guesses from the filename.
The launch command is shown in full before you press start, and a model's form
opens with whatever it was last launched with successfully. The Library orders
itself by what you have run most recently, so the models you actually use stay at
the top.

![A running model, showing live KV cache, generation and prompt-eval rates, system
memory and swap, the model's own metadata, the launch settings that produced it,
and the full llama-server command](docs/running.png)

**Tells you whether it actually works.** **Test model** runs an ordered set of
checks against the running server — process, port, health endpoint, model list,
alias, and a real chat completion — and reports what passed, what only warned,
and how long the first token took. A ready model also opens llama.cpp's own Web
UI in a second window; that chat interface is llama.cpp's, not Llamaport's.

**Sizes a launch against the real ceiling, and never forecasts speed.** Weights
and KV-cache arithmetic before launch, judged against what the GPU will actually
hand out and what is free right now, with a floor marked as a floor; live system
memory and swap while running. How fast a setting is gets measured or stays
unsaid — a forecast wrong by 2x gets believed, so it does not make one.

**Downloads with working resume.** Paste the URL of a `.gguf` on Hugging Face. It
transfers over four ranged connections, re-resolves the CDN redirect on every
resume — which is exactly where `curl -C -` fails — verifies sha256, and only
then moves the file into your models folder. Kill it, reboot, come back: it picks
up where it stopped. A server that will not serve ranges is refused outright,
because an unresumable 20 GB transfer is a trap, not a convenience. A second URL
queues behind the first rather than being refused.

![The Downloads screen, with a speed limit applied to the transfer already
running, several paused transfers each offering Resume or Discard, and a history
of completed downloads](docs/download.png)

## Requirements

- macOS on Apple Silicon or Intel
- `llama-server`, which Llamaport finds on `PATH` or in the usual Homebrew
  locations:

```bash
brew install llama.cpp
```

Llamaport does not bundle or manage llama.cpp. If it cannot find `llama-server`
it says so and lets you point at one in Settings.

## Install

Download a `.dmg` from [Releases](../../releases), open it, and drag Llamaport to
Applications. Two are published, and on Apple Silicon either one works:

| File | For |
| --- | --- |
| `Llamaport_<version>_aarch64.dmg` | Apple Silicon, half the size |
| `Llamaport_<version>_universal.dmg` | Apple Silicon or Intel |

**The universal build has never been run on an Intel Mac.** Nobody here has one.
Its x86_64 half was launched under Rosetta and came up as a real app, which says
the slice is not a stub, but Rosetta is not Intel hardware. Reports either way
are wanted — particularly whether your `llama-server` has any GPU to offload to,
since the default `-ngl all` assumes one.

On Intel that means macOS 26 Tahoe and no further, because macOS 27 dropped Intel
Macs entirely. Nothing here needs a newer one; it is why yours has stopped being
offered them. Rosetta is not involved either way — a universal app runs natively
on both architectures, which is the whole point of shipping one.

**The first launch will be blocked.** This build is not signed with an Apple
Developer certificate, so macOS quarantines it and refuses to open it. That is
expected, and it is the price of a beta that costs nothing to publish. To open it
anyway:

1. Try to open Llamaport. macOS refuses and says it cannot verify the developer.
2. Open **System Settings → Privacy & Security**, scroll to **Security**.
3. There is now a line about Llamaport with an **Open Anyway** button. Click it,
   then confirm.

Or, if you would rather do it in one line:

```bash
xattr -dr com.apple.quarantine /Applications/Llamaport.app
```

Only run that on software you meant to download. If being unsigned is a problem
for you, build from source instead — the result is identical and locally signed.

**On v0.6.0 and earlier, macOS says "Llamaport.app is damaged and can't be opened"
instead.** It is not damaged. Those builds shipped without a bundle signature, so
macOS could not check them at all and refused rather than asking. The steps above
do not help, because no **Open Anyway** button appears for that message — use the
`xattr` line, or download v0.6.1 or later, which are signed well enough for macOS
to ask the ordinary question.

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

That builds for the machine you are on. For the universal build, which runs on
both architectures:

```bash
rustup target add x86_64-apple-darwin
CI=true bun run tauri build --target universal-apple-darwin
```

For development:

```bash
bun run tauri dev
```

## What it does not do

- **No chat UI.** `llama-server` already ships one; Llamaport opens it.
- **No Windows or Linux.** macOS only: the memory readings, the move-to-Trash and
  the config location are all Darwin, and none of it has an equivalent written.
- **No saved profiles to manage.** Three presets — Default, Best speed and
  Model suggested — set the fields nobody wants to hand-tune, and each model
  remembers only what it last launched with.
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
