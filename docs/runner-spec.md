# Runner — specification

List the GGUF models in a directory, launch one under `llama-server`, and show
what it is doing while it runs.

The problem it solves: the launch command is stable across models except for
three values, yet it gets retyped or hunted out of shell history every time, and
nothing records which context a model supports or what a given `-c` will cost.

```
llama-server -m "$HOME/models/Qwen3.6-35B-A3B-UD-Q3_K_XL.gguf" \
  --alias qwen3.6-35b-a3b --host 127.0.0.1 --port 8888 \
  --jinja -c 65536 -ngl all -np 1 \
  --flash-attn on --cache-type-k q8_0 --cache-type-v q8_0
```

**In scope:** list models with real metadata, launch one, report what it costs,
stop it, say whether it is working.

**Out of scope:** a chat UI (`llama-server` ships one), cross-platform support,
managing the llama.cpp installation, saved profiles or presets, API keys,
binding anywhere but loopback.

## Design

### Catalog

Read GGUF headers directly rather than guessing from filenames. The header is
magic, version, tensor count, KV count, then typed key/value pairs; strings are
`u64` length plus bytes.

Walk the **entire** KV block. `tokenizer.ggml.tokens` runs to hundreds of
thousands of length-prefixed strings and `tokenizer.chat_template` sits after it,
so a fixed-size prefix read silently misses the template. Skip array payloads
rather than parsing them, except integer arrays short enough to be per-layer
values. For skips under 64 KB read through the buffer instead of seeking, since
seeking discards it.

Keys worth extracting: `general.architecture`, `general.name`,
`general.size_label`, `{arch}.context_length`, `{arch}.block_count`,
`{arch}.embedding_length`, `{arch}.attention.head_count`,
`{arch}.attention.head_count_kv`, `{arch}.attention.key_length`,
`{arch}.attention.value_length`, `{arch}.expert_count`, and whether
`tokenizer.chat_template` exists.

Identity is `(file size, sha256 of the first 4 KB)` — free during a scan that
already reads the header, and stable across renames and directory moves. Do not
hash whole files; these are 13–21 GB.

Group `name-00001-of-00003.gguf` shard sets into one entry, parse only the first
shard, sum the sizes, and mark incomplete sets rather than offering to run them.
Show files that fail to parse, greyed with the error — a truncated GGUF is
usually a failed download, which is exactly what the list should reveal.

### Launching

Build argv, never a shell string. `Command::spawn` takes an argument vector, so
command injection through form fields is structurally impossible; render a
shell-quoted version for display only, and always show it.

**Probe the binary, do not assume its flags.** Run `--help` at startup, collect
the long flags into a set, and gate every option on what that build accepts.
`--flash-attn` takes `on|off|auto` on current builds and was a bare switch on
older ones — detect which by looking for `[on` or `on|off` on its help line.
`--jinja` and `--slots` default to enabled. Cache the probe by binary mtime and
size.

Append `--metrics` unconditionally where supported: it costs nothing and the
telemetry view depends on it.

Discovery order for the binary: a configured path, then `which`, then
`~/.local/bin`, then `/opt/homebrew/bin` and `/usr/local/bin`. A GUI app launched
from Finder inherits a minimal PATH, so `which` alone will not find a Homebrew
install.

Derive the alias from the model's display name when the user has not set one, and
clamp the requested context to `{arch}.context_length`.

### Process supervision

One model at a time. Spawn with piped stdout and stderr, then:

```
Idle → Starting → Ready → Stopping → Idle
                    ↓
                 Crashed
```

`Starting` means the process is alive; `Ready` means `GET /health` answered.
These are different claims and loading 20 GB takes real time, so do not conflate
them. Read `/props` once on `Ready` to confirm the server got the context that
was asked for.

Classify an unexpected exit by phase. Before `Ready` it is a configuration
problem — too much context, an unsupported flag, a corrupt shard — so do not
restart, and surface the last 20 stderr lines inline, because the reason is
always there. After `Ready` it is more likely transient: restart once, then stop.

Mirror log lines to a file as they arrive and serve that file when the in-memory
buffer is empty. The lines that explain a crash are the ones worth keeping, and
they die with the process otherwise.

Report through a trait rather than calling the UI framework directly. That is
what makes spawn → Ready → telemetry → stop testable against a stand-in HTTP
server, with no window.

### Ports and stray servers

**A busy port must refuse the launch.** Falling forward to the next free port
produces a second server on a port no client is configured for, and with 15–20 GB
models it silently doubles memory use. Name the occupant and distinguish another
`llama-server` from an unrelated process — the remedies differ.

Refuse equally when the requested model is already running somewhere.

Find stray servers by **scanning for `llama-server` processes**, not by reading a
pidfile: a pidfile only ever knows the last pid written to it, and a hard kill of
the app leaves children it will never hear about again. Extract each one's port
and model from its command line, report them, and never terminate one without
being asked.

### Memory

Report; do not forecast.

Two numbers are exact and belong on screen before launch:

```
weights = file size
kv      = layers × ctx × kv_heads × (k_dim × bpe_k + v_dim × bpe_v)
```

