# Current

```text
Branch:   `main`, clean and pushed. **`v0.5.0` is tagged there and published**,
          and for once the tag and the artefacts agree by check rather than by
          assumption: `git describe --exact-match` ran before the build, and both
          `.dmg`s were downloaded back from the release and `cmp`-ed, exit 0 on
          each. **Tag with `git tag -a`** — `git tag` alone makes a lightweight
          tag, which `describe` ignores, so the check fails while nothing is
          wrong. That is what it caught on its first real outing
          ([intent/release.md](../intent/release.md)).
Task:     **Tune is built, seen, committed and released as `v0.5.0`**
          ([intent/tune.md](../intent/tune.md)). The suite no longer writes into
          `~/Library/Application Support/llamaport`; `speeds.json` has a store;
          the runner records a run when it settles; Tune measures a ladder that
          reproduces `tools/fits.py --run`'s ordering on Ornith; and the app now
          has one opinion, offered rather than applied.
Mode:     none in progress.
Blocker:  none. **Four standing items, all of one kind — something nobody has
          looked at.** Two are owed against the release and two are older.

          Owed, and both needing a human at the machine. The **Gatekeeper
          prompt**: `~/Downloads/Llamaport_0.5.0_aarch64.dmg` is sitting there
          **with quarantine set for the first time**, downloaded through Chrome,
          but macOS denies this agent's shell any access to that folder, so
          opening it is the author's. Installing from anywhere else sets no
          quarantine and proves nothing. And the **Dock click**, because a
          synthesized press cannot settle it.

          Older: a queued download row with nothing on disk behind it has never
          been seen coming back from a restart, and no Intel Mac has run the
          universal build.

          **Two things stopped being owed on 2026-09-01.** `/Applications` holds
          v0.5.0, where it had held v0.3.1 for three releases — which was the
          mechanical reason the rest kept going unmet. And five launches gave five
          usable windows, `1060x720` each, read from the window server rather than
          judged by eye: strong evidence the window bug is absent rather than the
          check as written, since `open -a` is not a Finder double-click and
          nothing confirmed no other app was fullscreen
          ([intent/release.md](../intent/release.md)).

          **`benchmarks.json` is left alone.** This app had a benchmark store
          once, deleted by `31031b2` as scope discipline and never recorded in
          KIS. The author decided 2026-08-31 to start clean, so `speeds.json` is
          new and that file stays where it sits. Now written down in
          [intent/roadmap.md](../intent/roadmap.md) and
          [intent/tune.md](../intent/tune.md), which is the part that was missing.
Next:     **Open `~/Downloads/Llamaport_0.5.0_aarch64.dmg`** and install from it,
          following only what the README says. That is the Gatekeeper check, owed
          since v0.1.0 and now a double-click. Then close the window and click the
          Dock icon.

          After that, the rest of [intent/direction.md](../intent/direction.md):
          the pi button, and search. Neither is planned.
```

Run and verify commands: [knowledge/technical.md](../knowledge/technical.md).
Plans and decisions: [intent/roadmap.md](../intent/roadmap.md).
What the app is for: [intent/direction.md](../intent/direction.md).

## Where the project actually is

The runner lists, launches, supervises and tests models; the downloader fetches
from Hugging Face with resume that survives a kill, a queue and a rate limit.
Both are done and specified — [docs/runner-spec.md](../../docs/runner-spec.md),
[intent/downloader.md](../intent/downloader.md).

Since 2026-08-31 the app sizes a launch against ceilings that are real. It reads
the GPU working set from the build rather than assuming installed memory, can
leave the context and layer offload unset for llama.cpp to fit, charges the cache
only to layers that hold one, and agrees with Finder about file sizes. The model
screen is four figures and a verdict where it was ten form fields and four lines
of prose.

Since 2026-09-01 it also measures. Every run that serves a request records what it
got, at the settings that got it; a ladder measures on request; and the app has
one opinion where it had none, offered rather than applied.

That is four phases, each carrying its own decisions and proof:
[figures.md](../intent/figures.md), [fitting.md](../intent/fitting.md),
[screen.md](../intent/screen.md), [tune.md](../intent/tune.md). What the app is
*for* was rewritten on 2026-08-31 by its only user, reversing two standing
decisions ([direction.md](../intent/direction.md)).

**Nothing has converted, and nobody has looked.** Stars, forks and watchers were
0 after the 2026-08-08 distribution push and have not been checked since — two
releases and a rewritten purpose later. Show HN is drafted and unposted,
deferred by the author rather than blocked
([intent/release.md](../intent/release.md)).

## Proof

The four commands green at `v0.5.0`, each status captured on its own line and
never after a pipe. **234 tests**, up from 202 at v0.4.0 and 180 at v0.3.1; the
thirty-two are Tune's ([intent/tune.md](../intent/tune.md)). Two suites need the real machine:
`real_launch` proves a run is recorded against the real binary and a real model,
and `real_tune` checks the candidate picker against `tools/fits.py` on the file
in the models directory, with the ladder itself behind `--ignored`.

Proof sits with the work it belongs to, not here. Every phase file carries its
own, including the mutation records: [figures.md](../intent/figures.md),
[fitting.md](../intent/fitting.md), [screen.md](../intent/screen.md),
[last-used.md](../intent/last-used.md),
[downloader.md](../intent/downloader.md), and each release's artefact proof in
[release.md](../intent/release.md).

**Two gaps, recorded rather than papered over.**
[persistence.md](../intent/persistence.md) marks three parcels done in 2026-08-03
with no proof section; it was not recorded at the time and inventing one now
would be worse than the gap. And **sixteen defects across four phases** were found
by looking at the built app, none by the suite — eleven on 2026-08-31 and five
more in Tune's panel on 2026-09-01. A constraint now, in
[knowledge/technical.md](../knowledge/technical.md).
