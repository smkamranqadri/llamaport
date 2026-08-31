# Screen

Planned 2026-08-31, the first phase cut from [direction.md](direction.md). It
does two things: stop the memory panel comparing against a number nothing
allocates from, and get the seven settings the author has never touched out of
the way.

Live status is in [state/current.md](../state/current.md), not here.

## What is wrong

**The memory panel measures against installed RAM.** 34.36 GB on this machine.
Nothing allocates from that figure. The GPU working set is 25,559 MiB and what
is actually free was 14.83 GB when this was found
([knowledge/technical.md](../knowledge/technical.md)). Every verdict the panel
has ever given was against the wrong ceiling.

**It explains itself in four lines of prose.** The author's words on reading
them: "I don't know thing making me confuse, I am not expert." That is the
sentence this phase exists to answer.

**Launch Settings offers ten controls and the author has used three.** Across 21
launches only context and port ever changed; layer offload, both cache types,
slots, flash attention, jinja and extra arguments were identical every time.

## What this phase deliberately is not

The mockup the author approved shows the app **deciding** the settings and
saying what it traded. That cannot ship yet, and the reason is a measurement
rather than a scheduling preference.

The only rule available without measuring is "largest context and most precise
cache that fits". Measured on Ornith it picks **the slowest of three
candidates** — 30.5 tok/s where 65,536 with a full-precision cache gives 41.6
([direction.md](direction.md)). Shipping it would replace a default the author
never chose with one this project has measured as wrong.

So the suggestion, the "chosen" labels and the Tune button all belong to the
next phase, where a measurement can make them true. **This phase is smaller than
the mockup on purpose**, and that is the cost of not shipping a bad default.

## Decisions

- **The ceiling is read, never computed.** `llama-server --list-devices` returns
  `MTL0: Apple M2 Pro (25559 MiB, 25558 MiB free)` with no model loaded. It goes
  on `Capabilities`, which is already probed once and cached and already reaches
  both the plan and the Settings screen.
- **A build that cannot report devices says so.** The panel shows the ceiling as
  unknown rather than falling back to installed RAM, because falling back is the
  bug. Gated on the probe like every other flag.
- **No suggestion, so no "chosen" labels.** Advanced shows the seven fields with
  their current values and lets them be edited. Nothing claims to have decided
  anything until something has measured it.
- **`tools/fits.py` is the cross-check.** The panel and the script read the same
  ceiling and size the same launch; where they disagree, one is wrong. The
  script already agrees with `estimate.rs` on two real files.

## Parcel 1 — the real ceiling — DONE 2026-08-31

`probe.rs` parses `--list-devices` into `Capabilities`. `lib.rs` carries it to
the plan. `sysmem.rs` already reports what is free and what is swapping; nothing
there changes.

## Parcel 2 — the panel is a glance

Four figures and no paragraph: what this launch wants, the GPU limit, what is
free now, swap. The bar marks the limit rather than installed memory. The seven
untouched fields move behind Advanced, still editable, and the three the author
uses stay in front.

## Proof — parcel 1, 2026-08-31

Four commands green, **202 tests**, up from 197. `Capabilities` now carries the
device list and `device_budget_mib()`; `LaunchPlan` carries it as
`deviceBudgetBytes`, 26,800,553,984 bytes on this machine, which is the 25,559
MiB `tools/fits.py` reads and not the 34.36 GB installed.

**Two of five new tests were weaker than they looked, and mutation testing is
what said so.**

- Removing the filter that ignores a device reporting no memory changed nothing,
  because `max()` over `[0, 25559]` is 25559 either way. The case that filter
  actually guards is a build reporting *only* memoryless backends, where without
  it the budget is `Some(0)` — which reads on screen as "nothing fits", a worse
  lie than "unknown". A test for that now exists and fails under the mutation.
- Gating the `--list-devices` call on the flag being present also survived, and
  that one is correct: an older build prints an error, `parse_devices` finds
  nothing, and the result is the same empty list. The gate saves a process spawn
  and changes no behaviour, so there is nothing to assert. Recorded rather than
  covered by a test that would only appear to test it.

The third mutation, collapsing total and free into one figure, failed the parse
test as it should.

## Out of scope

Tune and everything resting on it. Named choices. Hunting versus working as
separate screens — this phase touches what both would share and must not answer
that question. pi. Search.

## Acceptance

- `Capabilities` carries MTL0 at 25,559 MiB on this machine, read rather than
  computed, with a test over the parse.
- A build that does not report devices produces none, and the panel says the
  ceiling is unknown. Covered by a test, because the fallback is the defect.
- The panel shows Ornith at 65,536 with an f16 cache wanting 22.51 GB against a
  25.73 GB usable limit, 17.65 GB free, and swap.
- No paragraph remains in the memory panel.
- Launch Settings shows alias, port and context. The other seven are behind
  Advanced, still editable, and editing one still launches with it.
- A model whose cache cannot be sized still says so, as it does today.
- The panel's ceiling and its "wants" figure match `tools/fits.py` for the same
  model and settings.

## Verification

The four commands, each status captured on its own line and never after a pipe.
Every new test gutted and watched to fail
([knowledge/technical.md](../knowledge/technical.md)).

Then on screen. Every screen defect this project has found came from looking and
none from the suite, and this phase is almost entirely screen
([knowledge/technical.md](../knowledge/technical.md) carries that as a
constraint).

## Risks

- The glance has to carry what the prose was carrying. Whether four figures make
  a 7% overshoot legible is a judgement only the author can make, and it is the
  most likely thing to come back wrong.
- Assumes `--list-devices` exists on builds worth supporting. Where it does not,
  the answer is honesty rather than a computed guess.
- This phase looks like less than the mockup promised. If that reads as
  backwards rather than careful, the alternative is folding Tune in and doing
  one larger phase — bigger, slower to prove, and it was offered.
