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
2. **Launch.** Built 2026-09-02, **not done until the author's look** — the
   proof section says why. The three presets over the machinery that already
   exists:
   Default is the launch defaults, Best speed is the measured `speeds.json`
   row, Model suggested is the offered-not-applied opinion from fitting. A
   preset whose backing fact is missing renders disabled with the action that
   would create it (Measure, for Best speed). Context and port visible;
   ProfileForm survives intact behind Advanced, because Settings' launch
   defaults share it.
3. **Running view.** Built 2026-09-02, look owed. The four cards, the Test
   results row, the address line with copy. The pi button moves and must not
   change: its 22 tests are the guard ([pi.md](pi.md)).
4. **Downloads, Settings, first run, Activity.** Restyles for the first two;
   the starter list (frontend-hardcoded, models proposed at this phase's
   planning) for the third; the CPU% telemetry field, its tests, and the
   Activity screen for the fourth.

## What still does not match, screen by screen

Written 2026-09-02 from the author's own list after six passes, so the next
session does not rediscover it. **One screen per task**, each finished by
putting the artboard and the app side by side — the method is in
[knowledge/technical.md](../knowledge/technical.md), and not using it is what
made the first four passes worthless.

1. **Stopped model screen** — the artboard has exactly two rows, Advanced and
   Full command. The app still shows Speed, Model details and Logs there. The
   author has ruled on the last one: **no log on a model that is not
   running.** Decide where the facts and the measurement history go before
   deleting their rows.
2. **Library rows** — the artboard's row is a dot, the name, its badges, then
   one stat sentence and the action. The app still shows the file name under
   the title and three separate columns, and the running row carries no
   **Use in pi**.
3. **The stray-server banner** — the artboard is one line: the sentence, then
   Stop it and Ignore at the right. The app stacks a bullet list under a
   two-line heading.
4. **Measure** — the artboard gives it a **whole screen** ("Measuring best
   speed", the four tries with their verdicts, Cancel, Use fastest so far).
   The app runs it inside the Speed row, which is why the author says the
   measure screen does not match: it was never built.
5. **Empty Library** — the artboard offers starter models sized to the Mac.
   The app prints "Models directory not found". This one is phase 4 and is
   not owed yet.

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

### Phase 2 — built 2026-09-02, look owed

Files: `src/Presets.tsx` (new), `src/ModelDetail.tsx`, `src/ProfileForm.tsx`,
`src/App.css`. The model screen's Launch settings section became "How should
it run?": three preset cards over `Presets.tsx`, then ProfileForm, whose
visible grid is now Port and Context with Alias moved behind Advanced —
Settings' launch defaults share the same form and follow. Sections reordered
for a stopped model: launch, memory, speed, model, context, command, logs.

Decisions made at the keyboard:

- **A preset owns six fields** — ctx, ngl, parallel, both cache types, flash
  attention — and never alias, port, jinja or extra arguments, which are the
  user's. Selection is derived by comparing those six against the form.
- **The built-in default is already the fit** (`ctx` Auto, `ngl` auto), so
  Default and Model suggested coincide until the author saves custom
  defaults. One highlight, by rank: Best speed, then Model suggested, then
  Default. No preset matching shows "Custom settings" under the cards.
- **Best speed disabled offers Measure in place** — the same `tuneStart` the
  Speed panel runs, same blocked reason. An "observed, not measured"
  suggestion enables the card but says what it is.
- **`SpeedConfidence` is** `neverMeasured | observed | tuned` — the first
  build guessed `"measured"` and tsc caught it.

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

**Why not done**: the Library was captured rendering correctly after the
change, but the presets live on the model screen, behind a click. Driving
that click from the session required taking window focus from the author,
who was using the machine; after one stray click landed in their browser
the attempt was abandoned rather than repeated. The author's look owes:
the three cards and their selection following the form, Measure on a
disabled Best speed, Alias sitting under Advanced, and the reordered
sections.

**Found on the way, unrelated to the phase**: the unusable-window bug
reproduced — see the roadmap's risk item, updated with the observation.

**Reworked the same day on the author's screenshot** — "model screen does not
match designs", and it did not: the presets had been dropped into the old
screen, which still opened every panel at once. Second pass, matching the
mockup's shape: the header carries badge chips (quant, parameters, context,
size, MoE, no-template) instead of the file path; Memory, Speed, Model
details, Full command and Logs became folded disclosure rows, each with a
one-line summary (`launchCost` feeds both the Memory row's line and the bar
inside, so they cannot disagree); the Context panel is gone, its three facts
now living in the header badge, the context field and the Memory row;
TunePanel lost its own panel chrome and lives inside the Speed row; "reveal
in Finder" moved into Model details (it had also been sitting on a CSS class
phase 1 deleted). The context field label reads "How much it can hold".
Statuses after the rework, each captured on its own line:

