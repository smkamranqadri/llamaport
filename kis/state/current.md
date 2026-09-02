# Current

```text
Branch:   `main`, working tree clean. **`v0.6.1` is tagged at `5d8957d` and HEAD
          has moved past it. Run `git rev-list --count v0.6.1..HEAD` before
          building a release artefact** — the distance is deliberately not
          written here, because the commit that records it increments it. Why
          it matters is in [knowledge/technical.md](../knowledge/technical.md).
Task:     **Redesign phase 4 is one screen from done.** Downloads (`21cec86`,
          `e85ecae`) and **Activity Monitor** (`e897b2a`) are built on
          2026-09-03 — the last a new screen with new telemetry, a row per
          `llama-server` the app knows and the machine's figures under them
          ([intent/redesign.md](../intent/redesign.md)). **Settings is all that
          is left**, and its artboard predates the Appearance section that now
          sits in it.

          Before them: **Appearance** shipped the same day
          ([intent/appearance.md](../intent/appearance.md)), and the redesign's
          five-item list closed with the measurement inside the model screen.

          **The app is the author's: never launch, capture or drive it** — ask
          him for the screenshot, and render what can be rendered without it
          ([knowledge/technical.md](../knowledge/technical.md) Verify). **His
          eyes are owed a running transfer** on Downloads and **a live Activity
          screen**: rates, percentages and a progress bar are what a render
          cannot prove.

Mode:     Standard. Nothing is part-finished.

Blocker:  none. **The unusable-window bug has escalated and now wants its own
          task**: eight sightings on 2026-09-02, one of them with nobody
          touching the window. It is no longer one launch in three. **The recovery is
          scriptable** — an `osascript` clicking the tray's Show window restored
          it three times from the session, so it does not need a human at the
          machine, and it is the one thing that may touch the window under the
          ruling above, because it is a recovery rather than a capture.
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
Next:     **Settings**, which closes phase 4 and the redesign with it. The
          artboard draws three cards — Models folder with a Change…, llama-server
          as a verdict line over its path, and Launch defaults folded behind an
          "Edit defaults" row. The app has the same three as open panels, plus
          the **Appearance** section the drawing never had.

          Waiting on the author at the machine: **the README's screenshots**
          ([release.md](../intent/release.md) phase 3), **the four unverified
          v0.6.1 checks**, and the **unusable-window bug**, which still wants its
          own task.

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
strays ([intent/redesign.md](../intent/redesign.md)). The
redesign's own last item added none: it is a screen, and `tune::Report`'s new
`candidates` field is a copy of the list the ladder is already given. Each phase
file carries the count it left behind, so the history is not repeated here.

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
