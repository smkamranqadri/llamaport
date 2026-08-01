# Local runtime manager — design

Phase 0 audit and target design. No production code changed to produce this
document.

## 1. What the application actually is

| Concern | Reality |
| --- | --- |
| Framework | Tauri v2 (`src-tauri/tauri.conf.json`, identifier `com.mkamran.llama-cpp-hub`) |
| Runtime layer | Rust, `src-tauri/src/*.rs`, ~2500 lines |
| UI layer | React 19 + TypeScript, Vite 7, plain CSS. No component or state library |
| Package manager | bun |
| Persistence | One JSON file: `~/Library/Application Support/llama-cpp-hub/config.json` (`store.rs`). No database |
| Process manager | `runner.rs` — one supervised `llama-server` child, std threads |
| Command builder | `profile.rs` — `Profile::args()` renders argv, `render_command()` renders display text only |
| Metrics source | `ureq` HTTP against `/health`, `/props`, `/metrics` on the child |
| Log storage | In-memory `VecDeque` capped at 2000 lines inside `runner.rs`. **Not persisted** |
| Testing | `cargo test` only. 40 passing, 1 `#[ignore]`d. **No frontend test runner** |
| Type/lint | `bun run build` = `tsc && vite build`. No ESLint. clippy and rustfmt installed but not wired |
| Release | `bun run tauri build`, bundle targets `all`. No signing or notarisation configured |
| Version control | **None — the directory is not a git repository** |

## 2. Data flow, as built

```
Library.tsx (row click)
  └─> ModelDetail.tsx  ── getLaunchPlan(modelId, draft?) ──> lib.rs::launch_plan
                                                               │
        build_plan(): catalog cache lookup ──────────────────┐  │
                      store::Config.default_profile ────────┤  │
                      store::Config.overrides[model_id] ────┤  │
                      resolve(): merge patch, derive alias,  │  │
                                 clamp ctx to header max ────┘  │
                      probe::probe() (cached Capabilities) ─────┤
                      Profile::args() ──> argv                  │
                      render_command() ──> preview string       │
                      estimate::estimate() ──> memory panel     │
  <──────────────────────────── LaunchPlan ─────────────────────┘

ModelDetail "Run" ── runnerStart(modelId, draft) ──> lib.rs::runner_start
   guards: unparseable GGUF, incomplete shard set, capability error
   build_plan() again ──> LaunchSpec ──> Runner::start()
                                          │
   spawn_run(): baseline system memory, find_free_port(), Command::spawn(argv),
                pidfile write, generation++ ─┬─> stdout/stderr reader threads
                                             ├─> health+telemetry thread
                                             └─> waiter thread (try_wait 250ms)
                                                   │
   EventSink (trait) ──> TauriEvents ──> app.emit()  │
        runner:state / runner:log / runner:telemetry │
                                                     │
App.tsx listeners ──> setRunner / setLogs / setTelemetry ──> props into ModelDetail
Exit ──> classify by phase ──> CalibrationSample ──> SampleSink ──> Config
```

Two details that matter for later phases:

- **argv is never shell-interpreted.** `Command::spawn` passes an argument
  vector; `render_command()` exists only to display. Command injection through
  form fields is structurally impossible today. `rawArgs` is still a real risk
  for a different reason — see §4.
- **`build_plan` is called on every keystroke** (200 ms debounce) so the preview
  and estimate stay live. It must remain cheap: it reads a cached catalog and
  cached capabilities, never rescans.

## 3. Existing abstractions worth extending

| Seam | Where | Extend for |
| --- | --- | --- |
| `EventSink` trait | `runner.rs` | Phases 3–4 can drive health checks and benchmarks headlessly, as `runner_lifecycle.rs` already does |
| `Capabilities` + flag probing | `probe.rs` | llama.cpp version differences; already gates `--flash-attn` value form, `--metrics`, `--no-jinja` |
| `Profile` / `ProfilePatch` | `profile.rs` | Phase 2 named profiles and templates. Sparse-patch merge already exists |
| `CalibrationSample` | `estimate.rs` | Phase 4 benchmark rows are the same shape plus timings |
| `LaunchSpec` | `runner.rs` | Carries everything a benchmark row needs to record |
| `parse_metrics` / `kv_usage` | `runner.rs` | Pure functions, fixture-testable |
| `catalog::scan` | `catalog.rs` | Already surfaces unparseable files rather than hiding them |

