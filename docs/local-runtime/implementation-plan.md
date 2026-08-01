# Implementation plan

One phase per session where practical. Every phase ends with `current-state.md`
updated and the verification commands below run with output recorded.

## Verification commands

```bash
bun run build                                    # tsc typecheck + vite build
cargo test --manifest-path src-tauri/Cargo.toml  # 40 tests, 1 ignored
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path src-tauri/Cargo.toml --check
```

clippy and fmt are installed but were never wired into this project. Phase 1
adopts them; if the existing code produces warnings, fixing them is in scope for
Phase 1 only (it is a one-off adoption cost, not an unrelated refactor).

Manual verification requires clicking Run in the app: computer-use cannot drive
an unbundled dev binary, so a human confirms UI behaviour and pastes or
screenshots the result.

---

## Phase 0 — Audit and design ✅

Deliverables: the four documents in `docs/local-runtime/`. No production code.

---

## Phase 1 — Real system memory and safety status

**Prerequisite fixes** (small, required for correctness of everything after):

- `store::save()` becomes atomic (temp file + rename).
- `Config` gains `#[serde(flatten)] extra` so unknown keys survive.

**New:** `src-tauri/src/sysmem.rs` wrapping `libc` — `hw.memsize`,
`kern.memorystatus_vm_pressure_level`, `vm.swapusage`, and `proc_pid_rusage`
`ri_phys_footprint`. Every accessor returns `Option`.

**New:** `safety.rs` (or a section of `sysmem.rs`) computing the green/yellow/red
state from OS pressure ∨ projected headroom ∨ swap, with the OS signal
authoritative when it reports warn/critical.

**Wire:** extend `Telemetry` with pressure, swap, footprint, headroom; extend
the pre-launch `LaunchPlan` with installed memory, current pressure and
projected headroom so the warning appears *before* Run.

**UI:** memory panel shows predicted breakdown (weights / KV / overhead /
total), actual after launch, pressure badge, swap, headroom; "Unavailable" per
missing field.

**Tests:** normalisation of each sysctl into typed values from fixture bytes;
safety-state truth table including one-metric-unavailable cases; headroom
arithmetic. Pure functions, no live syscalls in assertions.

**Acceptance:** metrics update while running; reset on stop; formatting correct;
one failed metric does not blank the others; warning appears when a launch would
leave macOS short.

**Risk:** `libc` sysctl work is `unsafe`; keep it in one module behind a safe
API and test the parsing, not the syscall.

---

## Phase 2 — Workload presets and improved profiles

Schema v2 with migration from v1 (current shape has no `schemaVersion`; absence
means v1). Migration must be idempotent, preserve `overrides` and `lastRun`, and
never drop unknown keys.

Named profiles with `builtIn` flag, workload category, optional `apiKeyRef`.
Four templates. Duplicate, rename with collision detection, reset a built-in
without touching user profiles.

**Tests:** v1→v2 migration including unknown-key preservation; duplicate-name
handling; built-in reset leaves user profiles intact; template values produce
the expected argv.

**Acceptance:** existing config on this machine loads unchanged; the two
`lastRun` entries and current `defaultProfile` survive.

---

## Phase 3 — Server health and model test

`health.rs`, ordered timed checks, `Redacted` newtype for secrets, reasoning
detection by response-shape probing. Deterministic short prompt.

**Tests:** check sequencing and short-circuit behaviour against a stand-in
server (extend the existing `runner_lifecycle.rs` fixture approach); redaction
of keys in every output path; response-shape detection from recorded fixtures.

---

## Phase 4 — Benchmark history

`benchmarks.json`, append-only, atomic rewrite. Table, sort, filter, compare,
delete, export. Fields per the brief, including llama.cpp version from
`Capabilities` and peak swap from Phase 1 sampling.

**Tests:** round-trip persistence, filter/sort, CSV escaping, comparison
arithmetic.

**Goal:** an objective Q3 vs Q4 comparison on this Mac.

---

## Phase 5 — Pi and Picot integration

Read-only, permission-gated inspection of the Pi config shape. Preview and copy
only; never write. Configurable application paths.

**Tests:** preview generation from several config shapes; assertion that no
write path exists.

---

## Phase 6 — Security guardrails

Effective-argv validation (structured **and** `rawArgs`), loopback detection,
non-loopback confirmation, API key presence warning, port validation and
conflict detection, redaction everywhere.

**Tests:** loopback detection table (127.0.0.1, ::1, localhost, 0.0.0.0, LAN
IPs), `rawArgs` host override detection, port validation, secret redaction,
argv construction.

---

## Phase 7 — UI and usability

Filename on hover, copy server URL, health indicator distinct from process
state, three context figures with careful wording, risk marker on the context
slider, log persistence across crash, reveal in Finder, open project folder,
optional minimise to menu bar, responsiveness under GPU load.

---

## Phase 8 — Documentation and release readiness

User documentation and a redacted diagnostics export.

---

## Open sequencing question

The downloader is unbuilt (Downloads is a placeholder) yet the brief lists it as
existing. It is not in phases 1–8. Options: leave Downloads as a visible
placeholder, remove the nav entry until built, or insert it as a phase. Needs a
decision before Phase 7 touches navigation.
