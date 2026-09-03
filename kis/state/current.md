# Current

```text
Branch:   `main`, working tree clean. **`v0.6.1` is tagged at `5d8957d` and HEAD
          has moved past it. Run `git rev-list --count v0.6.1..HEAD` before
          building a release artefact** — the distance is deliberately not
          written here, because the commit that records it increments it. Why
          it matters is in [knowledge/technical.md](../knowledge/technical.md).

Task:     **Discover is built and the sidebar can be translucent.** Discover
          landed in two commits on 2026-09-03 (`7c9369f`,
          `ba4a8f6`) and all five parcels
          ([intent/discover.md](../intent/discover.md)). It is a live screen over
          the Hugging Face API: a sort, filters that combine, a parameter band,
          a search whose results get the
          same quantisation pick and fit treatment as a browse row, and a
          Download that hands a shard set to the queue this app already had.
          **No sidebar entry is disabled any more.**

          It was the third time Discover was planned, against an entry asking
          that it not be. What made the difference was answering the objection
          rather than arguing past it — the research is why, and it changed the
          plan twice: against the live API, which dropped two of the five drawn
          chips on measurement, and against Unsloth's shipped hub, which
          corrected four facts and supplied the fit badge this project then
          refused.

          **The author has looked six times and asked for nineteen things; all
          are built** (`d9ed796`, `ca5d7af`, `8c8db9d`, `951fd98`, `3c3bbd8`,
          `4073600`, `8ef7078`…`335fa13`). Sorts and filters now split, which is
          the shape offered while planning and declined — using it is what
          changed the answer. Then an owner's picture on all three list screens,
          cached on disk. Then **macOS vibrancy behind the sidebar**
          ([intent/appearance.md](../intent/appearance.md)), which took four
          commits because **a window effect cannot be checked the way every other
          UI change here is** — headless Chrome will not draw an
          `NSVisualEffectView`.

          **Two of the fifteen were things nobody reported.** Discover was
          offering models `llama-server` cannot run, one GGUF repository in six,
          found because a downloaded speech model turned `real_models.rs` red.
          And **the MoE mark shipped dead in `8c8db9d`**: `expand=gguf` never
          reached the listing URL, so every row came back unmarked, and an inline
          test asserting the URL must *not* carry it kept the suite green. Both
          are in [knowledge/technical.md](../knowledge/technical.md).

          **The app is the author's: never launch, capture or drive it** —
          everything here was rendered in headless Chrome, which goes before the
          ask and never instead of it
          ([knowledge/technical.md](../knowledge/technical.md) Verify).

Mode:     Standard. Nothing is part-finished.

Blocker:  none.

          Three things are open and none blocks: **the two picker calls the
          author has not ruled on** (below); **the unusable-window bug**, whose
          sightings, sizes, falsified hypotheses, scripted recovery and two traps
          are in [intent/roadmap.md](../intent/roadmap.md) risks — it wants its
          own task; and **the checks nobody has run**, in
          [intent/release.md](../intent/release.md).

Next:     **A seventh look.** Six rounds, nineteen things, and the rate is not
          obviously falling. Unseen running: the split controls, the parameter
          band, the MoE badge — which has never once worked in a build the author
          has opened — the loading state, the confirmation and the sidebar count.
          Avatars and the vibrancy are the exceptions; both were seen on the
          author's own machine.

          **Two numbers in the vibrancy are untuned** — the 40% tint and the
          `underWindowBackground` material were set against each other and only
          one of them moved. Neither is worth touching without a look.

          Nothing is planned beyond answering the next look. Discover took five
          rounds of correction and the sidebar four, and none of the nine came
          from a plan.

          **A release is the standing next thing after that**, and it now carries
          Discover, the avatars, the vibrancy and a private API
          ([release.md](../intent/release.md)). The README screenshots are still
          owed and still predate everything.

          Then the **README's screenshots**, which have now waited behind two
          phases and are owed for all three images — every one predates the
          redesign. Then a **release**: run
          `git rev-list --count v0.6.1..HEAD` first, and see
          [release.md](../intent/release.md), which now carries what the next one
          owes — including **a security review of the network surface**, as
          v0.2.0 had.

          Recorded and not fixed: the memory-safety badges, the Starting pill
          and the warning badge still use fixed ambers and greens that no
          palette moves ([intent/appearance.md](../intent/appearance.md)). They
          are written `var(--x, fallback)` and are correctly not flagged by the
          new stylesheet test.

          Still unplanned: per-field override, and **MoE launches**
          ([intent/moe.md](../intent/moe.md)), blocked on timing `-ncmoe`
          against the quant the author runs today. Library search is still
          open ([intent/gaps.md](../intent/gaps.md)) and is now the only search
          this app lacks.
```

Run and verify commands: [knowledge/technical.md](../knowledge/technical.md).
Where the project stands, and what is planned: [intent/roadmap.md](../intent/roadmap.md).
What the app is for: [intent/direction.md](../intent/direction.md).

## Proof

The four commands green, each status captured on its own line and never after a
pipe. **310 tests**, from 261 before Discover.

Two suites need something this machine has and CI would not: `real_launch` and
`real_tune` need the binary and a real model, and **`real_hub` needs the
network** — fourteen tests holding the assumptions the Hugging Face parsers rest
on, so a change at the API's end fails here rather than on screen.

**`tests/stylesheet.rs` is the only test this project has of the frontend.**
There is no framework for one, and it exists because Discover painted its chips
with `--muted`, a token `App.css` has never defined and the fourth time this
project has done that. The rule is in
[knowledge/technical.md](../knowledge/technical.md).

Proof sits with the work it belongs to, not here. Every phase file carries its
own, including the mutation records: [tune.md](../intent/tune.md),
[figures.md](../intent/figures.md), [fitting.md](../intent/fitting.md),
[screen.md](../intent/screen.md), [last-used.md](../intent/last-used.md),
[downloader.md](../intent/downloader.md), [pi.md](../intent/pi.md),
[appearance.md](../intent/appearance.md), [discover.md](../intent/discover.md),
and each release's artefact proof in [release.md](../intent/release.md).

**One gap, recorded rather than papered over.**
[persistence.md](../intent/persistence.md) marks three parcels done in 2026-08-03
with no proof section; it was not recorded at the time and inventing one now
would be worse than the gap.

The running defect tally, and the argument it makes about when a phase is done,
is a constraint in [knowledge/technical.md](../knowledge/technical.md).
