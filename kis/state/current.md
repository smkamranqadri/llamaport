Figures, Fitting and Screen each carry their own proof
([intent/figures.md](../intent/figures.md),
[intent/fitting.md](../intent/fitting.md),
[intent/screen.md](../intent/screen.md)). 202 tests, up from 180 at v0.3.1.

**Eleven defects across the three phases were found by looking at the built app
and none by the suite** ([knowledge/technical.md](../knowledge/technical.md)
carries that as a constraint). The Screen ones are the sharpest: a value correct
on its own and wrong beside its neighbour, twice, one level apart — a verdict
that ignored free memory, then a free figure that ignored what was being asked of
it. No assertion can see either.

# Current

```text
Branch:   `main`, clean and pushed. `v0.4.0` is tagged at HEAD, and both built
          `.dmg`s came from that commit.
Task:     none in progress. **v0.4.0 is published** — Figures, Fitting and Screen
          together ([intent/release.md](../intent/release.md)). The app reads the
          real GPU ceiling, can leave the context and offload for llama.cpp to
          fit, sizes the cache by what each layer actually holds, and agrees with
          Finder about file sizes.
Mode:     —
Blocker:  none. Three items sit in the after-the-tag list against v0.3.2, all
          needing ten minutes with the display free and nothing fullscreen: five
          launches of the installed build, closing the window and clicking the
          Dock icon, and downloading a `.dmg` through a browser to meet a real
          Gatekeeper prompt. Two more are unrelated and unchanged: a queued row
          with nothing on disk behind it has never been seen coming back from a
          restart, and no Intel Mac has run the universal build.

          One thing the author should know rather than a blocker: the stored
          launch defaults hold `ctx 65536, ngl "all"`, and those seed every model
          never launched, so Auto stays inert for new models until Settings is
          changed. Not migrated, because a stored `port 8080` says those defaults
          were saved deliberately and the two fields inside them cannot be told
          apart from a choice.
Next:     **Tune** is the phase the approved mockup is waiting for and the one
          that makes a suggestion honest. Arithmetic picks 262,144 with a
          quantised cache for Ornith; measurement picks 65,536 at full precision,
          27% faster — the number the author had been typing by hand
          ([intent/direction.md](../intent/direction.md)). `tools/fits.py --run`
          already prototypes it, including the finding that a 0.5B mispredicts a
          35B in both the size of the penalty and where the winner sits.

          Then the rest of [intent/direction.md](../intent/direction.md): the pi
          button — pi reaches 2 of 19 launched models and points at a port the
          app rarely uses — and search. Neither is planned.

          Three conditions are owed against v0.4.0 and want ten minutes with the
          display free and nothing fullscreen: five launches of the installed
          build, closing the window and clicking the Dock icon, and downloading a
          `.dmg` through a browser to meet a real Gatekeeper prompt.
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

**v0.4.0 is published**, unsigned, as the Latest GitHub release with **two
`.dmg`s** attached — `aarch64` and `universal`:
https://github.com/smkamranqadri/llamaport/releases/tag/v0.4.0. It carries three
phases at once — Figures, Fitting and Screen — for the reason recorded in
[intent/release.md](../intent/release.md). Two releases shipped on 2026-08-31,
v0.3.2 and this. The app is macOS-only for reasons that are Darwin's rather than
ARM's ([knowledge/technical.md](../knowledge/technical.md)), and no Intel Mac has
run it. Anyone still on v0.2.0 should upgrade for the path traversal v0.2.1
fixed; v0.1.0 has no history file and is unaffected. Every release so far
([intent/release.md](../intent/release.md)).

**Nothing built has been installed since v0.3.1.** `/Applications` holds that
build, so the machine the author uses daily is two releases behind what the
repository does — which is also why the three after-the-tag conditions keep going
unmet.

**The front page leads with `docs/launch.gif`**, and the Show and tell post at
https://github.com/ggml-org/llama.cpp/discussions/26772 is the one outward-facing
thing that goes stale on its own — edit it whenever a release changes what it
claims. Both are owned by [intent/release.md](../intent/release.md).

**Nothing has converted yet.** Stars, forks and watchers were all still 0 at the
end of 2026-08-08, after the subreddit posts were up and drawing views. Views
without stars is the measurement that matters, and the honest reading of one
evening is that it is too early to read anything at all. The `homebrew/cask`
notability gate ([intent/release.md](../intent/release.md)) is untouched.

Discover was planned and then dropped ([intent/roadmap.md](../intent/roadmap.md)).

Apart from the files named above, `git log` is the record.

## Proof

The four commands green over the working tree at `v0.4.0`, each status captured
on its own line and never after a pipe: `cargo fmt --check`, `cargo clippy
--all-targets -- -D warnings`, `cargo test`, `bun run build`. **202 tests**, up
from 180 at v0.3.1.

`v0.4.0`'s artefacts are proved in [intent/release.md](../intent/release.md) —
both `.dmg`s downloaded back byte-identical, and the change provable on both
layers, the frontend by its bundle digest and the Rust by a string only it
introduces, appearing twice in the universal binary and once in the aarch64 one.

**Eleven defects across Figures, Fitting and Screen were found by the author
looking at the built app, and none by the suite.** The Screen pair states the
pattern most clearly: a verdict that ignored free memory, then a free figure that
ignored what was being asked of it — each correct alone, wrong beside its
neighbour, and unreachable by any assertion. That is a constraint now, in
[knowledge/technical.md](../knowledge/technical.md).

Each phase carries its own proof and its own mutation record:
[figures.md](../intent/figures.md), [fitting.md](../intent/fitting.md),
[screen.md](../intent/screen.md). `tools/fits.py` is the standing second opinion
on `estimate.rs` and agreed with it on two real files.

Proof sits with the work it belongs to, not here:

- Last used, including what the author's screenshots settled that no test could
  reach — [intent/last-used.md](../intent/last-used.md).
- The download queue draining ~48 GB unattended —
  [intent/downloader.md](../intent/downloader.md).
- The Persistence phase — [intent/persistence.md](../intent/persistence.md).
- The v0.2.0 release review — [intent/release.md](../intent/release.md).
