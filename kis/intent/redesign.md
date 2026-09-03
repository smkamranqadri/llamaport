# Redesign

Planned 2026-09-02, completed 2026-09-03, shipped in
[v0.7.0](release.md). The reference was OrbStack. The approved mockups are
the spec: https://claude.ai/code/artifact/f92717ab-9ddc-4ae7-8fb2-58b13137d257,
eleven artboards including the edge states (stray llama-server banner, empty
first run) and one light-mode variant.

## Decisions from the interview

- **Layout is sidebar plus one content pane**, chosen over OrbStack's
  three-zone layout after seeing both drawn.
- **Sidebar entries**: Library, Discover, Downloads, Activity Monitor, with
  Settings at the bottom.
- **pi is a button on the running model**, not a sidebar entry.
- **The launch form shrinks behind three presets**, Default, Best speed and
  Model suggested, with context and port still visible and everything else
  behind an Advanced disclosure.
- **The running view is four cards**, Memory, Speed, Context and Health,
  each figure paired with a plain-language gloss.
- **First run offers starter models** sized to the machine's memory, with
  paste-a-URL still available.
- **Theme follows macOS.** Later extended: see [appearance.md](appearance.md),
  where following macOS becomes one of three modes and four ported palettes
  join the built-in one.
- **Activity Monitor has a CPU column and no GPU column.** Per-process GPU
  usage has no public macOS API, so the column is dropped for good rather
  than deferred.
- **README screenshots were taken once**, against the finished UI, on
  2026-09-04 for v0.7.0.

## Phases

1. **Shell and Library**, 2026-09-02: sidebar, the visual system (tokens,
   rows, buttons, cards), Library grouped into Running and Stopped, the
   orphan banner restyled. Discover and Activity Monitor shipped as disabled
   entries until their screens were built. Four resolutions from this phase
   still hold: the sidebar's "Now Running" strip is gone, replaced by a
   green dot on Library; a row's Run action sends no draft, so `runner_start`
   resolves the same remembered profile the model screen opens with; and the
   native title bar stays.
2. **Launch**, 2026-09-02: the three presets, built on machinery that
   already existed.
3. **Running view**, 2026-09-02: the four cards and the address line with
   Copy.
4. **Downloads, Settings, first run, Activity Monitor**: first run landed
   2026-09-02, the rest 2026-09-03.

## Design decisions

The decisions below still describe the shipped app, grouped by screen.

### Launch and presets

- A preset owns six fields: `ctx`, `ngl`, `parallel`, both cache types, and
  flash attention. Alias, port, `jinja` and extra arguments stay the user's.
- The built-in default is already the fit (`ctx` Auto, `ngl` auto), so
  Default and Model suggested coincide until custom defaults are saved.
- Best speed, when disabled, offers Measure in its place.
- `SpeedConfidence` is `neverMeasured | observed | tuned`.
- The memory line and the memory panel are both computed from one
  `launchCost`, so they cannot disagree.
- Context is a named dropdown, not a slider: the first option fits the
  model to memory and is marked recommended, then options step down from
  the model's maximum to 4,096.
- `ProfileForm` split into `ProfileFields` and `AdvancedFields`, so the
  model screen shows the advanced fields in a page-level row while Settings
  composes both.
- Design tokens were read off the mockup's own stylesheet, not from memory.
- Content fills the window; no fixed-width column crops it on a wide
  screen.

### Running view

- A running model shows no launch machinery; choosing settings is the
  stopped screen's job.
- No back button and no Library crumb; the sidebar is how you leave a
  model.
- Logs open only after a crash, not on every launch.
- The pressed preset card is remembered and stays highlighted until
  another edit clears it.

### First run

- The starters are three fixed Qwen models, verified live by HTTP HEAD so
  the size shown is the size that lands.
- The ceiling is the Metal working set, not installed memory; where the
  binary is not found the screen falls back to installed memory and says
  so.
- Four fit bands: fits easily (up to 50%), fits (up to 75%), tight, with
  little room for a conversation (up to 100%), and too big for this Mac
  (above 100%).
- `download_start` creates the models directory if it does not exist.
- Rescan stays in the header, because this is the one screen where the
  folder may be missing.

### Library rows

- One line per model: a status dot, the name, badges, one stat sentence,
  the action.
- The star and Delete controls appear on hover; a favourited model keeps
  its star visible at rest.
- The header carries the counts and the search field, not a Download
  model button.
- The pi icon is the installed pi package's own logo, not a drawn
  substitute.

### Stray-server banner

- One line per stray server; the alias wins over the file name.
- The banner exposed a defect: `refresh_processes` leaves `cmd` empty, so
  every orphan had been reported as an unknown model on an unknown port.
  The fix uses `refresh_processes_specifics` with full process details.

### Measure

- Built inside the model screen, not as its own screen.
- The measurement history stays, folded; it is `speeds.json`'s only route
  in the app.
- Each try is named in words with its arguments shown beside it.
- The caveats fold behind "Why this one"; one verdict line stays visible.
- The row folds itself and applies the answer once the ladder ends.
- `tune::Report` carries `candidates` from the start of the ladder, so a
  waiting row can be shown before any try finishes.

### Downloads

- A finished row is named by the catalog, so the Library and Downloads
  cannot call the same file different things.
- The download's quant has been carried from Rust since 2026-09-04,
  replacing a TypeScript copy of the same rule.
- The source URL is shown only after a failure; elsewhere it is the row's
  tooltip.
- The speed limit is a named choice on the status line (no limit, 0.5, 1,
  1.5 or 2 MB/s) plus a custom field.
- A third group, "Did not finish", separates failures from both active and
  finished transfers.
- Clear sits below both lists, because it removes complete and failed rows
  together.

### Activity Monitor

- Shows every `llama-server` the app knows: the running model, the
  measurement server, and any stray, each row saying which it is.
- CPU comes from `sysinfo`; memory comes from `sysmem`, the same source
  the running model's screen uses.
- The GPU card shows memory: what the running launch asks of the Metal
  working set against what that set will hand out.
- Polled, not pushed: figures update only while the screen is open.
- The measurement server is excluded from the stray-server scan.

### Settings

- Change opens a real folder picker (`tauri-plugin-dialog`), not a typed
  path.
- The binary's facts fold to one line: found or chosen, the version, and
  either full support or the name of a missing flag.
- Launch defaults fold behind "Edit defaults".
- Appearance keeps its own card.

## Corrections to the mockups

- The mockup's caption that downloads survive quitting the app is not
  accurate: a relaunch restores them Paused, with Resume available
  ([downloader.md](downloader.md)).
- The mockup's caption that the app watches the models directory is not
  accurate: nothing watches it, which is why Rescan exists.

## Verified

Verified 2026-09-03: all four checks passed after every unit, and the
author reviewed each screen on the running app. The folder picker was
confirmed separately on the signed bundle on 2026-09-04.
