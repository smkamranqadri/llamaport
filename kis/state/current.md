# Current

```text
Branch:   `main`, working tree clean. **`v0.7.0` is tagged at `12de686` and HEAD
          has moved past it. Run `git rev-list --count v0.7.0..HEAD` before
          building a release artefact** — the distance is deliberately not
          written here, because the commit that records it increments it. Why it
          matters is in [knowledge/technical.md](../knowledge/technical.md).

Task:     **Nothing is in progress.** v0.7.0 shipped 2026-09-04
          ([intent/release.md](../intent/release.md)): the redesign, Appearance,
          Activity Monitor, Discover, the avatars, the translucent sidebar, a
          CSP, the code review's 22 fixes and the security review's five, seven
          new screenshots in `assets/`. Two bundles published, verified,
          downloaded back and compared.

          **The app is the author's: never launch, capture or drive it** unless
          asked — on 2026-09-04 the author asked, for the screenshots, and it
          was run in dev mode and stopped when they were in. Render what can be
          rendered and then ask ([knowledge/technical.md](../knowledge/technical.md)
          Verify).

Mode:     Standard. Nothing is part-finished.

Command:  export PATH="$HOME/.cargo/bin:$PATH"   # cargo is not on a non-login PATH
          bun run build
          cargo test --manifest-path src-tauri/Cargo.toml
          cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
          cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
          Each status on its own line, never after a pipe. Running it: `bun run tauri dev`.

Blocker:  none. Open and not blocking:

          - **An Intel Mac** for the universal build, open since v0.3.0. Every
            other check v0.7.0 owed was met by the author on 2026-09-04
            ([intent/release.md](../intent/release.md)).

Next:     **Nothing chosen.** The author's call. The one standing candidate is
          the Show HN that has sat drafted since August
          ([intent/release.md](../intent/release.md)). Three things listed here
          on 2026-09-04 closed the same day without code: Library search had
          shipped on 2026-09-02, the window bug follows the session's launches
          and not the author's ([intent/roadmap.md](../intent/roadmap.md)), and
          the two picker calls were ruled kept as they are
          ([intent/discover.md](../intent/discover.md)).

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
