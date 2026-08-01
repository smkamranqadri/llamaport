# Current state

## Current phase

**Phase 1 — Real system memory and safety status. Code complete, awaiting manual
UI confirmation.**

## Completed work

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
cargo test --manifest-path src-tauri/Cargo.toml               # 63 passed, 1 ignored
bun run build                                                 # tsc + vite, clean
```

## Verification results

- **63 tests pass**, 1 ignored (`real_launch`, loads a real model). Up from 40.
  New: 5 store persistence, 7 sysmem, 11 safety.
- clippy clean at `-D warnings` for the first time; 4 findings fixed (2
  pre-existing OR-patterns, 1 identity op, 1 of mine).
- rustfmt applied across the codebase; check is clean.
- One test failure found and fixed during the run: a safety truth-table case
  asserted Red at exactly 2 GB headroom, but the rule is `< 2 GB`. The test was
  wrong, not the rule; it now also asserts the boundary explicitly.
- App rebuilt under `tauri dev` and relaunched without error.

## Known problems

1. **Manual UI confirmation outstanding.** The memory panel has not been seen
   rendered. computer-use cannot drive an unbundled dev binary, so a human must
   open a model and press Run.
2. **Calibration still has zero samples**, so overhead remains the 1.4 GB
   placeholder and every estimate reads "(uncalibrated)".
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

1. Confirm the panel renders: open a model, check the pre-launch numbers, press
   Run, check the running numbers, stop, confirm they reset.
2. Then **Phase 2 — workload presets and improved profiles**: schema v2 with
   migration from the current shape (absence of `schemaVersion` means v1), named
   profiles with `builtIn`, four workload templates, duplicate/rename/reset.
   `store.rs` is now ready for this: atomic writes and unknown-key preservation
   are in place and tested.
