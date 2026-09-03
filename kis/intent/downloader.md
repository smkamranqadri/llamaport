# Downloader

Design lives in [docs/downloader-spec.md](../../docs/downloader-spec.md). This
file holds only what the spec does not: decisions taken, what was carried
rather than met, and limits found in the building.

## Decisions

- **ureq and threads, not reqwest and tokio.** The codebase is blocking
  throughout, and `reqwest` is not otherwise in the macOS build graph.
- **The rate limit is live**, re-read on every charge, so a change mid-
  download applies to it. Its floor is app policy in `downloads.rs`, not an
  engine mechanism; the engine honours whatever it is handed.
- **No Hugging Face token.** Public repos only; a gated repo is detected on
  resolve and reported plainly, since a released app should not keep a
  bearer token in plaintext.
- **sha256 verification defaults to on**, in the background before the
  rename, and is skippable. Downloads land in the configured models
  directory.

## Phase 1: engine and Downloads screen

Completed 2026-08-02. Every closing condition was met except two, carried
forward rather than met: "beats a single connection" was never measured,
since the observed rate varied 90 KB/s to 1.4 MB/s and the line, not the
engine, looked like the limit; and stall detection ignores the spec's
sibling-progress condition, reissuing any segment silent past `stall_after`
rather than only one silent while its siblings move, accepted as is.

## Phase 2: the queue

Completed 2026-08-04. One transfer at a time stays true; a second request
now waits instead of being refused. The manager gains a `Queued` state and
one invariant, that nothing Active and something Queued means the head of
the queue starts, which every settle path goes through.

### Decisions

- **Both `start` and `resume` enqueue.** Queueing only new starts would move
  the wall rather than remove it: pausing one file and resuming another
  would otherwise meet the same refusal with a queue running behind it.
- **A queued row is persisted, and comes back Paused.** Nothing on disk
  describes it, so `downloads.json` has to hold it or quitting loses the
  URL. Auto-resuming it on return was rejected: the config directory is
  untrusted input.
- **One invariant, no stop button.** Pausing one file starts the next; a
  queue-paused flag was deferred as extra state to persist on every settle
  path, added only if the missing control actually bites.

### Scope

`Queued` on the job, the invariant in the manager, `admit` and `resumable`
enqueueing rather than refusing, queued rows through persist and restore,
and a screen showing queue position instead of disabling Download or
blocking Resume and Retry. Out of scope: reordering, a depth limit, parallel
transfers, a queue-paused control, halting after repeated failures, Discover.

### Closing conditions

Met 2026-08-04: a second URL adds a Queued row without error; Complete,
Failed, Discarded and Paused each start the head of the queue; a duplicate
URL is refused against Active, Paused and Queued rows alike; three queued
jobs survive a quit and relaunch as three Paused rows with Resume live on
all three; and a file appearing in the models directory while a job waits is
not overwritten, settling the job Failed instead.

### Risks

`download.rs` is untouched by the queue; if the engine needed a change, the
design was wrong. The advance runs on the transfer's own thread after
`finish` releases the jobs lock, since starting it inside `finish` deadlocks
on the same mutex. `dest.exists()`, checked at enqueue, is re-checked when
the job starts since the first check goes stale while it waits, and a
restored queued row has no `.part` behind it, so `adopted()`'s usual
`resumable = false` cannot apply there.

## What the building found

Three defects, none caught by the suite, each found by reading what was on
disk and each now guarded by a test that fails without its fix: a path
traversal live in v0.2.0, where `restore` rebuilt `path` from an unvalidated
`file_name` so `../` in `downloads.json` could reach out of the models
directory; a restored queued row that zeroed its byte count while its
`.part` sat on disk, shadowing the `adopt` row that knew better; and a queue
that survived exactly one restart, since a row restored as Paused stopped
being written to the only file that remembered it. That last one produced
the rule in [knowledge/technical.md](../knowledge/technical.md) that the
history file holds whatever nothing else on disk describes.

## Known limits of the engine

Found while planning Discover in August 2026; they are facts about the engine
and outlive that plan.

- Hugging Face omits `x-linked-size` and `x-linked-etag` on non-LFS files. The
  `lfs` flag on a tree entry is what says the size headers will exist, and
  the quant picker skips non-LFS entries since 2026-09-04
  ([review.md](review.md)).
- A quant too big for one file ships as `{name}-00001-of-00003.gguf`. The
  engine takes one file per job, so a split set queues as several URLs, one
  at a time but unattended.
- `resolution_against_a_silent_server_is_bounded_by_its_timeouts` guards less
  than its name claims: it pins only that some timeout bounds resolution.
  Tighten it if that code is touched.

## Verified

Verified 2026-08-02 and 2026-08-04: all four checks passed. A real transfer
was killed mid-flight, resumed, and verified against its sha256, and a
four-deep queue drained about 48 GB unattended in the running app.
