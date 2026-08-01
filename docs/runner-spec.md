# Runner — specification

How the app lists local GGUF models and runs one under `llama-server`. Built.
The companion document, [downloader-spec.md](downloader-spec.md), covers the
unbuilt half.

Everything below that reads like an odd constraint is one: each was measured on a
32 GB M2 against llama.cpp build 10090 and Qwen3.6-35B-A3B, and each replaced an
assumption that turned out to be wrong.

## What it replaces

```
llama-server -m "$HOME/models/Qwen3.6-35B-A3B-UD-Q3_K_XL.gguf" \
  --alias qwen3.6-35b-a3b --host 127.0.0.1 --port 8888 \
  --jinja -c 65536 -ngl all -np 1 \
  --flash-attn on --cache-type-k q8_0 --cache-type-v q8_0
```

Stable across models except `-m`, `--alias` and `-c`, yet retyped every time,
with nothing recording which context a model supports or whether the chosen `-c`
fits in memory.

## Scope

**In:** list models with real metadata, launch one, show what it costs, stop it,
say whether it works.

**Out:** a chat UI (`llama-server` ships one), cross-platform support, managing
the llama.cpp install, profiles or saved presets, API keys, non-loopback binding.

## Modules

```
catalog.rs   scan the models directory, group shard sets
gguf.rs      hand-rolled GGUF header parser
probe.rs     locate llama-server, parse --help into a capability set
profile.rs   the values one launch uses, rendered to argv
estimate.rs  memory prediction and residency calibration
sysmem.rs    native macOS memory readings — the only unsafe in the project
safety.rs    green/amber/red judgement over those readings
runner.rs    supervise one child: health gate, telemetry, orphan detection
health.rs    the model test
store.rs     config, atomic writes, schema migration
```

```
Library → ModelDetail
  launch_plan(model_id, draft?) → resolve alias and clamp context
                                → argv + command preview
                                → memory estimate + safety verdict
                                → port conflict check
  Run → Runner::start → spawn + reader/health/waiter threads
                      → EventSink → Tauri events → React
  Exit → classified by phase → calibration sample
```

`launch_plan` runs on every keystroke. It must read cached catalog and cached
capabilities, never rescan.

## Constraints that were learned the hard way

### Parse the whole GGUF KV block

`tokenizer.ggml.tokens` runs to hundreds of thousands of length-prefixed strings
and `tokenizer.chat_template` sits *after* it, so a fixed-size header read misses
the template. Walk the block, skipping array payloads; read through the buffer
for skips under 64 KB, since seeking discards it.

Identity is `(size, sha256 of the first 4 KB)` — free during a scan that already
reads the header, and stable across renames and moves.

### Probe the CLI; never assume a flag

`--flash-attn` changed from a bare switch to taking `on|off|auto`. `--jinja` and
`--slots` are enabled by default on current builds. Parse `--help` at startup,
gate every flag on what this build accepts, and cache by binary mtime and size.

Append `--metrics` silently — it costs nothing and the telemetry view needs it.

### No per-process memory figure describes this workload

One running model, three sources, all correct and all different:

| Source | Reported |
| --- | --- |
| `sysinfo` process memory (RSS) | 16.2 GB |
| Activity Monitor / `ri_phys_footprint` | 1.28 GB |
| System wired memory | 20.0 GB |

`-ngl all` puts weights and KV cache in Metal buffers the kernel owns. Predict
and calibrate **machine-wide**; show the process footprint only if it is labelled
as excluding GPU-resident weights.

Readings come from `sysctl` and `proc_pid_rusage` via `libc`, not from parsing
`vm_stat` or `memory_pressure`. Every accessor returns `Option`; a size mismatch
returns `None` rather than trusting the bytes. One unavailable metric must render
"Unavailable" without blanking the rest.

### Calibrate a ratio, not an overhead

Loading a 15.7 GB model with a 2.7 GB KV cache grew machine memory by ~10.6 GB —
less than the weights alone, because mmapped pages and Metal buffers are not all
counted as used. An additive fit therefore sees a negative residual on every run
and never accumulates a sample.

Predict two distinct quantities: **nominal** (`weights + KV + overhead`, what the
model needs on paper) and **machine impact** (`residency × (weights + KV)`, what
used memory will grow by). The safety verdict consumes machine impact.
Uncalibrated it falls back to nominal, which over-predicts here — the safe
direction. Discard ratios outside 0.1–2.0 as measurement noise.

