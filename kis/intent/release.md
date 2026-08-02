# First beta release

Planned 2026-08-02. Not started.

A public, unsigned, MIT-licensed macOS beta on GitHub, tagged `v0.1.0` and
marked pre-release.

## Decisions

- **Public GitHub release, not a private handout.** Chosen over sending a `.dmg`
  to a few people directly, accepting that it pulls in a licence, a README for
  strangers, and an issue tracker someone is on the hook for.
- **Unsigned, and said out loud.** There is no Apple Developer Program
  membership and buying one costs $99/yr before anything says the app is wanted.
  A downloaded unsigned app is quarantined and blocked, and the old
  Control-click-Open shortcut is gone in recent macOS, so the README carries the
  System Settings route and the release notes lead with "unsigned". This loses a
  real share of testers. Accepted: the audience compiles llama.cpp already, and
  notarization is a build-time concern that can be added later without
  invalidating anything a tester did.
- **MIT.** What this ecosystem runs on — llama.cpp itself is MIT. A public repo
  with no LICENSE is all-rights-reserved, so publishing without one would
  contradict the point of publishing.
- **Both roadmap risks are fixed here, and documented.** This is the step the
  roadmap said to decide `rawArgs` before.
- **No auto-updater, no CI.** The updater needs signing keys, a hosted manifest
  and a maintenance commitment; CI needs runner config and secrets for a single
  target that builds on the Mac in front of us. Both are right after the beta
  says whether anyone cares. Neither is right before.

## What is actually hard here

Not the packaging. There is no remote, so this publishes a repository that has
never had an outside reader, and the README is written for its author: it
assumes the reader knows what `llama-server` is, has no screenshot, and never
says who the app is for. That decides whether anyone downloads anything.

Two first-launch failures outrank both recorded blockers. `llama-server` is a
hard dependency the app does not ship, and the models directory defaults to
`~/models`, which does not exist on a fresh Mac. A tester who gets past
Gatekeeper lands on an empty Library pointing at a missing folder with no binary
found. Both empty states have to explain themselves.

## Phases

**1 — Blockers.** Startup asserts a usable window rather than leaving it to a
tray item nobody knows exists. Fix the unit mix in `show_main_window`, which
compares `outer_size()` in physical pixels against 600/400 and then sets a
`LogicalSize`. Reject `--host` and `--port` inside `rawArgs`, which the app
already owns as real fields. Document the loopback-only, no-auth posture.

**2 — Identity.** Three SVG marks, one chosen, `tauri icon` regenerates all 16
icon files. The source SVG is committed as text. Independent of phase 1, and it
needs a review round, so it goes first.

**3 — Public face.** README rewritten for a stranger — what it is, who it is
for, `brew install llama.cpp` first, screenshots, the unsigned-app steps. MIT
LICENSE. A pass over the two empty states above.

**4 — Ship.** Build the `.dmg`, publish the repo, tag `v0.1.0`, GitHub release
marked pre-release.

## Closing conditions

- Launching the built `.app` five times from Finder shows a usable window every
  time, with the tray never the only way to get one.
- `rawArgs` containing `--host` or `--port` is refused with a message naming the
  field that owns it, covered by a test.
- No Tauri logo anywhere in the bundle; the Dock icon is the new mark.
- No `llama-server` found, and no models directory, both say what to do rather
  than showing an empty list.
- The `.dmg` is downloaded *through a browser* and opened following only what
  the README says. Copying the file locally does not count: quarantine is set by
  the download, and without it the test does not reproduce what a tester meets.
- LICENSE present; the README answers what, who and how without assuming the
  reader has seen the code.

## Verification

The four commands ([knowledge/technical.md](../knowledge/technical.md)), each
status captured directly. Then `bun run tauri build`, and the closing conditions
against the built `.app` and the downloaded `.dmg` — not against `tauri dev`,
because dev is exactly what a tester does not run.

`/security-review` on the phase 1 diff. The app runs an unauthenticated local
server and phase 1 touches the argv seam; that is the one place where a small
mistake is a real vulnerability in a public build.

## Carried, not met

The window bug has never been reproduced on demand — it was seen twice in one
session and not since. The fix is structural rather than a repair of a known
cause, and the proof is repeated launches, which is weaker than this project
normally accepts. The release notes should say so rather than claim it fixed.
