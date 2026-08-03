# Current

```text
Branch:   main, clean and pushed, tagged `v0.2.0`. The tag and `main` are the
          same commit, so the published build and the tree agree for the first
          time since v0.1.0.
Task:     none in progress. The Persistence phase is finished and shipped.
Mode:     —
Blocker:  none. Two things owed rather than blocking: the README's "Open Anyway"
          steps have never met a real Gatekeeper prompt, and Discard and the
          Downloads History pages have never been looked at in the running app.
Next:     nothing planned. The next move is whatever v0.2.0 says — install
          friction, bug reports, or silence. Do not plan features against
          silence. A browser download of the `.dmg` would settle Gatekeeper.
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
after a pipe. 164 tests.

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
