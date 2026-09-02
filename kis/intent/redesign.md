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
2. ~~**Launch.**~~ Done 2026-09-02. The look it owed was closed by item 1's
   second pass, which is the same screen. The three presets over the machinery
   that already exists:
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
   the CPU% telemetry field, its tests, and the Activity screen for the last.
   **The first run left this phase on 2026-09-02** — the author asked for it
   ahead of the rest and it is done; its proof is below. **Done 2026-09-03**:
   Downloads (`21cec86`, `e85ecae`), Activity Monitor (`e897b2a`) and Settings
   (`9e16cb5`), each proved below. **The phase is closed, and the redesign with
   it.**

## What still does not match, screen by screen

Written 2026-09-02 from the author's own list after six passes, so the next
session does not rediscover it. **One screen per task**, each finished by
putting the artboard and the app side by side — the method is in
[knowledge/technical.md](../knowledge/technical.md), and not using it is what
made the first four passes worthless.

1. ~~**Stopped model screen**~~ — done 2026-09-02, the author signing off on
   screen after three rounds of his own corrections. The artboard has exactly
   two rows, Advanced and Full command; the app also showed Speed, Model
   details and Logs. The three questions that had to be answered before
   deleting anything were put to the author and are answered — proof below.
2. ~~**Library rows**~~ — done 2026-09-02, the author signing off on screen
   after two rounds of his own corrections. Proof below.
3. ~~**The stray-server banner**~~ — done 2026-09-02, and it was hiding a
   defect rather than only a layout. Proof below.
4. ~~**Measure**~~ — done 2026-09-02, the author signing off after one round of
   his own corrections. **Built in the model screen rather than as its own
   screen**, on the author's ruling: "I don't mind to do in current screen
   instead of new screen but should be same design". Proof below.
5. ~~**Empty Library**~~ — done 2026-09-02, pulled forward out of phase 4 by
   the author, who put the artboard and the app side by side and said to fix
   this one first. Proof below.

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

### Phase 2 — built 2026-09-02, look closed by item 1's second pass

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

### First run — empty Library, done 2026-09-02

Pulled out of phase 4 by the author. Files: `src/FirstRun.tsx` (new),
`src/Library.tsx`, `src/icons.tsx`, `src/api.ts`, `src/types.ts`,
`src/App.css`, `src-tauri/src/lib.rs`.

The screen the app printed was **"Models directory not found"** over a path.
It is now the artboard's: a cube, "Get your first model", three starter cards
with a badge, a sentence and a size, and a paste-a-link row. `Library`'s
header subtitle reads "No models yet" while the list is empty.

Decisions made at the keyboard, all four with the author:

- **The starters are three fixed Qwen models**, named by the author:
  `Qwen3.5-2B-Q4_K_M` (1.3 GB), `Qwen3.6-35B-A3B-UD-Q4_K_M` (22.1 GB) and
  `Qwen3.8-27B-UD-Q4_K_M` (16.5 GB), all from `unsloth`. Every URL and byte
  count was **verified live by HTTP HEAD** rather than estimated, so the size
  beside a card is the size that lands. An earlier pass offered Llama 3.2 3B,
  Qwen 2.5 7B and Phi-4 and filtered the list by memory; the author replaced
  both the models and the filtering.
- **The ceiling is the Metal working set, not installed memory.** A new
  `machine_memory` command carries `device_budget_bytes` off the same
  `capabilities()` the launch plan uses — it discovers `llama-server` with no
  model named, so `--list-devices` is reachable on a first run. This machine
  reads 25.0 GB where installed memory says 32.0 GB. Where the binary has not
  been found the screen falls back to installed memory **and says so in the
  sentence**, because falling back silently is the defect
  [screen.md](screen.md) closed.
- **Four bands, because three lied.** ≤50% of the ceiling "fits easily", ≤75%
  "fits", ≤100% "tight — little room for a conversation", above it "too big for
  this Mac". The first build said "too big" at 75%, which is false: 22.1 GB
  under a 25.0 GB ceiling loads and leaves nothing for the cache. A memory sum
  says a launch is allowed and never that it is good
  ([knowledge/technical.md](../knowledge/technical.md)). Nothing is filtered
  out; the wording carries the judgement and every card stays downloadable.
- **`download_start` creates the models directory.** The author's is
  `/Users/mkamran/models1` and does not exist; admission, the `.part` and the
  room check all assume a directory to write in, so every Download button would
  have failed on the machine this screen was built for.

