# Current state

## Current phase

**Phase 5 — Pi and Picot integration. Code complete, awaiting manual UI
confirmation.**

Phase 4 complete, then reworked after first real use exposed three defects (see
below).

Phase 1 complete and confirmed in the running app (process footprint read 1.3 GB
against Activity Monitor's 1.28 GB).

## Completed work — Phase 5

- **`agents.rs`** — endpoint details, permission-gated Pi inspection, configuration
  preview generation, application detection. **Never writes a Pi file.**
- **Inspection returns structure, never contents.** `models.json` holds API keys
  for cloud providers, so returning the file would leak unrelated secrets into
  event payloads and logs. The app reports provider names, base URLs, model ids
  and *whether* a key is set — with a test asserting a cloud key cannot appear in
  the serialised result.
- **The preview mirrors the installed Pi version** rather than a schema this app
  invented: `api` and extra keys such as `compat` are copied from the existing
  local provider when one is found, and omitted when it is not.
- **The generated key is always a placeholder.** Echoing a real key into text the
  user will copy and possibly paste elsewhere is the leak `Redacted` exists to
  prevent.
- **Port mismatch is surfaced.** The user's Pi points at 8888; the app frequently
  ends up on 8889 or 8890 after a port fallback, which silently breaks Pi. The
  Connect screen now says so explicitly.
- **Applications are detected, not hard-coded** — Picot, VS Code, Cursor, iTerm
  in /Applications and ~/Applications, with the Pi session folder if present.
- Fixed a **flaky test**: four tests shared one scratch directory keyed by process
  id and ran in parallel, so one could delete another's fixture mid-read.
  Directories are now unique per call; verified over three consecutive runs.

Known shape of the user's Pi setup, from the granted inspection: provider
`local-llama`, `api: openai-completions`, `baseUrl: http://127.0.0.1:8888/v1`,
`compat` block present, `contextWindow: 64512` (1024 below the served 65536).

## Phase 4 rework — what the first real benchmark exposed

Running it against the real Q3 and Q4 produced a screen the user correctly called
unclear, and diagnosis found three separate faults:

1. **The probe measured a workload nobody runs.** 16 generated tokens at a
   context depth of 17 tokens, with 13 of 17 prompt tokens served from cache
   (`prompt_n: 4`). It reported 33.5 tok/s where real use at ~20k context gives
   12–15. The number was internally consistent and completely unrepresentative.
   Benchmarks now prefill to a working depth (default 8K) with `cache_prompt`
   disabled and a per-run nonce so the prefill cannot be cache-served, then
   generate 256 tokens. Depth is recorded on the row; comparisons refuse to
   treat different depths, or a legacy shallow probe, as comparable.
2. **A healthy reasoning model was reported as FAILED.** The probe read only
   `delta.content`, but the model emits `reasoning_content` first and spent its
   whole 16-token budget thinking. Both the streaming reader and the completion
   check now recognise reasoning fields; a reasoning-only answer warns rather
   than fails, and the budget rose to 96 tokens.
3. **The comparison had unlabelled columns** and headlined two misleading memory
   figures. Columns are now labelled A and B with model, quantisation, context
   and depth; peak process footprint is labelled as excluding GPU-resident
   weights; and the swap delta is gone, because machine-wide swap is not
   attributable to one model.

The model test and the benchmark are now separate actions: the test stays
instant and shallow and records nothing, the benchmark takes about a minute and
is the only thing that writes a row.

## Completed work — Phase 4

- **`benchmarks.rs` with `benchmarks.json` beside the config**, never inside it
  (D11): history grows without bound and every profile edit would otherwise
  rewrite it, risking settings for the sake of a log. Atomic write shared with
  the config store via `store::write_atomic`.
- **A row records the settings, not just the result** — model file and size,
  architecture, quantisation, context, both cache types, ngl, parallel slots and
  the llama.cpp build — because the feature exists to compare quantisations
  like-for-like and a number without its configuration proves nothing.
- **Recorded automatically when a model test completes**, using the timings
  `health.rs` already produces plus peak process footprint and peak swap now
  tracked across the run by the telemetry loop.
- **Query is a tested Rust function**, not frontend filtering: filter by model
  and quantisation, sort by date, generation, prompt eval, first token or peak
  memory. Rows missing the sort key sink to the bottom in *both* directions — a
  missing measurement is neither fast nor slow.
- **Export** to CSV or JSON, written into the app support directory and the path
  returned, avoiding a file-dialog dependency. CSV escapes separators and quotes;
  missing measurements are blank cells, never zeroes.
- **History capped at 500 rows**, oldest first.
- **UI** — Benchmarks screen with filters, sort, per-row note and delete, and a
  two-run comparison showing percentage deltas with direction awareness (higher
  generation is better, lower first-token latency is better). The comparison
  warns when the two runs differ in more than quantisation, so a difference is
  not silently attributed to the wrong cause.

## Completed work — Phase 3

- **`redact.rs`** — `Redacted` newtype with no `Display`, a placeholder `Debug`
  and `Serialize`, and the value reachable only through `expose()`. Formatting a
  struct containing a secret therefore cannot print it, and every deliberate use
  is greppable. Plus argv redaction (`--api-key VALUE` and `--api-key=VALUE`)
  and header redaction, both ready for phases 6 and 8.
- **`health.rs`** — eight ordered, individually timed checks: process alive, port
  reachable, `/health`, `/v1/models`, alias advertised, chat completion,
  streaming, reasoning. Verdict is Passed / Passed with warnings / Failed.
- **Failure handling is graded, not binary.** An unadvertised alias warns (the
  request still works); a failed stream warns; an unreachable port stops the run
  rather than reporting later checks it never performed.
- **Reasoning is detected, not assumed** — `reasoning_content`, `reasoning`,
  `thinking`, or inline `<think>` tags, in that order.
- **Timings prefer the server's own figures** (`timings.prompt_per_second`,
  `predicted_per_second`) and fall back to wall-clock only when absent.
