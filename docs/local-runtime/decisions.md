# Decisions

Append-only. Each entry records what was decided, why, and what would reverse it.

---

## D1 — Tauri v2 + React/TS, not Electron or SwiftUI

**Date:** before this plan. **Status:** settled.

~10 MB binary, no bundled Chromium, and Rust suits the two hard parts: child
process supervision and (later) a segmented downloader. Cost was a one-time
`rustup` install.

**Reverses if:** the app needs to ship on Windows/Linux with heavy native
integration that Tauri makes awkward. No current pressure.

---

## D2 — Model identity is `(file_size, sha256 of first 4 KB)`

Free during the scan that already reads the header. Profiles survive renames and
directory moves. Full-file hashing would mean reading 21 GB on first sight.

**Reverses if:** collisions appear in practice. None expected in a personal
library.

---

## D3 — Runner reports through an `EventSink` trait, not `AppHandle`

Decouples process supervision from Tauri so the lifecycle is testable headlessly.
`runner_lifecycle.rs` drives spawn → Ready → telemetry → stop against a stand-in
HTTP server with no window.

**Consequence:** phases 3 and 4 can be tested the same way. Do not reintroduce a
direct `AppHandle` dependency in the runner.

---

## D4 — Architecture is shown on the model detail page, never in list rows

`general.architecture` is the llama.cpp architecture id, not the model's release
identity: `Qwen3.6-35B-A3B` reports `qwen35moe`, `GLM-4.7-Flash` reports
`deepseek2`. Beside the model name it reads as a contradictory version claim.

Renaming it to something friendlier was rejected: the raw id is what llama.cpp
accepts and reports, which is exactly what matters when a build refuses to load
a model.

---

## D5 — KV occupancy is derived, not read

llama.cpp build 10090 exposes no `kv_cache_*` metric series. Occupancy is
computed as `n_tokens_max / n_ctx`, falling back to `kv_cache_usage_ratio` when
an older build provides it. Validated against `/slots`, which reported
`n_prompt_tokens` 20768 against `n_tokens_max` 20768.

**Consequence:** any metric added in later phases must assume names can vanish
between builds. Probe, do not assume.

---

## D6 — Memory is measured machine-wide, not per process

Three sources disagreed on one running model on this 32 GB M2:

| Source | Reported |
| --- | --- |
| `sysinfo` process memory (RSS) | 16.2 GB |
| Activity Monitor process column (physical footprint) | 1.28 GB |
| System wired memory | 20.0 GB |

With `-ngl all`, weights and KV cache live in Metal buffers attributed to the
kernel, not the process. Calibration therefore records peak system-wide growth
against a pre-launch baseline, and the running panel shows system used/total and
swap so it agrees with Activity Monitor rather than contradicting it.

**Phase 1 nuance:** the brief asks for "memory used by the running llama-server
process". That will be shown using `proc_pid_rusage` `ri_phys_footprint` — the
Activity Monitor number — but labelled as the process footprint and presented
*alongside* system pressure, never as the model's true cost.

---

## D7 — Negative calibration residuals are dropped, not clamped

A run whose observed growth is below the predicted weights+KV base says nothing
about compute overhead — the machine may have evicted as fast as the model
loaded, or the weights may never have become fully resident. Clamping to zero
would bias the fitted overhead downward, which is the dangerous direction.

---

## D8 — K and V cache dimensions are summed separately

`kv = layers × ctx × kv_heads × (k_dim × bpe_k + v_dim × bpe_v)`, not
`2 × head_dim`. Latent-attention architectures size K and V differently:
`GLM-4.7-Flash` reports a 576-wide key with one KV head.

**Open:** MLA stores a compressed latent rather than per-head K/V, so this still
over-counts `deepseek2`. Treat that family's estimate as provisional.

---

## D9 — Command injection is prevented structurally, not by escaping

`Command::spawn` receives an argument vector and no shell is involved.
`render_command()` shell-quotes for *display only*. Phase 6 should document this
rather than add escaping that implies a shell exists.

**But:** `rawArgs` remains a genuine risk because it can reintroduce `--host` or
`--api-key` past the structured fields. Validation must run on effective argv.

---

## D10 — Native sysctl over shell parsing *(proposed, Phase 1)*

`hw.memsize`, `kern.memorystatus_vm_pressure_level`, `vm.swapusage` and
`proc_pid_rusage` via `libc`, rather than parsing `memory_pressure`, `vm_stat`
or `sysctl` output. Isolated in `sysmem.rs`; the parsing of raw structs is what
gets fixture tests, not the syscalls.

---

## D11 — Benchmarks live in `benchmarks.json`, not `config.json` *(proposed, Phase 4)*

Same technology, no new dependency, but isolated: history grows unboundedly and
must never put profile data at risk during a rewrite.