Record a sample on **both** exit paths. An explicit stop takes the child out from
under the waiter thread, so a waiter-only implementation records nothing during
normal use and calibration never converges.

### The kernel's pressure signal outranks any percentage

`kern.memorystatus_vm_pressure_level` is authoritative when it reports warning or
critical. Projected headroom and swap are heuristics on top. Worst signal wins.

Subtract memory attributable to a running model before projecting a replacement —
one model runs at a time, and double-counting reports an ordinary model swap as
impossible.

### Metric names vanish between builds

Build 10090 exposes **no** `kv_cache_*` series. Occupancy is derived as
`n_tokens_max / n_ctx`, with `kv_cache_usage_ratio` as a fallback for older
builds. Counters are cumulative: rates are deltas between polls, a decrease means
the process restarted, and the first poll yields no rate. Show the server's own
last-request figures when idle, or a live delta reads zero the instant generation
stops and looks broken.

### Reasoning models answer in a different field

Qwen-family models emit `reasoning_content` deltas before any `content`, and will
spend a small token budget entirely on thinking. A probe that reads only
`content` reports a healthy server as failed. Detect reasoning across
`reasoning_content`, `reasoning`, `thinking` and inline `<think>` tags, treat a
reasoning-only answer as a warning rather than a failure, and give the probe
enough budget to finish thinking.

### A busy port refuses the launch

Falling forward to the next free port was the original design. It produced two
copies of the same 15.7 GB model resident simultaneously, on a port no client was
configured for. Refuse, name the occupant, and distinguish another llama-server
from an unrelated process — the remedies differ.

Refuse equally when the model is already running elsewhere.

### Find orphans by scanning

A pidfile only ever knows the last pid written to it. `tauri dev` SIGKILLs the
app on every rebuild, so the exit handler never stops the child and servers
accumulate invisibly — two ran for hours, one costing measurable inference speed.
Scan for `llama-server` processes, report them with model and port, and never
kill one without being asked.

### Depth changes throughput

Decode speed roughly halves between an empty context and a working one:

| Depth | Prompt eval | Decode |
| --- | --- | --- |
| 17 tokens | noise (4 uncached) | 34.4 tok/s |
| 7,091 tokens | 445.7 tok/s | 24.2 tok/s |
| 17,272 tokens | 373.7 tok/s | 17.0 tok/s |

Any measurement presented as performance must state the depth it was taken at,
and must disable prompt caching with a per-run nonce or it measures nothing.

## Persistence

One file, `~/Library/Application Support/llama-cpp-hub/config.json`, written to a
temporary path and renamed. Holds the models directory, the llama-server path,
calibration samples, and the settings each model was last launched with.

`Profile` carries `#[serde(default)]` per field: without it, one missing key
fails the whole document and `unwrap_or_default()` silently discards every other
setting.

Unknown keys from a newer build survive a round-trip. Keys this build
deliberately retired are dropped at migration — preserving those forever is the
unknown-key rule applied where it does not belong.

## Settings, deliberately not profiles

There is no profile system: no templates, no named configurations, no merge
layers. A model's form opens with whatever that model was last launched with, and
a **successful** launch updates it. Settings that failed to start are not what
anyone wants to return to.

## Testing

Process lifecycle is driven through an `EventSink` trait rather than `AppHandle`,
so spawn → Ready → telemetry → stop is testable without a window. The health
checks run against a hand-rolled stand-in server, because the interesting cases
are failures a real model cannot be asked to produce on demand: a 500 on chat, an
unadvertised alias, a dead port, a dead pid.

Verification gate: `bun run build`, `cargo test`, `cargo clippy --all-targets --
-D warnings`, `cargo fmt --check`. One ignored test loads a real model; run it
deliberately.

## Known gaps

- Calibration has no samples yet, so estimates use the pessimistic nominal figure.
- MLA architectures (`deepseek2`) over-count KV — the compressed latent is not
  per-head.
- Headroom thresholds (2 GB red, 4 GB amber) are a judgement call, unvalidated
  against daily use.
- `rawArgs` bypasses structured validation; `--host 0.0.0.0` typed there would
  expose an unauthenticated server. Default binding is loopback.
- No API keys, so a server behind authentication cannot be tested.
