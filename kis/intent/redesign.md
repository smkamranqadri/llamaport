# Redesign

Planned 2026-09-02, the second phase set cut from [direction.md](direction.md),
and **finished 2026-09-03**. The author's words that opened it: "I don't like
the ui, not user friendly specially someone who don't know all the technical
stuff." The reference he pointed at is OrbStack; the screens were interviewed,
mocked, revised once on feedback, and approved the same day. The approved
mockups are the spec:
https://claude.ai/code/artifact/f92717ab-9ddc-4ae7-8fb2-58b13137d257 — eleven
artboards, including the edge states (stray llama-server banner, empty first
run) and one light-mode variant.

Live status is in [state/current.md](../state/current.md), not here.

## What was decided in the interview

- **Layout is sidebar plus one content pane**, not OrbStack's three zones. The
  author chose it over the three-zone option seeing both drawn.
- **Sidebar**: Library, Discover, Downloads, Activity Monitor; Settings at the
  bottom. **Discover shipped disabled** — the author chose a grayed "coming soon"
  entry over leaving it out, so the layout was final from day one. Its screen was
  designed (artboard "Discover — build later") and not built here; it was built
  on 2026-09-03 and the entry is live ([discover.md](discover.md)).
- **pi is a button on the running model, not a nav entry.**
- **The launch form shrinks behind three named presets** — Default / Best speed
  / Model suggested — with context and port still visible and everything else
  behind an Advanced disclosure. This is [direction.md](direction.md)'s "named
  choices" remainder, finally planned.
- **The running view is four cards** — Memory, Speed, Context, Health — in plain
  language, each technical figure paired with a gloss. A Test results row
  carries the health checks with a Test again action.
- **First run offers starter models sized to the machine's memory**, with
  paste-a-URL still available.
- **Theme follows macOS**, both appearances. SUPERSEDED — see
  [appearance.md](appearance.md): following macOS is now one of three modes, and
  four ported palettes sit beside the built-in one.
- **Activity Monitor ships with a CPU% column and no GPU column.** The author's
  choice, made knowing the telemetry had neither: per-process GPU has no public
  macOS API, so that column is dropped for good rather than deferred.
- **The README screenshots stay deferred behind this work.** Every phase would
  invalidate them again; they are done once, against the finished redesign
  ([release.md](release.md) phase 3).

## Phases — all four done

1. **Shell and Library** — `c6ac59f`, 2026-09-02. Sidebar, the visual system
   (tokens, rows, buttons, cards), Library grouped Running / Stopped, orphan
   banner restyled. No behaviour change. Four resolutions taken at the keyboard
   and still binding: **Activity Monitor shipped disabled too** (its screen is
   phase 4); **the sidebar's Now Running strip is gone**, replaced by a green
   dot on Library; **a row's Run sends no draft**, so `runner_start` resolves
   the same remembered profile the model screen opens with; **the native title
   bar stays**.
2. **Launch** — 2026-09-02. Three presets over machinery that already existed:
   Default is the launch defaults, Best speed the measured `speeds.json` row,
   Model suggested the offered-not-applied opinion from fitting. A preset whose
   backing fact is missing renders disabled with the action that would create it
   (Measure, for Best speed). ProfileForm survives behind Advanced, because
   Settings' launch defaults share it.
3. **Running view** — 2026-09-02. Four cards, the Test results row, the address
   line with Copy. The pi button moved and did not change: its 22 tests are the
   guard ([pi.md](pi.md)).
4. **Downloads, Settings, first run, Activity** — the first run left early on
   the author's ask (2026-09-02); the rest landed 2026-09-03 as `21cec86`,
   `e85ecae` (Downloads), `e897b2a` (Activity) and `9e16cb5` (Settings).

## What did not match, screen by screen — all five closed

Written 2026-09-02 from the author's own list after six passes on one screen.
**One screen per task**, each finished by putting the artboard and the app side
by side — the method is in
[knowledge/technical.md](../knowledge/technical.md), and not using it is what
made the first four passes worthless.

1. ~~Stopped model screen~~ — 2026-09-02, three rounds of the author's
   corrections.
2. ~~Library rows~~ — 2026-09-02, two rounds.
3. ~~The stray-server banner~~ — 2026-09-02, and it was hiding a defect rather
   than only a layout.
4. ~~Measure~~ — 2026-09-02, one round. **Built inside the model screen rather
   than as its own screen**, on the author's ruling: "I don't mind to do in
   current screen instead of new screen but should be same design."
