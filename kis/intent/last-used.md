# Last used

Asked for by the author on 2026-08-08, using the app: the Library shows when a
file landed on disk and nothing about when it was last run, so there is no way
to see which models are actually in use and which are dead weight.

Live status is in [state/current.md](../state/current.md), not here.

## What the ask turned out to be

Not a new column. The Library row already ends with a muted relative time —
the file's mtime — and the ask is that this cell say something more useful when
there is something more useful to say. So it becomes one **recency** cell: the
last launch when there has been one, the mtime otherwise.

The data did not exist. `Config.last_used` is a map of `Profile`, keyed on the
catalog identity, and carries no time at all.

## Decisions

- **Ready, not spawn.** `runner.start` returns as soon as the process exists —
  state Starting — which is where `last_used` is written today. A model that
  spawns and dies loading weights would read "used today" from that point. The
  stamp is taken when the run reaches Ready instead, so the column claims only
  what happened.
- **One cell, not a sixth column.** Two unlabelled relative times side by side
  is a puzzle in a list with no header row, and the row already carries a star
  and a Delete button.
- **The two meanings are told apart by weight, not by a label.** A real launch
  renders in normal text, the mtime fallback keeps the existing muted grey, and
  each carries its own `title`. Without that the merged cell says "today" for two
  different facts with nothing on screen to separate them — which is the reason
  a merge was argued against before it was chosen.
- **The list sorts by that same value, newest first**, favourites still
  partitioned above. This retires `arrange`'s "never reshuffle" rule rather than
  contradicting it: a list may reorder for a reason the user can see, and the
  cell it sorts on is on the row. If the cell ever goes, the sort goes with it.
- **A model with neither date sits last** in its partition, in scan order. The
  sort is stable, so nothing else moves for no reason.
- **The field is `lastLaunched`.** `lastRun` is retired in `migrate` and stripped
  from `extra`; a real field of that name would be claimed by serde first and
  adopt a timestamp from a build several schemas old.
- **The map is never trimmed.** One `u64` per model ever launched, kept even when
  the file is gone — the same reasoning already written down for `favourites`:
  the volume may not be mounted, and forgetting is unrecoverable.
- **Sorting happens in `arrange`**, not in the row. It is the only thing that
  knows the config, and it is the half of this that can be tested at all.

## Shape — DONE 2026-08-08

`last_launched: BTreeMap<String, u64>` in `Config`, schema 6 -> 7, written by
the `runner:state` listener when a snapshot arrives Ready. Telemetry re-emits
Ready on every tick, so the write is guarded: `stamp_if_newer` takes it only when
the stored stamp predates this run's `started_secs`, which makes it one write per
run. The decision is a pure function so it is testable; the listener stays glue.

The value stored is `started_secs` itself rather than the moment Ready arrived —
a few seconds earlier, indistinguishable at "today" resolution, and it makes the
guard the same comparison as the write, so no clock reading enters the function
at all. The plan had it taking a `now`; it does not need one.

`ModelEntry` gains `last_launched_secs`, set by `arrange` the way `favourite`
already is. Both times reach the screen and the row decides which to show.

Touches `store.rs`, `catalog.rs`, `lib.rs`, `types.ts`, `Library.tsx`.

## Acceptance

- A model launched to Ready shows its launch time and sorts to the top of its
  partition, and is still there after restarting the app.
- A model that spawns and exits before Ready keeps showing its mtime, in grey,
  and does not move.
- A model with neither date sits at the bottom of its partition in scan order.
- A favourite with an old date still sorts above a non-favourite launched today.
- `config.json` holds one `lastLaunched` entry per launched model and the value
  does not move while the server sits Ready.
- A v6 config loads at 7 with favourites, `lastUsed` and launch defaults intact,
  and `lastRun` is not adopted as the new field.

## Picked up on the way

All four asked for by the author on 2026-08-08 while looking at the list this
work reordered. They are one story: the row has three flex children — star, row
button, trailing button — and everything that dressed the row had been put on the
middle one.

- **The running highlight stopped short at both ends.** It was on `.model-row`.
  Moved to `.model-item`, which is where the row's border already lives for
  exactly this reason, and placed after the hover rule so hovering a running row
  does not take the tint away.
