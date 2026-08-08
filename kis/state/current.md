# Current

```text
Branch:   `main`, tagged `v0.2.1` at the last commit. Working tree carries the
          Last used work, uncommitted and unreleased.
Task:     **Last used** — done 2026-08-08, verified in the running app.
          [intent/last-used.md](../intent/last-used.md).
Mode:     — (it was Standard; the work is closed).
Blocker:  none. Owed rather than blocking: `docs/library.png` is now stale — it
          is on the README's front page and shows the old last column, the old
          within-group order and no Stop button. Only the author can retake it
          (`screencapture -o -w docs/library.png`), the same denial
          [intent/release.md](../intent/release.md) already records. Then two CSS
          fixes made last and not yet looked at — the hover no longer splitting
          into two colours on a running row, and that row's figures lining up
          with the others. Plus the two standing ones: the README's "Open Anyway"
          steps have never met a real Gatekeeper prompt, and a queued row with
          nothing on disk behind it has never been seen coming back from a
          restart.
Next:     commit, and decide whether this rides alone or waits for company. It
          is a schema bump, so a build carrying it cannot be downgraded without
          the older build ignoring `lastLaunched` — additive, but worth saying in
          release notes. Nothing else planned.
```

Run and verify commands: [knowledge/technical.md](../knowledge/technical.md).
Plans and decisions: [intent/roadmap.md](../intent/roadmap.md).

## Where the project actually is

The runner lists, launches, supervises and tests models. The downloader fetches
from Hugging Face in four ranged segments, survives a kill, resumes from its
sidecar, verifies sha256 and lands the file in the models directory.

Downloads outlive the app and now queue. A second URL waits its turn instead of
being refused, and every way a transfer ends — finished, failed, paused,
discarded — starts the next one ([intent/downloader.md](../intent/downloader.md)).
A transfer is paused or discarded rather than cancelled, an interrupted one comes
back from the `.part` on disk, and what has no `.part` is remembered by
`downloads.json` instead. The Library stars models and deletes them to the Trash.
Settings holds the values a never-launched model opens on
([intent/persistence.md](../intent/persistence.md)).

The Library also orders itself by what you have been running. Its last cell shows
the last launch where there has been one and the file's mtime otherwise, told
apart by weight, and the list sorts on that value with favourites still above
([intent/last-used.md](../intent/last-used.md)). A model is dated only by a run
that reached Ready. The running row is tinted end to end and ends in Stop rather
than the Delete every other row ends in.

A ready model offers **Web UI**, which opens `llama-server`'s own interface in a
second app window. The app has no chat of its own and is not getting one
([knowledge/project.md](../knowledge/project.md)).

**v0.2.1 is published**, unsigned, as a GitHub pre-release with the `.dmg`
attached: https://github.com/smkamranqadri/llamaport/releases/tag/v0.2.1. Its
notes tell v0.2.0 users to upgrade, because the path traversal it fixes is live
in that build. v0.1.0 is unaffected by that one — it has no history file at all —
and should upgrade for the v0.2.0 fixes instead
([intent/release.md](../intent/release.md)).

Discover was planned and then dropped ([intent/roadmap.md](../intent/roadmap.md)).

Apart from the files named above, `git log` is the record.

## Proof

The four commands were last run green over the working tree: `cargo fmt
--check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`,
`bun run build` — all exit 0, each status captured on its own line rather than
after a pipe. 180 tests, up from 177 before Last used. `$PIPESTATUS` is a bash
name and this is zsh; it came back empty and reported nothing, which is the same
trap under a new spelling.

The published `.dmg` was downloaded back from GitHub and is byte-identical to
what was built. Mounted, it carries `Llamaport.app` at 0.2.1 with the right
identifier; the binary contains `is already in the queue` and no longer contains
`this app downloads one file at a time`, so the queue is provably in the
artefact rather than only in the tree. 0.2.1 is also installed over the 0.2.0 in
`/Applications` — by local copy, which sets no quarantine attribute and
therefore still does not test Gatekeeper.

Queue, 2026-08-04:

- **A four-deep queue drained itself in the running app**, unattended, one file
  at a time and in the order it was given: Ternary-Bonsai 27B, then 8B, then 4B,
  then 1.7B, alongside North-Mini-Code resuming from a 19 GB `.part` and
  finishing. Around 48 GB through four consecutive hand-offs without a click.
  That is the invariant, proved by the app rather than by the suite.
- Queueing, the advance on a pause, and Discard were each confirmed on screen.
  Discard had never been looked at in the running app before this.
- **Resume takes its turn**: two paused rows recovered after a restart, both
  clicked, one started and the other waited.
- Three defects, none found by the suite. One by reading the code — a path
  traversal live in v0.2.0 — and two by reading `downloads.json` between runs:
  a restored row that zeroed a byte count its `.part` contradicted, and a queue
  that survived exactly one restart. Each has a test that fails without its fix.

Last used, 2026-08-08, in the dev app against the real models directory and the
author's own live config:

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
- Not proven by me: that the launched cell reads differently from the muted one
  (the author confirmed that on screen — the accessibility tree carries no
  styling and screenshots were blocked at both Orca and `screencapture`), and
  the last two CSS fixes — the hover no longer splitting into two colours, and
  the running row's figures lining up with the rest. Those two build clean and
  their causes were read off the stylesheet, but nobody has looked at them.

Older proof is with the work it belongs to: the Persistence phase in
[intent/persistence.md](../intent/persistence.md), the v0.2.0 release review in
[intent/release.md](../intent/release.md).
