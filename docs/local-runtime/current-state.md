# Current state

## Current phase

**Phase 0 — Audit and design. Complete.** Awaiting go-ahead for Phase 1.

## Completed work

- Repository audited: framework, runtime, UI, persistence, process manager,
  command builder, metrics, logs, tests, release path.
- Data flow documented end to end in `design.md` §2.
- Risks catalogued in `design.md` §4 (18 items).
- Target design and phase boundaries written.

Pre-existing application (built before this plan): model catalog with GGUF
header parsing, launch profiles, RAM estimator with calibration hooks, supervised
`llama-server` runner with health gate and telemetry, tray, Settings.

## Files changed

Phase 0 added documentation only. No production code modified.

- `docs/local-runtime/design.md` (new)
- `docs/local-runtime/implementation-plan.md` (new)
- `docs/local-runtime/current-state.md` (new)
- `docs/local-runtime/decisions.md` (new)

## Commands run

```
find . -type f            # repository inventory
wc -l src/* src-tauri/**  # 4651 lines total
node -e ...package.json   # scripts: dev, build, preview, tauri (no lint)
cargo clippy --version    # 0.1.97 available, not wired into the project
cargo fmt --version       # 1.9.0 available, not wired
git status                # fatal: not a git repository
cat ~/Library/Application Support/llama-cpp-hub/config.json
```

## Verification results

- Test suite as of last full run: **40 passing, 1 ignored** (`real_launch`,
  which loads a real model). An earlier session message said "44 tests" — that
  was a miscount; 40 is correct.
- `bun run build` passes (tsc + vite).
- clippy and fmt have **never been run** against this project; unknown warning
  count. Phase 1 adopts them.
- Live config on this machine parses and contains: default profile
  (127.0.0.1:8888, ctx 65536, ngl all, np 1, flash-attn on, q8_0/q8_0, jinja),
  no overrides, **zero calibration samples**, two `lastRun` entries.

## Decisions taken (Phase 0)

- **D12** — git initialised, one commit per phase.
- **D13** — Downloads keeps its visible placeholder; still unbuilt, still
  unscheduled. Phase 7 nav work must not hide it.
- **D14** — orphaned `llama-server` processes are surfaced with Stop/Adopt,
  never auto-killed. Supersedes `runner::reap_orphan()`; implement in Phase 1.

## Known problems

1. **Downloads is unbuilt** and unscheduled (D13). Placeholder is intentional.
2. **`reap_orphan()` still auto-kills** until D14 is implemented.
3. **Config drops unknown keys and writes non-atomically.** Must be fixed at the
   start of Phase 1 or Phase 2 migration cannot satisfy "never lose unknown
   fields".
4. **`rawArgs` bypasses structured validation**, including host. Phase 6 must
   validate effective argv, not just form fields.
5. **No API key support** anywhere in the codebase.
6. **Calibration has zero samples**, so the memory estimator still uses the
   1.4 GB placeholder overhead. One observed run suggested ~120 MB.
7. **Per-process memory is misleading on Apple Silicon** — three sources
   disagreed by more than 10× on this machine. Phase 1 must present predicted,
   actual, pressure and swap as four distinct things.
8. **No frontend test runner.** Keep logic in Rust so rule 10 is satisfiable.
9. **MLA (`deepseek2`) KV estimate is over-counted** for GLM-4.7-Flash.
10. A `llama-server` may currently be running on port 8888 started through the
    app; `runner.pid` exists in the config directory.

## Exact next step

Begin **Phase 1**, in this order:

1. Make `store::save()` atomic (temp file + rename) and add
   `#[serde(flatten)] extra: Map<String, Value>` to `Config`; test unknown-key
   round-trip.
2. Add `src-tauri/src/sysmem.rs` with four `libc` readings — `hw.memsize`,
   `kern.memorystatus_vm_pressure_level`, `vm.swapusage`,
   `proc_pid_rusage`/`ri_phys_footprint` — each returning `Option`.
3. Add safety-state computation (OS pressure authoritative, headroom and swap
   heuristics secondary) with a truth-table test including
   one-metric-unavailable cases.
4. Replace `reap_orphan()` auto-kill with surface + Stop/Adopt (D14).
5. Extend `Telemetry` and `LaunchPlan`; render the memory panel with per-field
   "Unavailable" handling.
6. Adopt clippy and rustfmt; fix any pre-existing warnings (one-off adoption
   cost, in scope for Phase 1 only).
7. Run all four verification commands; record output here; commit.
