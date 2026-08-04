# Current

```text
Branch:   `main`, clean, pushed, tagged `v0.2.1`. The tag, the tree and the
          published build agree.
Task:     none in progress. The download queue is done, released and installed.
Mode:     —
Blocker:  none. Two things owed rather than blocking: the README's "Open Anyway"
          steps have never met a real Gatekeeper prompt, and a queued row with
          nothing on disk behind it has never been seen coming back from a
          restart — every relaunch so far recovered rows that had a `.part`.
Next:     nothing planned. Same rule as after v0.2.0: the next move is whatever
          the release says — install friction, bug reports, or silence. Do not
          plan features against silence. A browser download of the `.dmg` would
          settle Gatekeeper in five minutes whenever it is wanted.
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
after a pipe. 177 tests, up from 164 before the queue.

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

Older proof is with the work it belongs to: the Persistence phase in
[intent/persistence.md](../intent/persistence.md), the v0.2.0 release review in
[intent/release.md](../intent/release.md).