- **A running row offers Stop where the others offer Delete.** Deleting it was
  refused anyway, so that slot held a disabled button explaining the refusal;
  it now holds the thing you were going to go looking for. Not a quiet button:
  the quiet ones are invisible until the row is hovered, and the running row is
  the one worth acting on without hunting.
- **Hover was two colours on a running row.** `.model-row:hover` survived the
  move and painted opaque `--bg-hover` over the middle child only, so a hovered
  running row read accent, grey, accent. Deleted — `.model-item:hover` already
  covers the full width.
- **The running row's figures fell out of line.** "Stop" is a shorter word than
  "Delete", and the row button beside it is `flex: 1`, so the grid grew and took
  its right-aligned figures with it. Both buttons now share `.row-action` and one
  width.

## Out of scope

A sixth column. A header row for the list. Any change to the row's CSS grid.
`ModelDetail`. Stamping a Web UI open or a health test as use. A cap on the map.

## Verification

The four commands in [knowledge/technical.md](../knowledge/technical.md), green,
each status captured directly. Each new test checked against a gutted
implementation. Then the app itself, because tests have never been enough to
close work here: launch a real model and watch it move, read `config.json`
between runs, and force a pre-Ready exit with a `rawArgs` flag `llama-server`
rejects to confirm nothing moves.

## Proof — 2026-08-08

Gathered in the dev app against the real models directory and the author's own
live config, then closed against the built 0.3.0. Moved here from State once the
release shipped.

- **The list reordered itself around two real launches.** Before: the starred
  Q3_K_XL on top at 14 days, then 2 / 4 / 4 / 4 / 5 days. qwen2.5-0.5b was last.
  After launching it, it sat second at "today"; after Bonsai-27B it sat second
  and the 0.5b third — both reading "today" and still ordered correctly against
  each other, 17:43:34 above 17:35:40. The favourite never moved off the top
  despite being the oldest thing in the list.
- **The sort runs on real seconds, not on the rendered words.** The three Bonsai
  files all read "4 days ago" and ordered 1.7B, 8B, 27B — their mtimes are 04:16,
  04:02, 03:41.
- **A launch that never reached Ready left no mark.** Ternary-1.7B was tried and
  did not come up; its id is absent from `lastLaunched`, and its row still reads
  its own mtime. This is the acceptance check that was expected to need staging,
  and it arrived by accident instead.
- **One write per run, proved by the file rather than the test.** `config.json`
  was stamped at 17:35:41 and its mtime had not moved three minutes later with
  the server still Ready and telemetry ticking throughout. Same again for the
  second run: written 17:43:39, unchanged two minutes on.
- The live config went 6 -> 7 in place with `favourites`, `lastUsed` (12 models)
  and `launchDefaults` all intact, and `lastLaunched` holding only the models
  that actually served.
- Each of the three new tests was watched to fail against a gutted
  implementation: the guard removed (`the same run was written twice`), the field
  renamed to `lastRun` (`a retired key was adopted as the new one` — the trap is
  real, a v1 config's 2026 timestamp was adopted), and the mtime fallback dropped
  from `recency` (the order collapsed).
- **A later run replaces the earlier one rather than adding a row.** The 27B was
  launched again at 17:58:10 and its 17:43:34 stamp was overwritten in place. Two
  models launched, three successful runs, two entries.
- **Stop was clicked in the Library and the server went away**: one
  `llama-server` before, none after, the sidebar back to "No model running", the
  row back to Delete. The row kept its "today" and its position, because stopping
  is not un-launching. Exactly one Stop appeared in the list, on the running row.
- **Everything visual was closed by the author's screenshots of the built 0.3.0**,
  after the accessibility tooling was uninstalled mid-session and neither it nor
  `screencapture` could reach the window. In one frame: the running Bonsai-27B
  row tinted from the star at one end to its Stop button at the other, the row
  below it hovered and uniformly one colour, "today" bright against a muted "14
  days ago", and every recency value flush right including the two rows carrying
  buttons. That settles the weight distinction, the tint, the hover and the
  alignment — the four things no test could reach.
