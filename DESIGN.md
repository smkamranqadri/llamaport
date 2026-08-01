# llama-cpp-hub — Design

A macOS desktop app for running and downloading local GGUF models. Replaces a
hand-written `llama-server` command line and an external download manager with a
single tool.

## Problem

Current workflow is two shell commands held in memory.

Running a model:

```
llama-server \
  -m "$HOME/models/Qwen3.6-35B-A3B-UD-Q3_K_XL.gguf" \
  --alias qwen3.6-35b-a3b \
  --host 127.0.0.1 --port 8888 \
  --jinja -c 65536 -ngl all -np 1 \
  --flash-attn on --cache-type-k q8_0 --cache-type-v q8_0
```

Everything except `-m`, `--alias` and `-c` is stable across models, but the whole
line gets retyped or shell-history-hunted each time. Nothing records which
context length a given model actually supports, or whether the chosen `-c` will
fit in RAM.

Downloading a model:

```
curl -L --limit-rate 10M -C - \
  -o ~/models/gemma-4-26B-A4B-it-UD-Q4_K_M.gguf \
  "https://huggingface.co/unsloth/gemma-4-26B-A4B-it-GGUF/resolve/main/gemma-4-26B-A4B-it-UD-Q4_K_M.gguf"
```

This fails to resume reliably, and is slow. Both have identifiable causes, see
[Downloader](#downloader). Files are 13–21 GB, so a failed resume is expensive.
The current fallback is Neat Download Manager, which leaves downloads outside any
model-aware tool.

## Goals

- List local models with real metadata, not filename guesses.
- Launch `llama-server` from a saved per-model profile, with the exact command
  line always visible.
- Predict memory footprint before launch.
- Download GGUF files with multi-connection transfer and resume that survives
  app restarts.
- Browse Hugging Face for GGUF repos and pick a quant without leaving the app.

## Non-goals

- Chat UI. `llama-server` already ships one at `/`; the app links to it.
- Cross-platform support. macOS/Apple Silicon only.
- Managing llama.cpp itself (building, updating). The app consumes whatever
  `llama-server` is on PATH.
- Ollama/Modelfile integration.

## Decisions

| Area | Choice | Rationale |
| --- | --- | --- |
| Shell | Tauri v2 + React/TS | ~10 MB binary, no bundled Chromium. Rust suits the two hard parts: child-process supervision and a segmented downloader. Requires a one-time `rustup` install. |
| Downloader | Built in-app | The Hugging Face signed-URL resume problem needs handling regardless; `aria2c` would not remove that work, only add a bundled binary. |
| Concurrency | One model at a time | 13–21 GB of weights per model. Starting a model stops the current one. |
| Lifecycle | Menu bar resident, single instance | Server and downloads survive closing the window. A second instance would race on the port and on partial files. |
| v1 scope | Run + download + HF browse | Full replacement of the current workflow. |
| Model identity | `(file_size, header hash)` | Free during the scan; profiles survive renames and directory moves. |
| Crash policy | Split by phase | A crash before `Ready` is a config error; after `Ready` it is likely transient. |
| Navigation | Sidebar + drill-down | The model detail page needs full width for form, estimator, preview, and logs. |
| Telemetry | `/metrics` polling + RSS sampling | KV usage is the direct feedback on whether `-c` was right; RSS makes the RAM estimator self-correcting. |

## Architecture

Four Rust modules behind Tauri commands, three React screens, one tray item.
The modules do not depend on each other; the frontend composes them.

```
src-tauri/
  catalog/     scan dir, parse GGUF headers, watch for changes (watcher: step 3)
  runner/      supervise one llama-server child
  downloader/  queue + segmented transfer engine
  hf/          Hugging Face API client (holds the token)
  store/       JSON config persistence
src/
  Library/     local models, launch
  Discover/    HF search, quant picker
  Downloads/   queue, progress, pause/resume
```

Config lives at `~/Library/Application Support/llama-cpp-hub/config.json`.
Models directory defaults to `~/models`, configurable.

### IPC surface

Commands (frontend → Rust):

- `catalog_list() -> Model[]` — performs the scan; Rescan in the UI is the same
  call, so there is no separate `catalog_rescan`
- `catalog_dir_info() -> DirInfo`
- `set_models_dir(path) -> DirInfo`
- `runner_start(model_id, profile) -> ()`
- `runner_stop()`
- `runner_status() -> RunnerState`
- `download_enqueue(spec) -> download_id`
- `download_pause(id)` / `download_resume(id)` / `download_cancel(id)`
- `download_list() -> Download[]`
- `hf_search(query, page) -> Repo[]`
- `hf_files(repo_id) -> RepoFile[]`

Events (Rust → frontend):

- `runner:state` — state machine transitions
- `runner:log` — stdout/stderr lines
- `download:progress` — throttled to ~4 Hz per active download
- `catalog:changed` — filesystem watcher fired; arrives with the downloader,
  since until then nothing appears in the models directory unattended

## Catalog

### Model identity

Launch profiles, download history, and the "already have this quant" check in
Discover all key off model identity, so it has to survive ordinary file
management.

Identity is `(file_size, sha256 of the first 4 KB)`. The header hash is free —
that region is already read to parse metadata — and the pair does not collide in
practice across a personal library. Full-file sha256 is correct but would mean
hashing 21 GB on first sight of any model not downloaded through the app.

Path is a mutable attribute, not the key. Renaming a file or moving the models
directory carries its profile along, and a re-downloaded identical file readopts
the profile it had before.

### GGUF parsing

Read the header directly rather than shelling out. Layout:

```
magic     "GGUF"  (4 bytes)
version   u32
n_tensors u64
n_kv      u64
kv pairs  [ key: string, type: u32, value: typed ]
```

Strings are `u64` length + UTF-8 bytes. Values are typed scalars or arrays.
A hand-rolled reader is roughly 150 lines and avoids a dependency on a crate
that may lag GGUF version bumps.

The KV block has to be walked in full rather than read as a fixed-size prefix:
`tokenizer.ggml.tokens` runs to hundreds of thousands of length-prefixed
strings, and keys that matter — `tokenizer.chat_template` among them — sit after
it. Array contents are skipped, not parsed, except for integer arrays short
enough to be per-layer values. Skips under 64 KB read through the buffer
instead of seeking, since seeking discards it.

Keys consumed:

| Key | Use |
| --- | --- |
| `general.architecture` | Arch badge; prefix for arch-scoped keys below |
| `general.name`, `general.size_label` | Display |
| `{arch}.context_length` | Cap the context slider at what the model supports |
| `{arch}.block_count` | Layer count, for the RAM estimate |
| `{arch}.embedding_length` | Head dim, for the RAM estimate |
| `{arch}.attention.head_count` | Head dim, for the RAM estimate |
| `{arch}.attention.head_count_kv` | KV cache size (GQA/MQA aware) |
| `{arch}.attention.key_length` | Head dim override when present |
| `tokenizer.chat_template` | Presence decides whether `--jinja` is meaningful |

Quantization comes from the filename (`Q4_K_M`, `UD-Q3_K_XL`) with the GGUF
file-type field as a cross-check.

### Sharded models

Files matching `*-00001-of-000NN.gguf` collapse into one catalog entry. Parse
only shard 1; sum sizes across shards; pass shard 1 to `-m` and let llama.cpp
find the rest. Flag incomplete shard sets in the UI rather than offering to run
them.

### RAM estimator

The most useful number the UI can show, given file sizes here.

```
k_dim      = key_length ?? (embedding_length / head_count)
v_dim      = value_length ?? k_dim
kv_elems   = block_count * n_ctx * head_count_kv * (k_dim + v_dim)
kv_bytes   = kv_elems * bytes_per_element(cache_type)
total      = file_size + kv_bytes + overhead
```

K and V are summed separately rather than doubling one dimension, because
latent-attention architectures size them differently. `GLM-4.7-Flash` in the
local library reports `deepseek2` with one KV head and a 576-wide key — the
symmetric form would be wrong for it. Its MLA cache is compressed rather than
per-head, so treat that family's estimate as provisional until RSS calibration
has samples for it.

`bytes_per_element`: `f16` = 2, `q8_0` = 1.0625 (34-byte blocks of 32), `q4_0` =
0.5625. Overhead is a flat allowance for compute buffers; calibrate against
observed RSS once the runner exists rather than guessing in the spec.

Mixture-of-experts models need no special case: llama.cpp keeps all experts
resident, so `file_size` already accounts for them, and `head_count_kv` covers
the cache correctly. MoE affects the compute buffer, not the total.

Display predicted total against installed RAM, with a warning band as the
estimate approaches physical memory. This is what prevents picking a context
length that silently swaps.

## Runner

One child process at a time.

```
Idle → Starting → Ready → Stopping → Idle
                     ↓
                  Crashed
```

`Starting` spawns `llama-server` with the profile's flags and polls
`GET http://127.0.0.1:{port}/health` until it returns ok — process-alive is not
the same as model-loaded, and loading a 21 GB file takes real time. stdout and
stderr stream into a ring buffer (last ~2000 lines) surfaced in a log pane.

A pidfile in the app support directory lets a relaunched app detect and adopt or
kill a process orphaned by a crash.

### Launch profiles

Per-model, persisted, seeded from a global default:

```jsonc
{
  "alias": "qwen3.6-35b-a3b",
  "host": "127.0.0.1",
  "port": 8888,
  "ctx": 65536,
  "ngl": "all",
  "parallel": 1,
  "flash_attn": true,
  "cache_type_k": "q8_0",
  "cache_type_v": "q8_0",
  "jinja": true,
  "raw_args": []
}
```

The rendered command line is always visible and copyable. `raw_args` is the
escape hatch for anything the UI does not model.

Resolution is two-layer: a global default profile, overridden per model by only
the fields actually changed. A model's stored profile is a sparse patch, so
raising the default `parallel` propagates to every model that never overrode it.
The UI marks overridden fields and offers a per-field reset.

`ctx` is clamped to `{arch}.context_length` from the header. `port` defaults to
8888 and falls forward to the next free port if occupied, reporting the
substitution rather than failing to launch.

### Failure handling

An unexpected exit is classified by the phase it happened in.

Before `Ready`, the cause is nearly always the launch config — too much context
for available memory, a flag this build does not support, a corrupt or
incomplete shard set. Never auto-restart; the same config will fail the same
way. Stop, and surface the last 20 stderr lines inline with the error, because
the reason is always there and making the user go find it is the difference
between a five-second fix and a confusing one.

After `Ready`, a crash is more likely transient. Auto-restart once with the same
profile; if that also fails, stop and treat it as the first case.

### Flag probing

llama.cpp's CLI drifts between releases (`--flash-attn` recently changed from a
bare flag to taking a value). At startup, run `llama-server --help`, parse the
supported flag list, and disable controls the installed build does not support.
Cache the result keyed by the binary's mtime and size.

