# First beta release

Planned 2026-08-02, shipped 2026-08-03.
https://github.com/smkamranqadri/llamaport/releases/tag/v0.1.0

All four phases closed. What is left is not part of this plan: the README's
"Open Anyway" steps have never met a real Gatekeeper prompt, because the
verification download used `curl`, which sets no quarantine attribute. Proof and
its limits are in [state/current.md](../state/current.md).

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

**1 — Blockers — DONE 2026-08-02.** `show_main_window` is already called at the end of `setup`;
the planning note that said only the tray called it was wrong. The real defect
is that it returns early when there is no window, so the observed "no window at
all, empty Window menu" case has no recovery — it rebuilds from config instead.
Also fix the unit mix, which compares `outer_size()` in physical pixels against
600/400 and then sets a `LogicalSize`, so a 400x300 window reads as fine on a 2x
display. Reject `--host` and `--port` inside `rawArgs`, which the app already
owns as real fields. Update the README's security paragraph, which describes the
pass-through this phase removes.

**2 — Identity — DONE 2026-08-02.** A porthole mark, chosen over a harbour arrow
and a llama head. `icon.svg` is the committed source; regenerate with
`bunx tauri icon src-tauri/icons/icon.svg` and delete the iOS and Android sets
it emits unasked. Reasoning for the choice is in the commit, not repeated here.

Running `tauri build` for this phase surfaced a phase 4 problem early: the `.app`
bundles cleanly but `bundle_dmg.sh` fails, leaving only the `rw.*` intermediate.
No stale volume was mounted.

The cause was confirmed rather than guessed: `bundle_dmg.sh` drives Finder
through Apple events to set the disk image window, and an unrelated `osascript`
from the same shell returned "Not authorized to send Apple events to System
Events (-1743)".

**Solved 2026-08-03 without granting anything**, by setting `CI` so create-dmg
skips the Finder styling. The command and why it is mandatory belong in the
README's build section, where a contributor hits the problem; KIS does not keep
a second copy.

**3 — Public face — DONE 2026-08-03.** README rewritten for a stranger, MIT
LICENSE, `docs/library.png` on the front page, and both first-launch empty states
now say what to do rather than only what is wrong.

The screenshot was the author's to capture: screen recording is denied to the
agent, and going around a denial is not an option. Retake it with
`screencapture -o -w docs/library.png` if the UI changes — the window alone, not
the desktop. It shows `/Users/mkamran/models` in the subtitle, which is the
account name otherwise scrubbed from the repo; raised before capture and shipped
anyway, so it is a decision rather than an oversight.

The version display was added here after the app turned out to expose its
version nowhere — no About item, nothing in the UI. Counted as release
infrastructure rather than a feature: a beta whose bug reports cannot be tied to
a build wastes everyone's time. It reads the bundle version, not
`CARGO_PKG_VERSION`, so it always matches the `.dmg` filename.

The fresh-user check was run for real rather than reasoned about — the bundled
binary launched against a throwaway `HOME`, which reproduces no config and no
models directory exactly, and the author confirmed the empty state reads. The
missing-`llama-server` state could not be reproduced the same way: `discover`
falls back to hardcoded Homebrew paths, so short of hiding the author's binary
it will always find one. Only the wording changed there, on a render path that
already worked.

**4 — Ship — DONE 2026-08-03.** `.dmg` built, 54 commits pushed, `v0.1.0` tagged,
GitHub release marked pre-release with the `.dmg` attached and notes leading with
"unsigned".

`origin` is `git@github.com:smkamranqadri/llamaport.git`, renamed from
`llama-port` so one name covers repo, app, crate and bundle identifier. The repo
was already public, so the first push was the moment of publication; it was held
until phase 3 had landed a LICENSE and a stranger-facing README, because a public
repo with no licence is all-rights-reserved.

What the pre-publication scan found, kept because the next release needs it too:
no secrets, no `.env` or key files tracked, every "token" match a lockfile hash
or prose about Hugging Face tokens and GGUF tokenizers. `git grep` for a home
path before every push — proof entries are written by pasting real output, so the
account name comes back. All 54 commits are authored
`Muhammad Kamran <smkamranqadri@yahoo.com>`, now permanent.

`/security-review` was unusable throughout this work because it diffs against
`origin/HEAD`, which did not resolve without a remote. It should work now.

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

`/security-review` does not run on this project: it diffs against
`origin/HEAD...` and there is no remote. Phase 1's argv seam was reviewed by
hand instead, and that review is what added the argv ordering below. Decide
whether that skill is usable here before phase 4 relies on it.

The review's one structural finding is worth keeping: `check_raw_args` is a
blocklist, and llama.cpp adds flags faster than this app tracks them. So the
app's `--host` and `--port` are appended *after* `rawArgs`, and win by being
last. `rawArgs` still overrides everything else, which is what it is for. Where
the server binds no longer depends on the blocklist being complete.

## Carried, not met

The window bug has never been reproduced on demand — it was seen twice in one
session and not since. The fix is structural rather than a repair of a known
cause, and the proof is repeated launches, which is weaker than this project
normally accepts. The release notes should say so rather than claim it fixed.
