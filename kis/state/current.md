# Current

```text
Branch:   `main`, clean, two docs commits ahead of `origin` and unpushed.
          `v0.3.0` is tagged behind them; the code at the tag and the code at
          HEAD are still identical — everything since it is documentation.
Task:     none in progress. Distribution is done bar one upload: the repository
          has a description, topics and a committed social preview card, the
          release is no longer a pre-release, the front page leads with a clip,
          and a Show and tell post is live in llama.cpp's own Discussions
          ([intent/release.md](../intent/release.md)).
Mode:     —
Blocker:  none. Three standing items, none touched by any of this: the README's
          "Open Anyway" steps have never met a real Gatekeeper prompt, a queued
          row with nothing on disk behind it has never been seen coming back
          from a restart, and no Intel Mac has run the universal build. One new
          bug, and unlike the old window one it reproduces on demand: closing
          the window leaves the app running and the Dock icon will not bring it
          back, only the Window menu will.
Next:     upload `docs/social-preview.png` in the repository settings. It is
          browser-only, nothing else can do it, and Reddit and Hacker News wait
          on it — both render the card, and neither gives a second first post.
          After that the same rule as after every release: the next move is
          whatever the audience says. Do not plan features against silence. A
          browser download of a `.dmg` would still settle Gatekeeper in five
          minutes whenever it is wanted.
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

**v0.3.0 is published**, unsigned, as the Latest GitHub release with **two
`.dmg`s** attached — `aarch64` and `universal`, the same build widened rather than
a second one: https://github.com/smkamranqadri/llamaport/releases/tag/v0.3.0. It
was a pre-release until 2026-08-08, when the badge was dropped without rebuilding
anything ([intent/release.md](../intent/release.md)). The aarch64
one is installed in `/Applications`. The app is macOS-only for reasons that are
Darwin's rather than ARM's ([knowledge/technical.md](../knowledge/technical.md)),
and no Intel Mac has run it. Anyone still on v0.2.0 should upgrade for the path
traversal v0.2.1 fixed, which is live in that build; v0.1.0 is unaffected by that
one — it has no history file at all. Every release so far
([intent/release.md](../intent/release.md)).

**The front page leads with a clip rather than a still.** `docs/launch.gif` runs
from the Library to Test model reporting every check passed, and is the first UI
artefact here the agent captured rather than the author. The Show and tell post
in llama.cpp's Discussions is the one outward-facing thing that goes stale on its
own: https://github.com/ggml-org/llama.cpp/discussions/26772 — edit it whenever a
release changes what it claims, as part of shipping rather than after it.

Discover was planned and then dropped ([intent/roadmap.md](../intent/roadmap.md)).

Apart from the files named above, `git log` is the record.

## Proof

The four commands were last run green over the working tree: `cargo fmt
--check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`,
`bun run build` — all exit 0, each status captured on its own line rather than
after a pipe. 180 tests, up from 177 before Last used.

**v0.3.0 shipped 2026-08-08** and both published assets were downloaded back
byte-identical to what was built, as with all three releases before it. The
universal one carries both slices and its x86_64 half was launched under Rosetta
and seen registering as a foreground app — which says the slice is real, and says
nothing about an Intel Mac ([intent/release.md](../intent/release.md)). Every
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
