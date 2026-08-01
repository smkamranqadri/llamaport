# llama.cpp hub — design

A macOS app for running local GGUF models: list what is in the models directory,
launch one under `llama-server`, and watch it while it runs.

Downloading models with resume was the other half of the original goal and is
**not built**. Its specification is in [docs/downloader-spec.md](docs/downloader-spec.md).

## Problem

Two shell commands held in memory. Running a model:

```
llama-server -m "$HOME/models/Qwen3.6-35B-A3B-UD-Q3_K_XL.gguf" \
  --alias qwen3.6-35b-a3b --host 127.0.0.1 --port 8888 \
  --jinja -c 65536 -ngl all -np 1 \
  --flash-attn on --cache-type-k q8_0 --cache-type-v q8_0
```

Everything but `-m`, `--alias` and `-c` is stable across models, yet the whole
line gets retyped or hunted out of shell history. Nothing records which context
length a model supports, or whether the chosen `-c` will fit in memory.

Downloading a model with `curl -L -C -` fails to resume and is slow, for reasons
covered in the downloader spec.

## Goals

- List local models with real metadata, not filename guesses.
- Launch `llama-server` with the exact command line always visible.
- Predict memory before launch, and show what actually happened after.
- *(Unbuilt)* Download GGUF files with resume that survives a restart.

## Non-goals

- A chat UI. `llama-server` ships one; the app links to it.
- Cross-platform. macOS on Apple Silicon only.
- Managing llama.cpp itself. The app consumes whatever `llama-server` is on PATH.
- Profiles, presets or saved configurations — see D20.

## Architecture

Rust behind Tauri commands, React for the UI, one JSON file for persistence.

```
src-tauri/
  catalog.rs   scan the models directory, group shards
  gguf.rs      hand-rolled GGUF header parser
  probe.rs     find llama-server, parse --help into a capability set
  profile.rs   the values one launch uses, rendered to argv
  estimate.rs  memory prediction and residency calibration
  sysmem.rs    native macOS memory readings (the only unsafe)
  safety.rs    green/yellow/red judgement over those readings
  runner.rs    supervise one child, health gate, telemetry, orphan detection
  health.rs    the model test
  store.rs     config, atomic writes, schema migration
src/
  Library      the model list
  ModelDetail  facts, launch form, memory, command, telemetry, logs
  Settings     models directory, llama-server path, calibration state
```

### Flow

```
Library → ModelDetail
  launch_plan(model_id, draft?) → resolve (alias, context clamp)
                                → Profile::args() → argv + preview
                                → estimate + safety assessment
                                → port conflict check
  Run → runner_start → Runner::start → spawn + three threads
                                     → EventSink → Tauri events → React
       settings remembered on success (D20)
  Exit → classified by phase → calibration sample
```

`launch_plan` runs on every keystroke, so it reads a cached catalog and cached
capabilities and never rescans.

## What the runtime knows

**Catalog.** GGUF headers are read directly. The KV block is walked in full
rather than sampled, because `tokenizer.ggml.tokens` runs to hundreds of
thousands of entries and `tokenizer.chat_template` sits after it. Identity is
`(size, hash of the first 4 KB)`, so renames and moves do not lose a model.

**Capabilities.** llama.cpp's CLI drifts. `--help` is parsed at startup and flags
are gated on what this build actually accepts — `--flash-attn` changed from a
switch to taking `on|off`, and `--metrics` is appended silently because the
telemetry view needs it.

**Memory.** No per-process figure describes this workload: on one running model
this machine reported 16.2 GB (RSS), 1.28 GB (physical footprint) and 20 GB of
wired memory. `-ngl all` puts weights and KV cache in Metal buffers the kernel
owns. Prediction is therefore machine-wide, and calibration fits a *ratio* rather
than an additive overhead — the machine grew by ~10.6 GB loading a model whose
weights alone are 15.7 GB, so residuals are reliably negative and an additive fit
never converges. See D6, D15.

**Safety.** The kernel's own pressure signal is authoritative; projected headroom
and swap are heuristics layered on top; the worst signal wins.

**Telemetry.** `/metrics` at 1 Hz. Build 10090 exposes no `kv_cache_*` series at
all, so occupancy is derived from `n_tokens_max / n_ctx`. Rates are deltas
between polls, with the server's own last-request figures shown when idle.

**Ports.** A busy port refuses the launch. Falling forward was the original
design and it produced two copies of the same model on a 32 GB machine, reachable
by nothing. See D17.

**Orphans.** Found by scanning for `llama-server` processes, not by trusting a
pidfile. Reported, never killed without being asked. See D14.

## Persistence

One file, `~/Library/Application Support/llama-cpp-hub/config.json`, written
through a temporary file and renamed. It holds the models directory, the
llama-server path, calibration samples, and the settings each model was last
launched with. Unknown keys from a newer build survive a round-trip; keys this
build deliberately retired are dropped at migration.

## Open questions

- A conservative default for the residency ratio before calibration has samples.
- MLA architectures (`deepseek2`) still over-count KV, since the compressed
  latent is not per-head.
- Whether the headroom thresholds (2 GB red, 4 GB amber) are right for daily use.
