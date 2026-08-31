# Tune

Planned 2026-08-31, the second phase cut from [direction.md](direction.md) and
the one the approved mockup waits for. [Screen](screen.md) shipped the half that
could be true without a measurement. This is the measurement.

Live status is in [state/current.md](../state/current.md), not here.

## What is wrong

**The app has no opinion, and the only rule available without measuring is
wrong.** "Largest context and most precise cache that fits" picks 262,144 with a
`q8_0` cache on Ornith. Measurement picks 65,536 at `f16`, 27% faster on
generation — the number the author had been typing by hand for 17 of 21 launches
([direction.md](direction.md)).

**Every run already measures itself and the reading is thrown away.**
`llama-server` returns `prompt_per_second` and `predicted_per_second` with each
completion, and the telemetry loop diffs the same counters off `/metrics` on
every poll for the live view (`runner.rs:512`). Nothing survives the process it
was measured from. Nineteen models have been launched and nothing on disk says
what any of them did.

## The correction this phase makes to direction.md

[direction.md](direction.md) says the first move is to remember what ordinary use
already demonstrated, and that measuring by launching is the fallback. Half of
that stands; half is reversed.

A passive record says what a model got at the settings it ran at. It cannot say
which setting is *faster*. `tools/fits.py` sends an identical prompt to every
candidate precisely because comparing configs on different prompts compares
nothing, and this project has already been burned by exactly that: `q8_0` read as
within 2% of `f16` against a 36-token prompt and 27% slower against a real one.

So ordinary use is evidence and a controlled run is the verdict. Both ship here,
and the passive half is ranked only under the gate below.

## Decisions

- **Both halves in one phase.** The author's call, against a recommendation to
  ship the benchmark alone and file the passive record as a later item. What it
  buys is that the 19 models already launched stop being blank.

- **A record is keyed on the model plus everything that can move the number.**
  `ctx`, `ngl`, both cache types, `flash_attn`, `parallel` and `rawArgs`
  verbatim. Alias, host and port are excluded because they cannot move it.
  Stamped with the build, which `Capabilities.version` already reads. Ranking
  happens within one build; a record from another is shown, marked stale, and not
  ranked against current ones. Only this shape answers "did that change help",
  which [direction.md](direction.md) calls the whole point of an optimizer.

- **A third file, `speeds.json`.** The same argument that split `downloads.json`
  out of the config ([knowledge/technical.md](../knowledge/technical.md)): this
  is history that grows on every run, and an unreadable history should cost the
  user their history and nothing else — not the models directory, the binary path
  and 21 remembered launches. Read back as untrusted input like everything else
  in that directory.

- **Written on settle, from totals, never from a tick.** One row per run that
  reached Ready and generated something, holding the run's own totals — an
  average over the whole run rather than an instant. Never trimmed: history is
  never trimmed, and a cap would be a number nobody has evidence for.

- **The app has no opinion until something has been measured.** This reverses the
  approved mockup's "Before launch" tab, which shows an arithmetic suggestion
  labelled *never measured*. The label is thin protection for a rule this project
  has measured as picking the slowest of three, and prose beside a figure is where
  all eleven defects of 2026-08-31 lived. A model nobody has run opens exactly as
  it does today: its last successful launch, else the launch defaults.

- **Passive rows may be ranked, gated on workload.** The author's call, against a
  recommendation that only a Tune run ranks. A row is ranked only if the run did
  real work — a minimum of prompt tokens and of generated tokens — so a warm-up
  never becomes a verdict. Every ranked row shows the workload it came from, so
  the caveat sits in the table rather than only in a sentence beside it. The
  hazard is recorded rather than argued away: those rows did different work and
  their ordering can be wrong.

- **Tune runs the ladder on its own port and refuses while a model runs.** One
  model at a time is an existing invariant. Killing a server the author may be
  talking to through pi is not Tune's business, and relaunching it afterwards is
  a second failure path that has to work when the ladder has just crashed.

