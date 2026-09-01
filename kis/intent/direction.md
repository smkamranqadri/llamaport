# Direction

Set 2026-08-31 by the author, who is the app's only user. It replaces the scope
the project has been building against, and it was prompted by the author saying
a screen this app had just shipped was confusing — "I am not expert".

## What the app is for

Two jobs. They are not the same job and probably should not be the same screen.

- **Hunting.** Find a model, get it running fast, decide, move on. Most models
  get discarded. What matters is speed to a verdict.
- **Working.** Settle on one model and use it through **pi**. What matters is
  that it stays put, holds enough context to do real work, and that pi can
  reach it.

## What was asked for

1. Download a model. Pasting a URL today; searching later.
2. Launch it.
3. Get the best speed out of it — from the model where the file says, and by
   measuring where it does not.
4. See memory without opening Activity Monitor.
5. One place to choose: a default, an optimum, or what the model suggests.
6. One click to point pi at the running model.
7. Later: search for the best model, download it, run it.

## What the machine says

Read off the author's own config, not asked for:

- **Across 21 launches, two fields were ever changed: context and port.** Layer
  offload, both cache types, slots, flash attention, jinja and extra arguments
  were identical every single time. The launch form offers ten controls and
  seven of them have never been used.
- **`q8_0` for both cache types was inherited 21 times and never chosen.** It
  halves cache memory and costs some output quality. That is a trade the author
  did not know was being made, which is the clearest evidence that a default
  nobody picks is not a neutral thing.
- **pi can reach 2 of the 19 models this app has launched.** Its `local-llama`
  provider points at port 8888; 17 of 21 launches were on 8080. The file is
  hand-maintained and has fallen far behind.
- **pi has five local providers**, two of them (`mlx-lm`, `omlx`) pointing at
  **8080 — the port this app usually launches on**. `omlx` is the author's
  default provider. So a model launched here can be reached by pi under a name
  that describes something else entirely. There is also an enabled entry,
  `llama-cpp/qwen2.5-0.5b-instruct`, whose provider no longer exists.

## The measurement is already happening and is thrown away

Asked by the author on 2026-08-31 — are the run logs kept for analysis? They are
not, and it is worse than that.

- `last-run.log` is **wiped at the start of every run**. Only the most recent
  survives, capped to the ring buffer's size. No history exists.
- `config_dir()` has **no test override**, and `runner_lifecycle.rs` drives the
  real runner against a `python3 -m http.server` stand-in. So `cargo test`
  writes `last-run.log` and `runner.pid` into the live application support
  directory. The log sitting there now is test output. `config.json` and
  `downloads.json` are never written by that path, so what the author has tuned
  was never at risk — but the one artefact worth analysing is destroyed by
  running the suite.

**What that means for the optimizer.** Every run already prints its own speed:
`prompt eval time = 43.20 ms / 36 tokens (833.26 tokens per second)` and the
matching `eval time` line. The app separately scrapes `/metrics` on every poll
for the live view and discards that too. So for any model the author has already
used, its real speed at its real settings is knowable without benchmarking
anything at all.

This changes what **Tune** is. Measuring by launching a model several times is
the fallback, not the first move: the first move is to remember what ordinary
use already demonstrated.

**Half of that was reversed when Tune was planned** ([tune.md](tune.md)). A
record of ordinary use says what a model got at the settings it ran at; it cannot
say which setting is faster, because those runs did different work — which is the
same mistake as the 36-token prompt below, one level up. So the remembering is
evidence and the controlled run is the verdict, and both ship. It also merges with
[gaps.md](gaps.md)'s item on persisting telemetry, which was filed as a
nice-to-have and is now load-bearing.

## "Fit does not mean it works"

The author's objection on 2026-08-31, and it overturned the design that was
forming. Named choices — Fast, Long, Quality — were about to be built on a table
showing every model fitting comfortably at its best setting. That table compared
against **installed** memory, which is not a number anything allocates from.

Against the real ceilings the picture inverts
([knowledge/technical.md](../knowledge/technical.md)): the Metal working set is
26.80 GB, not 34.36; only 14.83 GB was actually available at the time; and swap
was 87% full. The inherited `q8_0` cache, which had just been described here as
a needless quality cost, is what lets a 35B hold its full 262,144-token context
on the GPU at all.

So the app cannot pick settings by arithmetic. A sum can say a launch is not
allowed; it can never say a launch is good. That makes measurement the spine of
the optimizer rather than a refinement of it, and it means the screen's job is
to show what is actually free right now — which is what the author asked for in
item 4 and is the same question.

## Measured: "largest that fits" is the wrong rule

`tools/fits.py --run` on Ornith 1.0 35B, 2026-08-31, one 3,794-token prompt for
every candidate so each did identical work:

     context  cache   generation   prompt
      65,536  f16     41.6 tok/s   508 tok/s   fastest prompt, ties fastest generation
       8,192  f16     41.7 tok/s   487 tok/s
     262,144  q8_0    30.5 tok/s   419 tok/s   what the arithmetic chose

**Arithmetic and measurement disagree by 27% of generation speed.** The rule this
direction was about to be built on — take the largest context and most precise
cache that fits — picks the slowest of the three. A full cache is expensive to
read through, and quantising it to buy context costs more than the context is
worth on this model.

**The measured winner is 65,536 with a full-precision cache, which is what the
author had been typing by hand for 17 of 21 launches.** The guess beat both the
app's default and the rule that was going to replace it.

**Three readings of the same question, each overturned by a better measurement.**
q8_0 was called a needless quality cost, then load-bearing for context, then
within 2% of f16 on speed — that last one measured against a 36-token prompt,
which is startup overhead rather than throughput. With a real prompt it is 27%
slower. Nothing here should be built on a single reading, and the small model was
a poor guide to the large one: it put the context penalty at 19% where the 35B
says 27%, with the winner in a different place.