```text
build: 0
cargo test status: 0        (256 passed, 5 ignored)
clippy status: 0
fmt status: 0
```

**Third pass, on "still does not match"** — and the second pass had not
matched either. Both earlier passes kept the old screen's shape and added to
it; the mockup's launch screen is six things and nothing else: header with
badge chips and Run, "How should it run?" over three preset cards, two
fields, a one-line memory verdict, then Advanced and Full command. What that
cost:

- **The panel wrapper is gone.** No uppercase `<h2>` heading over the
  presets — a sentence-case group label, as drawn.
- **The memory panel became a sentence**: a coloured dot and
  "Memory ≥ 1.3 GB of 25.0 GB · fits", which opens the existing bar and four
  figures. `launchCost` computes it once for both, so the line and the panel
  under it cannot disagree — the failure [screen.md](screen.md) closed.
- **Context is a named dropdown, not a slider.** "Fitted to memory —
  recommended", then the model's maximum and halves of it down to 4,096, with
  the hint saying what a number is worth in pages. This is
  [direction.md](direction.md)'s "named choices" applied to the field itself:
  21 launches only ever chose round numbers, never anything between them.
- **`ProfileForm` split into `ProfileFields` and `AdvancedFields`**, so the
  model screen can put the advanced seven in a page-level disclosure row
  while Settings keeps composing both with its own fold.
- **Model details, Speed and Logs are rows after the two the mockup draws**,
  because the facts have to stay reachable; Speed appears only once something
  has been measured and Logs only once something has been printed, so the
  common screen is the mockup's.

**Verified on screen this time.** The two earlier passes were handed over
unseen and both were wrong, so the third was captured before reporting: the
webview forgets the open model on hot reload, the app could not be focused
(System Events refuses, `-25208`) and its webview exposes no accessibility
tree, so a three-line temporary auto-select was patched into `App.tsx`, the
window captured by id, and the patch reverted — `git status` confirms
`App.tsx` unmodified, and the four commands were re-run green afterwards.
The capture shows the shape above.

**Fourth pass, on "header bar, spacing, icons"** — the third had the right
shape and none of the mockup's craft. The artboard's stylesheet was read
against `App.css` line by line, and the app had never been given it:

- **The design tokens were the mockup's, and now are.** Dark
  `--bg #1e1e20`, `--bg-sidebar #26262a`, `--bg-card #252529`, a new
  `--bg-card2` for controls, `--input`, and a `--faint` distinct from
  `--text-muted` for the sidebar's section labels. Light values follow the
  same set.
- **`.screen-header` is a bar**, not a heading block: one row 62px tall,
  full-bleed with negative margins, a rule under it, the crumb inline before
  a 16px title where a 20px one sat above a path. Every screen gets it.
- **The content is held to 1040px.** A preset card stretched across a 1700px
  window is empty space with a word in it, which is what the author's
  screenshot showed.
- **Icons where the mockup draws them**: SVG chevrons on every disclosure
  (rotating on open) and on the crumb, in place of `▸`/`←` characters, and a
  play glyph on a larger Run.
- **The form controls stopped being the system's**: 8px radius, the `--input`
  ground, a drawn chevron on selects and no spinner on numbers. A macOS
  select painted in its own colours on a dark card is the "foreign object"
  the author was seeing.
- Buttons to the mockup's metrics (7px radius, `--bg-card2`), preset cards to
  its 14px/11px, the sidebar to 216px.

Captured again by the same route and checked against the artboard. The
window came up 166×164 on the way — **the unusable-window bug, twice in one
session now**, both recorded on the roadmap.

**Fifth pass, on "measure again is missing, still not correct, can't you
screenshot and verify with mock"** — and the answer to the question is yes,
which is the finding that matters. Four passes had been checked against
*memory of the code that generated the mockup*, never against the mockup
rendered. Doing that took one command and found in a minute what four
rounds of the author's time had not:

- **Measure again was missing entirely.** The card only offered Measure when
  nothing had been measured; the artboard offers "Measured here on Aug 30 ·
  Measure again" whether or not it has a figure. It now does, as a link on
  one line inside the card.
- Two preset descriptions were **rewritten prose, not the approved copy**:
  Default is "Safe settings that work on any Mac.", Model suggested is
  "What the model file itself recommends."