- **`tools/fits.py` stays the oracle.** The same relationship it has with
  `estimate.rs`. Tune's candidate picker and prompt sizing are ports of its
  `candidates()` and `long_prompt()`, and where the Rust and the script disagree
  about a file in the models directory, one of them is wrong.

- **The winner is offered, never applied.** No merging is a durable decision
  ([knowledge/project.md](../knowledge/project.md)). A measurement does not get to
  rewrite a form the user is looking at.

## Parcel 0 — the suite stops writing to the live directory — DONE 2026-08-31

`store::config_dir()` has no test override (`store.rs:63`), and `runner.rs` builds
the pidfile and `last-run.log` from it (`runner.rs:641,645`) while
`runner_lifecycle.rs` drives the real runner against a `python3 -m http.server`
stand-in. So `cargo test` already destroys the one artefact worth analysing, and
the log sitting in Application Support now is test output.

`config.json` and `downloads.json` escape only because that path never writes
them. A speed row written on settle would not escape: the suite would write into
the author's real records and then the optimizer would rank test runs.

First, and not optional.

### Proof — 2026-08-31

The four commands green, **203 tests**, up from 202. `store::use_config_dir` takes
the directory once through a `OnceLock`; `config_dir` reads it; the tests that can
start a runner call `common::isolate_config_dir` and get one named for their
process under `TMPDIR`.

**Reproduced first.** `last-run.log` moved from 20:48 to 21:31 across an unfixed
`cargo test` and came back holding `http.server` request lines and the crash
test's own `error: failed to load model`. After the fix, a full run left every
file in `~/Library/Application Support/llamaport` byte-for-byte and
mtime-for-mtime unchanged, and the run log appeared under
`$TMPDIR/llamaport-test-<pid>/` instead.

**The first version of the test was worthless and the mutation is what said so.**
`isolate_config_dir` returned the directory *in effect* rather than the one it
asked for, so with the override gutted it returned the live directory and
`log_path().starts_with(isolated)` was trivially true — the test passed against
the exact defect it was written for. `use_config_dir` now returns nothing and the
helper returns its own path; under the same mutation the test fails naming
`/Users/…/Application Support/llamaport/last-run.log`.

Not fixed here, and not this parcel's business: `last-run.log` is still wiped at
the start of every run, so there is still no history. That is parcels 1 and 2.

## This was built once already

Found 2026-08-31 while capturing parcel 0's reproduction, by looking at the
directory rather than at the code.
`~/Library/Application Support/llamaport/benchmarks.json` has been sitting there
since 2026-08-01: schema 1, a `records` array, each record carrying
`modelFile`, `modelSizeBytes`, `architecture`, `quantisation`, `ctx`,
`cacheTypeK`, `cacheTypeV`, `ngl`, `parallel`, `llamaVersion`, `promptTokens`,
`promptTps`, `timeToFirstTokenMs` and `depthTokens`.

That is parcel 1's record, designed once and deleted. `31031b2` removed
`benchmarks.rs`, the Benchmarks screen and the benchmark half of `health.rs` as
part of cutting 6,707 lines of Rust to 5,061 — scope that "was never part of the
original goal" while the downloader remained unbuilt. The commit left the file on
disk deliberately: removing a feature should not delete the user's data.

**KIS records none of it.** Not in the roadmap's "Decided against", not in the
durable decisions, nowhere. [direction.md](direction.md) brought measurement back
into scope without anyone knowing it had been here before — which is the thing
the Discover entry exists to prevent, happening again to a different feature.

**Decided 2026-08-31: start clean.** Parcel 1 writes its own store and does not
read the deleted implementation or adopt `benchmarks.json`. That file stays on
disk untouched, for the same reason `31031b2` left it there. What the removal was
for is now written down, which is the part that was missing.

Two things follow from it anyway:

- The removal was scope discipline, not a fault in the design, and this phase is
  the author asking for the feature back as the app's user. That is the argument
  for building it again, and it is a different argument from the one that built it
  the first time.
