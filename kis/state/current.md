# Current

```text
Branch:   `main`, clean and pushed. `v0.4.0` is tagged at `6735316`, **four memory
          commits behind HEAD** — both `.dmg`s were built from the tag, not from
          HEAD. Check `git describe --exact-match` before building a release
          artefact: State claiming tag and HEAD were identical is what let a build
          of HEAD reach the v0.3.0 release
          ([intent/release.md](../intent/release.md)).
Task:     **Tune is built and seen — all five parcels**, uncommitted
          ([intent/tune.md](../intent/tune.md)). The suite no longer writes into
          `~/Library/Application Support/llamaport`; `speeds.json` has a store;
          the runner records a run when it settles; Tune measures a ladder that
          reproduces `tools/fits.py --run`'s ordering on Ornith; and the app now
          has one opinion, offered rather than applied.
Mode:     Phase, complete. Nothing is committed: 14 files changed, 6 added.
Blocker:  none. **The Speed panel was looked at on 2026-09-01, five defects came
          out of it, and the fixes were confirmed on screen by the author**
          ([intent/tune.md](../intent/tune.md)). None of the five was visible to
          the suite; four were in the panel as first written and the fifth
          appeared only after fixing those, on a different model.

          **`benchmarks.json` is left alone**: this app had a benchmark store
          once, deleted by `31031b2` as scope discipline and never recorded in
          KIS. The author decided 2026-08-31 to start clean, so `speeds.json` is
          new and that file stays where it sits. Written down now in
          [intent/roadmap.md](../intent/roadmap.md) and
          [intent/tune.md](../intent/tune.md), which is the part that was missing.

          Five standing items, all of one kind — something nobody has looked at.

          Three are owed against v0.4.0 and want ten minutes with the display
          free and nothing fullscreen: five launches of the installed build,
          closing the window and clicking the Dock icon, and downloading a `.dmg`
          through a browser to meet a real Gatekeeper prompt. **`/Applications`
          still holds v0.3.1**, so the machine used daily is two releases behind
          the repository — the mechanical reason these keep going unmet.

          Two are unrelated and older: a queued download row with nothing on disk
          behind it has never been seen coming back from a restart, and no Intel
          Mac has run the universal build.
Next:     **Commit Tune, then decide what its release is** — `v0.4.0` is tagged
          well behind HEAD and `/Applications` still holds `v0.3.1`. After
          that, the rest of [intent/direction.md](../intent/direction.md): the pi
          button, and search. Neither is planned.

          Three rows for Ornith are in the author's real `speeds.json`, written by
          a Tune run this session started without being asked — legitimate data at
          the author's own settings, kept rather than deleted, and easily removed
          if unwanted. Two of the plan's
          decisions go against the approved mockup and are argued in
          [intent/tune.md](../intent/tune.md): no suggestion until something has
          measured, and rows from ordinary use may be ranked under a workload
          gate.

          After Tune, the rest of [intent/direction.md](../intent/direction.md):
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

Since 2026-08-31 the app also sizes a launch against ceilings that are real. It
reads the GPU working set from the build rather than assuming installed memory,
can leave the context and layer offload unset for llama.cpp to fit, charges the
cache only to layers that hold one, and agrees with Finder about file sizes. The
model screen is four figures and a verdict where it was ten form fields and four
lines of prose.

That is three phases, each carrying its own decisions and proof:
[figures.md](../intent/figures.md), [fitting.md](../intent/fitting.md),
[screen.md](../intent/screen.md). What the app is *for* was rewritten the same
day by its only user, reversing two standing decisions
([direction.md](../intent/direction.md)).

**Nothing has converted.** Stars, forks and watchers were 0 after the 2026-08-08
distribution push and nobody has checked since. Show HN is drafted and unposted,
deferred by the author rather than blocked
([intent/release.md](../intent/release.md)).

## Proof

The four commands green at `v0.4.0`, each status captured on its own line and
never after a pipe. **202 tests**, up from 180 at v0.3.1. Green again on HEAD at
**234**, the thirty-two new tests being Tune's
([intent/tune.md](../intent/tune.md)). Two suites need the real machine:
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
would be worse than the gap. And eleven defects across the three phases of
2026-08-31 were found by the author looking at the built app, none by the suite —
a constraint now, in [knowledge/technical.md](../knowledge/technical.md).