Sum K and V separately rather than doubling one dimension: latent-attention
architectures size them differently — `deepseek2` reports a 576-wide key with a
single KV head. Bytes per element: `f16`/`bf16` 2.0, `q8_0` 34/32, `q5_1` 24/32,
`q5_0` 22/32, `q4_1` 20/32, `q4_0` and `iq4_nl` 18/32.

These answer the question actually being asked — what does this `-c` cost, and
what does `q4_0` save over `q8_0` — and neither depends on the machine.

**Do not predict total memory impact.** On Apple Silicon with `-ngl all`, weights
and KV cache live in Metal buffers attributed to the kernel, so no per-process
figure describes the workload: one running model reports 16.2 GB as RSS, 1.28 GB
as physical footprint, and 20 GB of system wired memory, all three correct. The
ratio of machine growth to nominal weights+KV varies between 0.42 and 0.85 for
the *same model at the same context*, because residency depends on what else is
resident and what the OS evicts. A forecast that can be wrong by 2× is worse than
none, because it will be believed.

Instead, sample the machine while a model runs: installed memory (`hw.memsize`),
memory in use, swap (`vm.swapusage`), macOS pressure
(`kern.memorystatus_vm_pressure_level` → 1 normal, 2 warning, 4 critical), and
the process footprint (`proc_pid_rusage` → `ri_phys_footprint`, the figure
Activity Monitor shows). Label the footprint as excluding GPU-resident weights or
it will be misread.

Read these through `libc`, not by parsing `vm_stat` or `memory_pressure` output.
Return `Option` from every accessor and treat a size mismatch as `None` rather
than trusting the bytes. Render a missing reading as "Unavailable"; one
unavailable metric must not blank the panel.

### Telemetry

Poll `/metrics` once a second while `Ready`.

**Metric names change between builds.** Build 10090 exposes no `kv_cache_*`
series at all: derive context occupancy as `n_tokens_max / n_ctx`, falling back
to `kv_cache_usage_ratio` where an older build provides it. Probe, do not assume.

Counters are cumulative, so throughput is a delta between polls: the first poll
yields no rate, and a counter that decreases means the process restarted and the
baseline must be dropped rather than differenced. A bare delta reads zero the
instant generation stops, which looks broken — fall back to the server's own
`predicted_tokens_seconds` and `prompt_tokens_seconds`, which persist between
requests, and label them as last-request figures.

Worth showing: context occupancy, generation and prompt-eval rates, cumulative
tokens, queue depth (`requests_processing`, `requests_deferred`), and the machine
memory readings above.

### The model test

An ordered list of checks, each timed and reported separately, so a partial
failure says which part failed:

```
process alive → port reachable → /health → /v1/models → alias advertised
             → chat completion → streaming → reasoning detection
```

Grade the results. An alias the server does not advertise is a **warning** — the
request still works, the server simply reports a different id. A failed stream is
a warning. An unreachable port **stops the run**: a report claiming chat
completion passed against a dead port is worse than no report. Overall verdict is
Passed, Passed with warnings, or Failed.

**Reasoning models answer in a different field.** Qwen-family models emit
`reasoning_content` deltas before any `content`, and will spend a small token
budget entirely on thinking — so a probe reading only `content` reports a healthy
server as broken. Detect reasoning across `reasoning_content`, `reasoning`,
`thinking` and inline `<think>` tags; treat a reasoning-only answer as a warning,
not a failure; and give the probe enough budget (~96 tokens) to finish thinking.

Keep the prompt short, `temperature: 0`, and the budget small — the probe must
not consume context the user wanted for real work.

### Persistence

One JSON file under `~/Library/Application Support/`, written to a temporary path
and renamed so an interrupted write cannot truncate it. It holds the models
directory, the llama-server path, and the settings each model was last launched
with.

There is no profile system. A model's form opens with whatever that model was
last launched with, and a **successful** launch updates it — settings that failed
to start are not what anyone wants to return to.

Give the settings struct `#[serde(default)]` per field. Without it one missing
key fails the whole document, and a fallback to defaults then discards every
other setting in the file.

Preserve unknown keys through a load/save round-trip so an older build cannot
delete what a newer one wrote — but drop keys a version deliberately retires,
which is a migration's job rather than the unknown-key rule's.

## Gotchas to build in from the start

- **Recompute the launch preview on every keystroke**, from a cached catalog and
  cached capabilities. It must never rescan the directory or re-probe the binary.
- **State the context depth of any throughput figure.** Decode speed roughly
  halves between an empty context and a working one — 34 tok/s at 17 tokens,
  24 at 7k, 17 at 17k on the same model. A measurement without its depth is
  meaningless.
- **Disable prompt caching with a per-run nonce** when measuring prompt speed, or
  the server serves the prefill from cache and the number describes nothing.
- **Show the full filename on hover**, and warn only when a chat template is
  *absent*. A badge that appears on every model carries no information.
- **Closing the window should hide it**, leaving the server running under a menu
  bar item. Quitting stops the server; the child is not killed on drop.
- **Assert a usable window frame on startup** — unminimise, restore the
  configured size if smaller than the minimum, centre, focus. An unbundled binary
  with a tray icon can start without activating.