- One record in that file reports `promptTokens: 17`. Whatever it measured, it
  measured startup — the same defect as the 36-token prompt, in the feature this
  one replaces.

## Parcel 1 — the record — DONE 2026-08-31

`speeds.rs` holds `SpeedKey`, `Source` and `SpeedRecord`; `store.rs` owns the
file, as it already owns `downloads.json`. No UI, and no caller until parcel 2.

`SpeedKey::of` is built from a `Profile`, which is what makes the exclusions
checkable: alias, host and port identify a server rather than its speed, and
leaving them out is the difference between one row per settings change and one
row per port. `jinja` is out too — it changes the template, not the throughput at
a given size.

Figures are stored as totals and the rates are derived, so nothing on disk can
disagree with itself. `rate` refuses a zero denominator rather than returning
infinity.

### Proof — 2026-08-31

The four commands green, **210 tests**, up from 203. `cargo fmt` failed once and
was applied before the run above.

**Seven new tests, five gutted and watched to fail:**

- `load_speeds` stops filtering unsound rows -> the dropped-row test fails.
- `append_speed` overwrites instead of appending -> the two-rows test fails.
- `load_speeds` insists the file parses -> the unreadable-file test fails.
- `rate` accepts a zero denominator -> the rate test fails.
- `SpeedKey::of` forgets the context -> the key-separates test fails.

`the_key_ignores_what_cannot_move_the_number` has no gutting that breaks it: it
asserts an absence. It is a guard against a later change adding a port or an alias
to the key, not a test of code that exists, and it is recorded that way rather
than dressed up as coverage.

**Known limit, not engineered around.** `append_speed` reads and rewrites the
whole file per run. At one row per launch that is nothing for years, and a cap
would still be a number nobody has evidence for.

## Parcel 2 — the passive half — WRITING DONE 2026-08-31, screen still to come

The runner keeps the last totals each telemetry tick reported and writes one row
when the run settles. A run that generated nothing writes nothing.

**The screen half is not built.** Showing the rows with their workloads, ranked
under the gate, is where the caveat has to be visible, and it belongs with parcel
4's suggestion rather than on its own — nothing else on the model screen would
have anywhere to put it. Recorded here rather than quietly folded in.

### Decisions taken while building it

- **The runner writes the row itself**, as it already writes the pidfile and
  mirrors the log. The alternative — emit an event and let `lib.rs` persist it, as
  the downloader does — puts the write behind a listener, and `Ready` is announced
  on every telemetry tick. Writing at the settle point is once per run by
  construction.
- **Taking the counters is the once-per-run guard.** Both settle paths call the
  same function and the second finds nothing left. No flag, no bookkeeping.
- **Their presence is also the Ready gate.** The telemetry loop is the only writer
  and it only runs after Ready, so a run that never served has no totals to find.
- **The key is built by the caller.** `LaunchSpec` carries a `SpeedKey` and the
  build string, so the runner records what it is told rather than knowing which
  settings can move a number.

### Proof — 2026-08-31

The four commands green, **215 tests**, up from 210. `cargo clippy` failed once on
a `&PathBuf` in the new test helper and `cargo fmt` once; both fixed before the
run above.

**End to end against the real server**, not only the stand-in: `real_launch`
(ignored by default) now sends a completion and reads the row back. On
`qwen2.5-0.5b` at 8,192 with build `10360 (48d22e295)` it recorded 148.6 tok/s
generation where the server's own `predicted_per_second` said 148.389 — the small
difference is that one is derived from the run's totals and the other is that
request's own figure. The row on disk was read, not inferred:

```json
{ "key": { "modelId": "28468760-89e2c3e7f5216ab0", "ctx": 8192, "ngl": "auto",
           "cacheTypeK": "q8_0", "cacheTypeV": "q8_0", "flashAttn": true,
           "parallel": 1, "rawArgs": [] },
  "llamaVersion": "10360 (48d22e295)", "source": "observed",
  "promptTokens": 6.0, "promptSeconds": 0.053,
  "genTokens": 48.0, "genSeconds": 0.323 }
```