5. ~~Empty Library~~ — 2026-09-02, pulled forward out of phase 4 by the author.

## Risks named at planning

- The pi button regressing while its screen is rebuilt under it.
- ProfileForm's shared use by Settings.
- No frontend suite exists: the phases are proved by the four commands green
  plus the author's eyes on the built app, which is this project's normal.

## Proof

**The four commands ([knowledge/technical.md](../knowledge/technical.md)) ran
green after every unit below**, each status captured on its own line and never
after a pipe; only the test count is repeated here, because that is the part
that moves. No unit changed a test's meaning without recording the mutation that
watched it fail.

### Phase 1 — 256 tests

`src/App.tsx`, `src/Library.tsx`, `src/App.css`, `src/icons.tsx` (new). The
Library's list class became `.model-cards` so Downloads, sharing `.model-list`,
kept its look until its own phase. Captured with nothing running.

### Phase 2, and the six passes it took — 256 tests

`src/Presets.tsx` (new), `src/ModelDetail.tsx`, `src/ProfileForm.tsx`,
`src/App.css`.

Decisions that still describe the app:

- **A preset owns six fields** — ctx, ngl, parallel, both cache types, flash
  attention — and never alias, port, jinja or extra arguments, which are the
  user's. Selection is derived by comparing those six against the form.
- **The built-in default is already the fit** (`ctx` Auto, `ngl` auto), so
  Default and Model suggested coincide until custom defaults are saved. One
  highlight, by rank: Best speed, then Model suggested, then Default.
- **Best speed disabled offers Measure in place**, with the same blocked reason
  the Speed panel gives.
- **`SpeedConfidence` is** `neverMeasured | observed | tuned`.
- **The panel wrapper is gone**: a sentence-case group label, not an uppercase
  `<h2>`.
- **The memory panel became a sentence** — a coloured dot and "Memory ≥ 1.3 GB
  of 25.0 GB · fits" — opening the bar and four figures. `launchCost` computes
  it once for both, so line and panel cannot disagree: the failure
  [screen.md](screen.md) closed.
- **Context is a named dropdown, not a slider**: "Fitted to memory —
  recommended", then the model's maximum and halves down to 4,096, the hint
  saying what a number is worth in pages. [direction.md](direction.md)'s named
  choices applied to the field itself — 21 launches only ever chose round
  numbers.
- **`ProfileForm` split into `ProfileFields` and `AdvancedFields`**, so the
  model screen can put the advanced seven in a page-level row while Settings
  composes both.
- **The design tokens are the mockup's**, read off its stylesheet rather than
  from memory, and `.screen-header` is a 62px bar with a rule under it on every
  screen. Icons where the mockup draws them; form controls stopped being the
  system's (8px radius, `--input` ground, drawn chevrons, no number spinners).
- **The content fills the window.** A 1040px column added in the fourth pass was
  reasoned from a 1180px mockup where it never binds; on a 1700px window it left
  a third of the pane empty. Removed in the sixth.
- **`.screen-header` takes `order: -1`**, because `App` renders the stray-server
  banner before the screen's own fragment and it drew above the title bar.

**Five of the six passes were wrong, and the reason is one fact**: four of them
were checked against *memory of the code that generated the mockup*, never
against the mockup rendered. Rendering it took one command and found in a minute
what four rounds of the author's time had not — Measure again missing entirely,
two preset descriptions rewritten instead of the approved copy, a memory line
that never said "comfortably", and an Advanced row that could name a different
preset from the highlighted card. **The method that came out of it is in
[knowledge/technical.md](../knowledge/technical.md) under Verify**, and it is
the reason every screen since has been rendered before hand-over.

**One deliberate deviation, confirmed rather than reopened**: the artboard's
launch screen has two rows, Advanced and Full command. The app has three more —
Model details, Speed and Logs — because they are the only route to the file's
diagnostics, the measurement history ([tune.md](tune.md)) and a crash's output.

### Phase 3 — running view, 256 tests

`src/ModelDetail.tsx`, `src/App.css`. Four cards (Memory, Speed, Context,
Health), the address line "Other apps reach this model at
`http://localhost:<port>/v1`" with Copy — the thing a person actually needs from
a running server, which the old screen never printed — and a Details group of
folded rows.

Four corrections from the author, all removals but one:

