# Review

Asked for 2026-09-04: a two-axis code review of the whole tree, then a plan "for
all the issues". The review ran against `kis/knowledge/technical.md` and the
Fowler smell baseline on one axis and against every intent file, the two
`docs/*-spec.md` files and the README on the other. Twenty-two findings survived
verification against the code; the author chose all of them.

Live status is in [state/current.md](../state/current.md), not here.

## What was found

Ten defects, seven hygiene breaches, five refactors.

- **Six commands block the main thread.** `launch_plan`, `runner_start`,
  `tune_start`, `get_settings`, `speeds_for` and `orphan_status` are `fn`, and the
  first three reach `runner::inspect_port` — two 3 s HTTP calls when the port is
  busy — and `capabilities()`, which spawns the binary three times on a cache miss.
  `launch_plan` is re-invoked on a 200 ms debounce from the form, so a busy port
  freezes the window per keystroke: the defect `discover_browse` was made `async`
  for ([knowledge/technical.md](../knowledge/technical.md)).
- **The quant picker ignores `lfs`.** `hub::Entry.lfs` is parsed and never read;
  `quant::candidates` filters on `.gguf` and sidecars only. The review's stated
  reason — that the engine refuses a file with no size headers — turned out wrong
  on reading `download.rs`: it falls back to `content-length`. The real reason is
  that Hugging Face keeps every real model in LFS, so a git-tracked `.gguf` is a
  stub, and a stub wins the smallest-first fallback.
- **The Web UI button reads "Open chat"** (`ModelDetail.tsx`), against
  [knowledge/project.md](../knowledge/project.md): "the button reads Web UI". The
  window title is right.
- **`bundle.security.csp` is `null`**, which technical.md says should not stand
  and [release.md](release.md) lists first for the security review. Taken here.
- **`.advanced` is bordered with `var(--line, #2a2f36)`.** `--line` was never
  defined; the anchor is `--border`. The fallback paints a dark slate on both
  light palettes, and the fallback is what hides it from `stylesheet.rs`.
- **`.limit-select` sits in a status line and never opts out** of the global
  `select { width: 100% }` — the exact case technical.md names.
- **`.pi-new` and `.check-name` have no rule in `App.css`.** `.pi-new` renders
  unmarked beside the coloured `.pi-added`/`.pi-removed` it belongs with.
- **The README claims "No profiles or presets" and "no verdict in between".**
  The model screen ships three preset cards ([direction.md](direction.md)
  reversed the rule deliberately and nobody wrote it back) and `launchCost`
  prints "fits" / "over the GPU limit".
- **The probe cache lives for the process.** `docs/runner-spec.md` asked for it
  keyed on the binary's mtime and size; a Homebrew upgrade replaces the file in
  place and the app keeps yesterday's flags until it is relaunched.
- **[figures.md](figures.md) records a memory-panel hint** naming its unit and
  the Library's as the same bytes counted the other way. The redesign dropped it
  with the three rows. **Ruled 2026-09-04: the screen stays; figures.md is
  corrected.**

Hygiene: `hub::valid_tree_path` restates `valid_segment` inline, against "the one
rule"; `AppState.orphan` is written and never read; `downloads::Runtime` mirrors
`Downloads` field for field; `tune::Candidate` carries `cache_k` and `cache_v`
always equal; two stacked doc blocks on `show_main_window`, `history_path`'s
comment above `avatar_path`, a `Profile::default()` comment on `SettingsScreen`,
an empty `impl Config {}`; nested ternaries at `Discover.tsx:294`,
`Presets.tsx:204`, `TunePanel.tsx:207`; and `Downloads.tsx`'s TypeScript copy of
`catalog::quant_from_name` — a recorded deviation in [redesign.md](redesign.md),
now reversed: the job carries its quant the way it carries its owner.

Refactors: `Sparkline` verbatim in `ModelDetail.tsx` and `Activity.tsx`, their
stat `Card` near-identical; `compact()` and `counted()` character-identical and
`updated()` re-implementing `formatRelative`; `pi::document` and
`pi::settings_document` the same read-parse-check; `Presets.tsx` spelling its six
fields three times; and `ModelDetail.tsx` at 1,010 lines holding eight things.

## Plan

Four parcels, one commit each, the four commands green after every one. Visible
changes go last so the author looks once.