Its `promptTokens: 6` is the workload gate's whole argument, produced by accident:
a six-token prompt measured startup, and its 113 tok/s means nothing. That row is
stored and must never be ranked.

**Four of five mutations killed their test.**

- `record_speed` stops requiring that anything was generated -> the idle-run test.
- `stop()` no longer records -> the recorded-run test.
- The exit path no longer records -> the restarted-run test.
- The telemetry loop stops keeping the totals -> the recorded-run test.

The fifth: clearing the counters when a new generation starts changed nothing,
because `record_speed` takes them on every path that can precede one. **It was
deleted rather than kept and excused.** A second mechanism nothing can reach is
what the mutation was for.

`a_run_that_never_became_ready_is_not_recorded` has no gutting that breaks it, for
the same reason as parcel 1's key test: it asserts a structural absence.

### A real defect the repeats found, not the suite

The suite passed. Run three of five failed. `append_speed` read the file,
appended, and wrote it back with no lock, so two runs settling in the same instant
both read the old file and the second rename discarded the first row — a silently
lost run, against an invariant that says history is never trimmed. A `Mutex` held
across the read-modify-write fixes it; eight consecutive runs green after.

Removing that lock again to check the new test also exposed something older, now
in [knowledge/technical.md](../knowledge/technical.md): `write_atomic`'s temporary
is named after the destination rather than the writer, so concurrent writers to
one file collide outright. Every caller is serialised today. Any new one must be.

## Parcel 3 — the benchmark — DONE 2026-08-31

`tune.rs`: the candidate picker, the prompt, the measurement, and a `Tuner` that
runs the ladder on a thread and announces a whole `Report` on every change — the
way the runner announces state, because a screen that assembles progress from
increments gets it wrong the moment one is missed. `TunePanel.tsx` renders it.

### Decisions taken while building it

- **A candidate is measured through `Profile::args`**, not through a second argv
  builder. Tune therefore measures the command the app would actually run,
  including whatever the build accepts, and cannot drift from it.
- **The port is excluded from the key, so a Tune row and an ordinary row at the
  same settings share one.** Tune measures on 9977 and a launch does not; if port
  were part of the key those would never be comparable. That exclusion was made
  for a different reason in parcel 1 and turns out to be load-bearing here.
- **The child is killed by `Drop`.** Cancelling, a candidate that will not load
  and a panic all end the same way, and a `llama-server` holding tens of gigabytes
  must not outlive the function that started it.
- **A failure is a row.** "This did not load" answers the question the ladder
  asked, and the table has to be able to say it.
- **`detect_orphans` needed nothing.** It already finds every `llama-server`, so a
  stranded measurement is visible. What it did need was the opposite: `orphan_status`
  now excludes the server Tune is measuring right now, which is not something the
  user left behind.

### Proof — 2026-08-31

The four commands green, **227 tests**, up from 215. Clippy failed once on a
`contains`; fixed before the run above.

**The picker agrees with the oracle on the real file.** `tools/fits.py` on
`ornith-1.0-35b-Q4_K_M.gguf` finds 262,144 / `q8_0` the widest that fits and
262,144 / `f16` the first that does not, leaving a small and a middle rung.
`real_tune::the_candidates_agree_with_the_oracle` asserts exactly that list and
runs in the ordinary suite.

**The ladder reproduces the oracle's ordering**, run for real on the 21 GB model
— three launches, 135 seconds, every candidate answering the same 3,812-token
prompt:

```text
     262144 ctx / q8_0    30.1 tok/s generation ·  367.5 prompt   what arithmetic picks
       8192 ctx / f16     36.0 tok/s generation ·  461.3 prompt
      65536 ctx / f16     37.0 tok/s generation ·  450.2 prompt   fastest
```

