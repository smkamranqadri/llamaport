# Downloader milestone

Design lives in [docs/downloader-spec.md](../../docs/downloader-spec.md). This
file holds only what the spec does not: the decisions taken, what was carried
rather than met, and the limits found in the building.

Live status is in [state/current.md](../state/current.md), not here.

## Decisions

- **ureq + threads, not reqwest + tokio.** Enable ureq's default features for
  TLS, one thread per segment, positioned writes through `FileExt::write_at`.
  The codebase is blocking throughout and hands long work to `spawn_blocking`;
  8 sockets do not need an async runtime. `tokio` is already compiled in via
  tauri, but `reqwest` is not in the macOS build graph and would add ~40 crates.
- **The rate limit is live.** `Control` carries it and the bucket re-reads it on
  every charge, so a limit changed while a download runs applies to that
  download. The alternative — fixing it in the `Spec` at the start — was
  rejected: a limit is set while watching the transfer it is meant for.
- **The floor on a limit is app policy, not engine mechanism.** `normalized_rate`
  in `downloads.rs` bounds what may be asked for; the engine honours whatever it
  is handed, which is what lets a test drive it at a byte a second.
- **No Hugging Face token.** Public repos only; a gated repo is
  detected on resolve and reported plainly. Most GGUF quants are public, and a
  released app should not keep a bearer token in plaintext. The spec's
  strip-`Authorization`-on-redirect rule stays dormant until this is revisited.
- **sha256 verification defaults to on**, in the background, before the rename,
  and is skippable.
- **Downloads land in the configured models directory.**

## Phase 1 — engine and Downloads screen — DONE 2026-08-02

Every closing condition met except one, proved by a real 676 MB transfer that was
killed, resumed, verified and landed in the models directory.

Two carried forward rather than met:

- **"Beats a single connection" was never measured.** The observed rate varied
  90 KB/s to 1.4 MB/s and the line, not the engine, looked like the limit. It
  needs a like-for-like comparison against `curl` before anyone claims it.
- **Stall detection ignores the spec's sibling-progress condition.** Any segment
  silent past `stall_after` is reissued, bounded by the 5-attempt limit, rather
  than only one silent while siblings move. Deliberate and accepted.

## Phase 2 — the queue — DONE 2026-08-04

One transfer at a time stays true. What changes is what happens to the second
request: it waits instead of being refused.

The manager gains a `Queued` state and one invariant — **nothing Active and
something Queued means the head of the queue starts.** Every settle path goes
through it, so a transfer that completes, fails, is discarded or is paused all
hand the pipe to the next in line.

### Decided

- **Both `start` and `resume` enqueue.** They refuse today for the same reason
  and with the same sentence, and `resumable` says so on purpose. Queueing only
  new starts would move the wall rather than remove it: the first time a user
  pauses one file and tries to resume another, they meet "this app downloads one
  file at a time" with a queue visibly running behind it.
- **A queued row is persisted, and comes back Paused.** It is the first
  unfinished work in this app with nothing on disk describing it — no `.part`,
  no sidecar — so `downloads.json` has to hold it or quitting loses the URLs.
  It returns as a Paused row with zero bytes, a state the screen already draws
  and `resume` already re-validates the URL for. Rejected: coming back and
  carrying on, which would make launching the app fetch from a URL read off
  disk with no click. The config directory is untrusted input and that is the
  shape of the hole v0.2.0 fixed.
- **One invariant, no stop button.** Pause therefore starts the next file, and
  stopping everything means pausing each in turn. Accepted for a first cut
  rather than overlooked: a queue-paused flag is a second piece of state that
  has to persist, show, and be reasoned about on every settle path. Add it when
  the missing button actually bites.

### Scope

`Queued` on the job, the invariant in the manager, `admit` and `resumable`
enqueueing rather than refusing, queued rows through persist and restore, and a
screen that shows a queue position, stops disabling the Download button and
stops blocking Resume and Retry.

Out: reordering, a depth limit, parallel transfers, a queue-paused control,
halting after repeated failures, anything resembling Discover, and measuring
four segments against a single connection.

### Closing conditions

1. A second URL submitted during a transfer adds a Queued row and returns no
   error; the Download button is no longer disabled.
2. Complete, Failed, Discarded and Paused each start the head of the queue.
3. A duplicate URL is refused, and so is a different URL resolving to the same
   file name — against Active, Paused and Queued rows alike.
4. Resume on a paused row while something runs enqueues rather than refusing.
5. Quit with three queued, relaunch: three Paused rows, URLs intact, paths
   rebuilt from the models directory, Resume live on all three.
