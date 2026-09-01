# Roadmap

Goal: a releasable macOS app that both runs and downloads local GGUF models.

## Where this stands

Two subsystems, each with its own spec, module and screen, and both done. The
**runner** lists, launches, supervises and tests models
([docs/runner-spec.md](../../docs/runner-spec.md)). The **downloader** fetches
from Hugging Face with resume that survives a kill, a queue and a rate limit
([downloader.md](downloader.md)).

Since 2026-08-31 the app sizes a launch against ceilings that are real. It reads
the GPU working set from the build rather than assuming installed memory, can
leave the context and layer offload unset for llama.cpp to fit, charges the cache
only to layers that hold one, and agrees with Finder about file sizes. The model
screen is four figures and a verdict where it was ten form fields and four lines
of prose.

Since 2026-09-02 it also reaches outside itself: one click writes the provider and
the enabled entry pi needs to talk to the model this app is serving, after showing
what would change in both files.

Since 2026-09-01 it also measures. Every run that serves a request records what it
got, at the settings that got it; a ladder measures on request; and the app has
one opinion where it had none, offered rather than applied.

That is four phases, each carrying its own decisions and proof:
[figures.md](figures.md), [fitting.md](fitting.md), [screen.md](screen.md),
[tune.md](tune.md). What the app is *for* was rewritten on 2026-08-31 by its only
user, reversing two standing decisions ([direction.md](direction.md)).

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
   restart, and no Intel Mac has run the universal build. **The first is now a
   double-click** — v0.5.0's `.dmg` sits in `~/Downloads` with quarantine set
   ([release.md](release.md)).

   Two of this release's after-tag checks came off the list on 2026-09-01: the
   installed build is v0.5.0, and the Dock click was made by hand on it. Neither
   was one of the three above, which are older and stand.

9. ~~**Figures**~~ — done 2026-08-31, shipped in v0.4.0.
   [figures.md](figures.md). Two numbers the app printed were wrong: the KV term
   charged every layer a full-context cache, over-counting roughly fourfold on
   the hybrid the author runs while calling itself exact, and `formatBytes`
   divided by 1024 cubed while printing GB. Neither was a feature, which is why
   planning it did not test step 8's rule. It ended without a release, as
   planned; v0.3.2 carried it two hours later.

10. ~~**Fitting**~~ — done 2026-08-31, shipped in v0.4.0. [fitting.md](fitting.md).
   The app names `-c` and `-ngl` on every launch, and both overrule a llama.cpp
   default better than the value passed: `--fit` is on by default and adjusts
   only *unset* arguments, so filling them in switches it off once per launch.
   Context gains an Auto, `-ngl` stops insisting on `all`, and the cache figure
   Auto gives up before launch comes back at Ready from the server's own
   `n_ctx`. The load-bearing part is a capability gate: without `--fit`,
   omitting `-c` asks a 262,144-token cache of a 32 GB machine.

   The first item planned from [gaps.md](gaps.md), and the first planned for the
   author as a user of the built app rather than for an audience. Step 8's rule
   stands against imagined users; it never meant do not build what you need.

11. ~~**Screen**~~ — done 2026-08-31, shipped in v0.4.0. [screen.md](screen.md). The
   first phase cut from [direction.md](direction.md), which rewrote what this app
   is for after the author called a screen it had just shipped confusing. Read
   the GPU ceiling instead of comparing against installed RAM, turn the memory
   panel's four lines of prose into four figures, and put the seven settings the
   author has never touched behind Advanced.

   **Deliberately smaller than the mockup that was approved.** The app deciding
   the settings needs a measurement to be honest: the only rule available
   without one picks the slowest of three candidates on the model the author
   runs. Tune supplied that measurement and shipped in v0.5.0.

