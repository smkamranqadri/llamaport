# Review

A two-axis code review of the whole tree, run 2026-09-04: one axis against
[knowledge/technical.md](../knowledge/technical.md) and the Fowler smell
baseline, the other against every intent file, the two `docs/*-spec.md` files,
and the README. Twenty-two findings survived verification against the code;
the author accepted all of them.

## Findings

### Defects (ten)

- Six commands ran synchronously and blocked the main thread on port checks
  and probe calls, freezing the window on a busy port.
- The quant picker ignored the `lfs` flag, so a git-tracked stub `.gguf` could
  win the smallest-first fallback.
- The Web UI button read "Open chat" against the copy stated in
  [knowledge/project.md](../knowledge/project.md).
- `bundle.security.csp` was `null`.
- `.advanced` was bordered with an undefined `--line` token, which fell back
  to a colour that hid the border on light palettes.
- `.limit-select` did not opt out of the global full-width select rule.
- `.pi-new` and `.check-name` had no rule in `App.css`.
- The README claimed "No profiles or presets" and "no verdict in between",
  both since reversed by shipped features.
- The probe cache lived for the process instead of being keyed on the
  binary's mtime and size, so a Homebrew upgrade left stale flags until
  relaunch.
- [figures.md](figures.md) recorded a memory-panel hint the redesign had
  dropped. Ruled 2026-09-04: the screen stays as built, and figures.md is
  corrected to match.

### Hygiene (seven)

- `hub::valid_tree_path` restated `valid_segment` inline instead of calling it.
- `AppState.orphan` was written and never read.
- `downloads::Runtime` mirrored `Downloads` field for field.
- `tune::Candidate` carried `cache_k` and `cache_v` always equal.
- Several stale or duplicated doc comments, and an empty `impl Config {}`.
- Nested ternaries in three frontend files.
- `Downloads.tsx` kept a TypeScript copy of `catalog::quant_from_name`.
  Reversed: the job now carries its quant the way it carries its owner.

### Refactors (five)

- `Sparkline` duplicated verbatim across two components; their stat `Card`
  was near-identical.
- `compact()` and `counted()` were character-identical; `updated()`
  reimplemented `formatRelative`.
- `pi::document` and `pi::settings_document` repeated the same
  read-parse-check.
- `Presets.tsx` spelled its six fields three times.
- `ModelDetail.tsx` held eight responsibilities in 1,010 lines.

## Plan

Four parcels, one commit each, all completed 2026-09-04.

1. **Rust.** Thirteen commands made `async`, not six as first scoped.
   `quant::candidates` drops non-LFS entries. The probe cache stamps the
   binary's mtime and size before probing, not after. `valid_tree_path` calls
   `valid_segment` per segment. `AppState.orphan` and `downloads::Runtime`
   removed.
2. **Quant badge to Rust.** `DownloadJob.quant` is derived in Rust the way
   `owner` is, including for jobs restored from `downloads.json`.
3. **Frontend refactors.** `Sparkline`, `Card`, `rate`, `launchCost`, and
   `MemoryBar` consolidated; `formatCount` replaces two duplicate helpers;
   `pi.rs`'s two readers share one parser; nested ternaries became if-chains.
   `ModelDetail.tsx` reduced to 723 lines.
4. **Visible.** Button copy, CSP, and the four CSS fixes shipped together so
   the author reviews once. README and figures.md corrected; the redesign's
   recorded TypeScript-quant deviation marked reversed.

## Acceptance

- A non-LFS `.gguf` never appears in `candidates`.
- The probe re-runs after the binary's mtime changes.
- `valid_tree_path` rejects, per segment, what `valid_segment` rejects.
- A `DownloadJob`'s `quant` matches `catalog::quant_from_name`.
- `stylesheet.rs` passes with no bare `var(--x)` added.
- No "Open chat", `--line`, or `isQuantToken` remains under `src/`.
- The built app paints under the CSP and shows the Web UI button, the
  Advanced border on a light palette, the Downloads status line, and pi's
  "new file" mark.

## Verified

Verified 2026-09-04: all four checks passed after each parcel. The author
reviewed the visible changes on the built bundle on 2026-09-04.

## Risks

- Async only moves the cost: a busy port still spends up to six seconds in
  the probe call, during which the window now paints instead of freezing.
- A CSP cannot be checked by rendering. Headless Chrome does not enforce
  Tauri's policy, so a wrong CSP shows as a blank pane only in the built app.