6. A queued row in `downloads.json` whose URL is not a Hugging Face `.gguf` URL
   does not appear at all, and the engine is never reached for it.
7. A file that appeared in the models directory while a job waited is not
   overwritten — the job settles Failed saying it is already there.
8. Clear finished leaves queued rows alone.

### Risks

- `download.rs` is not touched. The queue is a manager concern; if the engine
  needs a change, the design is wrong and the work stops rather than spreads.
- The advance must run on the transfer's own thread after `finish` has released
  the jobs lock. Starting the next job from inside `finish` takes the same mutex
  and deadlocks. This is the most likely way to get it wrong.
- A discarded active job is removed from the list inside `finish`, so the
  advance must run after that removal or it reads the dead row as Active.
- `admit`'s duplicate scan is the only place where a missed case corrupts a file
  rather than annoying a user: two queued jobs for one file name would open a
  second `.part` over bytes the first is writing.
- `dest.exists()` is checked at enqueue and goes stale while the job waits. It
  has to be re-checked when the job actually starts.
- A restored queued row is the first Paused row with no `.part` behind it.
  `adopted()` sets `resumable = false` when the partial is gone; applying that
  here brings the whole queue back with dead Resume buttons.
- `blocked={active != null}` in `Downloads.tsx` disables Resume and Retry
  everywhere. Left in place it makes the feature invisible from the screen.

### What the building found

Three defects, none of them caught by the suite. All three were found by reading
what was on disk rather than by a test, and each now has one that fails without
its fix.

- **`restore` rebuilt `path` from an unvalidated `file_name`**, so `../` in
  `downloads.json` reached out of the models directory. It predates the queue and
  is **live in v0.2.0**. The existing test pinned the stored `path` and nobody had
  asked where the replacement came from.
- **A restored queued row zeroed its byte count** while its `.part` sat on disk,
  and — holding that file's path — shadowed the `adopt` row that knew better. The
  test that covered it seeded a row with no `.part`, which is the case that does
  not have the problem.
- **The queue survived exactly one restart.** A row restored as Paused stopped
  being written to the only file that remembered it. That is what produced the
  rule now in [knowledge/technical.md](../knowledge/technical.md): the history
  file holds whatever nothing else on disk describes.

The first two were caught by reading `downloads.json` between runs. Worth
repeating on anything that persists: read the file the app actually wrote.

## Verification

`cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check`,
`bun run build` — then a real transfer. Tests alone have never been enough to
close work on this subsystem, and were not enough for the speed limit either:
what proved that was watching a running download change rate.

Agreed 2026-08-02: the proof is a small (~1 GB) public GGUF from a repo such as
bartowski or unsloth, killed mid-flight and resumed, verified against the real
Hugging Face CDN. A full 13-21 GB run is a separate decision to be asked for
explicitly, not assumed.

Use the tdd skill for the engine: the retry taxonomy and resume logic are cheap
to get wrong and expensive to discover at 97% of 21 GB. The same applies to the
queue's settle-and-advance path for the same reason.

Phase 2 adds two proofs the suite cannot give. Two small public GGUFs queued
back to back, the first paused mid-flight, and the second seen starting on its
own. Then the app quit with one still queued and relaunched, and the row seen
coming back. The restore path also wants the test the resume fix got: a planted
`downloads.json` row whose URL is not Hugging Face, against an engine that
panics if it is reached at all.

## Known limits of the engine

Found while planning Discover, which was then dropped ([roadmap.md](roadmap.md)).
They are facts about the engine and outlive that plan.

- Hugging Face omits `x-linked-size` and `x-linked-etag` on non-LFS files, and
  the engine refuses a file with no declared size. In a repo tree an `lfs` object
  on the entry is exactly the condition under which those headers exist.
- A quant too big for one file ships as `{name}-00001-of-00003.gguf`. The engine
  takes one file per job, so a split set is three URLs — queued back to back
  since 2026-08-04, one at a time but unattended.
- ~~Pause is not a state.~~ It is one as of 2026-08-03: the UI wanted the button,
  and the row that survives a restart needed a name.
  [persistence.md](persistence.md).
- ~~One at a time is enforced by refusing, not queueing.~~ There is a `Queued`
  state as of 2026-08-04, and the rule it was written to explain is now in
  [knowledge/technical.md](../knowledge/technical.md). Phase 2 below.
- `resolution_against_a_silent_server_is_bounded_by_its_timeouts` guards less
  than its name claims: it pins that *some* timeout bounds resolution, not which
  one — removing either the read timeout or the overall timeout alone still
  passes. Tighten it if that code is touched.
