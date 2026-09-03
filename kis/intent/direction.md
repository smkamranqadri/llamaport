# Direction

Set 2026-08-31 by the author, who is the app's only user. It replaced the scope
the project had been building against.

## The two jobs

- **Hunting.** Find a model, get it running fast, decide, move on. Most models
  are discarded; what matters is speed to a verdict.
- **Working.** Settle on one model and use it through pi. What matters is that
  it stays put, holds enough context for real work, and that pi can reach it.

## What was asked for

1. Download a model, pasting a URL today and searching later. Search shipped
   2026-09-03 as Discover ([discover.md](discover.md)).
2. Launch it. Done; see [roadmap.md](roadmap.md).
3. Get the best speed out of it, from the model where the file says and by
   measuring where it does not. Shipped in v0.5.0 as Tune ([tune.md](tune.md)).
4. See memory without opening Activity Monitor. Shipped in v0.4.0
   ([screen.md](screen.md)).
5. One place to choose: a default, an optimum, or what the model suggests.
   Shipped as three named choices in the redesign: Default, Best speed, Model
   suggested ([redesign.md](redesign.md)).
6. One click to point pi at the running model. Shipped in v0.6.0
   ([pi.md](pi.md)).
7. Later: search for the best model, download it, run it. Shipped 2026-09-03 as
   Discover, answering fit and quantisation; speed is left to Tune once a file
   is on disk ([discover.md](discover.md)).

## Evidence from the author's own config

- Across 21 launches, only two fields were ever changed: context and port. The
  other seven launch controls were never touched.
- `q8_0` for both cache types was inherited 21 times and never chosen. It
  halves cache memory at some cost to output quality, a trade the author did
  not know was being made.
- pi could reach only 2 of the 19 models this app has launched: its
  `local-llama` provider pointed at port 8888, while 17 of 21 launches used
  8080.
- pi has five local providers, two of which (`mlx-lm`, `omlx`, the author's
  default) also point at port 8080, so a model launched here could be reached
  under a name that describes something else.

## "Fit does not mean it works"

The author's objection on 2026-08-31 overturned a design in progress: named
choices (Fast, Long, Quality) were about to be built on a table showing every
model fitting comfortably, compared against installed memory rather than the
real ceiling. Against the real ceiling the picture reverses: the Metal working
set was 26.80 GB, not 34.36 GB, and only 14.83 GB was actually available
([knowledge/technical.md](../knowledge/technical.md)). The `q8_0` cache is what
let a 35B model hold its full 262,144-token context on the GPU at all.

Arithmetic can say a launch is not allowed. It cannot say a launch is good.
That makes measurement the basis of the optimizer, and it means the screen's
job is to show what is actually free right now, which answers item 4 above.

## Measured: "largest that fits" is the wrong rule

`tools/fits.py --run` on Ornith 1.0 35B, 2026-08-31, one 3,794-token prompt for
every candidate, so each did identical work:

| Context | Cache | Generation | Prompt |
|---|---|---|---|
| 65,536 | f16 | 41.6 tok/s | 508 tok/s |
| 8,192 | f16 | 41.7 tok/s | 487 tok/s |
| 262,144 | q8_0 | 30.5 tok/s | 419 tok/s (arithmetic's choice) |

Arithmetic and measurement disagree by 27% of generation speed. Taking the
largest context and most precise cache that fits picks the slowest of the
three: a full cache is expensive to read through, and quantising it to buy
context costs more than the context is worth on this model.

The measured winner, 65,536 with a full-precision cache, is what the author had
been typing by hand for 17 of 21 launches.

Every run already reports its own speed in its log output. The app now records
that instead of discarding it ([tune.md](tune.md)).

## Decisions reversed

- **No profiles or presets, reversed** ([knowledge/project.md](../knowledge/project.md)).
  Not into a preset system: the seven untouched fields stop being the user's
  business and become the optimizer's. Choosing a named preset sets them, and
  the app states what it chose. They stay reachable under Advanced.
- **Discover, back in scope.** It had been dropped 2026-08-02
  ([roadmap.md](roadmap.md)) for a reason that still stands: Hugging Face's
  `?search=` is a substring match over repo ids and would be a worse browser
  tab. What shipped 2026-09-03 is a different problem, finding the best file
  for this machine by fit and quantisation, which a browser tab cannot do
  ([discover.md](discover.md)).

## The approved mockup

https://claude.ai/code/artifact/8d38ec5a-18fe-49ed-bf46-cdc7bf58620c, approved
2026-08-31, with every figure measured against Ornith on this machine. Most of
it shipped: the memory panel, collapsed Advanced and real ceiling in
v0.4.0 ([screen.md](screen.md)); the Tune button, its comparison table and the
stated choice in v0.5.0 ([tune.md](tune.md)). Three details were superseded: a
suggested setting shown before anything has measured, a per-field override
marker, and a speed panel reporting generation only, since the shipped table
reports both prompt and generation speed.

## Still out of scope

A chat UI. Non-macOS support. API keys and binding anywhere but loopback.
Managing the llama.cpp installation, though if the optimizer keeps finding the
build itself is the limit, that is the decision to revisit.

## Not settled

- What "best model" means. Discover answers the narrower question, the best
  file in a known repository, by fit and quantisation; it leaves the choice of
  repository to the reader and to Hugging Face's own rankings
  ([discover.md](discover.md)).
- Whether hunting and working are two screens or one.
- Per-field override under Advanced, drawn in the mockup, is not built. The
  fields are editable, but nothing marks one as the user's.
- The named choices themselves are settled: Default, Best speed, Model
  suggested.
