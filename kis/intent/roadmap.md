# Roadmap

Goal: a releasable macOS app that both runs and downloads local GGUF models.

## Subsystems

Two, each with its own spec, module and screen:

- **Runner** — done. [docs/runner-spec.md](../../docs/runner-spec.md).
- **Downloader** — engine and Downloads screen done.
  [downloader.md](downloader.md).

## Order

1. ~~**Rename to Llamaport**~~ — done 2026-08-02, before packaging, because the
   bundle identifier is free to change until something is signed and step 2 signs
   it. [rename.md](rename.md).
2. ~~**First beta release**~~ — shipped 2026-08-03, unsigned, MIT, `v0.1.0` as a
   GitHub pre-release. [release.md](release.md). Signing and notarization were
   deliberately out: no Developer ID, and one costs $99/yr before anything says
   the app is wanted. Revisit if the beta lands — and note the bundle identifier
   is now effectively fixed, since changing it strands existing installs.

3. ~~**Persistence**~~ — done 2026-08-03, all three parcels.
   [persistence.md](persistence.md). Downloads that survive a restart, Library
   favourites and delete, launch defaults. This was the beta talking: every item
   came from the author using the built app, which is what step 2 was for.
4. ~~**v0.2.0**~~ — shipped 2026-08-03 with three security fixes the release
   review turned up. [release.md](release.md).

5. ~~**Download queue**~~ — done 2026-08-04. [downloader.md](downloader.md),
   phase 2. A second request waits instead of being refused. One transfer at a
   time is unchanged; what changed is that the app stopped saying no. Like every
   item in step 3, this came from the author using the built app rather than
   from a feature list. Proved by a four-deep queue draining ~48 GB unattended,
   and it turned up a path traversal that is live in v0.2.0.

6. ~~**Last used**~~ — done 2026-08-08. [last-used.md](last-used.md). The Library
   said when a file landed on disk and nothing about when it was last run. That
   cell became a recency cell and the list now sorts on it, which took the config
   to schema 7. Like steps 3 and 5 it came from the author using the built app,
   and looking at the reordered list turned up four more things about the row —
   the running highlight, its hover, its alignment, and a Stop button where a
   running model used to offer a Delete it would have refused.

7. ~~**Distribution**~~ — done 2026-08-08. The release was made findable rather
   than extended: a description and topics where there had been none, a social
   preview, a clip on the front page in place of a still, a Show and tell post in
   llama.cpp's own Discussions, and four list submissions.
   [release.md](release.md). Unlike steps 3, 5 and 6 this came from nobody using
   the app — it came from the app having no readers.

8. **Two live actions, no feature work.** Show HN is drafted and unposted, and is
   the only launch channel open today; r/LocalLLaMA needs karma this account does
   not have. And issue 1 below is worth fixing before more people arrive.

   Three things owed rather than planned, all older than this release: the
   README's "Open Anyway" steps have never met a real Gatekeeper prompt, a queued
   row with nothing on disk behind it has never been seen coming back from a
   restart, and no Intel Mac has run the universal build. Now that v0.3.0 is
   public, the first is five minutes of downloading the `.dmg` through a browser.

## Decided against

**Discover — dropped 2026-08-02, after being planned.** An in-app Hugging Face
browser: search, repo file listing, quant selection. Planned in full, then cut
before any code was written.

Two reasons, and they should stop this being planned a third time. Hugging Face's
`?search=` is a substring match over repo ids ranked by download count, so an
in-app search would have been a worse version of the browser tab that is already
open. And pasting a URL into Downloads already closes the loop the project set
out to close: `curl` not resuming was the problem, not finding the file.

Its research was not wasted: what it established about the engine's limits is in
[downloader.md](downloader.md) under "Known limits of the engine".

## Risks

All three are owned by [release.md](release.md) now — the first two by its phase
1, the third by its phase 3 README rewrite. They stay listed here because the
roadmap is where a reader looks for what is unresolved, not because they are
unplanned.

- `rawArgs` is passed to `llama-server` verbatim, so `--host 0.0.0.0` typed there
  exposes an unauthenticated server to the network. Acceptable for a personal
  tool; a release blocker. Decided: `rawArgs` may not set what the app owns.
- The app can start with an unusable window. Observed twice in one session: once
  with no window at all and the Window menu empty, once at 60x60.
  `show_main_window` asserts a usable frame and does not reliably achieve it.
  Predates the downloader, and is a release blocker of its own. Never reproduced
  on demand, so the fix will be structural rather than a repair.
- The Dock icon does not reopen the main window once it has been closed. Closing
  hides it by design ([knowledge/project.md](../knowledge/project.md)), and the
  run loop never matches `RunEvent::Reopen`, so the only way back is the tray's
  Show window. Filed as issue 1, and unlike the risk above it reproduces on
  demand.
- ~~The runner spec has no "known gaps" section, but `README.md:50` sends readers
  to one.~~ Resolved by the phase 3 README rewrite; no such reference remains.
