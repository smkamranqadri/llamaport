# Screen

Planned and completed 2026-08-31, shipped in v0.4.0. The first phase cut from
[direction.md](direction.md). It replaces the memory panel's ceiling and trims
Launch Settings to the controls the author actually uses.

## Purpose

The memory panel compared usage against installed RAM, 34.36 GB on the
author's machine, though nothing allocates from that figure
([knowledge/technical.md](../knowledge/technical.md)). Every verdict the
panel gave was against the wrong ceiling. It also explained itself in four
lines of prose the author found confusing, and offered ten launch controls
when 21 launches had only ever changed two of them.

## What this phase is not

The approved mockup shows the app deciding launch settings and stating what
it traded off. That could not ship here: the only rule available without
measurement, "largest context and most precise cache that fits", picks the
slowest of three candidates on Ornith, 30.5 tok/s where 65,536 with a
full-precision cache gives 41.6 ([direction.md](direction.md)). Shipping it
would replace a default the author never chose with one measured as wrong.
The suggestion, "chosen" labels, and Tune wait for the phase that measures.

## Decisions

- **The ceiling is read from `llama-server --list-devices`, never computed**,
  through `Capabilities`, which is probed once, cached, and already reaches
  both the plan and the Settings screen.
- **A build that cannot report devices shows the ceiling as unknown**, rather
  than falling back to installed RAM, which was the original bug.
- **No suggestion, so no "chosen" labels.** Advanced shows all seven
  remaining fields with their current values, editable.
- **`tools/fits.py` is the cross-check**, reading the same ceiling and sizing
  the same launch as the panel.
- **The verdict is the worse of two questions: the GPU ceiling and what is
  free right now.** Comparing only against the GPU ceiling let the panel
  print "fits" in green with 0 MB free and elevated swap pressure,
  reintroducing the gap this phase exists to close.

## What was built

- Parcel 1, the real ceiling: `probe.rs` parses `--list-devices` into
  `Capabilities`, carried to the plan by `lib.rs`.
- Parcel 2, the panel as a glance: four figures and no paragraph, marking the
  bar against the GPU limit rather than installed memory. The seven untouched
  fields move behind Advanced; the three the author uses stay in front.

## Acceptance

- `Capabilities` carries the device ceiling read from `--list-devices`, with
  a test over the parse, and a build that cannot report devices produces
  none, so the panel says the ceiling is unknown.
- No paragraph remains in the memory panel; Launch Settings shows alias, port
  and context in front with the rest behind Advanced, still editable.
- The panel's ceiling and "wants" figure match `tools/fits.py`.

## Verified

Verified 2026-08-31: all four checks passed. The author confirmed the three
panel states on screen: green "fits" with memory to spare, amber "fits the
GPU, not what is free" under a smaller margin, and red "fits, but the
machine is under pressure" with no memory free.

## Out of scope

Tune and everything resting on it. Named choices. Hunting versus working as
separate screens. pi. Search.

## Risks

Whether four figures make a small overshoot legible on screen is a judgement
only the author can make. The phase also assumes `--list-devices` exists on
builds worth supporting; where it does not, the panel reports unknown.
