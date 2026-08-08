# Current

```text
Branch:   `main`, clean and pushed. `v0.3.0` is tagged two commits back, at the
          build record — the same shape as every release here; the code at the
          tag and the code at HEAD are identical.
Task:     none in progress. Last used is done, shipped and installed.
Mode:     —
Blocker:  none. Two standing items, both unchanged by this release: the README's
          "Open Anyway" steps have never met a real Gatekeeper prompt, and a
          queued row with nothing on disk behind it has never been seen coming
          back from a restart.
Next:     nothing planned. Same rule as after v0.2.0 and v0.2.1: the next move is
          whatever the release says — install friction, bug reports, or silence.
          Do not plan features against silence. A browser download of the `.dmg`
          would settle Gatekeeper in five minutes whenever it is wanted.
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

**v0.3.0 is published**, unsigned, as a GitHub pre-release with the `.dmg`
attached: https://github.com/smkamranqadri/llamaport/releases/tag/v0.3.0, and
installed in `/Applications`. Anyone still on v0.2.0 should upgrade for the path
traversal v0.2.1 fixed, which is live in that build; v0.1.0 is unaffected by that
one — it has no history file at all. Every release so far
([intent/release.md](../intent/release.md)).

Discover was planned and then dropped ([intent/roadmap.md](../intent/roadmap.md)).

Apart from the files named above, `git log` is the record.

## Proof

The four commands were last run green over the working tree: `cargo fmt
--check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`,
`bun run build` — all exit 0, each status captured on its own line rather than
after a pipe. 180 tests, up from 177 before Last used.

**v0.3.0 shipped 2026-08-08** and the published asset was downloaded back
byte-identical to what was built, as with all three releases before it. Every
build's artefact proof lives with its release in
[intent/release.md](../intent/release.md), including how a frontend-heavy change
had to be proved present when grepping the binary no longer could. Installing by
local copy sets no quarantine attribute, so no release yet has tested Gatekeeper.

Proof sits with the work it belongs to, not here:

- Last used, including what the author's screenshots settled that no test could
  reach — [intent/last-used.md](../intent/last-used.md).
- The download queue draining ~48 GB unattended —
  [intent/downloader.md](../intent/downloader.md).
- The Persistence phase — [intent/persistence.md](../intent/persistence.md).
- The v0.2.0 release review — [intent/release.md](../intent/release.md).
