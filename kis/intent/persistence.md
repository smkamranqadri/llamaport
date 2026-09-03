# Persistence

Asked for by the author on 2026-08-03 and completed the same day, shipped in
v0.2.0. Download history vanished on restart, an incomplete download appeared
to be lost, the Library could not mark a favourite or delete a model, and
there was nowhere to set launch defaults.

## The asks

The bytes were never lost. A cancelled or interrupted transfer leaves its
`.part` and sidecar in the models directory, and starting the same URL again
resumes it; what was lost on restart was the job row, and the files were
invisible to the Library besides, since `catalog::scan` filters on the
`.gguf` extension.

Two stores split the work: the disk owns unfinished transfers, through the
sidecar that already carries `sourceUrl`, `total` and per-segment progress,
and `downloads.json` owns finished history alone, since completed, failed
and discarded entries have no on-disk trace to recover from. A startup scan
for orphaned `.part` files keeps older partials from being left stranded.

## Decisions

- **`Paused` replaces `Cancelled`.** Cancel already kept the bytes; it was a
  pause with the wrong name. Pause keeps the files; Discard deletes them, on
  the settle path inside the spawned thread rather than when cancellation is
  signalled, since the engine keeps writing after `control.cancel()` returns.
- **Resume reuses the job's id**, so a paused transfer resumed twice is one
  history row, not three. Interrupted transfers are never auto-resumed; they
  come back Paused and wait to be asked. A sidecar without its `.part` is
  listed unresumable, with a Discard option, rather than hidden. History
  itself is uncapped; the screen paginates 25 rows at a time, since the whole
  file loads anyway and no cap has evidence behind it.
- **Launch defaults seed, they do not override.** A model with no `lastUsed`
  entry opens its form from the defaults; anything ever launched keeps
  opening on its own last launch. The field is `launchDefaults`, not the
  retired `defaultProfile`, which would otherwise let serde adopt settings
  from a build two schemas old.
- **Delete moves the model to Trash**, using `osascript -l JavaScript`
  reaching `NSFileManager` rather than Finder automation or an added crate.
  Refused for the running model, and a shard set deletes as one unit. The
  row supplies its own confirmation, since `window.confirm` returns no
  usable answer in this webview.

## What was built

Three parcels: downloads surviving a restart, on `downloads.json` beside the
config; favourites and delete, keyed on the existing model id, schema to 6;
and launch defaults, `launch_defaults: Option<Profile>` in `Config` with a
Settings panel reusing `ProfileForm`.

## Acceptance

- Quit mid-transfer, reopen: the job is Paused with the sidecar's bytes, and
  Resume finishes it to a verified sha256, including a `.part` left by an
  earlier build. A sidecar with no `.part` is unresumable and offers no
  Resume; Discard clears a job's `.part` and sidecar and frees the space.
- Completed and failed rows survive a restart; Clear empties them and keeps
  them empty. A favourited model sorts to the top and stays favourited after
  a restart, a rename, and a directory move.
- Delete puts the file in Trash, is refused for the running model, and takes
  every part of a shard set. A never-launched model opens on the defaults; a
  launched one opens on its own `lastUsed`, and a v1 config's `defaultProfile`
  is never adopted as `launchDefaults`.

## Out of scope

Queueing; at the time one at a time still refused rather than queued
(added 2026-08-04, [downloader.md](downloader.md)). Auto-resume on
launch. A named-preset profile system. A configurable history cap.

## Verified

No proof was recorded for this phase at the time, and the gap is kept as is
rather than filled with a reconstructed record.
