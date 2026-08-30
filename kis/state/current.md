# Current

```text
Branch:   `main`, clean and pushed. `v0.3.1` is tagged at HEAD, and the built
          `.dmg`s came from that commit.
Task:     none in progress. **Figures is done** — both parcels, 2026-08-31
          ([intent/figures.md](../intent/figures.md)). The KV term no longer
          charges every layer a full-context cache, and a header that names a
          layer interval without a window size now withholds the figure and says
          in words which layers it could not read. Units split by what they
          measure: files, disk and transfer rates decimal so they agree with
          Finder, memory binary so a 32 GB Mac still says 32 GB. It ends without
          a release by decision, so both fixes sit in the tree.

          Distribution before it is done: a description, topics and an uploaded
          social preview, the release is no longer a pre-release, the front page
          leads with a clip, a Show and tell post is live in llama.cpp's
          Discussions, four list submissions are open, and the author has posted
          to several subreddits and commented around them
          ([intent/release.md](../intent/release.md)).
Mode:     —
Blocker:  none. Figures came off this list on 2026-08-31: it was seen on screen,
          and being looked at is what caught a defect in it — the note saying
          which counting the memory figures use was missing from the one branch
          that most needed it. Three standing items remain, all of the same kind.
          The README's "Open Anyway" steps have
          never met a real Gatekeeper prompt; a queued row with nothing on disk
          behind it has never been seen coming back from a restart; and no Intel
          Mac has run the universal build. Two more are owed by v0.3.1, which
          shipped without them by decision: nobody has seen the Dock fix work in
          the built artefact, and the five-launch condition was not run
          ([intent/release.md](../intent/release.md)).
Next:     Nothing is half-finished, so the next move is a choice rather than a
          continuation. Three candidates, in the order they earn it:

          Commit what is in the tree — Figures as one commit, and the
          `ProfileForm.tsx` slider step apart from it, since it was asked for
          separately and belongs to no phase.

          `--fit` is the next phase when there is to be one, and it wants its own
          planning pass ([intent/gaps.md](../intent/gaps.md) item 2). It is the
          largest item in that file and it is no longer a nicety: the flag is on
          by default and this app disables it on every launch by naming `-c` and
          `-ngl` ([knowledge/technical.md](../knowledge/technical.md)).

          And the things needing no code at all: Show HN is drafted and unposted
          and is the one launch channel open today, and the two conditions v0.3.1
          owes are ten minutes with the display free — launch the installed build
          five times, then close its window and click the Dock icon. A browser
          download of a `.dmg` would still settle Gatekeeper in five minutes
          whenever it is wanted. After a release the same rule as ever: the next
          move is whatever the audience says. Do not plan features against
          silence.
```

Run and verify commands: [knowledge/technical.md](../knowledge/technical.md).
Plans and decisions: [intent/roadmap.md](../intent/roadmap.md).

## Where the project actually is

The runner lists, launches, supervises and tests models. The downloader fetches
from Hugging Face in four ranged segments, survives a kill, resumes from its
sidecar, verifies sha256 and lands the file in the models directory.

Downloads outlive the app and now queue. A second URL waits its turn instead of
being refused, and every way a transfer ends — finished, failed, paused,
discarded — starts the next one ([intent/downloader.md](../intent/downloader.md)).
A transfer is paused or discarded rather than cancelled, an interrupted one comes
back from the `.part` on disk, and what has no `.part` is remembered by
`downloads.json` instead. The Library stars models and deletes them to the Trash.
Settings holds the values a never-launched model opens on
([intent/persistence.md](../intent/persistence.md)).

The Library also orders itself by what you have been running. Its last cell shows
the last launch where there has been one and the file's mtime otherwise, told
apart by weight, and the list sorts on that value with favourites still above
([intent/last-used.md](../intent/last-used.md)). A model is dated only by a run
that reached Ready. The running row is tinted end to end and ends in Stop rather
than the Delete every other row ends in.

A ready model offers **Web UI**, which opens `llama-server`'s own interface in a
second app window. The app has no chat of its own and is not getting one
([knowledge/project.md](../knowledge/project.md)).

