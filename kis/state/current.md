# Current

```text
Branch:   `main`, clean and pushed. **`v0.5.0` is tagged at `551b44b`, five
          memory commits behind HEAD** — the artefacts were built from the tag.
          Check the distance before building a release artefact; assuming tag and
          HEAD were identical is what put a build of HEAD in the v0.3.0 release
          ([knowledge/technical.md](../knowledge/technical.md)).
Task:     **Tune is built, seen, committed and released as `v0.5.0`**
          ([intent/tune.md](../intent/tune.md)). The suite no longer writes into
          `~/Library/Application Support/llamaport`; `speeds.json` has a store;
          the runner records a run when it settles; Tune measures a ladder that
          reproduces `tools/fits.py --run`'s ordering on Ornith; and the app now
          has one opinion, offered rather than applied.
Mode:     **Standard — the pi button is built, seen, proved and unreleased**
          ([intent/pi.md](../intent/pi.md)). Planned, built and corrected on the
          screen 2026-09-02. A button in the Running panel beside Web UI and Test
          model shows a diff of one `llamaport` provider, then writes it into
          `~/.pi/agent/models.json` on confirm, leaving the other five providers
          and their keys alone.

          **Looking at it found both of its defects**, as always here: the label
          wrapped onto three lines in the sidebar it was first built into, and
          the write took the file from `600` to `644` — five API keys made
          world-readable, found in `ls -l` and not by any test.

          **pi answered a prompt through the entry on 2026-09-02**, which is the
          one check nothing in this repository could run. Using it found the
          feature was half of item 6: a provider is not enough, and the model had
          to be enabled by hand. The button now writes `enabledModels` in pi's
          settings too, prunes the dead entries of ours it had been leaving
          behind, and shows a line diff of both files rather than two blobs.
Blocker:  none. **Three standing items, all of one kind — something nobody has
          looked at.** One owed against the release, two older. What each proves
          and why a shortcut would not is in
          [intent/release.md](../intent/release.md).

          Owed, and needing a human at the machine: the **Gatekeeper prompt**.

          Older: a queued download row with nothing on disk behind it has never
          been seen coming back from a restart, and no Intel Mac has run the
          universal build.

          **Three stopped being owed on 2026-09-01**: `/Applications` holds
          v0.5.0 where it had held v0.3.1 for three releases, five launches gave
          five usable windows, and the **Dock click was made by hand on the
          installed v0.5.0** — window closed, icon pressed, window back. Owed
          since the fix shipped in v0.3.0, and the first hand click on a release
          build rather than a dev one.
Next:     **Decide whether this ships.** The pi button is built, seen and proved
          — every acceptance check is met, including the two only a human could
          run — and it is unreleased, like Tune was for a day. Nothing depends on
          it going out today, and nothing is committed yet.

          **Nothing about it is open.** The diff panel was used, both
          files were written with pi running, and the model was selectable
          straight away — no hand-enabling, no restart. Item 6 is one click.

          Still owed, and unchanged by this work:
          **open `~/Downloads/Llamaport_0.5.0_aarch64.dmg`.** The app is already
          installed, so this is not for the install — it is the only copy on this
          machine carrying a quarantine attribute, and opening it is the only way
          to meet the Gatekeeper prompt a downloader meets. Owed since v0.1.0,
          and the last of this release's checks that a human at the machine can
          settle. Nothing here can reach it: macOS denies the agent's shell
          `~/Downloads` entirely.

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
