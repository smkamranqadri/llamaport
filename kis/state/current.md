# Current

```text
Branch:   `main`, clean and pushed. **`v0.6.1` is tagged at `5d8957d`, behind
          HEAD by however many memory commits have landed since** — the
          artefacts were built from the tag, when the two were the same commit.
          **Run `git rev-list --count v0.6.1..HEAD` before building a release
          artefact**; the count is deliberately not written here, because
          recording it once made it wrong three times in one day — the commit
          that writes the number increments it. Assuming tag and HEAD are
          identical is what put a build of HEAD in the v0.3.0 release
          ([knowledge/technical.md](../knowledge/technical.md)).
Task:     **Redesign phases 2 and 3 are built and committed, and of the five
          things that did not match the artboards, four are done** — the empty
          Library (pulled forward out of phase 4 by the author), the stopped
          model screen, the Library rows and the stray-server banner. Only
          **item 4, the Measure screen**, is left
          ([intent/redesign.md](../intent/redesign.md)). Phase 1 landed as
          `c6ac59f`.

          **The method now works.** The artboards are rendered out of the
          canvas artifact and screenshotted headless, the app is captured by
          window id, and the two go side by side before anything is called
          done ([knowledge/technical.md](../knowledge/technical.md)). Three
          screens were signed off that way in one session, against six wasted
          passes for one screen before it.

          One screen per task, each finished by the capture. The six-pass
          history that established the rule is in
          [intent/redesign.md](../intent/redesign.md), not here.
Mode:     Phase — redesign, one item from the end of it.
Blocker:  none for the redesign. **The unusable-window bug has escalated and
          now wants its own task**: four more sightings on 2026-09-02, three
          on consecutive launches, where it used to be one in three and a
          restart always cleared it. It blocked a capture outright — the
          tray's Show window is the only recovery and it needs a human at the
          machine ([intent/roadmap.md](../intent/roadmap.md) risks).

          The **Gatekeeper check is met**, twice over on 2026-09-02 — the
          run that failed is why v0.6.1 exists
          ([intent/release.md](../intent/release.md)).

          **Four things nobody has looked at.** Two owed against v0.6.1 and
          needing a human at the machine: **five launches from Finder** with
          nothing fullscreen, and the **Dock click** — the 2026-09-01 evidence
          was against v0.5.0 and does not carry. Two older: a queued download row
          with nothing on disk behind it coming back from a restart, and an Intel
          Mac running the universal build.
Next:     **Item 4 of "What still does not match"** — the Measure screen, the
          last one. The artboard `Tune.dc.html` gives it a whole screen:
          "Measuring best speed", the four tries with their verdicts, Cancel
          and Use fastest so far. It was never built; the app runs the ladder
          inside a Speed row on the stopped model screen, which item 1 left
          showing only while a measurement runs, on purpose, for this
          ([intent/redesign.md](../intent/redesign.md)).

          **Item 1's look is still owed** — the stopped model screen was
          committed unverified and marked so; the author has said he will
          check it when the work reaches that screen. The README's
          screenshots, previously here, wait behind the whole redesign on the
          author's decision: every phase would invalidate them again, so they
          are done once, against the finished UI
          ([release.md](../intent/release.md) phase 3).

          The "named choices" remainder of
          [intent/direction.md](../intent/direction.md) is now owned by
          redesign phase 2. Still unplanned after it: per-field override,
          search — blocked on what "best model" means — and **MoE launches**
          ([intent/moe.md](../intent/moe.md)), blocked on timing `-ncmoe`
          against the quant the author runs today.
```

Run and verify commands: [knowledge/technical.md](../knowledge/technical.md).
Where the project stands, and what is planned: [intent/roadmap.md](../intent/roadmap.md).
What the app is for: [intent/direction.md](../intent/direction.md).

## Proof

The four commands green, each status captured on its own line and never after a
pipe. **256 tests**; the twenty-two newest are the pi button's, four of them
watched to fail against a mutation ([intent/pi.md](../intent/pi.md)). Each phase
file carries the count it left behind, so the history is not repeated here.

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

Every defect this project has found came from looking at the built app rather
than from the suite. The tally and what it means are a constraint in
[knowledge/technical.md](../knowledge/technical.md).
