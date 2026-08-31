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
use already demonstrated. It also merges with
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

## The shape this implies

Not a plan. The pieces, so the plan has something to cut up.

- **The launch form shrinks** to what the author actually varies — a name, a
  port, and a context — plus one named choice that owns the rest.
- **The optimizer has an opinion the moment a model is opened**, worked out from
  the file and the machine. A **Tune** button measures for real, keeps the
  fastest, and remembers it for that model. Nobody waits minutes on a model they
  are about to discard.
- **pi gets a button.** Not automatic: the file is hand-edited and shared with
  four other providers, so writing behind the author's back is wrong. It should
  follow the conventions already in that file rather than invent one, and it
  has to deal with the 8080 collision.
- **The app should read the ceiling the way `tools/fits.py` does.** One call to
  `llama-server --list-devices` gives `MTL0: Apple M2 Pro (25559 MiB, 25558 MiB
  free)` with no model loaded. Until it does, the memory panel is comparing
  against installed RAM, which is the bug this direction started from.
- **Memory is a glance.** The current panel explains itself in four lines of
  prose. It should be a number, a bar, and nothing to read.

## Still out of scope

A chat UI. Non-macOS. API keys and binding anywhere but loopback. Managing the
llama.cpp installation — though "best performance" leans on the binary, so if
the optimizer keeps finding the build is the limit, that is the decision to
revisit next.

## Not settled

- What "best model" means, which item 7 cannot be built without.
- Whether a speed record is keyed on the model alone or on the model and its
  settings together. Only the second can answer "did that change help", which is
  the whole point of an optimizer.
- Whether hunting and working are two screens or one.
- What the named choices are called, and how many there are.