- The memory line was **bold, missing its em dash, and never said
  "comfortably"** — a verdict the artboard has and the code did not.
- The Advanced row says **"— using <preset> values"**, which needs the
  selected preset; `selectedPreset` and `presetName` moved out of the cards
  so the row and the highlight cannot name different presets.
- Row subtitles and field hints matched neither the artboard's wording nor
  its sentence case.

**How to compare, so this is never guessed at again** — recorded in
[knowledge/technical.md](../knowledge/technical.md) under Verify.

**One deliberate deviation, for the author to overturn**: the artboard's
launch screen has two rows, Advanced and Full command. The app has three
more — Model details, Speed and Logs — because they are the only way to
reach the file's diagnostics, the measurement history
([tune.md](tune.md)) and a crash's output. Dropping them to match the
sketch would delete working features, so they stay until the author says
otherwise.

**Sixth pass, on "content does not adapt with resizing window"** — the
1040px column added in the fourth pass was wrong. It was reasoned from a
mockup 1180px wide, where it never binds; on the author's 1700px window it
left a third of the pane empty. Removed: the content fills whatever width
the window has, which is what the app it is modelled on does. Proved at
1700×1000 by widening `tauri.conf.json` for the capture and putting it
back. The stray-server banner also rendered above the title bar, because
`App` puts it before the screen's own fragment; `.screen-header` now takes
`order: -1`.

### Phase 3 — built 2026-09-02, look owed

Prompted early by the author — "running also does not match design" — while
phase 2 was still open, so the two are one commit.

Files: `src/ModelDetail.tsx`, `src/App.css`. The running model's screen was
the old `Running` panel with a KV bar and eleven figures in a grid; it is
now the artboard's shape:

- **Four cards**: Memory (what the model uses, with the Mac's own use and
  pressure under it), Speed (writing speed, with prompt-reading speed
  beside it), Context (percent full, with the token counts the server
  reported), Health (Ready / Starting / Not answering / Problem, with the
  check tally once anything has been tested).
- **The address line**: "Other apps reach this model at
  `http://localhost:<port>/v1`" with Copy — the thing a person actually
  needs from a running server, which the old screen never printed.
- **A Details group** of folded rows: Test results (carrying a **Test
  again** button in its summary, so testing no longer needs the panel
  open), Live details (the old telemetry grid and its sparkline, kept
  rather than deleted), Launch settings (the presets and fields, folded
  away because a running model has already answered the question), then
  Full command, Model details, Speed and Logs as on the stopped screen.
- **The header** gains a Running pill and the artboard's actions: Open
  chat, Reload, Use in pi as the primary, Stop.

```text
build: 0
cargo test status: 0        (256 passed, 5 ignored — unchanged)
clippy status: 0
fmt status: 0
```

Both screens were captured at width and checked; the running one only after
`osascript -e 'tell application "System Events" to set visible of process
"llamaport" to true'`, which is what makes a window reporting `on=false`
capturable — recorded with the rest of the method in
[knowledge/technical.md](../knowledge/technical.md).

### Phase 3, second pass — the author's list, 2026-09-02

Four corrections, all of them removals but one:

- **A running model shows no launch machinery.** Speed, Full command,
  Advanced and Launch settings are gone from that state; what is left is
  Test results, Live details, Model details and Logs. Choosing settings is
  the stopped screen's job, and the running screen was carrying a second
  copy of it.
- **No back button and no Library crumb.** The sidebar is how you leave a
  model, so `onBack` is gone from `ModelDetail` and from `App`.
- **Logs no longer unroll themselves on a launch that works** — only a crash
  opens them. A thousand lines under four calm figures undoes the figures.
- **Default and Model suggested lit the wrong card.** The author found it:
  pressing Default highlighted Model suggested. Both are true — the
  built-in launch defaults *are* `ctx: auto, ngl: auto`, which is what the
  fit does — and the highlight was derived from the values alone, so the
  ranking picked the fit. Two changes: the pressed card is remembered and
  wins over the derived answer until any other edit clears it
  (`picked` in `ModelDetail`, `Which` exported from `Presets`), and the
  Default card now says why in its own hint — "The same as Model suggested,
  until you save your own in Settings".

```text
build: 0
cargo test status: 0        (256 passed, 5 ignored)
clippy status: 0
fmt status: 0
```

Captured after a clean restart, which is also what proved the Logs change:
React had kept the old `showLogs` across the hot reload, so the first
capture still showed them open.
