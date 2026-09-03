# Tune

Planned 2026-08-31, built 2026-08-31, corrected on screen 2026-09-01, shipped
in v0.5.0. The app now measures a model's speed instead of guessing it, then
offers one opinion.

## Purpose

The only rule available without measuring is "largest context and most
precise cache that fits". On the author's own model that rule picks the
slowest of three settings, 27% slower on generation than the fastest. Every
run already reports its own speed, and the reading was thrown away.

### The correction to direction.md

[direction.md](direction.md) said the first move is to remember what ordinary
use already demonstrated, and that measuring by launching is the fallback.
That is half right. A passive record says what a model got at the settings it
ran at; it cannot say which setting is faster, because comparing configs on
different prompts compares nothing. Ordinary use is evidence and a controlled
run is the verdict. Both ship here.

## Decisions

- **Both halves in one phase.** The author's call, against a recommendation to
  ship the benchmark alone and the passive record later. It means the 19
  models already launched stop being blank.
- **A record is keyed on the model plus everything that can move the number**:
  context, layer offload, both cache types, flash attention, parallel slots
  and raw arguments. Alias, host, port and the jinja template are excluded
  because none of them can move the number. Each record is stamped with the
  build.
- **A third file, `speeds.json`.** History that grows on every run belongs
  apart from `config.json`, on the same argument that split out
  `downloads.json` ([knowledge/technical.md](../knowledge/technical.md)). An
  unreadable history should cost the user their history and nothing else.
- **Written on settle, from the run's totals, never from a single tick.**
  History is never trimmed.
- **No opinion until something has been measured.** This reverses the
  approved mockup's "Before launch" suggestion, which showed an arithmetic
  guess before any launch. A model nobody has run opens as it does today: its
  last successful launch, else the launch defaults.
- **Passive rows may be ranked, gated on workload**: a minimum of 256 prompt
  tokens and 64 generated tokens, so a warm-up never becomes a verdict. Every
  ranked row shows the workload it came from.
- **Tune runs the ladder on its own port and refuses while a model runs.** One
  model at a time is an existing rule. Killing a server the author may be
  using, and relaunching it afterwards, is a second failure path Tune should
  not own.
- **`tools/fits.py` stays the oracle.** Tune's candidate picker and prompt
  sizing are ports of its `candidates()` and `long_prompt()`.
- **The winner is offered, never applied.** No merging is a durable decision
  ([knowledge/project.md](../knowledge/project.md)); a measurement does not
  rewrite a form the user is looking at.
- **The runner writes the row itself**, at the point a run settles, rather
  than through an event another module persists. Both settle paths call the
  same function and take the run's counters, which is the once-per-run guard.
- **Candidates are measured through `Profile::args`**, the same argument
  builder a real launch uses, so Tune measures the command the app would
  actually run.
- **A measured child is killed by `Drop`.** Cancelling, a candidate that will
  not load, and a panic all end the same way.
- **A failure is a row.** "This did not load" answers the question the ladder
  asked.
- **The measurement server is excluded from the orphan scan**, since it is
  not something the user left behind.
- **Two readings within 10% are not told apart, and the widest context among
  them is suggested**, with the reason shown. Context costs nothing when the
  speed is the same, so the widest of an indistinguishable pair is a stable
  answer.
- **Rows are grouped by settings and by build.** A slow run says the machine
  was busy, not that the setting is bad, and a group must not hide an older
  reading behind a number a different build produced.
- **Three states**: never measured, when nothing is ranked; observed, when
  only ordinary use is; tuned, when a ladder compared at least two settings.

## What was built

1. The suite stops writing to the live config directory
   (`store::use_config_dir`), so a speed record is not at risk of being
   overwritten by a test run.
2. The record: `speeds.rs` and the `speeds.json` store.
3. The passive half: the runner writes one row per settled run.
4. The ladder: `tune.rs` measures each candidate and reports a `Report` as it
   runs.
5. The suggestion and the Speed panel: `speeds::summarise` turns the rows into
   one opinion, shown above the ladder's table and the model's history.

Benchmarking was built once before and removed in commit `31031b2`, as scope
never part of the original goal; `benchmarks.json` was left on disk
deliberately, since removing a feature should not delete a user's data. This
phase writes its own store and does not read that file; it stays untouched.

## Measured ordering

Run for real against the 21 GB Ornith model, three candidates answering the
same prompt:

| Context / cache | Generation |
| --- | --- |
| 262,144 / `q8_0` | slowest |
| 8,192 / `f16` | within noise of the row below |
| 65,536 / `f16` | within noise of the row above |

The gap that is real is the 20-27% between the quantised 262,144 row and
either full-precision rung; the two full-precision rungs are not reliably
distinguishable from each other. That is why the tie rule suggests the widest
context among close readings rather than crowning whichever ran last.

Repeated runs also found a defect: `append_speed` read the file, appended, and
wrote it back with no lock, so two runs settling in the same instant both read
the old file and the second write discarded the first row. A `Mutex` held
across the read-modify-write fixed it. The related `write_atomic` rule, that
its temporary file is named after the destination rather than the writer and
every caller must stay serialised, now lives in
[knowledge/technical.md](../knowledge/technical.md).

## Acceptance

Met 2026-08-31.

- `cargo test` writes nothing into the live config directory.
- Two runs of one model at different contexts produce two rows, not one
  overwritten.
- A run that generated no tokens produces no row. A short warm-up produces a
  row that is stored and not ranked.
- A record stamped with a different build is shown, marked stale, and not
  ranked against current ones.
- Tune on Ornith reproduces `tools/fits.py --run`'s ordering and its margin.
- Tune refuses to start while a model is running, and a cancelled ladder
  leaves no `llama-server` on its port.
- Applying a suggestion fills the form and launches with those values;
  declining it leaves the form as it was.

## Verified

Verified 2026-08-31: all four checks passed after each parcel. The ladder
reproduced `tools/fits.py`'s ordering on the real 21 GB model. The author
confirmed the Speed panel's five corrections on screen on 2026-09-01: column
headers on the history table, a labelled "Generation" figure, the suggested
row marked in the list it is chosen from, no explainer shown once
measurements exist, and a single-sentence empty state.

## Out of scope

Named choices, which the redesign settled later ([redesign.md](redesign.md)),
per-field override markers, pi, search, and whether hunting and working are
one screen or two.

## Risks

- Ranking rows that did different work can be wrong. The workload gate and the
  shown workload column are the mitigation; a ranking that contradicts a Tune
  result on the same model is the signal to drop passive ranking, not to tune
  the gate.
- A full ladder takes minutes on a large model; the fallback, if that reads as
  too slow to press, is a Tune that measures only the current settings.
- The prompt is the measurement: everything here rests on it being long
  enough to measure throughput rather than startup. A ladder that spawns
  servers can strand one, so Tune's port stays inside the existing orphan
  scan.