---

## D12 — Git, with one commit per phase

**Date:** Phase 0. **Status:** settled by the user.

The directory was untracked. Initialised with a root `.gitignore` covering
`node_modules/`, `dist/`, `src-tauri/target/` and `.DS_Store`. Each phase ends
with a commit, giving a rollback point per phase and a readable history
alongside `current-state.md`.

---

## D13 — Downloads keeps its visible placeholder

**Date:** Phase 0. **Status:** settled by the user.

The Downloads screen is not implemented and is not covered by phases 1–8,
despite the brief assuming it exists. The nav entry stays and continues to say
the feature is unbuilt: honest about the gap, and Phase 7's navigation work must
leave it alone rather than quietly hiding it.

The downloader design (segmented ranges, signed-URL re-resolution on resume,
shared rate-limit token bucket) is preserved in the root `DESIGN.md` for
whenever it is scheduled.

---

## D14 — Orphaned servers are surfaced, never auto-killed

**Date:** Phase 0. **Status:** settled by the user. Supersedes the current
behaviour of `runner::reap_orphan()`.

A live pidfile on startup will be reported — "an orphaned llama-server is
running on port X" — with Stop and Adopt offered to the user. The app will not
kill a process the user did not ask it to kill.

The existing auto-kill relies on a pid plus a name check, which is weak
evidence: pids are recycled, and the check cannot distinguish our orphan from a
server started by hand. This session demonstrated the cost directly by killing a
user's manually-started server with an over-broad `pkill`.

**Implementation:** scheduled for Phase 1 alongside the other lifecycle-adjacent
fixes, or Phase 6 if it proves larger than expected.

---

## D15 — Memory prediction is multiplicative, not additive

**Date:** Phase 1. **Status:** settled. Supersedes the additive overhead model in D6.

Measured on the 32 GB M2 while running `Qwen3.6-35B-A3B-UD-Q3_K_XL`:

| | |
| --- | --- |
| Idle, before launch | 16.5 GB used |
| Running | 27.1 GB used |
| **Observed growth** | **≈10.6 GB** |
| Nominal weights + KV | 18.4 GB (15.7 + 2.7) |

The machine grew by less than the weights file alone. With mmap plus Metal, a
large fraction of weight pages never counts as used memory, so `observed −
nominal` is reliably negative. The additive fit discards negative residuals by
design (D7), which meant it could never accumulate a single usable sample on
this hardware: calibration would have sat at the placeholder constant forever
regardless of how many models were run.

The estimate now carries two distinct figures:

- **Nominal** — weights + KV + overhead. What the model needs on paper.
- **Machine impact** — `residency × (weights + KV)`, where residency is the
  median observed ratio across recorded runs. What used memory is expected to
  grow by, and what the safety assessment consumes.

Uncalibrated, machine impact falls back to nominal, which over-predicts on this
platform — the safe direction. Ratios outside 0.1–2.0 are discarded as
measurement noise rather than fitted.

**Reverses if:** a platform appears where the ratio is not stable across models,
in which case residency likely needs to be per-architecture or per-quantisation
rather than a single machine-wide constant.

---

## D16 — Phase 6 (security guardrails) skipped

**Date:** Phase 5 → 7 transition. **Status:** settled by the user.

Deliberately not implemented. What remains open as a result:

- **`rawArgs` bypasses structured validation.** Typing `--host 0.0.0.0` into extra
  arguments exposes an unauthenticated server to the LAN, because validation runs
  on the form fields rather than the effective argv. The default stays
  `127.0.0.1`, so this requires a deliberate act — which is why skipping is
  defensible for a single-user tool.
- **No API key storage.** The Connect screen reports authentication as "none",
  accurately. `Redacted` and argv redaction exist and are tested, so adding keys
  later is wiring rather than design.
- **No port conflict detection before launch.** Port fallback already works and
  reports the substitution, but nothing warns that the *reason* is another
  server.

The last item is the one that has actually caused harm: Pi is configured for
8888, fallback puts the server on 8889 or 8890, and Pi then fails silently. The
Connect screen's port-mismatch warning (Phase 5) covers the symptom; detecting
the conflict at launch would address the cause.

**Reverses if:** the server is ever bound off-loopback, or the app is used by
more than one person on a shared machine.

---

## D17 — A busy port refuses the launch; it never falls forward

**Date:** after Phase 7. **Status:** settled. Supersedes the fallback behaviour in
the original `DESIGN.md`.

Falling forward to the next free port was my decision in the first design
document, and it was wrong for an application whose stated rule is one model at a
time. Observed twice in one evening: the requested port was busy, the app quietly
started a second server on 8889 or 8890, and the result was two copies of the
same 15.7 GB model resident simultaneously — on a 32 GB machine — reachable by no
client, since Pi is pinned to 8888.

