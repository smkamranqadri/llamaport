# Current state

## Current phase

**Phase 2 — Workload presets and improved profiles. Code complete, awaiting
manual UI confirmation.**

Phase 1 complete and confirmed in the running app (process footprint read 1.3 GB
against Activity Monitor's 1.28 GB).

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
cargo test --manifest-path src-tauri/Cargo.toml               # 80 passed, 1 ignored
bun run build                                                 # tsc + vite, clean
```

## Verification results

- **80 tests pass**, 1 ignored (`real_launch`, loads a real model). Up from 40.
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
4. **Adopt is not implemented** for orphans — only Stop and Leave running. D14
   mentioned Adopt; it needs a runner path for a process we did not spawn (no
   stdout to attach), deferred rather than half-built.
5. Downloads still a placeholder (D13, intentional).
6. MLA (`deepseek2`) KV estimate still over-counts.
7. `rawArgs` still bypasses structured validation — Phase 6.
8. No API key support — Phase 3/6.

## Exact next step

1. Confirm in the app: Settings shows four built-in templates with badges and
   Duplicate/Reset; the model detail page shows a template row that changes
   context and cache types but leaves alias and port alone; "Save as profile"
   creates a user profile that then appears in both places.
2. Then **Phase 3 — server health and model test**: `health.rs` with ordered
   timed checks driven through the existing `EventSink`, a `Redacted` newtype so
   secrets cannot reach logs or diagnostics, and reasoning detection by probing
   the response shape rather than assuming a field name.