The binary path is discovered on PATH (`/opt/homebrew/bin/llama-server` here)
and overridable in settings, since llama.cpp builds are frequently local.

### Telemetry

Three endpoints, gated on what the flag prober found:

| Endpoint | When | Use |
| --- | --- | --- |
| `/health` | polled during `Starting` | Drives the transition to `Ready`. |
| `/props` | once on `Ready` | Static facts. Confirms the server got the context actually requested. |
| `/metrics` | 1 Hz while `Ready` | Everything below. Requires `--metrics`. |

`--metrics` costs nothing, so the runner appends it automatically when the build
supports it rather than exposing a checkbox. `--slots` needs no handling: build
10090 has it enabled by default (`--no-slots` disables), and `/slots` on an idle
server reports only `n_ctx` and `is_processing`.

**The KV metric does not exist on current builds.** Build 10090 exposes no
`kv_cache_*` series at all — the full set is `prompt_tokens_total`,
`prompt_seconds_total`, `tokens_predicted_total`,
`tokens_predicted_seconds_total`, `n_decode_total`, `n_tokens_max`,
`prompt_tokens_seconds`, `predicted_tokens_seconds`, `requests_processing`,
`requests_deferred`, `n_busy_slots_per_decode`. Occupancy is therefore *derived*
as `n_tokens_max / n_ctx`, falling back to `kv_cache_usage_ratio` when an older
build provides it. This is the design's headline number, so it is worth knowing
it is a derived proxy rather than a reading.

