# Current

```text
Branch:   `main`, working tree clean. **`v0.6.1` is tagged at `5d8957d` and HEAD
          has moved past it. Run `git rev-list --count v0.6.1..HEAD` before
          building a release artefact** — the distance is deliberately not
          written here, because the commit that records it increments it. Why
          it matters is in [knowledge/technical.md](../knowledge/technical.md).
Task:     **Redesign phases 2 and 3 are built and committed, and of the five
          things that did not match the artboards, four are done and signed
          off** — the stopped model screen (`e5e9f31`), the empty Library, the
          Library rows and the stray-server banner. The only look still marked
          owed is **phase 3's running view**, which the author's own five-item
          list never named — so it is probably closed and is not written down as
          such. Only **item 4, the Measure screen**, is unbuilt
          ([intent/redesign.md](../intent/redesign.md)). Phase 1 landed as
          `c6ac59f`.

          **One screen per task, each finished by the capture** — artboard
          rendered out of the canvas artifact, app captured, the two side by
          side ([knowledge/technical.md](../knowledge/technical.md)). Four
          screens have been signed off that way, against six wasted passes for
          one screen before the rule existed. **Ask the author to take the app
          half whenever they are at the machine**: it costs them a keystroke,
          costs the session nothing, and their eye is the acceptance test
          anyway.
Mode:     Phase — redesign, one item from the end of it.
Blocker:  none for the redesign. **The unusable-window bug has escalated and
          now wants its own task**: eight sightings on 2026-09-02, one of them
          with nobody touching the window. It is no longer one launch in three. **The recovery is scriptable after all**
          — an `osascript` clicking the tray's Show window restored it three
          times from the session, so it does not need a human at the machine.
          Two traps that cost time are recorded with it
          ([intent/roadmap.md](../intent/roadmap.md) risks): never resize the
          window through System Events, which collapses it outright, and never
          capture straight after a tray recovery, which returns a sheared image
          that lies about alignment.

          The **Gatekeeper check is met**, twice over on 2026-09-02 — the
          run that failed is why v0.6.1 exists
          ([intent/release.md](../intent/release.md)).

          **Four checks nobody has run**, each needing a human at the machine,
          listed in [intent/release.md](../intent/release.md) under "Unverified
          against v0.6.1".
Next:     **Item 4 of "What still does not match"** — the Measure screen, the
          last one. The artboard `Tune.dc.html` gives it a whole screen:
          "Measuring best speed", the four tries with their verdicts, Cancel
          and Use fastest so far. It was never built; the app runs the ladder
          inside a Speed row on the stopped model screen, which item 1 left
          showing only while a measurement runs, on purpose, for this
          ([intent/redesign.md](../intent/redesign.md)).

          The README's screenshots wait behind the whole redesign on the
          author's decision: every phase would invalidate them again, so they
          are done once, against the finished UI
          ([release.md](../intent/release.md) phase 3).

          Still unplanned: per-field override, search — blocked on what "best
          model" means — and **MoE launches**
          ([intent/moe.md](../intent/moe.md)), blocked on timing `-ncmoe`
          against the quant the author runs today.
```

Run and verify commands: [knowledge/technical.md](../knowledge/technical.md).
Where the project stands, and what is planned: [intent/roadmap.md](../intent/roadmap.md).
What the app is for: [intent/direction.md](../intent/direction.md).

## Proof

The four commands green, each status captured on its own line and never after a
pipe. **257 tests**, the newest being the orphan banner's alias, watched to fail
against a mutation ([intent/redesign.md](../intent/redesign.md)). Each phase file
carries the count it left behind, so the history is not repeated here.

Two suites need the real machine: `real_launch` proves a run is recorded against
the real binary and a real model, and `real_tune` checks the candidate picker
against `tools/fits.py` on the file in the models directory, with the ladder
itself behind `--ignored`.

Proof sits with the work it belongs to, not here. Every phase file carries its
own, including the mutation records: [tune.md](../intent/tune.md),
[figures.md](../intent/figures.md), [fitting.md](../intent/fitting.md),
[screen.md](../intent/screen.md), [last-used.md](../intent/last-used.md),
[downloader.md](../intent/downloader.md), [pi.md](../intent/pi.md), and each
release's artefact proof in [release.md](../intent/release.md).

**One gap, recorded rather than papered over.**
[persistence.md](../intent/persistence.md) marks three parcels done in 2026-08-03
with no proof section; it was not recorded at the time and inventing one now
would be worse than the gap.

The running defect tally, and the argument it makes about when a phase is done,
is a constraint in [knowledge/technical.md](../knowledge/technical.md).
