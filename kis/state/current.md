# Current

```text
Branch:   `main`, clean and pushed. **`v0.6.1` is tagged at `5d8957d`, four
          memory commits behind HEAD** — the artefacts were built from the tag,
          when the two were the same commit. Check the distance with
          `git rev-list --count v0.6.1..HEAD` before building a release
          artefact; assuming tag and HEAD are identical is what put a build of
          HEAD in the v0.3.0 release
          ([knowledge/technical.md](../knowledge/technical.md)).
Task:     **The pi button shipped as `v0.6.0`; `v0.6.1` fixed the packaging it
          exposed** ([intent/pi.md](../intent/pi.md),
          [intent/release.md](../intent/release.md)). The app writes the provider
          and the enabled entry pi needs, diffing both files first — item 6 is one
          click.
Mode:     none in progress.
Blocker:  none. The **Gatekeeper check is met**, twice over on 2026-09-02 — the
          run that failed is why v0.6.1 exists
          ([intent/release.md](../intent/release.md)).

          **Four things nobody has looked at.** Two owed against v0.6.1 and
          needing a human at the machine: **five launches from Finder** with
          nothing fullscreen, and the **Dock click** — the 2026-09-01 evidence
          was against v0.5.0 and does not carry. Two older: a queued download row
          with nothing on disk behind it coming back from a restart, and an Intel
          Mac running the universal build.
Next:     **The README's screenshots**, a release behind — they show a Running
          panel with no Use in pi button. The author's, per
          [release.md](../intent/release.md) phase 3.

          After that, what is left of [intent/direction.md](../intent/direction.md):
          the launch form shrinking behind named choices, per-field override, and
          search. None is planned, and search is blocked on what "best model"
          means.
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