**Two deliberate deviations from the artboard**, both offered to the author and
kept: Rescan stays in the header, because this is the one screen where the
folder may be missing and Rescan is the recovery after creating it; and one
quiet line names a models directory that does not exist yet, which the artboard
does not cover.

**Found on the way and written up rather than fixed**: the app can only build a
fully offloaded launch, so a 35B-A3B costs its whole 22.1 GB while doing 3B of
arithmetic per token. [moe.md](moe.md).

```text
build: 0
cargo test status: 0        (256 passed, 5 ignored — unchanged)
clippy status: 0
fmt status: 0
```

**Verified on screen and by the author.** The artboard was rendered out of the
canvas artifact and screenshotted headless, the app window captured by id, and
the two put side by side — the method in
[knowledge/technical.md](../knowledge/technical.md), used from the start this
time rather than after four wasted passes. The author's word on it: "First run
— empty Library is done."

### Item 1 — stopped model screen, built 2026-09-02

Files: `src/ModelDetail.tsx` alone. The three rows the artboard does not draw
were not deleted on sight: where their content goes was put to the author
first, because two of the three are the only route to something real.

- **Model details stays as a third row.** The author overturned the artboard
  rather than lose the file's facts — architecture, layers, KV heads, chat
  template, Reveal in Finder — on the one screen you read before running a
  model. The alternatives offered were an info button in the header opening a
  panel, and dropping the row to the running screen where phase 3 already put
  it.
- **Speed appears only while a measurement runs.** A stopped model's history is
  the one line the Best speed card already carries — "Measured here on Aug 31 ·
  Measure again" — and the ladder's tries belong to the Measure screen that
  item 4 builds. The row's summary is now the artboard's own wording, `2 of 4
  tries done`, off `TuneReport`.
- **Logs survive a crash.** The author's ruling was "no log on a model that is
  not running", and a crashed model is not running — which would have deleted
  the log at the one moment it is the only answer. Put back to them and
  settled: the row shows while running and after a crash, and is gone from a
  calm stopped screen.

```text
build: 0
cargo test status: 0        (256 passed, 5 ignored — unchanged)
clippy status: 0
fmt status: 0
```

**Why not done at the time**: committed without the author's eyes on it. The
side-by-side was set up — the Launch artboard rendered out of the canvas
artifact and screenshotted — but the app's window was hidden in the tray and
offscreen, and the one capture that landed was half-occluded. It shows three
disclosure rows where there were five, which is the change, and is not a look.

### Item 1, second pass — the look, 2026-09-02

Files: `src/App.tsx`, `src/App.css`. The artboard was rendered and put beside
the app before anything was changed, which found one defect and six metric
drifts.

**The defect: the sidebar went blank on a model screen.** Phase 3 removed the
`‹ Library` crumb on the ruling that the sidebar is how you leave a model — but
`active` was gated on `&& !selected`, so opening a model unlit Library and left
nothing on screen saying where you were. The artboard draws Library lit. The
guard is gone from all three entries; `selected` only ever coexists with the
library screen, so it was dead on the other two.

Six metrics read off the artboard's own stylesheet rather than from memory:
`.field-hint` to `--faint` at 11.5px, so a hint sits a step below the label
above it; the memory verdict to 12px; `.d-title` 600 → 550; `.radio` 15 → 16px;
`.badges-row` 6 → 5px.

Two spacing failures the drawing does not cover, both found by the author:

- **`.preset-foot` had `margin-top: auto`**, pinning "Measured here on Sep 2 ·
  Measure again" to the card's floor. Harmless on the artboard, where every card
  is one line; on the app it opened a hole under Best speed's description as
  soon as Default's own hint made the row two lines tall.
- **The flat row's rhythm belonged on the row, not its summary.** Padding under
  a summary lands *between the sentence and the figures* the moment the row is
  opened, so an opened memory panel sat 13px from Advanced. It is now a margin
  on `.disclosure.is-flat`, with the body's own padding above it.

**The author overruled the artboard on the gaps around the memory line**, which
draws them tighter than he wanted: 22px above, 24px below, against the drawing's
15 and 16.

**One deviation confirmed rather than reopened**: the memory line keeps a
chevron the artboard does not draw, because the four figures behind it — GPU
limit, free, swap, installed — have no other route on this screen. Same argument
that kept Model details as a third row.

```text
build: 0
cargo test status: 0        (257 passed, 5 ignored — unchanged)
clippy status: 0
fmt status: 0
```

**Verified on screen by the author, who took the captures himself** — the
session's own attempts were disturbing a machine he was working on, and he said
so. That is the cheaper route whenever the author is at the keyboard: no window
stealing, and his eye is the acceptance test anyway.

