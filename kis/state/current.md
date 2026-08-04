# Current

```text
Branch:   `main`, clean, pushed, tagged `v0.2.1`. The tag, the tree and the
          published build agree again.
Task:     none in progress. The download queue is done and committed.
Mode:     —
Blocker:  none. Two things owed rather than blocking: the README's "Open Anyway"
          steps have never met a real Gatekeeper prompt, and a queued row with
          nothing on disk behind it has never been seen coming back from a
          restart — every relaunch so far recovered rows that had a `.part`.
Next:     nothing planned. Same rule as after v0.2.0: the next move is whatever
          the release says — install friction, bug reports, or silence. Do not
          plan features against silence.
Status:   working in the app, uncommitted. Pre-flight added three things the plan
          missed — queue order once Resume can enqueue, which Options a queued
          job starts under, and `clear` wiping queued rows out of
          `downloads.json`. Three defects were then found by reading disk rather
          than by any test: `restore` rebuilt `path` from an unvalidated
          `file_name`, so `../` escaped the models directory; a restored queued
          row zeroed its byte count while its `.part` sat on disk, and shadowed
          the `adopt` row that knew better; and the queue survived exactly one
          restart, because a row restored as Paused stopped being written to the
          only file that remembered it. All three fixed, each with a test that
          fails without the fix.
```

Run and verify commands: [knowledge/technical.md](../knowledge/technical.md).
Plans and decisions: [intent/roadmap.md](../intent/roadmap.md).

## Where the project actually is

The runner lists, launches, supervises and tests models. The downloader fetches
from Hugging Face in four ranged segments, survives a kill, resumes from its
sidecar, verifies sha256 and lands the file in the models directory.

Downloads outlive the app. A transfer is paused or discarded rather than
cancelled, an interrupted one comes back from the `.part` on disk — including
partials left by builds that recorded nothing — and finished ones are kept in
`downloads.json` and paged on the screen. The Library stars models and deletes
them to the Trash. Settings holds the values a never-launched model opens on
([intent/persistence.md](../intent/persistence.md)).

A ready model offers **Web UI**, which opens `llama-server`'s own interface in a
second app window. The app has no chat of its own and is not getting one
([knowledge/project.md](../knowledge/project.md)).

**v0.2.0 is published**, unsigned, as a GitHub pre-release with the `.dmg`
attached: https://github.com/smkamranqadri/llamaport/releases/tag/v0.2.0. Its
notes tell v0.1.0 users to upgrade, because one of the security fixes is a hole
that is live in v0.1.0.

Discover was planned and then dropped ([intent/roadmap.md](../intent/roadmap.md)).

Apart from the files named above, `git log` is the record.

## Proof

The four commands were last run green over the working tree: `cargo fmt
--check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`,
`bun run build` — all exit 0, each status captured on its own line rather than
after a pipe. 174 tests, up from 164.

Queue, 2026-08-04:

- **A four-deep queue drained itself in the running app**, unattended, one file
  at a time and in the order it was given: Ternary-Bonsai 27B, then 8B, then 4B,
  then 1.7B, alongside North-Mini-Code resuming from a 19 GB `.part` and
  finishing. Around 48 GB through four consecutive hand-offs without a click.
  That is the invariant, proved by the app rather than by the suite.
- Queueing, the advance on a pause, and Discard were each confirmed on screen by
  the author. Discard had never been looked at in the running app before this.
- **Resume takes its turn**, seen rather than reasoned about: two paused rows
  recovered after a restart, both clicked, one started and the other waited.
- Every closing condition has a test, and **each new guard was gutted in turn to
  see which test caught it.** All were caught by the test meant for them, both
  halves of the persistence rule included. A green suite is not the claim; a
  suite that fails when the code is wrong is.
- **Two defects were found by reading the disk, not by the suite**, and both were
  in the restart path — see below. Neither would have been caught by any test
  written before them.
- Still unproved on screen: **a row with nothing on disk behind it surviving a
  restart.** Its write half is proved — a queued row was read out of
  `downloads.json` with the app shut. The read half has not been: every relaunch
  so far recovered rows that had a real `.part`, which is `adopt`, the path that
  predates this work. The fix that makes such a row survive a second restart
  landed after the last relaunch, so it has never run in the app at all.
  Conditions 6 and 7 are suite-only by nature.

The published `.dmg` was downloaded back from GitHub and is byte-identical to
what was built. Mounted, it carries `Llamaport.app` at 0.2.0 with the right
identifier, and the shipped binary contains the `O_NOFOLLOW` refusal string — so
the fixes are provably in the artefact, not only in the tree.

Persistence phase, 2026-08-03, three parcels, each confirmed by the author in
the running app:

- **The recovery was real, not seeded.** The models directory already held a
  16.45 GiB partial with 5.66 GiB on disk, left by an earlier build that recorded
  nothing. It was adopted and resumed. Separately, 676 MB was stopped at
  135,397,705 bytes and a manager built from scratch — holding nothing, which is
  what the app is on a restart — found it at 135,496,009 bytes and carried it to
  the expected sha256.
- **A defect was found by the author using the app, not by the suite.** Delete
  did nothing at all, because `window.confirm` returns no usable answer in this
  webview and the guard on it refused every delete. `tsc` was content, since
  `confirm` is typed as returning `boolean`. That is the third defect this
  project has found by looking rather than by testing.
- **Two tests were wrong and the code was right**, both caught by gutting the
  implementation and watching what failed: a discard test claimed to catch an
  early delete and did not, and an ordering test fed `arrange` unsorted entries
  and expected them sorted. Gutting is the only reason either was noticed.
- `clippy` failed once and was read as passing, because `echo $?` came after a
  `grep`. That is the exact trap
  [knowledge/technical.md](../knowledge/technical.md) already warns about, now
  having caught a second reader.

Release review, 2026-08-03 — the reason v0.2.0 is not what it was going to be:

- A review of the `v0.1.0..main` diff, run before publishing, found two live
  holes. Both were confirmed in the code before being acted on rather than taken
  on the reviewer's word.
- **A symlinked `.part` was written through**, turning an ordinary download into
  a write to wherever the link pointed. Predates the phase and is **live in
  v0.1.0**. Now `O_NOFOLLOW`.
- **Resume never re-validated its URL**, so a planted sidecar became a paused,
  resumable row that fetched from any host on click. Introduced by this phase and
  killed before it shipped.
- Each fix reproduces its exploit first and fails against the fix gutted: the
  symlink test asserts the victim file's contents survive, the sidecar test
  panics if the engine is reached at all.
- The reviewer was a code-review subagent, **not** the `/security-review` skill.
  That skill was failing on an unresolvable `origin/HEAD`, now fixed with
  `git remote set-head origin -a` — cloning writes that ref, adding a remote to
  an existing repository does not. It resolves but has still never reviewed
  anything here, and against a pushed `main` it has an empty diff to work with
  ([intent/release.md](../intent/release.md)).