**v0.3.1 is published**, unsigned, as the Latest GitHub release with **two
`.dmg`s** attached — `aarch64` and `universal`:
https://github.com/smkamranqadri/llamaport/releases/tag/v0.3.1. It is v0.3.0 plus
the Dock reopen fix and nothing else, and the first release published as Latest
rather than pre-release; v0.3.0 was promoted to that on the same day
([intent/release.md](../intent/release.md)). The 0.3.1 aarch64
one is installed in `/Applications`. The app is macOS-only for reasons that are
Darwin's rather than ARM's ([knowledge/technical.md](../knowledge/technical.md)),
and no Intel Mac has run it. Anyone still on v0.2.0 should upgrade for the path
traversal v0.2.1 fixed, which is live in that build; v0.1.0 is unaffected by that
one — it has no history file at all. Every release so far
([intent/release.md](../intent/release.md)).

**The front page leads with `docs/launch.gif`**, and the Show and tell post at
https://github.com/ggml-org/llama.cpp/discussions/26772 is the one outward-facing
thing that goes stale on its own — edit it whenever a release changes what it
claims. Both are owned by [intent/release.md](../intent/release.md).

**Nothing has converted yet.** Stars, forks and watchers were all still 0 at the
end of 2026-08-08, after the subreddit posts were up and drawing views. Views
without stars is the measurement that matters, and the honest reading of one
evening is that it is too early to read anything at all. The `homebrew/cask`
notability gate ([intent/release.md](../intent/release.md)) is untouched.

Discover was planned and then dropped ([intent/roadmap.md](../intent/roadmap.md)).

Apart from the files named above, `git log` is the record.

## Proof

The four commands were last run green over the working tree on 2026-08-31,
after both Figures parcels: `cargo fmt --check`, `cargo clippy --all-targets --
-D warnings`, `cargo test`, `bun run build` — all exit 0, each status captured
on its own line rather than after a pipe. **187 tests**, up from 180, the seven
new ones all in `estimate`.

Those seven are trusted, having been watched to fail: with `layer_split` gutted
to ignore the interval, exactly the three that describe layer kinds failed and
the nine older ones passed; with `window.min(ctx)` gutted to `window`, exactly
the one about a window wider than the context failed. A green suite was not the
claim either time ([knowledge/technical.md](../knowledge/technical.md)).

**190 tests**, after `gguf.rs` gained three of its own — it had none for the
new keys when the first report was written. Both were watched to fail: dropping
the second interval spelling, and never reading the window size.

Two of the three screen checks were pulled back into measurement on 2026-08-31,
by running the formatters against the real byte counts under `bun`. The Library
figure for `Qwen3.6-35B-A3B-UD-IQ4_NL.gguf` is 18.0 GB against Finder's 18.04
for the same 18,040,888,288 bytes; a 32 GiB machine still reports 32.0 GB; a
rate limit typed as 10 reads back as 10, and 2.5 and 0.5 likewise. The engine's
floor hint now reads 66 KB/s where it read 64 — the same constant, rendered
decimal.

Figures' own proof, including what llama.cpp settled about the defect on real
hardware, is in [intent/figures.md](../intent/figures.md). What it could not
reach is the rendering, which is on the standing list above rather than blocking
— and nothing reaches a user unlooked-at, because the phase ends without a
release.

**v0.3.0 shipped 2026-08-08.** Every build's artefact proof lives with its
release in [intent/release.md](../intent/release.md) — both assets downloaded
back byte-identical, the universal build's x86_64 half run under Rosetta, and how
a frontend-heavy change had to be proved present when grepping the binary no
longer could. Installing by local copy sets no quarantine attribute, so no
release yet has tested Gatekeeper.

Proof sits with the work it belongs to, not here:

- Last used, including what the author's screenshots settled that no test could
  reach — [intent/last-used.md](../intent/last-used.md).
- The download queue draining ~48 GB unattended —
  [intent/downloader.md](../intent/downloader.md).
- The Persistence phase — [intent/persistence.md](../intent/persistence.md).
- The v0.2.0 release review — [intent/release.md](../intent/release.md).