- **A running model shows no launch machinery.** Choosing settings is the
  stopped screen's job. (Amended 2026-09-03: Full command came back, below.)
- **No back button and no Library crumb.** The sidebar is how you leave a model.
- **Logs no longer unroll themselves** on a launch that works; only a crash
  opens them.
- **Default and Model suggested lit the wrong card.** Both are true — the
  built-in defaults *are* `ctx: auto, ngl: auto` — and the highlight was derived
  from values alone, so the ranking picked the fit. The pressed card is now
  remembered and wins until another edit clears it (`picked` in `ModelDetail`,
  `Which` from `Presets`).

### First run — empty Library, 256 tests

`src/FirstRun.tsx` (new), `src/Library.tsx`, `src/icons.tsx`, `src/api.ts`,
`src/types.ts`, `src/App.css`, `src-tauri/src/lib.rs`. "Models directory not
found" over a path became a cube, "Get your first model", three starter cards
and a paste-a-link row.

- **The starters are three fixed Qwen models**, named by the author, all from
  `unsloth`. Every URL and byte count was **verified live by HTTP HEAD**, so the
  size beside a card is the size that lands. An earlier pass offered other
  models and filtered by memory; the author replaced both.
- **The ceiling is the Metal working set, not installed memory.** A new
  `machine_memory` command carries `device_budget_bytes` off the same
  `capabilities()` the launch plan uses. Where the binary has not been found the
  screen falls back to installed memory **and says so**, because falling back
  silently is the defect [screen.md](screen.md) closed.
- **Four bands, because three lied.** ≤50% "fits easily", ≤75% "fits", ≤100%
  "tight — little room for a conversation", above "too big for this Mac". The
  first build said "too big" at 75%, which is false. Nothing is filtered out;
  the wording carries the judgement.
- **`download_start` creates the models directory**, which the author's machine
  needed — every Download button would otherwise have failed there.
- **Two deviations kept**: Rescan stays in the header, because this is the one
  screen where the folder may be missing; and one line names a models directory
  that does not exist yet, which the artboard does not cover.

**Found on the way and written up rather than fixed**: the app can only build a
fully offloaded launch ([moe.md](moe.md)). The author's word: "First run — empty
Library is done."

### Item 1 — stopped model screen, 257 tests

`src/ModelDetail.tsx`, `src/App.tsx`, `src/App.css`. The three rows the artboard
does not draw were not deleted on sight; where their content goes was put to the
author first.

- **Model details stays as a third row.** The author overturned the artboard
  rather than lose the file's facts on the one screen you read before running.
- **Speed appears only while a measurement runs**; the row's summary is the
  artboard's own `2 of 4 tries done`, off `TuneReport`.
- **Logs survive a crash.** The ruling was "no log on a model that is not
  running", and a crashed model is not running — which would have deleted the
  log at the one moment it is the only answer.

**The defect the side-by-side found**: the sidebar went blank on a model screen.
Phase 3 removed the crumb on the ruling that the sidebar is how you leave a
model, but `active` was gated on `&& !selected`, so opening a model unlit
Library and left nothing saying where you were. **Six metric drifts** were read
off the artboard's stylesheet at the same time, and two spacing failures the
drawing does not cover were found by the author: `.preset-foot`'s `margin-top:
auto` opened a hole under Best speed once another card's hint made the row two
lines tall, and a flat row's rhythm belonged on the row rather than its summary.
**The author overruled the artboard on the gaps around the memory line** — 22px
above, 24px below, against the drawing's 15 and 16 — and confirmed the chevron
the artboard does not draw, because the four figures behind it have no other
route on that screen.

Captured by the author himself, which is the cheaper route and is now the rule.

### Item 2 — Library rows

`src/Library.tsx`, `src/App.tsx`, `src/App.css`, `src/icons.tsx`. Three columns
and a file name became one line: a dot, the name, badges, one stat sentence, the
action. `rowStat` is that sentence — `1.7 GB memory · 285 tok/s` running,
`21.2 GB · ran yesterday` or `1.4 GB · added 12 days ago, never run` stopped. A
broken file replaces the sentence rather than adding a line.

- **The star and Delete wait for a hover.** The artboard draws neither, and this
  row is the only place either exists in the app. A favourited model keeps its
  star at rest, because a mark you cannot see is not a mark, and the hidden star
  holds its width so the stat column does not jump.
- **The header took the counts and the search, not the Download model button** —
  the author asked for all three, then removed the button on seeing it. Search
  filters display name *and* file name, since the row no longer shows the file.
  Rescan stays: nothing watches the models directory.
- **Use in pi** is on the running row. Its mark was a drawn π because `~/.pi`
  and `~/.superconductor` hold no brand asset; **corrected 2026-09-02 on "add
  actual pi icon"** — the installed package's README names the real one at
  `https://pi.dev/logo-auto.svg`, a filled blocky P with the i's square. The
  failed search had looked in the data directories and never in
  `~/.local/lib/node_modules/@earendil-works/pi-coding-agent`.