1. **Rust — done, `dbed3be`.** Thirteen commands rather than six: reading `lib.rs`
   found `orphan_stop`, `activity_snapshot`, `machine_memory`,
   `set_llama_server_path` and the three settings writers all reaching
   `inspect_port` or a fresh probe, and the reviewer caught the last three. Every
   one is `async fn` on `discover_browse`'s pattern, with a `Result` return where
   there was none, which `api.ts` does not notice. `quant::candidates` drops
   entries with `lfs == false`. `probe::Cache` stamps the binary's mtime and size
   **before** the probe runs — stamped after, a file replaced mid-probe would be
   remembered under the old flags for good — and re-probes when the stamp moves.
   `valid_tree_path` calls `valid_segment` per segment. `AppState.orphan` went,
   and with it a startup `detect_orphans` scan that fed nothing. `Runtime` went
   and `Downloads` is `Clone` with an `Arc` counter. `Candidate` holds one `cache`
   (`Profile` keeps both — llama-server takes them separately). The comments and
   the empty impl went.
2. **The quant badge moves to Rust.** `DownloadJob.quant`, derived by
   `catalog::quant_from_name` where `owner` is derived; `Downloads.tsx` loses
   `isQuantToken`/`quantOf`. The key on `downloads.json` is additive.
3. **Frontend refactors.** `Telemetry.tsx` takes `Card`, `Sparkline`, `rate` and
   `TelemetryPanel`, and Activity imports the first two. `launchCost` and
   `MemoryBar` move into `Memory.tsx` beside `Stat`. `compact`/`counted` become
   one `formatCount` in `format.ts`; `updated` uses `formatRelative`. Presets
   derives `pick` and `suggestionFields` from `SPEED_KEYS`. `pi::document` and
   `settings_document` share one read-and-parse. The three nested ternaries become
   if-chains. `Presets`' `Card` is a different shape and stays.
4. **Visible.** "Web UI" on the button. `csp` set to
   `default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'`.
   `.advanced` on `--border`; `.limit-select` opts out of the width; `.pi-new` gets
   a rule and `.check-name` is dropped. README's two lines rewritten to what the app
   does. figures.md's hint sentence corrected.

Out of scope: shortening `inspect_port`'s timeouts; the rest of the release's
security review; the two open quant-picker calls in [discover.md](discover.md);
the unusable-window bug; the README screenshots.

## Acceptance

- A non-LFS `.gguf` never appears in `candidates` — tested.
- `capabilities()` re-probes after the binary's mtime changes — tested.
- `valid_tree_path` rejects, per segment, what `valid_segment` rejects — tested.
- A `DownloadJob`'s `quant` matches `catalog::quant_from_name` — tested.
- Every new test watched to fail with the guarded code gutted, per
  [knowledge/technical.md](../knowledge/technical.md).
- `stylesheet.rs` green; no bare `var(--x)` added.
- `grep` finds no "Open chat", no `--line`, no `isQuantToken` under `src/`.
- The author's look at the built app: the pane paints under the CSP; the Web UI
  button; the Advanced border on a light palette; the Downloads status line; pi's
  "new file" mark.

## Risks

- **The CSP is the one change the render cannot check.** Headless Chrome does
  not enforce Tauri's policy; a wrong one is a blank pane. `devCsp` exists if dev
  and bundle need to differ. The author's look is the first check here, not the
  last.
- **Async only moves the cost.** A busy port still spends up to six seconds in
  `inspect_port`; the window paints during it, which is the ask.
- **Parcel 3 has no test.** `tsc` and the look are its checks; it is pure moves.
- A row in `downloads.json` from an older build reads `quant` as `None` and shows
  no badge, as an unparseable name does today.

## Proof

**Parcel 1, 2026-09-04.** The four commands green, each status on its own line:
`bun run build` 0, `cargo test` 0 (208 lib tests, 3 new), `cargo clippy
--all-targets -D warnings` 0, `cargo fmt --check` 0. The three new tests —
`a_file_outside_lfs_is_never_offered`, `a_replaced_binary_is_probed_again_and_an_unchanged_one_is_not`,
`a_tree_path_is_judged_one_segment_at_a_time` — each watched to FAIL with its
guard gutted (`if false`, `if unchanged || true`, `!path.is_empty()` alone) and
pass restored. A reviewer read the diff against this plan and re-ran the gates;
its two Important findings (three sync settings writers, the stamp taken after
the probe) are fixed above. Nothing in this parcel is visible, so nothing to look
at yet.
