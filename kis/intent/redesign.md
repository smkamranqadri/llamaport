# Redesign

Planned 2026-09-02, the second phase set cut from [direction.md](direction.md).
The author's words that opened it: "I don't like the ui, not user friendly
specially someone who don't know all the technical stuff." The reference the
author pointed at is OrbStack; the screens were interviewed, mocked, revised
once on feedback, and approved the same day. The approved mockups are the spec:
https://claude.ai/code/artifact/f92717ab-9ddc-4ae7-8fb2-58b13137d257 — eleven
artboards, including the edge states (stray llama-server banner, empty first
run) and one light-mode variant.

Live status is in [state/current.md](../state/current.md), not here.

## What was decided in the interview

- **Layout is sidebar plus one content pane**, not OrbStack's three zones. The
  author chose it over the three-zone option seeing both drawn.
- **Sidebar**: Library, Discover, Downloads, Activity Monitor; Settings at the
  bottom. **Discover ships disabled** — the author chose a grayed "coming soon"
  entry over leaving it out, so the layout is final from day one. Its screen is
  designed (artboard "Discover — build later") and not built here.
- **pi is a button on the running model, not a nav entry.**
- **The launch form shrinks behind three named presets** — Default / Best speed
  / Model suggested — with context and port still visible and everything else
  behind an Advanced disclosure. This is [direction.md](direction.md)'s
  "named choices" remainder, finally planned.
- **The running view is four cards** — Memory, Speed, Context, Health — in
  plain language, each technical figure paired with a gloss ("26% full",
  "roughly 24 pages of text"). A Test results row carries the health checks
  with a Test again action.
- **First run offers starter models sized to the machine's memory**, with
  paste-a-URL still available.
- **Theme follows macOS**, both appearances, as the app already does.
- **Activity Monitor ships with a CPU% column and no GPU column.** The choice
  was the author's, made knowing the telemetry has neither today: CPU comes
  from the `proc_pid_rusage` call `sysmem.rs` already makes for the footprint;
  per-process GPU has no public macOS API, so that column from the mockup is
  dropped for good rather than deferred.
- **The README screenshots stay deferred behind this work.** They were Next,
  they are a release behind, and every phase here would invalidate them again.
  They are done once, against the finished redesign
  ([release.md](release.md) phase 3).

## Phases

Four, in order, with the author using the built app between each — the
project's own tally says that is where defects come from
([knowledge/technical.md](../knowledge/technical.md)).

1. ~~**Shell and Library.**~~ Done 2026-09-02. The new sidebar (Discover
   disabled), the visual system (tokens, rows, buttons, cards), the Library
   grouped Running / Stopped with inline Run and Stop, the orphan banner
   restyled. No behavior changes. Four resolutions made at the keyboard:
   **Activity Monitor ships disabled too** — its screen is phase 4, and the
   author chose disabled entries over absent ones; **the sidebar's Now
   Running strip is gone**, replaced by the mockup's green dot on Library,
   so Stop lives on the running row and the model screen; **the row's Run
   sends no draft**, so `runner_start` resolves the same remembered profile
   the model's own screen opens with — no new machinery; **the native title
   bar stays** — the mockup's traffic lights are the real ones.
2. **Launch.** The three presets over the machinery that already exists:
   Default is the launch defaults, Best speed is the measured `speeds.json`
   row, Model suggested is the offered-not-applied opinion from fitting. A
   preset whose backing fact is missing renders disabled with the action that
   would create it (Measure, for Best speed). Context and port visible;
   ProfileForm survives intact behind Advanced, because Settings' launch
   defaults share it.
3. **Running view.** The four cards, the Test results row, the address line
   with copy. The pi button moves and must not change: its 22 tests are the
   guard ([pi.md](pi.md)).
4. **Downloads, Settings, first run, Activity.** Restyles for the first two;
   the starter list (frontend-hardcoded, models proposed at this phase's
   planning) for the third; the CPU% telemetry field, its tests, and the
   Activity screen for the fourth.

## Risks named at planning

- The pi button regressing while its screen is rebuilt under it.
- ProfileForm's shared use by Settings.
- No frontend suite exists: the phases are proved by the four commands green
  plus the author's eyes on the built app, which is this project's normal.

## Proof

### Phase 1 — 2026-09-02

Files: `src/App.tsx`, `src/Library.tsx`, `src/App.css`, `src/icons.tsx` (new).
The Library's list class became `.model-cards` so Downloads, which shares
`.model-list`, keeps its current look until its own phase.

```text
bun run build
build: 0
cargo test --manifest-path src-tauri/Cargo.toml
cargo test status: 0        (256 passed, 5 ignored — unchanged)
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
clippy status: 0
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
fmt status: 0
```

The built app was looked at, not only compiled: the dev app's window was
captured by window id with nothing running — sidebar sections, disabled
Discover and Activity entries, Settings at the bottom, card rows with
badges and a visible Run each, no group headers when nothing runs.

**Owed to the author's look before phase 2**: pressing Run from a row and
watching the Running group form with its tint and Stop; the orphan banner
(none existed to photograph); Ignore dismissing it.