- **The probe is deterministic and small**: fixed prompt, `temperature: 0`,
  `max_tokens: 16`, with a compile-time assertion that it stays ≤ 32.
- **UI** — "Test model" button in the Running panel; per-check status, detail
  and duration; results cleared when the run changes so a stale report cannot be
  read as current.

## Completed work — Phase 2

- **Schema v2 with migration.** `schemaVersion` added; absence means v1. The
  migration is a version stamp only — templates come from code, so nothing
  needed rewriting. Idempotent, and unknown keys still survive.
- **`Profile` gained `#[serde(default)]`.** Found by a migration test: a
  `defaultProfile` missing any single field failed to deserialise, and
  `unwrap_or_default()` then discarded the *entire* config — overrides,
  calibration, history. Any future field addition would have done this to a
  downgrade.
- **Named profiles.** `profiles.rs` with four built-in workload templates
  (Quality Coding 32K, Balanced 64K, Long Context 128K with q4_0 V cache,
  Lightweight 8K). Built-ins live in code with stable ids; a stored entry
  shadows one, which is what makes them editable, and reset drops the stored
  entry. User profiles are never touched by a reset, and built-ins cannot be
  deleted.
- **Templates are sparse patches**, never full profiles — they set context,
  cache types and slots, and deliberately never alias, host or port.
- **Name collisions are suffixed, not rejected** (" copy", " copy 2"), so saving
  cannot fail on a name the user cannot see.
- **Calibration recording bug fixed.** `stop()` took the child out from under the
  waiter thread, so the waiter returned before recording and *only an unexpected
  exit* ever banked a sample. Normal use is always an explicit stop, so the
  residency model from Phase 1 could never have calibrated. Samples are now
  captured on both paths, and zero/implausible observations are filtered when
  fitting rather than when capturing.

## Completed work — Phase 1

**Prerequisite fixes** (required before Phase 2 migration is possible)

- `store::save()` writes to a temp file and renames — an interrupted write can no
  longer truncate config.
- `Config` gained `#[serde(flatten)] extra`, so keys written by a newer build
  survive a load/save round-trip instead of being silently dropped.
- `store` split into `load_from`/`save_to` so persistence is testable without
  touching the real config directory.

**Native memory readings** — new `src-tauri/src/sysmem.rs`, the only `unsafe` in
the project:

