# Current

```text
Branch:   `main`, working tree clean. **`v0.6.1` is tagged at `5d8957d` and HEAD
          has moved past it. Run `git rev-list --count v0.6.1..HEAD` before
          building a release artefact** — the distance is deliberately not
          written here, because the commit that records it increments it. Why it
          matters is in [knowledge/technical.md](../knowledge/technical.md).

Task:     **The code review's 22 findings are in, four commits**
          ([intent/review.md](../intent/review.md)): `dbed3be`, `f105d10`,
          `3def994`, `e36dd86`. Every parcel proved and reviewed. **What is
          still owed is the author's look at a built bundle** — the bundle,
          because only a bundle carries the new CSP; the dev webview gets none.
          Four things to see, listed in review.md's Proof.

          **The app is the author's: never launch, capture or drive it.** Render
          what can be rendered and then ask
          ([knowledge/technical.md](../knowledge/technical.md) Verify).

Mode:     Phase, last step. Nothing part-finished; the look is the close.

Command:  export PATH="$HOME/.cargo/bin:$PATH"   # cargo is not on a non-login PATH
          bun run build
          cargo test --manifest-path src-tauri/Cargo.toml
          cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
          cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
          Each status on its own line, never after a pipe. Running it: `bun run tauri dev`.

Blocker:  none. Three things are open and none of them blocks:

          - **Two calls in Discover's quantisation picker the author has never
            ruled on**: it returns `Q4_K_M` when there is no ceiling to measure
            against, and it holds back llama.cpp's 1,024 MiB `--fit-target`
            margin, which no row displays. Both were decided mid-build and both
            are still only my judgement
            ([intent/discover.md](../intent/discover.md)).
          - **The unusable-window bug** — sightings, sizes, falsified
            hypotheses, scripted recovery and two traps in
            [intent/roadmap.md](../intent/roadmap.md) risks. It wants its own task.
          - **Five checks nobody has run**, in
            [intent/release.md](../intent/release.md).

Next:     **The look**, then **a release.** It is the first to carry Discover, the
          owner avatars, the translucent sidebar, a CSP and a **private API**, so a minor
          ([intent/release.md](../intent/release.md), which lists what it owes —
          including a security review of the network surface).

          **The README's screenshots come first and are owed for all three**;
          every one predates the redesign, and they now trail four phases. They
          need the author at the machine, like every capture.

          Two vibrancy numbers are untuned — the 40% tint and the
          `underWindowBackground` material were set against each other and only
          one of them moved. Neither is worth touching without a look.

          Unplanned: per-field override, and **MoE launches**
          ([intent/moe.md](../intent/moe.md)), still blocked on timing `-ncmoe`.
          Library search is open ([intent/gaps.md](../intent/gaps.md)) and is now
          the only search this app lacks.
```

Where the project stands: [intent/roadmap.md](../intent/roadmap.md).
What the app is for: [intent/direction.md](../intent/direction.md).

## Proof

The four commands green, each status captured on its own line. **310 tests**,
from 261 before Discover.

Three suites need something this machine has and CI would not: `real_launch` and
`real_tune` need the binary and a real model, and **`real_hub` needs the
network** — fourteen tests holding the assumptions the Hugging Face parsers rest
on, so a change at the API's end fails there rather than on screen.

**`tests/stylesheet.rs` is the only test this project has of the frontend.**
There is no framework for one, and it exists because Discover painted its chips
with a token `App.css` has never defined, for the fourth time in this project.

**Seen by the author on 2026-09-04**: Discover's split sort and filter controls,
the parameter band and the MoE badge, all reported working. That is those three
and not a sign-off on the phase.

Proof sits with the work it belongs to. Every phase file carries its own,
including the mutation records: [tune.md](../intent/tune.md),
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