### Item 2 — Library rows, done 2026-09-02

Files: `src/Library.tsx`, `src/App.tsx`, `src/App.css`, `src/icons.tsx`.

Three columns and a file name became the artboard's one line: a dot, the name,
its badges, one stat sentence, the action. `rowStat` is that sentence — a
running model says `1.7 GB memory · 285 tok/s` off telemetry the Library now
receives, a stopped one `21.2 GB · ran yesterday` or `1.4 GB · added 12 days
ago, never run`. A broken file replaces the sentence rather than adding a line
under it.

Decisions, all put to the author before anything was deleted:

- **The star and Delete wait for a hover.** The artboard draws neither, and the
  Library row is the only place either exists in the whole app — the model
  screen has no Delete and no favourites. Dropping them to match would have
  deleted two features. A favourited model keeps its star at rest, because a
  mark you cannot see is not a mark, and the hidden star still holds its width
  so the stat column does not jump between rows.
- **The header took the counts and the search, not the Download model button.**
  The author asked for all three, then removed the button on seeing it. Search
  filters on display name *and* file name, since the row no longer shows the
  file and that is often the half you remember. Rescan stays: nothing watches
  the models directory, so it is the only manual refresh.
- **Use in pi** is on the running row, opening the same `PiPanel` the model
  screen uses. Its mark was a drawn π, because `~/.pi` and `~/.superconductor`
  hold no brand asset. **Corrected 2026-09-02 on the author's "add actual pi
  icon"**: the installed package names the real one in its README —
  `https://pi.dev/logo-auto.svg`, a filled blocky P with the i's square, not a
  Greek letter at all. `PiIcon` is that artwork, cropped to its own bounds so it
  carries the weight of the stroked icons beside it, and the model screen's
  header button now takes it too. The search that found nothing looked in the
  data directories and never in the package that installs the command:
  `~/.local/lib/node_modules/@earendil-works/pi-coding-agent`.

Two corrections from the author on the first capture, both his eye and not the
suite's: the Download model button, and **a row 14px shorter than the drawing**.
The second had a cause worth keeping — the padding sat on the row *button*,
while the action buttons are its siblings on the item, so the tallest thing in
the row set no height at all. The padding moved to `.model-item`.

A third correction came after: **with nothing running there is no Running or
Stopped label, and the label was the only thing supplying the gap above the
list**, so the first row sat flush against whatever was above it.
`.model-cards` now carries its own top margin, cancelled by
`.group-label + .model-cards`.

### Item 3 — the stray-server banner, done 2026-09-02

Files: `src/App.tsx`, `src/App.css`, `src-tauri/src/runner.rs`.

The banner was a two-line heading over a bullet list. It is now the artboard's
one line per stray server: the sentence, the facts, then Stop it and Ignore at
the right.

**It was hiding a defect the redesign only exposed.** The line reads
`<model> · port <port> · probably left over…`, and the app had nothing to put
in either slot — every orphan it had ever reported said *unknown model* on an
unknown port. `parse_server_command` was correct and so were its tests;
`detect_orphans` called `sysinfo`'s plain `refresh_processes`, which leaves
`cmd()` empty, so the parser was never given anything to parse. Proved with a
throwaway test printing `cmd=[]` for a live `llama-server`, then fixed with
`refresh_processes_specifics` and `ProcessRefreshKind::everything()`. Recorded
as a constraint in [knowledge/technical.md](../knowledge/technical.md), and the
defect tally there moves to twenty.

**The alias now wins over the file name.** It is what the artboard draws
(`qwen3.5-2b`, not `Qwen_Qwen3.5-2B-Q4_K_M.gguf`), it is the id a client
addresses, and the file name was long enough to wrap the banner it sat in. One
new test, mutation-checked: gutting the preference to `let model = from_path;`
fails it with `left: Some("Qwen_Qwen3.5-2B-Q4_K_M")`.

```text
build: 0
cargo test status: 0        (257 passed, 5 ignored — one new)
clippy status: 0
fmt status: 0
```

**Verified on screen for both items**, the artboards rendered out of the canvas
artifact and the app captured by window id, with the author confirming each.
The running row needed a model actually running, so one was started for the
capture and stopped afterwards.

**Cost recorded rather than hidden**: editing `src-tauri/src/lib.rs` makes
`tauri dev` rebuild and restart, which stops whatever model is running. It
stopped the author's twice in this session. And **the unusable-window bug fired
four more times**, three on consecutive launches, which is an escalation the
roadmap now carries.