A launch onto a busy port now fails with a message naming the occupant, and
distinguishing another llama-server from an unrelated process because the remedy
differs.

Related: starting a model that is already running is refused outright, and
orphan detection now scans for `llama-server` processes rather than trusting a
pidfile that only ever knew the last pid written to it.

---

## D18 — The downloader is the outstanding half of the original goal

**Date:** after Phase 7. **Status:** acknowledged, not yet built.

The first design document set two goals: run local models, and download them with
resume that survives an app restart. Seven phases went into the runtime — memory
calibration, safety states, workload templates, health checks, benchmark history,
agent configuration — while the downloader, fully designed in `DESIGN.md`
§Downloader, was never started. The user's actual workflow for acquiring models is
still `curl` plus an external download manager, which is what the app was meant to
replace.

The design stands and needs no revision: manual redirect following to capture the
signed CDN URL, segmented ranged GETs, a `.part.json` sidecar so resume survives a
process exit, re-resolution of the expiring URL on every resume, `Authorization`
stripped on the cross-host redirect, and one shared rate-limit token bucket.

**Next session should start here.**

---

## D19 — Benchmarks, agent integration and profile CRUD removed

**Date:** after the runtime correction. **Status:** settled by the user.

An inventory found 29 commands and 6,707 lines of Rust, of which roughly a third
and half the command surface served features that were never part of the original
goal — while the resumable downloader, which was half of it, remained unbuilt.

Deleted: `benchmarks.rs`, `agents.rs`, the Benchmarks and Connect screens, the
benchmark half of `health.rs`, and the create/rename/duplicate/delete/reset
surface around named profiles.

Kept, deliberately:

- **The four workload templates**, as apply-only buttons. Per-model overrides
  already persist what a user changes; a second profile system on top of that was
  the part that had no reason to exist.
- **The model test**, reduced to "is this server working" — its diagnostic value
  was proven when it caught a reasoning model being reported as broken.
- **The memory estimator and safety verdict.** These solve the user's real
  problem on a 32 GB machine and found a real one: an orphaned server costing
  measurable inference speed.
- **`redact.rs`**, though nothing currently stores a secret. It is small, tested,
  and is the correct primitive if an API key ever lands.

Result: 5,061 lines, 108 tests, 17 commands, 9 frontend files. Existing
`benchmarks.json` is left on disk untouched — deleting a feature should not
delete the user's data. A stored `profiles` key in config now survives via the
`extra` catch-all rather than being dropped.

**Reverses if:** benchmarking becomes a recurring need rather than a
one-off question that has now been answered.

---

## D20 — No profile system; the last successful launch is remembered instead

**Date:** after D19. **Status:** settled by the user.

The remaining profile machinery is gone: the four workload templates, per-model
override patches, the saved global default, and the merge layering between them.
`ProfilePatch`, `profiles.rs` and the Settings profile editor are deleted.

In its place, one behaviour: **the settings a model was last launched with are
remembered, per model, and the form opens there.** Written only after a launch
succeeds, because settings that failed to start are not what anyone wants to
return to. There is no merging, no defaults layer and no second profile concept —
one entry per model, replaced wholesale each time.

A `Profile` is now simply the values one launch uses. The form is the launch.

Result: 5,061 → 4,742 lines of Rust, 108 → 87 tests, 17 → 13 commands.

**Reverses if:** the same model needs to be run several ways routinely, at which
point named configurations become worth their weight again — but per-model memory
covers the common case, which is running each model the same way every time.

---

## D21 — Retired keys are pruned at migration, not preserved forever

**Date:** consolidation. **Status:** settled.

The unknown-key rule (D-era Phase 1) exists so that running an older build cannot
delete settings a newer one wrote. It is the wrong rule for keys this build
*deliberately removed*: `defaultProfile`, `overrides`, `lastRun` and `profiles`
would otherwise be carried forward indefinitely as unrecognised data.

Schema v3 drops exactly those four, and keeps preserving everything else it does
not recognise. Removing a feature should clean up after itself; it should not
license discarding data that belongs to someone else.

---

## D22 — Dead code deleted rather than kept "for later"

**Date:** consolidation. **Status:** settled.

`redact.rs` (182 lines, 8 tests) protected API keys, but nothing in the app could
store one — every call site passed `None`. It was kept once on the argument that
it was the right primitive for a future feature; on review that is how dead code
accumulates. Deleted along with `Runner::is_busy`, which had no callers.

If keys arrive, the primitive is three commits back in the history and took an
hour to write. That is cheaper than carrying it untested-in-anger indefinitely.

**Consequence:** the model test no longer accepts an API key. A server behind
authentication cannot be tested until keys exist as a real feature.