Metric counters are cumulative, so rates are deltas between polls, not readings.
Two consequences: the first poll after `Ready` yields no rate, and a counter
that decreases means the process restarted and the baseline must reset.

Surfaced in the running-model view:

- **KV cache usage** — how full the allocated context is, derived as above. The
  most useful live number, because it is the direct feedback on whether the
  chosen `-c` was right.
- **Throughput** — prompt-eval and generation tokens/sec derived from the token
  and seconds counters, with a 60-second sparkline. A live delta alone is not
  enough: it reads zero the instant a request finishes, which looks broken. Fall
  back to the server's own `predicted_tokens_seconds` / `prompt_tokens_seconds`,
  which persist between requests, and label them as last-request figures.
- **Totals** — `tokens_predicted_total` and `prompt_tokens_total`, since
  cumulative work done is what the rate figures cannot tell you.
- **Queue depth** (`requests_processing`, `requests_deferred`). With `-np 1`,
  anything deferred is waiting.

### Memory calibration

**No per-process memory figure describes this workload.** Measured on a 32 GB
machine running Qwen3.6 at 65536 context:

| Source | Reported | Why |
| --- | --- | --- |
| `sysinfo` process memory (RSS) | 16.2 GB | Counts mmapped GGUF pages that are resident |
| Activity Monitor process column | 1.28 GB | Physical footprint, which excludes clean file-backed pages |
| System wired memory | 20.0 GB | Where `-ngl all` actually puts weights and KV cache, via Metal |

