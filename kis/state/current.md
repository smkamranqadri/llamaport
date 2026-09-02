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
Task:     **Redesign phases 2 and 3 are built and committed, and five things
          still do not match the artboards** — they are listed screen by
          screen in [intent/redesign.md](../intent/redesign.md) under "What
          still does not match". Phase 1 landed as `c6ac59f`.

          **Six passes were needed for one screen**, four of them wasted
          because the app was compared against the code that generated the
          mockup rather than the mockup rendered. The author called it, and
          the next session works **one screen per task**, each finished by
          the side-by-side capture in
          [knowledge/technical.md](../knowledge/technical.md).
Mode:     Phase — redesign, between phase 3 and the corrections above.
Blocker:  none. The **Gatekeeper check is met**, twice over on 2026-09-02 — the
          run that failed is why v0.6.1 exists
          ([intent/release.md](../intent/release.md)).

          **Four things nobody has looked at.** Two owed against v0.6.1 and
          needing a human at the machine: **five launches from Finder** with
          nothing fullscreen, and the **Dock click** — the 2026-09-01 evidence
          was against v0.5.0 and does not carry. Two older: a queued download row
          with nothing on disk behind it coming back from a restart, and an Intel
          Mac running the universal build.
Next:     **Item 1 of "What still does not match"** — the stopped model
          screen's rows, starting with the log the author does not want on a
          model that is not running
          ([intent/redesign.md](../intent/redesign.md)). One screen, one
          task, and the capture before it is called done. The README's
          screenshots, previously here, wait behind the whole redesign on the
          author's decision: every phase would invalidate them again, so they
          are done once, against the finished UI
          ([release.md](../intent/release.md) phase 3).

          The "named choices" remainder of
          [intent/direction.md](../intent/direction.md) is now owned by
          redesign phase 2. Still unplanned after it: per-field override, and
          search — blocked on what "best model" means.
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