12. ~~**Tune**~~ — built 2026-08-31, seen and corrected on screen 2026-09-01,
   shipped in v0.5.0 the same day.
   [tune.md](tune.md). The second
   phase from [direction.md](direction.md) and the one the approved mockup waits
   for: the app measures rather than guesses, and only then has an opinion. Five
   parcels, and the first is that `cargo test` currently writes into the live
   Application Support directory — a speed record written on settle would have
   the suite corrupting the records the optimizer ranks.

   **Two of its decisions go against the mockup and against the recommendation
   made while planning**, and both are recorded where they were made. No
   arithmetic suggestion ships, because the only rule available picks the slowest
   of three; and rows from ordinary use may be ranked, gated on workload, which
   is a comparison this project has already been burned by.

   Five parcels, all built. What it measured changed the rule it was built on: the
   two full-precision rungs are within noise of each other, so the app suggests the
   widest of what it cannot tell apart rather than the fastest reading. **The
   screen was then looked at, and five defects came out of it that the suite could
   not see — which is where this project finds its defects, every time.**

13. ~~**The pi button**~~ — built and proved 2026-09-02, shipped in v0.6.0. It writes
   the provider *and* enables the model, because a provider alone is not
   selectable; pi answered a prompt through the entry the app wrote, picked up
   live with no restart. [pi.md](pi.md). Item 6 of
   what was asked for, and the third phase cut from
   [direction.md](direction.md). The app knows the port, the alias and the
   context the server accepted; pi's hand-maintained file had fallen behind on
   all three and could reach 2 of the 19 models this app has launched. A diff and
   a confirm rather than a write on the click, because the file is not the app's.

   **Both of its defects came from looking at it**, and one of them was a
   security defect the suite could not have seen: the write took a file holding
   five API keys from `600` to `644`. The rule that came out of it is in
   [knowledge/technical.md](../knowledge/technical.md) and applies to every
   writer of a file the app does not own.

## Gaps

What two comparable tools do and this one does not, read off their source on
2026-08-30 and recorded in [gaps.md](gaps.md). A list, not a plan; nothing there
is scheduled, and each item that reopens a decision says so where it stands.

From **LlamaForge**: no Library search, no keyboard map, a failed launch that
shows the log instead of the reason, live telemetry that is never written down —
and in-app Hugging Face search with its trimmings, which is the item below.

From **Unsloth**, which ships a Tauri app on our stack doing our job: no
auto-update, a launch that always names `-c` and `-ngl all` where llama.cpp can
now size the context itself, no warning before a load that cannot fit, and a
handful of UI settlements around bounded figures. Two are ours rather than
theirs, and both were fixed by step 9: the KV estimate charged every layer a
full-context cache, and `formatBytes` divided by 1024³ while printing GB.

## Decided against

**Discover was dropped twice and is back in scope from 2026-08-31**
([direction.md](direction.md)), because the author asked for it as the app's
user. The entry below stays as written: its reason was never wrong, and it now
sets the bar. A search box over repo ids is still a worse browser tab. What is
in scope is finding the *best model for this machine*, which is a different
problem.

**Benchmarks — removed 2026-08-01, after being built.** Recorded here on
2026-08-31, having never been written down at the time.
`31031b2` deleted `benchmarks.rs`, the Benchmarks and Connect screens, the
benchmark half of `health.rs` and the profile CRUD surface: 6,707 lines of Rust to
5,061, 29 commands to 17, on the grounds that roughly a third of the app served
features that were never part of the goal while the resumable downloader — half of
that goal — remained unbuilt. The reason was scope discipline, not a fault in the
design.

`benchmarks.json` was left on disk untouched and is still there, holding records
from that day keyed on model, context, both cache types, `ngl`, `parallel` and
`llamaVersion`. **[tune.md](tune.md) is that feature coming back**, asked for by
the author as the app's user, and it should read the deleted implementation before
writing a new one.

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
- ~~The Dock icon does not reopen the main window once it has been closed.~~
  Fixed 2026-08-08, issue 1: the run loop matched only `ExitRequested` and `Exit`,
  so the hide-on-close decision ([knowledge/project.md](../knowledge/project.md))
  had no way back but the tray. One `RunEvent::Reopen` arm calling the
  `show_main_window` the tray already used. Shipped in v0.3.0, so every public
  build from then on carries it. **Confirmed on the installed v0.5.0 on
  2026-09-01** by a hand click — window closed, Dock icon pressed, window back.
  The 2026-08-08 click was on a dev build; this is the first on a shipped one
  ([release.md](release.md)).
- ~~The runner spec has no "known gaps" section, but `README.md:50` sends readers
  to one.~~ Resolved by the phase 3 README rewrite; no such reference remains.