The two per-process numbers disagree by more than 10×, and neither one is the
cost the user cares about. On Apple Silicon the Metal buffers are attributed to
the kernel, not the process, so the only honest observable is machine-wide.

Calibration therefore samples system-wide used memory immediately before spawn,
tracks the peak while running, and records the delta as `observed_total`. The
residual against predicted weights+KV fits the overhead constant. The running
view shows system used/total plus swap for the same reason — a per-process
figure there would contradict Activity Monitor and destroy trust in the panel.

Negative residuals are dropped rather than clamped: a machine already under
memory pressure can evict as fast as the model loads.

## Downloader

### Why the current command fails

`https://huggingface.co/{repo}/resolve/main/{file}` returns a 302 to a CDN URL
carrying an expiring signature. `curl -C -` re-requests the *original* URL on
resume and re-enters the redirect chain, which is where resume breaks down. The
fix is to re-resolve the redirect on every resume attempt and issue `Range`
requests against a freshly signed URL.

Speed is a separate issue: a single connection is the bottleneck, not the line.
Multiple ranged connections are why an external download manager is faster,
and why `--limit-rate 10M` felt necessary in the first place.

### Engine

Per file:

1. Follow redirects manually to capture the final CDN URL and the headers
   `x-linked-size` (true size) and `x-linked-etag` (sha256 for LFS files).
   Confirm `accept-ranges: bytes`.
2. Preallocate a sparse `{name}.part`, split into 4–8 segments.
3. Each segment runs a ranged GET, writing at its own offset with positioned
   writes. No seeking coordination between tasks.
4. A sidecar `{name}.part.json` holds
   `{ source_url, total, etag, segments: [{ start, end, completed }] }`,
   flushed every ~2 s. This is what makes resume survive a process exit rather
   than only a pause.
5. On completion, optionally verify sha256 against the etag (a background pass;
   21 GB takes a minute or two), then rename `.part` to the final name.

Resume re-resolves the redirect, compares the etag against the sidecar, and
restarts each segment from its `completed` offset. An etag mismatch means the
upstream file changed: discard and restart.

### States

```
Queued → Active → Verifying → Complete
           ↕
        Paused
           ↓
        Failed → (manual resume) → Active
```

`Failed` is always resumable from the sidecar; it is a parked state, not a lost
transfer.

### Failure taxonomy

Retries have to distinguish three cases. Treating them uniformly means either
abandoning recoverable transfers or hammering a wall.

| Class | Examples | Response |
| --- | --- | --- |
| Transient | connection reset, 5xx, timeout | Retry the segment, exponential backoff, 5 attempts, then park the download in `Failed`. |
| Signature expiry | 403 on a CDN URL that was working | Not a failure. Re-resolve the redirect and continue. Routine on multi-hour transfers and should be invisible. |
| Fatal | 404, etag changed, no `accept-ranges` | Stop, explain, do not retry. |

Stall detection is separate and necessary: a segment reporting zero bytes for
30 s while sibling segments progress has hung without raising anything. Kill and
reissue that range. Without it a transfer can sit at 97% indefinitely.

### Gotchas to build in from the start

- **Strip `Authorization` on the cross-host redirect.** Forwarding the HF token
  to the CDN produces confusing 403s on gated repos.
- **Share one token bucket across all segments.** Otherwise a 10 MB/s rate limit
  becomes 10 MB/s × segment count.
- **One file at a time by default.** Parallel large files compete for the same
  pipe and multiply the failure surface.
- **Check free disk before enqueueing.** These are 13–21 GB files.

## Discover (Hugging Face)

- Search: `GET /api/models?search={q}&filter=gguf&sort=downloads`
- Files: `GET /api/models/{id}/tree/main?recursive=true` — yields paths, sizes,
  and LFS oids.

Group a repo's files by quant, show size against free disk, and mark quants
already present locally. Handle shard sets as a single selectable item that
enqueues every part. An optional HF token in settings, held Rust-side, enables
gated repos.

## Screens