| Value | Source |
| --- | --- |
| Installed unified memory | `sysctl hw.memsize` |
| macOS pressure | `sysctl kern.memorystatus_vm_pressure_level` (1/2/4) |
| Swap in use | `sysctl vm.swapusage` → `xsw_usage` |
| Process footprint | `proc_pid_rusage` → `ri_phys_footprint` (Activity Monitor's column) |

A size mismatch on any sysctl returns `None` rather than trusting the bytes.

**Safety state** — new `src-tauri/src/safety.rs`, pure functions. Kernel pressure
is authoritative; headroom and swap are heuristics; the worst signal wins.
Thresholds: headroom < 2 GB red, < 4 GB yellow; swap ≥ 6 GB red, ≥ 2 GB yellow.
Memory attributable to a running model is subtracted before projecting a
replacement launch, so swapping models is not double-counted.

**D14 implemented** — `reap_orphan()` (auto-kill) replaced by `detect_orphan()`
(report only), `stop_orphan()` (re-verifies the process first) and
`dismiss_orphan()`. The app no longer kills anything the user did not ask it to.

**Surfaced in the UI** — pre-launch panel shows predicted breakdown, safety
badge, reasons, installed / in use / swap / pressure / projected headroom, and
states plainly that prediction and actual will differ. Running panel adds
pressure, swap, headroom, process footprint (labelled "excludes GPU-resident
weights"). Missing readings render "Unavailable", never zero.

**Tooling adopted** — clippy (`-D warnings`) and rustfmt now clean across the
project; four pre-existing findings fixed.

## Files changed

New: `src-tauri/src/sysmem.rs`, `src-tauri/src/safety.rs`, `src/Memory.tsx`,
`docs/local-runtime/*`.

Modified: `store.rs`, `runner.rs`, `lib.rs`, `estimate.rs`, `gguf.rs`,
`catalog.rs`, `probe.rs`, `Cargo.toml` (+libc), `types.ts`, `api.ts`,
`App.tsx`, `ModelDetail.tsx`, `App.css`, `tests/runner_lifecycle.rs`.

## Commands run

```
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check     # clean
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings   # clean
cargo test --manifest-path src-tauri/Cargo.toml               # 135 passed, 1 ignored
bun run build                                                 # tsc + vite, clean
```

## Verification results

- **135 tests pass**, 1 ignored (`real_launch`, loads a real model). Up from 40.
  New: 5 store persistence, 7 sysmem, 11 safety.
- clippy clean at `-D warnings` for the first time; 4 findings fixed (2
  pre-existing OR-patterns, 1 identity op, 1 of mine).
- rustfmt applied across the codebase; check is clean.
- One test failure found and fixed during the run: a safety truth-table case
  asserted Red at exactly 2 GB headroom, but the rule is `< 2 GB`. The test was
  wrong, not the rule; it now also asserts the boundary explicitly.
- App rebuilt under `tauri dev` and relaunched without error.

## Known problems

1. **Calibration has zero samples so far**, but recording now works on the
   normal stop path. Three start-to-stop cycles will fit a residency.
2. **Residency is fitted machine-wide, not per model.** If Q3 and Q4 turn out to
   have materially different ratios, one constant will be wrong for both (D15).
3. **Headroom thresholds are unvalidated against real use.** 2/4 GB were chosen
   for a 32 GB machine also running an editor, browser and coding agent. They
   will fire often on this hardware; whether that is signal or noise needs a few
   days of use.
4. **`profileName` on benchmark rows is always null.** Templates are applied to
   the launch form rather than bound to a run, so the app does not know which
   named profile produced it. Fixing it means recording the last applied
   template per model at apply time.
5. **Adopt is not implemented** for orphans — only Stop and Leave running. D14
   mentioned Adopt; it needs a runner path for a process we did not spawn (no
   stdout to attach), deferred rather than half-built.
5. Downloads still a placeholder (D13, intentional).
6. MLA (`deepseek2`) KV estimate still over-counts.
7. `rawArgs` still bypasses structured validation — Phase 6.
8. No API key support — Phase 3/6.

## Exact next step

1. Confirm in the app: run "Test model" on the Q3 and then the Q4 variant, open
   Benchmarks, select both and check the comparison reads sensibly.
2. Then **Phase 6 — security guardrails**: validate the *effective* argv rather
   than the form fields, since `rawArgs` can still reintroduce `--host 0.0.0.0`
   past every structured guard; add API key storage, which the Connect screen
   currently has to describe as "none"; port validation and conflict detection.

Superseded — Phase 5 was **Pi and Picot integration**: read-only, permission-gated
   inspection of `~/.pi/agent/settings.json`, preview and copy only, never a
   write, with configurable application paths. Note the app still has no API key
   storage, so the "connect" surface can only describe an unauthenticated
   localhost endpoint until Phase 6.

Superseded plan for reference — Phase 4 was **benchmark history**: `benchmarks.json` beside config (never
   inside it), append-only with atomic rewrite, recording the fields listed in
   the brief plus llama.cpp version from `Capabilities` and peak swap from the
   Phase 1 sampling. Table, sort, filter, compare two runs, delete, export
   JSON/CSV. No charts. The goal is an objective Q3 vs Q4 comparison; note that
   `health.rs` already produces most of the per-run numbers a benchmark row
   needs.