The runner's decoupling from `AppHandle` (done in phase 2 of the original build)
is what makes phases 3 and 4 testable without a window.

## 4. Risks

**Correctness and safety**

1. **`rawArgs` bypasses every structured guard.** A user can type
   `--host 0.0.0.0` or `--api-key secret` into extra arguments; it is appended
   after the structured flags, so it wins on most llama.cpp options. Phase 6
   host validation is defeated unless `rawArgs` is scanned for the same keys.
2. **No API key support at all.** No `--api-key` flag, no redaction path. Any
   non-loopback binding today is unauthenticated by construction.
3. **Non-loopback binding is unvalidated.** `host` is a free-text field.
4. **Port conflict detection is optimistic.** `find_free_port` binds and drops,
   a TOCTOU race, and only reports the substitution after the fact.

**State and persistence**

5. **Config has no schema version and drops unknown fields.** `serde` with
   `#[serde(default)]` silently discards keys it does not know, and `save()`
   rewrites the whole file. A newer app version writing config, then an older
   one saving it, loses data. Phase 2 requires migration; this must be fixed
   first.
6. **`save()` is not atomic** — `fs::write` straight onto the live path. A crash
   mid-write truncates config.
7. **Benchmark history cannot live in `config.json`.** It grows unboundedly and
   every profile edit would rewrite it.
8. **Logs are lost on app restart.** Phase 7 explicitly wants them preserved
   across a crash.

**Process lifecycle**

9. **Orphan reaping kills without asking.** `reap_orphan()` kills a live pidfile
   process on startup after verifying the name contains `llama-server`. It
   cannot distinguish our orphan from a server the user started by hand — and
   this session already demonstrated the cost of that confusion by killing a
   user's manually-started server with an over-broad `pkill`.
10. **Competing servers are undetected.** A `llama-server` the user started
    outside the app holds port 8888; the app silently falls forward to 8889 and
    reports it, but never says *why* the port was busy.
11. **`Child` is not killed on `Drop`.** Every exit path must go through
    `Runner::stop`. `RunEvent::Exit` covers normal quit; a hard crash of the app
    leaves the server running, which is what the pidfile exists for.

**Prediction accuracy**

12. **Per-process memory is not meaningful on Apple Silicon.** Measured on this
    machine: RSS 16.2 GB, Activity Monitor 1.28 GB, wired memory 20 GB, for one
    model. `-ngl all` puts weights and KV cache in Metal buffers attributed to
    the kernel. Documented in `DESIGN.md`; Phase 1 must present this honestly
    rather than pick one number.
13. **Calibration has zero samples so far** and the fitted overhead is unused;
    the 1.4 GB default is a placeholder, and one real measurement (Devstral,
    8K ctx) suggested ~120 MB.
14. **MLA architectures are mis-estimated.** `GLM-4.7-Flash` reports `deepseek2`
    with a compressed latent cache; the k+v formula over-counts it.

**Compatibility**

15. **llama.cpp metric names are unstable.** Build 10090 has no `kv_cache_*`
    series at all; occupancy is derived from `n_tokens_max / n_ctx`. Any check
    added in phases 3–4 must assume names can vanish.
16. **A GUI app launched from Finder has a minimal PATH**, so binary discovery
    cannot rely on `which`.

**Process**

17. **No version control.** Phased work with rollback is materially harder, and
    "preserve progress" currently means these documents alone.
18. **No frontend test runner**, so any logic placed in React is untestable
    under rule 10. Mitigation: keep computation in Rust, keep components thin.

## 5. Target design