Navigation is a sidebar (Library, Discover, Downloads, Settings) with drill-down
into a full-width detail page, rather than a three-column split. The model
detail page carries a form, an estimator panel, a command preview, and a log
pane; a third column would be cramped at any reasonable window size.

The sidebar footer holds a persistent now-running bar — alias, port, KV
percentage, Stop — so the server's state is visible from every screen without
navigating.

### Library

Header: models directory, free disk, rescan, filter field. Rows carry display
name (`general.name`, falling back to filename), quant chip, MoE marker, size,
max context, chat-template indicator, and last-run time. Default sort is last
used, descending.

Architecture is deliberately *not* in the row. It is the llama.cpp architecture
id rather than a property of the model as released — `Qwen3.6-35B-A3B` reports
`qwen35moe`, and `GLM-4.7-Flash` reports `deepseek2` — so next to the model name
it reads as a contradictory version claim. It belongs on the detail page, where
there is room for it to be labelled, and where it is actually decision-relevant.

Row states beyond normal:

- **Running** — pinned to the top, showing live telemetry.
- **Incomplete shard set** — disabled, with the missing parts named.
- **Unparseable** — greyed but *visible*, with the parse error. A truncated GGUF
  is usually a failed download, which is exactly what the user needs to see.

Empty state: models directory missing or containing no GGUF files offers a
directory picker and a link to Discover.

### Model detail

1. Header facts — filename, path with Reveal in Finder, size, arch, quant,
   parameter count, chat template, max context.
2. Launch form — alias, port, context slider (capped at header max), ngl,
   parallel, flash-attn, cache type K/V, jinja, raw args. Fields overriding the
   global default are marked and individually resettable.
3. RAM estimate — updates live as the context slider moves; shows the
   weights / KV cache / overhead breakdown against installed RAM.
4. Command preview — monospace, copyable.
5. Primary action — Run, or Stop and Reload when this model is the running one.
6. Logs — collapsed while idle, auto-expanded on `Starting`, pinned to the last
   20 lines on a crash.

### Discover

Defaults to trending GGUF repos so the screen is never blank without a query.
Results show repo id, author, downloads, likes, and last updated. Expanding a
repo lists quant variants with filename, quant chip, size, and per-quant state:
Download, In library, or Downloading. Shard sets are one row with a part count
and enqueue every part together. Quants exceeding free disk are marked. Gated
repos show a lock and link to the token field in Settings.

### Downloads

Sections for active, queued, completed, and failed. Active rows show speed, ETA,
elapsed, and a progress bar subdivided by segment, expandable into per-segment
detail — primarily a debugging affordance, but during the downloader build it is
what distinguishes "stuck" from "segment 4 stalled at offset X". Queued rows are
reorderable. Failed rows show the error class and a Resume. Completed rows show
the checksum result and link into Library.

Global controls: rate limit, segment count, pause all.

### Settings

Models directory. llama-server path with detected version and the probed flag
list. Global default launch profile, using the same form as model detail.
Download defaults — segments, rate limit, verify checksum. HF token, stored in
the macOS keychain rather than `config.json`. Launch at login.

### Tray

Running model with alias, port and KV percentage, or a recent-models submenu
when idle. Aggregate download progress. Stop, Show Window, Quit.

## Application lifecycle

Single instance, enforced by Tauri's single-instance plugin; a second launch
focuses the existing window. This is not cosmetic — two instances race for port
8888, and two downloaders writing into the same `.part` at different offsets
corrupt it silently, surfacing hours later as a checksum failure.

Closing the window hides it; the app stays in the menu bar with the server and
any transfers running. Quit is explicit: stop the server (the child would
otherwise be orphaned), pause active downloads, flush every sidecar. A quit
while downloads are active warns first.

## Build order

1. **Catalog** — scan, GGUF parser with tests against the six local models,
   Library list. Pure and testable, no UI risk. *Done.*
2. **Runner** — spawn, health poll, logs, profiles, tray. At this point the app
   already replaces the shell command.
3. **Downloader** — headless engine first, driven by tests including a
   kill-mid-transfer resume case. Then the Downloads screen.
4. **Discover** — HF client and quant picker on top of a working downloader.

## Open questions

- Whether to verify sha256 by default, given the time cost on 21 GB files.
- Conservative default for the overhead constant, used until RSS calibration has
  enough samples.
- Whether Discover should surface non-GGUF repo files at all, or filter them out
  entirely.
- `Modelfile-qwen36-unsloth` sits in the models directory. Ignore non-GGUF files
  in v1; revisit if Ollama interop ever matters.