Two corrections from the author's first capture: the Download model button, and
**a row 14px shorter than the drawing** — the padding sat on the row *button*
while the action buttons are its siblings, so the tallest thing in the row set
no height at all. A third came after: with nothing running there is no group
label, and the label was the only thing supplying the gap above the list.

### Item 3 — the stray-server banner, 257 tests (one new)

`src/App.tsx`, `src/App.css`, `src-tauri/src/runner.rs`. A two-line heading over
a bullet list became one line per stray server.

**It was hiding a defect the redesign only exposed.** The line reads `<model> ·
port <port> · probably left over…`, and the app had nothing for either slot —
every orphan it had ever reported said *unknown model* on an unknown port.
`parse_server_command` was correct and so were its tests; `detect_orphans`
called `sysinfo`'s plain `refresh_processes`, which leaves `cmd()` empty, so the
parser was never given anything to parse. Proved with a throwaway test printing
`cmd=[]` for a live `llama-server`, fixed with `refresh_processes_specifics` and
`ProcessRefreshKind::everything()`, and recorded as a constraint in
[knowledge/technical.md](../knowledge/technical.md).

**The alias now wins over the file name** — what the artboard draws, the id a
client addresses, and short enough not to wrap the banner. Mutation-checked:
gutting the preference to `let model = from_path;` fails the new test with
`left: Some("Qwen_Qwen3.5-2B-Q4_K_M")`.

**Cost recorded rather than hidden**: editing `src-tauri/src/lib.rs` makes
`tauri dev` rebuild and restart, which stops whatever model is running. It
stopped the author's twice in one session.

### Item 4 — Measure, 257 tests

`src/TunePanel.tsx`, `src/ModelDetail.tsx`, `src/Disclosure.tsx` (new),
`src/App.css`, `src/icons.tsx`, `src/types.ts`, `src-tauri/src/tune.rs`.
`295ea04` and `02b1c1a`. The artboard's whole screen is drawn inside the Speed
row, which cost nothing: the drawing is a list of tries and one button, and the
row already had a header with a place for Cancel.

- **The history stays, folded.** `speeds.json` has no other route in the app.
- **A try is named in words with its arguments beside it** — `16k context · full
  precision`, then `16,384 · f16 — the largest that fits`.
- **The caveats fold.** One verdict line stays; the 10% tie rule, the
  observed-not-measured warning and the shared-prompt note went behind "Why this
  one".
- **The row folds itself when the ladder ends, and applies the answer** — the
  author's correction: it had been pinned open, leaving four tries on screen
  while the one preset they were measured for sat unchanged above.
- **`tune::Report` carries `candidates`** from the moment the ladder starts. It
  never named the tries it had not run, so a waiting row was not expressible,
  and a list that grows from nothing cannot say how far it has to go.
- **The marked row is the one the button applies**, not always the quickest
  reading: once the ladder is over that is the suggestion, which prefers the
  widest context among readings too close to separate.

**Found on the way**: `.tune-head`, `.tune-note` and `.tune-row.is-unranked`
were painted with `--muted`, and `.tune-row.is-fastest` with `--surface-raised`
— neither has ever been a token in this stylesheet, so three greys rendered as
body text and a highlight as nothing.

### Phase 4, Downloads — 258 tests

`src/Downloads.tsx`, `src/App.css`. Two panels and a history table became the
artboard: a paste field and Get in the header, one card per transfer, Finished
rows with a tick, a name and Show.

- **A finished row is named by the catalog**, which reads the GGUF's own name,
  so the Library and this screen cannot call the same file different things. A
  row in flight has no GGUF to read, so its file name stands in and its quant
  badge is parsed by a TypeScript copy of `catalog.rs`'s rule. **Reversed
  2026-09-04**: the job carries its quant from Rust, by the one rule
  ([review.md](review.md)).