That is the whole argument for Tune, and it is now a measurement rather than an
intuition.

It was written here as "the app suggests by arithmetic, marks the suggestion as
unmeasured, and Tune is what makes it true". **The first half was dropped when
Tune was planned** ([tune.md](tune.md)): a label is thin protection for a rule
measured as picking the slowest of three, and prose beside a figure is where
every defect of this day was found. Until something has been measured, the app
says nothing.

## Decisions this reverses

Both are written into [knowledge/project.md](../knowledge/project.md) and are
reversed deliberately, not overlooked.

- **"No profiles or presets" is reversed**, but not into a preset system. The
  seven untouched fields stop being the user's business and become the
  optimizer's: choosing Speed or Long context or Quality sets them, and the app
  says in one line what it chose. They stay reachable under Advanced for when
  something is broken. The old rule was right that a form full of knobs nobody
  sets is bad; it drew the wrong conclusion, which was to keep the knobs and
  refuse to have an opinion.
- **"Discover" comes back into scope**, having been dropped twice with a note
  asking that it not be planned a third time
  ([roadmap.md](roadmap.md)). The note's reason still stands — Hugging Face's
  `?search=` is a substring match over repo ids and would make a worse browser
  tab. So the ask is not "add a search box". It is item 7: **find the best model
  for this machine**, which is a different problem and needs fit, quantisation
  and speed to be part of the answer. If it cannot be better than the browser
  tab, it should not ship.

## The approved mockup

https://claude.ai/code/artifact/8d38ec5a-18fe-49ed-bf46-cdc7bf58620c — approved
by the author on 2026-08-31. Two tabs, and every figure in it measured against
Ornith on this machine rather than invented.

**All of it has now shipped, in two releases.** The memory panel, the collapsed
Advanced and the real ceiling landed in v0.4.0 ([screen.md](screen.md)); the
Tune button, its comparison table and the app stating a chosen setting landed in
v0.5.0 ([tune.md](tune.md)). Its "After Tune" tab shows measurement overruling
arithmetic by 27%; the app shipped showing 22% on the same model, measured on a
machine that was busier. That disagreement is the mockup's argument, and it
survived being measured again.

Three details in it were superseded rather than built. Its "Before launch" tab
shows the app suggesting before anything has measured, which [tune.md](tune.md)
decided against — that tab now reads as a model nobody has run does: what fits,
and no opinion. Advanced is drawn with a per-field override marker, which nothing
ships — the fields are editable but nothing marks one as the user's. And its
speed panel reports generation only; the prompt figures it once showed came from
a 36-token prompt and were withdrawn as meaningless. The shipped table reports
both, because prompt eval is the number that matters to an agent sending long
context.

## The shape this implies

Not a plan. The pieces, so the plan has something to cut up. **Five of the seven
are now built** — everything below marked shipped landed in v0.4.0 or v0.5.0,
except the pi button, which shipped in v0.6.0. What is left is the launch
form shrinking behind named choices, and per-field override.

- **The launch form shrinks** to what the author actually varies — a name, a
  port, and a context — plus one named choice that owns the rest.
- **The optimizer has an opinion the moment a model is opened.** *Shipped in
  v0.5.0, and half-reversed on the way*: the opinion comes from measurement
  rather than from the file and the machine, because arithmetic alone picks the
  slowest candidate. A model nobody has run gets no opinion. The **Tune** button
  measures for real and remembers it, so nobody waits minutes on a model they are
  about to discard.
- **pi gets a button.** *Shipped in v0.6.0* ([pi.md](pi.md)). Not
  automatic: the file is hand-edited and shared with four other providers, so
  writing behind the author's back is wrong — it shows a diff and writes on
  confirm. It follows the conventions already in that file rather than inventing
  one. The 8080 collision turned out not to be a collision: a `baseUrl` there is
  a declaration, not evidence anything is bound to the port, so the panel names
  the overlap and never refuses.
- **Tune is the script's `--run`, rewritten in Rust.** *Shipped in v0.5.0.* Not a
  shell-out: a shipped app cannot depend on python3 being present. `fits.py`
  stays the oracle the Rust is checked against, and it agrees with it on both the
  candidates and the ordering for a real 21 GB file.
- **Advanced is per-field override, not a form behind a toggle.** Each decided
  value is editable in place; touching one marks it the user's and the optimizer
  stops deciding that field for that model, with a way back to the suggestion.
  The existing rule carries it — a model opens on its last successful launch —
  so an override persists like a hand-set context does today.
- **The app should read the ceiling the way `tools/fits.py` does.** *Shipped in
  v0.4.0.* One call to `llama-server --list-devices` gives `MTL0: Apple M2 Pro
  (25559 MiB, 25558 MiB free)` with no model loaded, and the panel compares
  against that rather than against installed RAM — the bug this direction started
  from.
- **Memory is a glance.** *Shipped in v0.4.0.* The current panel explains itself in four lines of
  prose. It should be a number, a bar, and nothing to read.

## Still out of scope

A chat UI. Non-macOS. API keys and binding anywhere but loopback. Managing the
llama.cpp installation — though "best performance" leans on the binary, so if
the optimizer keeps finding the build is the limit, that is the decision to
revisit next.

## Not settled

- What "best model" means, which item 7 cannot be built without. Tune answers
  half of it for a model already on disk — what this machine gets out of it — and
  none of it for a model that is not.
- Whether hunting and working are two screens or one.
- What the named choices are called, and how many there are.