### Item 4 — Measure, done 2026-09-02

Files: `src/TunePanel.tsx`, `src/ModelDetail.tsx`, `src/Disclosure.tsx` (new),
`src/App.css`, `src/icons.tsx`, `src/types.ts`, `src-tauri/src/tune.rs`.
Commits `295ea04` and `02b1c1a`.

The artboard's whole screen is drawn inside the Speed row instead, which the
author allowed and which cost nothing: what the drawing is, is a list of tries
and one button, and the row already had a header with a place for Cancel.

Four decisions, all put to the author before anything was written:

- **The history stays, folded.** The artboard draws only the tries; `speeds.json`
  has no other route in the app, so "What this model has done" is a folded row
  under the list. Same argument that kept Model details and the memory chevron.
- **A try is named in words with its arguments beside it** — `16k context ·
  full precision`, then `16,384 · f16 — the largest that fits`. The words are
  what the person choosing reads and the arguments are what anyone checking a
  launch needs, so neither replaces the other.
- **The caveats fold.** One verdict line stays; the 10% tie rule, the
  observed-not-measured warning and the shared-prompt note went behind "Why
  this one".
- **The row folds itself when the ladder ends, and applies the answer** — the
  author's correction on the first look: it had been pinned open, leaving four
  tries and their prose on screen while the one preset they were measured for
  sat unchanged above. Closing now selects Best speed, and Use these settings
  closes the row too.

Two things the drawing could not be built from:

- **The report never named the tries it had not run yet**, so a waiting row was
  not expressible. `tune::Report` now carries `candidates` from the moment the
  ladder starts. A list that grows from nothing cannot say how far it has to go.
- **The marked row is the one the button applies, not always the quickest
  reading.** Once the ladder is over that is the suggestion, which prefers the
  widest context among readings too close to separate. Marking the fastest while
  the sentence underneath named another setting is the screen contradicting
  itself, which is what the mock render showed.

**Found on the way**: `.tune-head`, `.tune-note` and `.tune-row.is-unranked`
were painted with `--muted`, and `.tune-row.is-fastest` with
`--surface-raised` — neither has ever been a token in this stylesheet, so three
greys rendered as body text and a highlight as nothing. Recorded in the defect
tally ([knowledge/technical.md](../knowledge/technical.md)).

```text
build: 0
cargo test status: 0        (257 passed, 5 ignored — unchanged)
clippy status: 0
fmt status: 0
```

**Verified by the author, and by rendering the new markup rather than the app.**
The app is his and stays his ([knowledge/technical.md](../knowledge/technical.md)
under Verify): the panel's own DOM was rendered against `App.css` in headless
Chrome, in both appearances and in all three states — running, finished, folded
— and put beside the artboard. His word on it: "all good".

### Phase 4, Downloads — done 2026-09-03

Files: `src/Downloads.tsx`, `src/App.css`. Two panels — Fetch a model, Speed
limit — and a History table became the artboard: a paste field and Get in the
header, one card per transfer, then Finished rows with a tick, a name and Show.

Decisions, the first two the author's:

- **A finished row is named by the catalog**, which reads the GGUF's own name,
  so the Library and this screen cannot call the same file different things. A
  row still in flight has no GGUF to read, so its file name stands in and its
  quant badge is parsed by a TypeScript copy of `catalog.rs`'s rule.
- **The URL survives only on a failure.** It is the one moment somebody needs to
  read the address they asked for; elsewhere it is the row's tooltip.
- **The speed limit is a named choice on the status line**, not a panel: No
  speed limit, then 0.5, 1, 1.5 and 2 MB/s — the author's ladder — and
  "Something else…", which opens a field. Past two megabytes the useful figure
  depends on a line nobody here has seen, so there is no rung worth guessing.
  This is [direction.md](direction.md)'s named choices applied to the last
  free-typed number on any screen. The engine's 64 KB/s floor is named beside
  the field, because typing is the only way under it.
- **A third group, "Did not finish."** The artboard draws Downloading and
  Finished; a failure is neither, and filing it under Finished would be the
  screen lying about what happened.
- **Clear sits below both lists.** `downloads::clear` removes complete *and*
  failed rows, so a Clear under the Finished group would have said otherwise.

**The artboard's own caption was false, and matching it would have shipped the
lie.** It reads "Downloads survive quitting the app — they pick up where they
left off"; [downloader.md](downloader.md)'s verification records that a
relaunch restores them as **Paused** with Resume live. The line now says so, and
the rule is in [knowledge/technical.md](../knowledge/technical.md).