Against `fits.py`'s recorded run: 30.5, 41.7, 41.6. **The ordering holds and the
absolute figures do not** — this machine had 2.91 GB of swap in use and a build
running. That is an argument for measuring on the machine as it is rather than
trusting a stored number, and it is why a record is stamped with its build and
kept per run rather than averaged.

**Then through the app itself**, which is what proves `lib.rs` passes the right
profile — the thing State had recorded as owed. Three rows landed in the real
`speeds.json` carrying `"ngl": "all"`, the author's own remembered setting, and
`"source": "measured"`.

### The finding that matters for parcel 4

Across the two ladder runs the two full-precision rungs swapped places: 65,536
beat 8,192 by 3% from the command line, 8,192 beat 65,536 by 6% through the app.
`fits.py` had them at 41.7 and 41.6 — a tie.

**The two f16 rungs are within noise of each other, and the gap that is real is
the 20-27% to the quantised 262,144 row.** A suggestion that names a winner
between two readings a few percent apart will flip between runs and read as
broken. Parcel 4 has to say what is settled — a full-precision cache at a
moderate context — rather than crown one rung over its neighbour.

### What was not verified on screen

The panel was seen empty and mid-run: the Tune button in place between Context and
Launch settings, and "Measuring 1 of 3" with a Cancel button while the real ladder
ran. **The finished table, the fastest marker and the verdict line were not seen.**
Screenshots were being taken of the whole display and the next one caught the
author's own window rather than the app, so capturing stopped there; the images
were deleted. Those three want a look before the phase is called done.

## Parcel 4 — the suggestion — DONE 2026-08-31, unseen on screen

`speeds::summarise` turns the rows into one opinion; `speeds_for` serves it;
the Speed panel shows it above the ladder's table and a history of what the model
has done.

### The rule, and where each number came from

- **A row is ranked only if the run did real work** — 256 prompt tokens and 64
  generated. Floors, not thresholds: the evidence is a 36-token prompt that put
  `q8_0` within 2% of `f16` and a 3,794-token prompt that put it 27% behind, and
  the six-token row the real server wrote during parcel 2. Everything is kept and
  shown; ranking is what is withheld.
- **Two readings within 10% are not told apart, and the widest context among them
  is suggested.** The threshold is read off the disagreement rather than picked:
  the same two rungs came out 65,536 ahead by 2.8%, 8,192 ahead by 6.3%, and level
  at 0.2%. Anything below 6.3% crowns whichever ran last. Context costs nothing
  when the speed is the same, which is what makes the widest a stable answer
  rather than an arbitrary one.
- **Rows are grouped by settings *and* build**, and the best of each group
  represents it. A slow run says the machine was busy, not that the setting is
  bad. Merging across builds hid an older reading behind a number it did not
  produce — found by a test, not by reasoning.
- **Three states.** Never measured when nothing is ranked; observed when only
  ordinary use is, and it says on screen that those runs answered different
  questions; tuned when a ladder compared at least two settings and the
  suggestion is one of them.
- **The suggestion is offered, never applied.** One button fills the form, which
  is where every other launch decision is already made.

### Proof — 2026-08-31

The four commands green, **234 tests**, up from 227. **Seven new tests, all seven
gutted and watched to fail**: the workload floor dropped, the tie broken by speed
instead of context, noise set to nothing, a stale reading ranked anyway, groups
merged across builds, ordinary use claiming to have compared, and a group keeping
its first run rather than its best.

The tests are written against the ladder as it actually came out through the app,
so they encode the finding rather than a hypothetical: 262,144/`q8_0` at 30.0
tok/s, 8,192/`f16` at 38.9, 65,536/`f16` at 36.6. The rule suggests **65,536 with
a full-precision cache** — not the fastest reading, the widest of the two nothing
could tell apart — and reports it as 21% faster than what fitting the largest
context would have chosen.

### Looking found five, as it always does — 2026-09-01

The suite was green through every one of them.

- **The history had no column headers.** `38.9 tok/s   489 tok/s` side by side
  with nothing saying which was generation and which was prompt eval. The header
  existed but belonged to the live ladder's table, so it appeared only while
  measuring — exactly when the numbers are least readable anyway.
