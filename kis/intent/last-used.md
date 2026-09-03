# Last used

Asked for by the author on 2026-08-08 and completed the same day, shipped in
v0.3.0. The Library showed when a file landed on disk and nothing about when
it was last run, so there was no way to tell which models were in use.

## The ask

Not a new column. The Library row already ends with a muted relative time,
the file's mtime, and the ask was that this cell say something more useful
when there is something more useful to say. It became one recency cell: the
last launch when there has been one, the mtime otherwise.

## Decisions

- **The stamp is taken at Ready, not at spawn**, so a model that spawns and
  dies loading weights does not read "used today".
- **One cell, not a second column, told apart by weight rather than a
  label.** A real launch renders in normal text; the mtime fallback keeps
  the existing muted grey, and each carries its own `title`.
- **The list sorts on that same value, newest first, favourites still
  partitioned above**, with a model that has neither date sitting last in
  its partition in scan order. If the cell ever goes, the sort goes with it.
- **The field is `lastLaunched`.** `lastRun` is a retired key stripped from
  `extra`; reusing it would let serde adopt a timestamp from an old build.
  The map is never trimmed, and sorting happens in `arrange`, the only place
  that knows the config.
- **The value stored is `started_secs`, not the moment Ready arrived.** The
  two are indistinguishable at "today" resolution, and it makes the write's
  guard, `stamp_if_newer`, the same comparison as the write, giving one
  write per run despite Ready being re-emitted on every telemetry tick.

The shape: `last_launched: BTreeMap<String, u64>` in `Config`, schema 6 to
7. `ModelEntry` gains `last_launched_secs`, set by `arrange` the way
`favourite` already is.

## Acceptance

- A model launched to Ready shows its launch time, sorts to the top of its
  partition, and stays there after a restart. One that exits before Ready
  keeps its mtime, in grey, and does not move. A favourite with an old date
  still sorts above a non-favourite launched today.
- `config.json` holds one `lastLaunched` entry per launched model, unmoved
  while the server sits Ready. A v6 config loads at 7 with favourites,
  `lastUsed` and launch defaults intact, and `lastRun` is not adopted as the
  new field.

## Picked up on the way

Four row fixes the author asked for the same day, all from one cause: the
row has three flex children, star, row button, trailing button, and
everything that dressed the row had been put on the middle one. The running
highlight moved from `.model-row` to `.model-item`, after the hover rule so
hovering a running row keeps its tint. A running row offers Stop where the
others offer Delete, replacing a disabled button that only explained the
refusal. The now-redundant hover rule on `.model-row` was removed, and both
action buttons share one class, `.row-action`, and one width, so a running
row's figures no longer fall out of alignment.

## Out of scope

A sixth column. A header row for the list. Any change to the row's CSS grid.
`ModelDetail`. Stamping a Web UI open or a health test as use. A cap on the map.

## Verified

Verified 2026-08-08: all four checks passed. A real model was launched and
watched to move in the Library, `config.json` was read between runs to
confirm one write per run and correct migration from v6, and a pre-Ready
exit was forced to confirm the row does not move.