### 5.1 Memory and safety (Phase 1)

Prefer native APIs over parsing `memory_pressure` or `vm_stat` output. A new
`src-tauri/src/sysmem.rs` wraps `libc`:

| Value | Source |
| --- | --- |
| Installed unified memory | `sysctlbyname("hw.memsize")` |
| Memory pressure level | `sysctlbyname("kern.memorystatus_vm_pressure_level")` → 1 normal / 2 warn / 4 critical |
| Swap used | `sysctlbyname("vm.swapusage")` → `struct xsw_usage` |
| Process footprint | `proc_pid_rusage(pid, RUSAGE_INFO_V4)` → `ri_phys_footprint`, the number Activity Monitor shows |
| System used / total | existing `sysinfo` |

Every field is `Option`. One failed read must not blank the panel — the UI
renders "Unavailable" per field, never a fabricated zero.

The safety state combines the OS pressure level with projected headroom and is
**not** a fixed percentage:

```
state = worst_of(
  os_pressure_level,                       // authoritative when it says warn/critical
  headroom_state(installed - used - predicted_total),
  swap_state(swap_used)
)
```

`headroom_state` is a heuristic and must be labelled as such. All wording
distinguishes *predicted before launch* from *actual after launch* from
*system-wide pressure* from *swap in use*.

Polling: the existing 1 Hz telemetry thread already stops cleanly on state
change and generation bump; system memory is sampled on the same tick, so no new
timer is introduced. Pre-launch (idle) sampling is on demand, not polled.

### 5.2 Profiles and templates (Phase 2)

Extend, do not replace. `Config` gains:

```jsonc
{
  "schemaVersion": 2,
  "profiles": [ { "id", "name", "builtIn", "workload", "apiKeyRef", ...Profile } ],
  "defaultProfile": { ... },   // retained
  "overrides": { "<modelId>": ProfilePatch },  // retained
  "<unknown keys preserved>"
}
```

Two hard requirements from §4: `Config` must carry
`#[serde(flatten)] extra: Map<String, Value>` so unknown keys survive a
round-trip, and `save()` must write to a temp file and rename.

Templates (Quality Coding, Balanced, Long Context, Lightweight) ship as built-in
profiles marked `builtIn: true`, resettable individually, never deleting user
profiles. They are starting points calibrated for a 32 GB M2, not universal
truth, and the UI must say so.

### 5.3 Health and model test (Phase 3)

A `health.rs` module driven through the existing `EventSink`, running an ordered
check list, each timed and independently reportable: process alive → TCP reachable
→ `/health` → `/v1/models` → alias present → non-streaming chat → streaming chat
→ TTFT and totals. Reasoning-field detection must probe the response shape rather
than assume a key. Result is Passed / Passed with warnings / Failed with per-check
durations. Redaction happens at the *source*: a `Redacted` newtype for secrets so
they cannot reach logs, previews, or diagnostics by accident.

### 5.4 Benchmarks (Phase 4)

Separate file `benchmarks.json` in the same directory — same technology, no new
dependency, but isolated from config so history growth never risks profile data.
Append-only with atomic rewrite on delete. Table, sort, filter, compare two runs,
delete, export JSON/CSV. No charts; the project has no charting system.

### 5.5 Agent integration (Phase 5)

Read-only inspection of `~/.pi/agent/settings.json` **only with explicit
permission**, and never a write. The app generates a preview and copies it; the
user pastes. Application paths (Picot, VS Code) are configurable, never
hard-coded.

### 5.6 Security (Phase 6)

Loopback default stays. Non-loopback requires explicit confirmation and warns
harder when no API key is set. Critically, validation must run against the
*effective* argv — structured fields **and** `rawArgs` — since `rawArgs` can
reintroduce `--host`. Command construction is already injection-safe by argv;
document that rather than adding escaping theatre.

## 6. Sequencing note

The requested phase list contains no downloader work, but the Downloads screen
is a placeholder and downloading is not implemented. See
`current-state.md` §Known problems.
