# Persistence phase

Asked for by the author on 2026-08-03, using the app: download history vanishes
on restart, an incomplete download appears to be lost, the Library cannot mark a
favourite or delete a model, and there is nowhere to set launch defaults.

Three parcels, executed in order. Live status is in
[state/current.md](../state/current.md), not here.

## What the asks turned out to be

**The bytes were never lost.** A cancelled or interrupted transfer leaves its
`.part` and sidecar in the models directory and starting the same URL again
resumes — recorded in [downloader.md](downloader.md) as "Pause is not a state".
What is lost on restart is the *job row*, so there is no button to press. The
files themselves are invisible to the Library because `catalog::scan` filters on
the `.gguf` extension, so they accumulate unseen.

That reframes parcel 1: it is not "persist the job list", it is "let the disk
speak". Two stores, one job each.

- The **disk** owns unfinished transfers. A sidecar carries `sourceUrl`, `total`
  and per-segment `completed` ([download.rs](../../src-tauri/src/download.rs)),
  which is everything a Paused row needs, and it is the only copy that cannot go
  stale while a transfer runs.
- **`downloads.json`** owns finished history alone — complete, failed,
  discarded — because those have no on-disk trace to recover from. Nothing
  writes byte counts to it, so nothing in it can disagree with reality.

Scanning for orphaned `.part` files was cut from the first draft of this plan
and put back. Without it the parcel fixes nothing that already happened:
`downloads.json` starts empty on first run, so every partial the author already
has stays as unreachable as before. It is also not extra work — it is the
sidecar read the restore already needed, minus the requirement that a row exist
first.

## Decisions

- **`Paused` replaces `Cancelled`.** Cancel already keeps the bytes; it was a
  pause wearing the wrong name.
- **Pause keeps, Discard deletes.** Discard removes the `.part` and sidecar and
  frees the space. It must delete on the settle path inside the spawned thread,
  not where the cancel is signalled — `control.cancel()` returns while the
  engine is still writing.
- **Resume reuses the job's id** rather than starting a new one, so a paused
  transfer resumed twice is one row in the history and not three.
- **Interrupted transfers are never auto-resumed.** They come back Paused and
  wait to be asked. Auto-resume fires network traffic at startup unasked, and
  with several partials it would need a queue — which this app deliberately does
  not have.
- **A sidecar without its `.part`, or a `.part` without its sidecar, is junk.**
  `download.rs` already trusts a sidecar only alongside the file it describes.
  Such a pair is listed as unresumable with a Discard rather than hidden, because
  hidden is how it got to be a problem.
- **History is uncapped and the screen paginates** — 25 finished rows at a time,
  client-side, since the whole file loads anyway. A cap would be a number nobody
  has evidence for, and the author downloads many files.
- **Launch defaults seed, they do not override.** A model with no `lastUsed`
  entry opens its form from the defaults instead of `Profile::default()`.
  Anything ever launched still opens on its own last successful launch. This is
  not the profile system v3 removed, and it does not reopen that decision.
- **The field is named `launchDefaults`, not `defaultProfile`.** `migrate` strips
  `"defaultProfile"` from `extra` because v3 retired it. A real field of that
  name would be claimed by serde before `extra` ever saw it, silently adopting
  launch settings from a build two schemas old. The retired key stays retired.
- **Delete moves to Trash.** A mistaken delete of a 20 GB file should be
  recoverable. Refused for the model the runner is currently running, and a
  shard set is deleted as one unit.

## Parcel 1 — downloads survive a restart — DONE 2026-08-03

`downloads.json` beside the config, written atomically on state transitions and
never on a progress tick. `DownloadState` gains `Paused`. A startup scan of the
models directory for `*.gguf.part.json` synthesizes a Paused row per sidecar, and
runs again when the models directory changes. Pause, Resume and Discard commands.
Downloads paginates its finished rows.

Touches `downloads.rs`, `download.rs` (one public accessor — `Sidecar` is
private), `store.rs`, `lib.rs`, `Downloads.tsx`, `api.ts`, `types.ts`.

Use the tdd skill here. The state machine and the reconciliation are the same
class of thing as the resume logic: cheap to get wrong, expensive to discover.

## Parcel 2 — favourites and delete

`favourites` in `Config`, keyed on the existing model id — `(size, hash of the
leading bytes)`, stable across renames and directory moves
([catalog.rs](../../src-tauri/src/catalog.rs)), the same key `lastUsed` uses.
Schema goes to 6. A star in the row; favourites sort above everything, then
alphabetical as now. Delete moves to Trash.

Trash is unsettled in one respect. `osascript` telling Finder to delete matches
the existing `open -R` shell-out and adds no dependency, but triggers a one-time
Automation prompt that on an unsigned app looks alarming. The `trash` crate uses
`NSFileManager` and prompts for nothing, at the cost of the objc2 tree against a
dependency list that is five long and deliberate. **Take osascript, then look at
the real dialog.** Swap if it is as bad as suspected — it is one function.

## Parcel 3 — launch defaults

`launch_defaults: Option<Profile>` in `Config`, a Settings panel, and seeding in
`build_plan` where `resolve` currently falls back to `Profile::default()`.

## Acceptance

- Quit mid-transfer, reopen: the job is Paused with the bytes the sidecar
  records, and Resume finishes the file to a verified sha256.
- A `.part` left by the *current* build, before any of this exists, is Paused
  after upgrading and resumes to a verified file.
- A sidecar with no `.part` beside it is listed unresumable, offers no Resume,
  and Discard removes it.
- Discard on a running transfer leaves no `.part` and no sidecar, and the space
  comes back.
- Completed and failed rows survive a restart; Clear empties them and they stay
  empty across a restart.
- A favourited model sorts to the top and is still favourited after a restart,
  after a rename, and after a directory move.
- Delete puts the file in Trash, is refused for the running model, and takes
  every part of a shard set.
- A never-launched model opens on the defaults; a launched one opens on its own
  `lastUsed`.
- A v1 config on disk does not have its old `defaultProfile` adopted as
  `launchDefaults`.

## Out of scope

Queueing — one at a time still refuses rather than queues. Auto-resume on
launch. A named-preset profile system. A configurable history cap.

## Verification

The four commands in [knowledge/technical.md](../knowledge/technical.md), green,
with each status captured directly. Then per parcel, because tests alone have
never been enough to close work on the downloader and were not enough for the
speed limit either:

- **P1** — a real Hugging Face transfer of about 1 GB, killed by quitting the
  app, restored, resumed and sha256-verified. Plus a real Discard, and a real
  pre-existing `.part` recovered.
- **P2** — a real delete into a real Trash, and a favourite surviving a real
  restart.
- **P3** — both seeding paths in the running app.

Each new test checked against a gutted implementation. The UI half is covered by
`tsc` and by looking at the screen; there is still no frontend test framework.
