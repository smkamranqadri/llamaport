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

Next:     **The same warning is owed on v0.5.0 and every release before it.**
          v0.6.0's notes now carry one — a warning block naming the dialog, the
          `xattr` line, and a link to v0.6.1, replacing an opening line that told
          readers to right-click Open, which never worked. But the defect goes
          back to v0.1.0, and five older releases still describe a flow that
          cannot work. v0.6.1 is the newest, so the repository's download
          button is safe; only someone browsing older tags is caught.

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
[downloader.md](../intent/downloader.md), [pi.md](../intent/pi.md), and each
release's artefact proof in [release.md](../intent/release.md).

**Two gaps, recorded rather than papered over.**
[persistence.md](../intent/persistence.md) marks three parcels done in 2026-08-03
with no proof section; it was not recorded at the time and inventing one now
would be worse than the gap. And **nineteen defects across five phases** were found
by looking at the built app, none by the suite — eleven on 2026-08-31, five more
in Tune's panel on 2026-09-01, and three in the pi button on 2026-09-02: a label
that wrapped, a file mode read out of `ls -l`, and a provider that turned out not
to be selectable. **A twentieth was worse**: five releases shipped a bundle macOS
refuses to open, and only downloading one through a browser exposed it
([intent/release.md](../intent/release.md)). A constraint now, in
[knowledge/technical.md](../knowledge/technical.md).