- **The URL survives only on a failure** — the one moment somebody needs the
  address they asked for. Elsewhere it is the row's tooltip.
- **The speed limit is a named choice on the status line**, not a panel: No
  speed limit, then 0.5, 1, 1.5 and 2 MB/s — the author's ladder — and
  "Something else…", which opens a field. Past two megabytes the useful figure
  depends on a line nobody here has seen. [direction.md](direction.md)'s named
  choices applied to the last free-typed number on any screen; the engine's
  64 KB/s floor is named beside the field, because typing is the only way under
  it.
- **A third group, "Did not finish."** A failure is neither downloading nor
  finished, and filing it under Finished would be the screen lying.
- **Clear sits below both lists**, because `downloads::clear` removes complete
  *and* failed rows.

**The artboard's caption was false, and matching it would have shipped the
lie**: "Downloads survive quitting the app — they pick up where they left off",
where [downloader.md](downloader.md) records that a relaunch restores them
**Paused** with Resume live. The rule this produced is in
[knowledge/technical.md](../knowledge/technical.md).

Orphaned rules went with the rewrite: `.download-row`, `.download-progress`,
`.panel-head`, `.badge-quiet`, and the verifying tint on `.kv-bar`.

### Phase 4, Activity Monitor — 261 tests (three new)

`src-tauri/src/activity.rs` (new), `src-tauri/src/lib.rs`, `src/Activity.tsx`
(new), `src/App.tsx`, `src/api.ts`, `src/types.ts`, `src/App.css`. `e897b2a`.

- **Every `llama-server` the app knows**, the author's call over "only the
  running model": the model, the measurement's own server, and any stray, each
  row saying which it is.
- **A CPU column and no GPU column**, as the interview decided.
- **The GPU card carries memory instead** — the author's pick of three: what the
  running launch asks of the Metal working set against what that set will hand
  out, both already read for the launch plan.
- **CPU comes from `sysinfo`, not `proc_pid_rusage`** — a deviation from the
  interview's implementation note, recorded rather than quietly taken. A
  percentage is a difference between two samples, so `activity::Monitor` holds
  one `System` between polls, and `sysinfo` is already this app's process source
  for the orphan scan. Memory stays on `sysmem::process_footprint_bytes`, so
  this table and the running model's screen cannot disagree.
- **Polled, not pushed**: read only while open, and the interval it chooses is
  the window every CPU figure is averaged over.
- **Two deviations from the drawing**: the cards sit under the table rather than
  pinned to the window's bottom edge, and a stray row is marked amber.

**A defect the tests caught before the author could.** The orphan scan skips the
runner's child and knows nothing about Tune's, so a measurement in progress was
about to be listed as somebody's stray server — the stray banner's own defect,
one layer up. Mutation-checked: dropping the `continue` in `known_processes`
fails with `left: [42, 99]`.

### Phase 4, Settings — 261 tests, and the redesign closes

`src/SettingsScreen.tsx`, `src/App.css`, `src-tauri/Cargo.toml`,
`src-tauri/src/lib.rs`, `src-tauri/capabilities/default.json`, `package.json`.
`9e16cb5`. Four panels became four cards; `.panel` is Settings' alone, so the
change reaches nothing else.

- **Change… opens the real folder picker**, the author's call over a typed path:
  `tauri-plugin-dialog` is a new dependency in a signed app, and a "…" button
  that opens nothing is a promise the screen does not keep. The binary uses the
  same picker; "Find it for me" clears the choice back to the automatic search.
- **The binary's six facts become one line**, the author's call over folding
  them: found-or-chosen, the version, and either "everything Llamaport needs is
  supported" or the name of the missing flag. It checks `--metrics` and
  `--cache-type-k`; `--fit` is deliberately not among them, because without it
  the app resolves Auto itself.
- **Launch defaults folds behind "Edit defaults"**, whose summary is what the
  fold hides.
- **Appearance keeps its card.** The artboard predates it and draws three.

**The artboard's caption was false for the second time**: "Llamaport watches it,
so files you drop in show up in the Library" — nothing watches the models
directory, which is why Rescan exists and why item 2 kept it.

### Sign-off

**Downloads, the Activity table and the folder picker were reviewed by the
author on the running app, 2026-09-03**: "download, activity table and pick all
good, reviwed". Every earlier unit was signed off as it landed. **The notarised
bundle is a separate question** for the picker, and is in
[release.md](release.md) under "Unverified against v0.6.1".