Rules the rewrite orphaned went with it: `.download-row`, `.download-progress`,
`.panel-head`, `.badge-quiet`, and the verifying tint on `.kv-bar`.

```text
build: 0
cargo test status: 0        (258 passed, 5 ignored — unchanged)
clippy status: 0
fmt status: 0
```

Rendered against `App.css` in both appearances before hand-over, including the
failure state the artboard does not draw. **The author's eyes are still owed a
running transfer**: the bar and the figures line are the two things a render
cannot prove.

### Phase 4, Activity Monitor — done 2026-09-03

Files: `src-tauri/src/activity.rs` (new), `src-tauri/src/lib.rs`,
`src/Activity.tsx` (new), `src/App.tsx`, `src/api.ts`, `src/types.ts`,
`src/App.css`. A disabled sidebar entry became a screen.

- **Every `llama-server` the app knows**, the author's call over "only the
  running model": the model, the server a measurement launches on its own port,
  and any stray the orphan scan found, each row saying which it is.
- **A CPU column and no GPU column**, as the interview decided. Per-process GPU
  has no public macOS API and neither does overall utilisation, so the
  artboard's GPU column and its `38%` card are dropped for good.
- **The GPU card carries memory instead** — the author's pick of three: what the
  running launch asks of the Metal working set against what that set will hand
  out, both already read for the launch plan.
- **CPU comes from `sysinfo`, not `proc_pid_rusage`** — a deviation from the
  interview's implementation note, recorded rather than quietly taken. A
  percentage is a difference between two samples, so `activity::Monitor` holds
  one `System` between polls and that one sample answers per-process and
  machine-wide alike; `sysinfo` is already this app's process source for the
  orphan scan. Memory stays on `sysmem::process_footprint_bytes`, so this table
  and the running model's own screen cannot disagree about the same number.
- **Polled, not pushed.** The screen is read only while open, and the interval
  it chooses is the window every CPU figure is averaged over.

**A defect the tests caught before the author could.** The orphan scan is told
to skip the runner's child and knows nothing about Tune's, so a measurement in
progress was about to be listed as somebody's stray server — the stray banner's
own defect, one layer up. Excluded in `known_processes`, with three tests and
the exclusion mutation-checked: dropping the `continue` fails it with
`left: [42, 99]`.

```text
build: 0
cargo test status: 0        (261 passed, 5 ignored — three new)
clippy status: 0
fmt status: 0
```

Two deviations from the drawing, both deliberate: the cards sit under the table
rather than pinned to the window's bottom edge, so they follow the window
instead of floating over it; and a stray row is marked amber, because it is the
one thing on that screen this app did not start.

### Phase 4, Settings — done 2026-09-03, and the redesign closes

Files: `src/SettingsScreen.tsx`, `src/App.css`, `src-tauri/Cargo.toml`,
`src-tauri/src/lib.rs`, `src-tauri/capabilities/default.json`, `package.json`.
Four panels became four cards with a ground of their own and sentence-case
titles; `.panel` is Settings' alone, so the change reaches nothing else.

- **Change… opens the real folder picker**, the author's call over keeping a
  typed path: `tauri-plugin-dialog` is a new dependency in a signed app, and a
  "…" button that opens nothing is a promise the screen does not keep. The field
  is read-only and shows what was chosen. The binary uses the same picker, and
  "Find it for me" clears the choice back to the automatic search.
- **The binary's six facts become one line**, the author's call over folding
  them away: found-or-chosen, the version, and either "everything Llamaport
  needs is supported" or the name of the flag that is missing. `--metrics` and
  `--cache-type-k` are what it checks; `--fit` is deliberately not among them,
  because without it the app resolves Auto itself.
- **Launch defaults folds behind "Edit defaults"**, whose summary is what the
  fold hides — "Built-in · fitted context · port 8080".
- **Appearance keeps its card.** The artboard predates it and draws three; this
  screen has four.

**The artboard's caption was false for the second time.** It reads "Llamaport
watches it, so files you drop in show up in the Library" — nothing watches the
models directory, which is why Rescan exists and why item 2 kept it. The line
now says to press Rescan. The rule this produced is in
[knowledge/technical.md](../knowledge/technical.md).

```text
build: 0
cargo test status: 0        (261 passed, 5 ignored — unchanged)
clippy status: 0
fmt status: 0
```

**Owed to the author's eyes, and to a signed build**: the picker is the first
native dialog this app opens, and a `dev` window says nothing about how it
behaves inside a notarised bundle — recorded in
[release.md](release.md) under "Unverified against v0.6.1".
