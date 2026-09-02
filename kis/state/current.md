# Current

```text
Branch:   `main`, working tree clean. **`v0.6.1` is tagged at `5d8957d` and HEAD
          has moved past it. Run `git rev-list --count v0.6.1..HEAD` before
          building a release artefact** — the distance is deliberately not
          written here, because the commit that records it increments it. Why
          it matters is in [knowledge/technical.md](../knowledge/technical.md).
Task:     **The redesign is finished.** All four phases and all five of the
          author's mismatches are done: Settings closed it on 2026-09-03
          (`9e16cb5`), after Downloads (`21cec86`, `e85ecae`) and Activity
          Monitor (`e897b2a`) the same day
          ([intent/redesign.md](../intent/redesign.md)). Every sidebar entry but
          Discover — disabled by design — now leads somewhere.

          **Two things arrived with it that are not the drawing's**: the
          Appearance section ([intent/appearance.md](../intent/appearance.md)),
          and **`tauri-plugin-dialog`**, this app's first native dialog, behind
          Settings' Change… button.

          **Signed off by the author on 2026-09-03** on the running app —
          "download, activity table and pick all good, reviwed" — so nothing in
          the redesign is owed a look. **The one thing still unseen is the
          picker inside a notarised bundle**, which a dev window cannot answer
          ([release.md](../intent/release.md)).

          **The app is the author's: never launch, capture or drive it** — ask
          him for the screenshot, and render what can be rendered without it
          ([knowledge/technical.md](../knowledge/technical.md) Verify).

Mode:     Standard. Nothing is part-finished.

Blocker:  none.

          Two things are open and neither blocks: **the unusable-window bug**,
          whose sightings, sizes, falsified hypotheses, scripted recovery and
          two traps are all in [intent/roadmap.md](../intent/roadmap.md) risks —
          it wants its own task; and **the checks nobody has run**, listed in
          [intent/release.md](../intent/release.md) under "Unverified against
          v0.6.1".

Next:     **The README's screenshots**, which have waited behind the whole
          redesign for exactly this moment — the UI is finished, so they can be
          taken once ([release.md](../intent/release.md) phase 3). They need the
          author at the machine, like every capture.

          Then a **release**, which is the first that would carry the redesign,
          the appearance work and a new plugin. Run
          `git rev-list --count v0.6.1..HEAD` before building anything, and work
          through [release.md](../intent/release.md)'s five unverified checks,
          the newest being **the folder picker inside a notarised bundle**.

          Recorded and not fixed: the memory-safety badges, the Starting pill
          and the warning badge still use fixed ambers and greens that no
          palette moves ([intent/appearance.md](../intent/appearance.md)).

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
pipe. **261 tests**, the newest three being Activity's row assembly, one of them
watched to fail when the measurement's own server stops being excluded from the
strays ([intent/redesign.md](../intent/redesign.md)). Each phase file carries
the count it left behind, so the history is not repeated here.

Two suites need the real machine: `real_launch` proves a run is recorded against
the real binary and a real model, and `real_tune` checks the candidate picker
against `tools/fits.py` on the file in the models directory, with the ladder
itself behind `--ignored`.

Proof sits with the work it belongs to, not here. Every phase file carries its
own, including the mutation records: [tune.md](../intent/tune.md),
[figures.md](../intent/figures.md), [fitting.md](../intent/fitting.md),
[screen.md](../intent/screen.md), [last-used.md](../intent/last-used.md),
[downloader.md](../intent/downloader.md), [pi.md](../intent/pi.md),
[appearance.md](../intent/appearance.md), and each release's artefact proof in
[release.md](../intent/release.md).

**One gap, recorded rather than papered over.**
[persistence.md](../intent/persistence.md) marks three parcels done in 2026-08-03
with no proof section; it was not recorded at the time and inventing one now
would be worse than the gap.

The running defect tally, and the argument it makes about when a phase is done,
is a constraint in [knowledge/technical.md](../knowledge/technical.md).