- **"Measured 36.6 tok/s" did not say measured what.** Now "Generation".
- **Nothing marked the suggested row.** The list is ordered by speed, so 8,192 sat
  on top while the suggestion read 65,536, and the only thing reconciling them was
  a sentence. The suggested row is now labelled and highlighted in the list it is
  chosen from.
- **The Tune explainer read as "nothing has been measured"** while three
  measurements sat below it.
- **The never-measured state said the same thing twice**, once as a verdict and
  once as an instruction. One sentence of fact, one of what to do.

The first four were found in the panel as first written; the fifth was found after
fixing them, on a different model, which is the argument for looking at more than
one state.

### Confirmed on screen — 2026-09-01

The author brought the window back and looked at Ornith with its three rows. All
five fixes render: the `CONTEXT · CACHE / GENERATION / PROMPT EVAL` headers, the
Generation label, the highlighted `65,536 · f16  suggested` row, no explainer
above measurements, and the single-sentence empty state on a model with nothing
recorded.

The figures agree with each other, which is the check this panel most needed:
36.6 against 30.0 is the 22% it claims, and 38.9 against 36.6 is 6.3% — inside
the 10% the tie sentence describes, which is why the slower of the two is the one
suggested.

**Left alone deliberately.** With the form already at 65,536, "Use these settings"
changes only the cache types, and those sit behind Advanced — so its visible
effect is the Command panel switching to `--cache-type-k f16`. Nothing on screen
misstates anything, so this is recorded rather than fixed.

## Out of scope

Named choices. [direction.md](direction.md) lists their names and their number as
unsettled, and one measured opinion does not need three labels. Per-field
override markers, which nothing needs until something suggests values. pi.
Search. Whether hunting and working are two screens or one.

## Acceptance

- `cargo test` writes nothing into `~/Library/Application Support/llamaport`.
  Proved by looking at the directory across a run, not only by assertion —
  the file that is being protected is the one the suite has been overwriting.
- Two runs of one model at different contexts produce two rows, not one
  overwritten.
- A run that generated no tokens produces no row. A 36-token warm-up produces a
  row that is stored and not ranked.
- A record stamped with a different build is shown, marked stale, and not ranked
  against current ones.
- Tune on Ornith reproduces `tools/fits.py --run`'s ordering and its margin:
  65,536 / `f16` fastest, 262,144 / `q8_0` about 27% behind on generation.
  Disagreement with the oracle is a finding.
- Tune refuses to start while a model is running, and cancelled mid-ladder leaves
  no `llama-server` on its port.
- Applying a suggestion fills the form and launches with those values; declining
  it leaves the form as it was.

## Verification

The four commands, each status captured on its own line and never after a pipe
([knowledge/technical.md](../knowledge/technical.md)). Every new test gutted and
watched to fail. `tools/fits.py --run` as the cross-check for parcel 3.

Then on screen. Every screen defect this project has found came from looking and
none from the suite, and parcels 2 and 4 are almost entirely screen.

And on disk, for parcels 1 and 2: read `speeds.json` between runs rather than
trusting that it was written, which is the rule that caught four defects already.

## Risks

- **Ranking rows that did different work can be wrong.** Accepted deliberately.
  The workload gate and the workload column are the whole mitigation, and if a
  ranking turns out to contradict a Tune result on the same model, that is the
  signal to drop the passive ranking rather than to tune the gate.
- **Tune is minutes on a 21 GB model** — three or four loads plus their prompts.
  If it reads as too slow to ever press, the fallback is a Tune that measures
  only the current settings and appends one row per press.
- **The prompt is the measurement.** Everything this phase claims rests on the
  prompt being long enough to measure throughput rather than startup. That is the
  single lesson of the 2% that became 27%.
- **A ladder that spawns servers can strand one.** The orphan scan already exists
  for the runner; Tune's port has to be inside it.
