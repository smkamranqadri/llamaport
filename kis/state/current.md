# Current

```text
Branch:   `main`, clean and pushed. **`v0.6.1` is tagged at HEAD** — annotated,
          and both artefacts were built from it, so there is no distance to
          check this time. That will stop being true the moment a memory commit
          lands on top; check it before building anything
          ([knowledge/technical.md](../knowledge/technical.md)).
Task:     **The pi button shipped as `v0.6.0`; `v0.6.1` fixed the packaging it
          exposed** ([intent/pi.md](../intent/pi.md),
          [intent/release.md](../intent/release.md)). The app writes the provider
          and the enabled entry pi needs, diffing both files first — item 6 is one
          click.
Mode:     none in progress.
Blocker:  none. **The Gatekeeper check is met.** Owed since v0.1.0, run twice on
          2026-09-02: the first run refused v0.6.0 as *damaged* and exposed five
          releases shipping an unsealed bundle behind a README that could not
          work; the second, against v0.6.1, got the ordinary
          unidentified-developer dialog and **Open Anyway** let it through
          ([intent/release.md](../intent/release.md)). The rule that would have
          caught it at v0.1.0 is now a before-the-tag gate and a fact in
          [knowledge/technical.md](../knowledge/technical.md).

          **Two standing items, both older, both something nobody has looked at:**
          a queued download row with nothing on disk behind it has never been seen
          coming back from a restart, and no Intel Mac has run the universal
          build.

          Owed against v0.6.1 and needing a human at the machine: **five launches
          from Finder** with nothing fullscreen, and the **Dock click**. The
          evidence from 2026-09-01 was against v0.5.0 and does not carry.

Next:     **v0.6.0 is still published and still refuses to open**, with release
          notes that say nothing about it. Anyone landing on it meets the dialog
          that started this. Adding a line pointing at v0.6.1 is the smallest
          thing that stops that, and it is not yet done.

          Then the README's screenshots, which are a release behind — they show a
          Running panel with no Use in pi button. The author's, per
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
pipe. **256 tests**, up from 234 when v0.5.0 shipped; the twenty-two are the pi
button's, four of them watched to fail against a mutation
([intent/pi.md](../intent/pi.md)). At `v0.5.0` itself it was **234**, up from 202
at v0.4.0 and 180 at v0.3.1; those thirty-two are Tune's
([intent/tune.md](../intent/tune.md)). Two suites need the
real machine: `real_launch` proves a run is recorded against the real binary and
a real model,
and `real_tune` checks the candidate picker against `tools/fits.py` on the file
in the models directory, with the ladder itself behind `--ignored`.

Proof sits with the work it belongs to, not here. Every phase file carries its
own, including the mutation records: [tune.md](../intent/tune.md),
[figures.md](../intent/figures.md), [fitting.md](../intent/fitting.md),
[screen.md](../intent/screen.md), [last-used.md](../intent/last-used.md),
[downloader.md](../intent/downloader.md), and each release's artefact proof in
[release.md](../intent/release.md).

**Two gaps, recorded rather than papered over.**
[persistence.md](../intent/persistence.md) marks three parcels done in 2026-08-03
with no proof section; it was not recorded at the time and inventing one now
would be worse than the gap. And **sixteen defects across four phases** were found
by looking at the built app, none by the suite — eleven on 2026-08-31 and five
more in Tune's panel on 2026-09-01. A constraint now, in
[knowledge/technical.md](../knowledge/technical.md).
