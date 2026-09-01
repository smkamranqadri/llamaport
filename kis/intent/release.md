# Releases

## v0.5.0 — shipped 2026-09-01

https://github.com/smkamranqadri/llamaport/releases/tag/v0.5.0 — Latest,
unsigned, both `.dmg`s attached. **Tune**, the whole phase in one release
([tune.md](tune.md)): every run records what it did, a ladder measures on
request, and the app has one opinion where it had none. A minor rather than a
patch — the model screen now tells you something it never told you. No schema
change; the config stays at 7 and `speeds.json` is a file of its own.

`Llamaport_0.5.0_aarch64.dmg`, 4,292,416 bytes, sha256
`0eff24e7886f8d9698f2c33be8a896b6a6e3f04f099eea17599c6c30a95dd009`.
`Llamaport_0.5.0_universal.dmg`, 8,608,076 bytes, sha256
`b3706b05805790945a249408ca275ea9c8aaaac4ffe7ee4b5e1d73411c0a50c7`. Both
downloaded back from the published release and compared with `cmp`, exit 0 on
each. The universal one carries `x86_64 arm64`.

Provable on both layers: the frontend bundle `index-DsHm8385.js` is embedded, and
`speeds.json` and `tune:report` are strings only this release's Rust introduces —
each appears **twice** in the universal build against once in the aarch64 one,
which is what says the fat binary is not a stub half, and **zero** times in the
0.3.1 binary still sitting in `/Applications`.

**The `git describe --exact-match` check caught something on its first real
outing.** `git tag v0.5.0` makes a *lightweight* tag, and `describe` without
`--tags` only considers annotated ones — so the check failed while the tag was
sitting on HEAD exactly as intended. Every earlier tag here is annotated
(`git tag -l v0.4.0 --format='%(objecttype)'` says `tag`, a lightweight one says
`commit`). The tag was recreated with `-a`. Left as it was, this release would
have shipped a tag unlike every other and the check would have read as a failure
on every future release. **Tag with `git tag -a`.**

Account name: 455 occurrences in the aarch64 binary and 916 in the universal,
the same cargo and source paths counted once per slice. The only one outside a
build path is the bundle identifier `com.mkamran.llamaport`, as in every release
here.

### Installed, and two of the three owed checks moved — 2026-09-01

**`/Applications` holds v0.5.0**, replacing the v0.3.1 that had sat there for
three releases. Installed from the asset downloaded back from the published
release, byte-identical to the build. The installed binary carries
`speeds.json`, `tune:report` and `index-DsHm8385.js`, so what is installed is
what was published, checked rather than assumed.

**Five launches, five usable windows**, each `1060x720` at the same position,
onscreen and opaque, verified through `CGWindowListCopyWindowInfo` rather than by
eye. *Not* the check exactly as written: `open -a` goes through LaunchServices as
a Finder double-click does, but it is not a double-click, and nothing verified
that no other app was fullscreen — the condition that made the v0.3.1 attempt
noise. Call it strong evidence that the window bug is not present, not the check
discharged.

**The Gatekeeper one is now one double-click away and still owed.** The `.dmg`
was downloaded through Chrome and is in `~/Downloads` with its quarantine
attribute set — the first time that has been true here. It could not be carried
further from this side: macOS denies the agent's shell any access to
`~/Downloads` (`Operation not permitted` on `ls`, `xattr` and `shasum`, while
`test -e` still finds the file). Opening *that* copy is what meets a real prompt;
installing from anywhere else sets no quarantine and proves nothing.

**The Dock click is unchanged and needs a human** — a synthesized press cannot
settle it, which is why it says so.

## v0.2.1 — shipped 2026-08-04

https://github.com/smkamranqadri/llamaport/releases/tag/v0.2.1 — pre-release,
unsigned, the download queue plus a path traversal fix.

`Llamaport_0.2.1_aarch64.dmg`, 4.0 MB, sha256
`e8d5f70988b2db16007d39ebe468dab7cf9432c539363de25e03d7f89e792e95`. Built with
`CI=true`, installed over the 0.2.0 in `/Applications`, and the published asset
downloaded back and compared byte for byte — identical, as with both previous
releases. The process from v0.2.0 worked unchanged.

**It is owed rather than optional.** v0.2.0 carries a path traversal: `restore`
rebuilds a row's path as `models_dir.join(file_name)` without checking the name,
so `../` in `downloads.json` reaches out of the models directory and a resume
writes there. Found while building the queue, by reading the file the app wrote.

The artefact was checked rather than assumed. It carries `Llamaport.app` at 0.2.1
with the right identifier; the binary contains `is already in the queue`, and no
longer contains `this app downloads one file at a time` — the refusal the queue
replaced is gone from the shipped build, not just from the tree. The 451
occurrences of the account name are cargo and source paths compiled in for panic
messages; v0.2.0 has 452 of the same, so it is not a regression, and nothing
outside build paths leaks.

Installing by local copy sets no quarantine attribute, so this **does not** settle
the Gatekeeper question — that still needs a download through a browser.

## v0.2.0 — shipped 2026-08-03

https://github.com/smkamranqadri/llamaport/releases/tag/v0.2.0 — pre-release,
unsigned, the whole Persistence phase plus three security fixes.

The process below worked unchanged: bump three version files, four commands,
`CI=true bun run tauri build`, mount and check the `.dmg`, scan for a home path
and secrets, tag, push, `gh release create --prerelease`, then download the
published asset back and compare digests. Both builds were byte-identical to
what was published.

**A security review is now part of shipping, because this one changed the
release.** It found a symlinked `.part` being written through — live in v0.1.0,
predating everything the phase touched — and a resume that never re-validated its
URL, which the phase had just introduced. Both were confirmed in the code before
being acted on, both fixed, and the `.dmg` rebuilt. Had it not run, the second
would now be in a public unsigned build.

The reviewer was a general code-review subagent given the `v0.1.0..main` diff and
told where to look, **not** the `/security-review` skill.

**That skill's blocker is now understood and fixed, 2026-08-03.** It failed on
`origin/HEAD...` for a *second* reason after the remote was added:
`refs/remotes/origin/HEAD` never existed locally. Cloning sets it; a remote added
to an existing repository does not. `git remote set-head origin -a` writes it,
and both `git log` and `git diff` against `origin/HEAD...` then resolve.

Two things about it are still worth knowing. It has never actually reviewed
anything here — the ref resolves, but nobody has run the skill to completion. And
`origin/HEAD...main` is an empty diff whenever `main` is pushed, so it has
nothing to look at unless there is uncommitted or unpushed work. Reviewing a
*release* means diffing the previous tag, which is what was done by hand for
v0.2.0 and is not what this skill does.

## v0.1.0 — first beta

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

The screenshot was the author's to capture: screen recording was denied to the
agent **as of 2026-08-03**, and going around a denial is not an option. That
denial was lifted on 2026-08-08 — see Distribution below, which is the current
answer. Retake it with `screencapture -o -w docs/library.png` if the UI changes —
the window alone, not the desktop. It shows `/Users/mkamran/models` in the subtitle, which is the
account name otherwise scrubbed from the repo; raised before capture and shipped
anyway, so it is a decision rather than an oversight.

**Retaken 2026-08-08**, by the author, from the built 0.3.0 — along with two
screens the README had never shown, running and Downloads. The denial held again
that day: neither `screencapture` nor the accessibility tooling could reach the
window, so every image here is the author's.

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

**Split 2026-08-31, after the same two went unmet across two releases.** They
were not being skipped out of carelessness. They were written as gates on
publishing, and half of them cannot be performed until a release exists — the
Gatekeeper one names a download *through a browser*, which needs a published
asset, so as a pre-publish gate it was unsatisfiable from v0.1.0 onward. A
condition that cannot be met does not get met, and then reads as a lapse. So
they are two lists now, and the second is expected to be settled after the tag
rather than before it.

### Before the tag — these gate the build

- `rawArgs` containing `--host` or `--port` is refused with a message naming the
  field that owns it, covered by a test.
- No Tauri logo anywhere in the bundle; the Dock icon is the new mark.
- LICENSE present; the README answers what, who and how without assuming the
  reader has seen the code.
- The change is demonstrable **in the artefact**, not inferred from the tree. A
  frontend change is proved by its new bundle digest, a Rust change by a string
  only it introduces. v0.3.1 could do neither and had to infer; v0.3.2 has both.

### After the tag — these need a published asset or a human at the machine

Owed against the *next* release if they go unmet, and recorded as owed rather
than quietly carried.

- Launching the installed `.app` five times from Finder shows a usable window
  every time, with the tray never the only way to get one. **Never with anything
  fullscreen** — that is what made the v0.3.1 attempt pure noise.
- Closing the window and clicking the Dock icon brings it back. Needs a real
  click; a synthesized press cannot settle it.
- No `llama-server` found, and no models directory, both say what to do rather
  than showing an empty list.
- The `.dmg` is downloaded *through a browser* and opened following only what
  the README says. Copying the file locally does not count: quarantine is set by
  the download, and without it the test does not reproduce what a tester meets.
  **This one is why the list was split** — it needs a published release.

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

## v0.3.0 — shipped 2026-08-08

https://github.com/smkamranqadri/llamaport/releases/tag/v0.3.0 — pre-release,
unsigned, Last used plus the row fixes it turned up. The published asset was
downloaded back and is byte-identical to what was built, as with all three
previous releases. The process from v0.2.0 worked unchanged.

`Llamaport_0.3.0_aarch64.dmg`, 4,221,465 bytes, sha256
`7bbb75f7479291031b3b86ad5c9122a71d8530c8ec554cf94ac29c81272babba`. Built with
`CI=true`, mounted and checked, and installed over the 0.2.1 in `/Applications`.
A minor rather than a patch because the config schema moves 6 -> 7.

**The v0.2.1 way of proving a feature is in the artefact did not work here.**
That release grepped the binary for a new string and for the string it replaced;
this change is mostly frontend, and Tauri compresses the embedded assets, so
`strings` finds neither. The replacement is better: Vite content-hashes its
filenames, so the name is a digest of the contents. `index-R_7rLVpB.js` carries
"Last launched" and no longer carries "before deleting it", `index-BaiJagCa.css`
carries `row-action` and `.model-item.is-running`, and the shipped binary embeds
exactly those two names and none from any earlier build. Use this for any release
that is frontend-heavy.

The account name appears 452 times in the binary, all cargo and source paths, the
same count as v0.2.0. Installing by local copy sets no quarantine attribute, so
Gatekeeper is still untested — as for every release so far.

### A universal build, added to the same release 2026-08-08

`Llamaport_0.3.0_universal.dmg`, 8,469,810 bytes, sha256
`bf0ce9392bc9bb9b122cb042556744f209acd8394a13a378185adc0515725051`, attached
alongside the `aarch64` one rather than replacing it. Same commit, same code, and
the same frontend digests are embedded — `index-R_7rLVpB.js` and
`index-BaiJagCa.css`, exactly the two the `aarch64` build carries — so this is
the shipped build widened, not a rebuild of something else. Downloaded back and
byte-identical, as with every release here.

**"Apple Silicon only" was never true of the code**, only of the artefact. The
platform coupling is Darwin's, itemised in
[knowledge/technical.md](../knowledge/technical.md); no `cfg(target_arch)` exists
anywhere in the source. Two lines of build recipe closed the gap:
`rustup target add x86_64-apple-darwin`, then `--target universal-apple-darwin`.

Both slices are present (`lipo -archs` on the mounted app: `x86_64 arm64`), and
the account name count doubles to 912 from 453 for exactly that reason — same
cargo and source paths, twice. Nothing names the account outside build paths.

**The x86_64 half was run, not merely inspected.** Launched under Rosetta against
a throwaway `HOME`, it registered with LaunchServices from the mounted volume as
`com.mkamran.llamaport`, `type="Foreground"`, `Arch=x86_64` — a real GUI app, so
the slice is not a stub. Rosetta is not Intel hardware and the README says so.
An actual Intel Mac remains untested and nobody here has one.

**The Intel audience is fixed and dated, which is the thing to weigh before
building universal again.** macOS 26 Tahoe is the last major release for Intel
Macs — macOS 27 is Apple Silicon only — so the reachable machines are four
2019–2020 models frozen on Tahoe, with security updates until roughly 2028. The
cost stays two lines of build recipe, so this is not an argument for stopping;
it is the reason not to spend anything further on Intel beyond them.

**Rosetta is a different subject and does not bear on this.** It translates Intel
code on Apple Silicon; a universal binary runs natively on both, so nothing
Llamaport ships ever invokes it. Rosetta was only reached here by forcing
`arch -x86_64` for the proof above, which is also what raised the macOS 26.4
"Intel app" notification on the author's machine — an artefact of the test, not
something any user of this app will see. Rosetta is full through macOS 27 and cut
back in macOS 28; neither date changes anything here.

**The screenshots were the author's, and they closed the last of the UI proof.**
Orca, the accessibility tooling this session had used all day, was uninstalled
partway through, so nothing could read or capture the running app. Three captures
from the built 0.3.0 settled what was owed: the weight distinction in the recency
cell, the running row tinted end to end, its hover staying one colour, and its
figures still flush with the rows above.

## v0.3.1 — shipped 2026-08-08

https://github.com/smkamranqadri/llamaport/releases/tag/v0.3.1 — the first
release published as **Latest rather than pre-release**, unsigned, both `.dmg`s
attached from the start. One fix, no schema change; the config stays at 7.

`Llamaport_0.3.1_aarch64.dmg`, 4,221,604 bytes, sha256
`848b8d837da822321262c14629547e2af253293e169a3ef5b3596f5f6e88063a`.
`Llamaport_0.3.1_universal.dmg`, 8,469,496 bytes, sha256
`bd73ca4a6182cb5df89c60c8748157c9fbf0d30fa9cc59efdac4e40954bb4a0d`. Both
downloaded back byte-identical, as with every release here. The universal one
carries `x86_64 arm64`. The account name appears 454 times in the aarch64 binary
and 912 in the universal, all cargo and source paths; the only hit outside them
is the bundle identifier itself, which is public.

**The frontend digests are unchanged from v0.3.0** — `index-R_7rLVpB.js` and
`index-BaiJagCa.css` — and that is the correct result rather than a stale build:
this release changes six lines of Rust and no frontend at all. The v0.3.0
technique proves a *frontend* change is present; it says nothing either way about
a Rust one, which is the gap below.

**Two closing conditions were not met, and it shipped anyway — a decision, not an
oversight.** The author chose to publish rather than hold.

- **That the fix is in the artefact was inferred, not seen.** The change adds no
  new string, so `strings` cannot find it, and the behaviour needs a real Dock
  click to demonstrate — a synthesized press cannot settle it (see Distribution).
  What is known: the tree was proved in a dev build, and the `.dmg` was built
  from the tagged commit.
- **The five-launch condition was not run.** A fullscreen app owned the display,
  which changes how a launched app gets a Space, and every reading taken in that
  state was noise: v0.3.1 showed no window on 4 of 5 launches and **the published
  v0.3.0 did the same on 3 of 3**, which is what proves the measurement rather
  than the build was at fault. Do not read those numbers as a defect in either
  build, and do not repeat a launch check while anything is fullscreen.

Both are owed against the next release rather than closed here.

## v0.3.2 — shipped 2026-08-31

https://github.com/smkamranqadri/llamaport/releases/tag/v0.3.2 — Latest,
unsigned, both `.dmg`s attached. Three corrected figures and one slider step; no
schema change, the config stays at 7.

`Llamaport_0.3.2_aarch64.dmg`, 4,222,157 bytes, sha256
`5a4c2356dbd44c2f9406b6a410950fe4be52f949f52fee62d27d38c100943d3e`.
`Llamaport_0.3.2_universal.dmg`, 8,472,231 bytes, sha256
`e9b6fd5e4df1bb375cfd07514badee8fc43ff69fd1be473abd28ee982ab3f141`. Both
downloaded back byte-identical. The universal one carries `x86_64 arm64`.

**The artefact gap v0.3.1 recorded is closed here, rather than merely absent.**
That release changed six lines of Rust with no new string, so `strings` could not
find the fix and its presence had to be inferred. This one is provable on both
layers independently: the frontend bundle `index-g7XWERza.js` is embedded, and
the Rust string "does full attention" appears in the binary — twice in the
universal, once per slice, which is also what says the fat binary is not a stub
half.

**Why it shipped now rather than at some later "end".** The plan had been to
gather an audience first and build after. No audience arrived, and meanwhile the
author is using the app daily and asking for things. Batching optimises for
readers who are not there yet at the cost of the one user who is, and holding
fixes repeats v0.3.0, which carried a known Dock bug through a whole release.
The cadence is now: publish whenever there is something worth installing.

Owed against the next release, and expected to be settled after this tag rather
than before it (see the split above): the five-launch check, the Dock click, and
the browser download meeting a real Gatekeeper prompt.

## v0.4.0 — shipped 2026-08-31

https://github.com/smkamranqadri/llamaport/releases/tag/v0.4.0 — Latest,
unsigned, both `.dmg`s attached. **A minor rather than a patch**: the context and
layer offload can be left unset for llama.cpp to fit, which changes what a
never-launched model opens on, and the model screen is reorganised. No schema
change; the config stays at 7.

`Llamaport_0.4.0_aarch64.dmg`, 4,226,049 bytes, sha256
`2a0c661626ddbf05ffe99f4054649e43979b0afe416840e1507b710e2ba2c4f1`.
`Llamaport_0.4.0_universal.dmg`, 8,489,627 bytes, sha256
`3244261b5748c68e9e19167abd43161ff1dc474309dee3e684ec2e5e72b84d36`. Both
downloaded back byte-identical. The universal one carries `x86_64 arm64`, and
each of its slices holds the change once — the frontend digest and the Rust
string both read twice there against once in the aarch64 build, which is what
says the fat binary is not a stub half.

Provable on both layers, as v0.3.2 established: the frontend bundle
`index-Cm_QE7SR.js` is embedded, and `"list-devices"` is a string only this
release's Rust introduces.

**Three phases in one release** — Figures, Fitting and Screen. That is more than
the cadence set on 2026-08-31 intends, and the reason is worth recording: the
cadence was agreed at midday and the phases were finished the same afternoon,
faster than a release was cut. Publishing after each would have been three
releases in six hours, which is a different failure. The rule stays "publish when
there is something worth installing"; the judgement is that three finished phases
in an afternoon is one such moment, not three.

Owed against the next release, unchanged and now three releases old: the
five-launch check, the Dock click, and the browser download meeting a real
Gatekeeper prompt (see the split above).

## Distribution — from 2026-08-08

Everything here is about being found. None of it changes the app.

**Nothing has converted, and nobody has looked.** Stars, forks and watchers were
0 after the 2026-08-08 push and have not been checked since — two releases and a
rewritten purpose later. Show HN is drafted and unposted, deferred by the author
rather than blocked.

**`v0.3.0` is no longer a pre-release.** It is the Latest release, both `.dmg`s
and their download counts intact, nothing rebuilt — the tag's code and HEAD's are
identical. The badge was the only thing standing between a drive-by reader and a
build that works, and a `github_latest` livecheck skips a pre-release entirely.

**The repository had no description, no topics and no social preview** six days
after being made public, which is where every announcement was going to land. All
three are now set. The description leads with "llama.cpp" rather than
"llama-server" because that is the term people search.
`docs/social-preview.png` is committed as the source; the upload itself is
browser-only, was done by the author, and lives on GitHub rather than in the repo
— `usesCustomOpenGraphImage` reads true, which is the only way to verify it.

**A Show and tell post is live in llama.cpp's own Discussions:**
https://github.com/ggml-org/llama.cpp/discussions/26772

That category is what replaced the third-party UI list llama.cpp's README used to
carry. The list is gone from the README, the wiki and `docs/`, so there is no
longer a PR to send — checked 2026-08-08, and worth re-checking before assuming
otherwise.

**The post is a standing obligation, not a one-off.** It claims what the app
does, that it has no chat of its own, that it is an unsigned beta, and that no
Intel Mac has run the universal build. Every one of those can go stale. Edit it
whenever a release changes what it claims, and treat that as part of shipping
rather than as publicity.

**Four list submissions are open**, all disclosing authorship:

- `jaywcjlove/awesome-mac`, 109k stars — https://github.com/jaywcjlove/awesome-mac/pull/2526
- `serhii-londar/open-source-mac-os-apps`, 50k — https://github.com/serhii-londar/open-source-mac-os-apps/pull/1252
- `rafska/awesome-local-llm`, 2.5k — https://github.com/rafska/awesome-local-llm/pull/171
- `vince-lam/awesome-local-llms` — https://github.com/vince-lam/awesome-local-llms/issues/66

Two large lists were checked and skipped as dead rather than missed:
`Hannibal046/Awesome-LLM` (27k stars, no push in a year) and
`underlines/awesome-ml` (16 months). A PR into an unmaintained list is a PR
nobody merges. The vince-lam one is an issue rather than a PR because its table
is generated weekly from the top 100 repositories by stars, which this will not
reach for a long while; the issue says so rather than pretending otherwise.

**What each channel will and will not accept, learned the hard way:**

- **r/LocalLLaMA** forbids primarily LLM-generated copy outright (rule 3), and
  requires disclosed affiliation with a 1-in-10 self-promotion guideline (rule
  4). Posting also needs karma this account does not have. Any post here has to
  be written by the author, in the author's words.
- **Hacker News** has no karma gate on submissions, which makes Show HN the one
  launch channel open today. Never solicit upvotes anywhere — HN detects voting
  rings and penalises the submission, which is the usual way a good Show HN
  dies. A Show HN also needs answering for its first three hours or it sinks.
- Reddit blocks every automated route to its rules and to posting, so anything
  about that site has to be checked by a person in a browser.

**Homebrew was considered and deferred.** `homebrew/cask` requires notability —
75 stars, 30 forks or 30 watchers — and this repository has none of them. An own
tap works today and needs nobody's permission, but Homebrew quarantines every
cask download and a cask cannot opt out, so an unsigned build would install and
then refuse to open unless the user passes `--no-quarantine`. Not worth
publishing until either the stars or the signature exist. Staying unsigned was
reaffirmed the same day, now with the price named rather than assumed: the
Gatekeeper wall is the largest single drop-off between a visitor and a star.

### The agent could capture the app this time

Screen recording and accessibility were granted on 2026-08-08, and the agent
drove the running app directly — `screencapture -v` over the window rect, System
Events `click at` for every click. The webview answers `click at` by naming the
button it hit, but exposes nothing to `entire contents`, and neither Page Down
nor a scroll bar will move the page, so anything below the fold stays out of
reach. This is the first UI artefact here that is not the author's.

`docs/launch.gif` replaces `docs/library.png` on the front page: eleven seconds,
391 KB, two cuts out of a twenty-second take, ending on Test model reporting
every check passed. `docs/launch.mp4` is the same cut for platforms that render
video better than a GIF. The still is kept but is no longer the front page: phase
3's retake command still names it, and the social preview card was built from it.

**A window bug turned up while recording, and this one reproduces on demand.**
Closing the main window is *meant* to hide it rather than quit
([knowledge/project.md](../knowledge/project.md), and the label check in
[knowledge/technical.md](../knowledge/technical.md)) — that decision is not the
bug and must not be "fixed" by making close quit. The bug is that nothing brings
the hidden window back: the Dock icon does nothing. The route back is the tray's
**Show window** — checked in the accessibility
tree with no window open, where the app menu offers nothing and the Window menu
holds only the standard macOS items, all inert. macOS convention is that the Dock
icon reopens, so this is a missing reopen handler: the run loop matches only
`ExitRequested` and `Exit`, never `RunEvent::Reopen`, while `show_main_window`
already exists and the tray already calls it. Filed as
https://github.com/smkamranqadri/llamaport/issues/1, the repository's first.
Unlike the window bug under "Carried, not met", it is reproducible.

**Fixed the same day**, by the one missing arm. It carries no test: the run loop
closure has no seam the harness reaches, and a test that cannot fail when the arm
is removed would be worse than none — so the proof is the app, in the pattern
this project already uses for what tests cannot reach. A dev build was run, its
window closed, the window count confirmed at zero, and the Dock icon clicked; the
window came back.

**The verification method mattered here.** A synthesized accessibility press on
the Dock tile restores the window only when another app holds focus, and does
nothing while llamaport is already frontmost — which is the scenario the bug
describes. That looked like a half fix and was not: the author clicked the Dock
icon by hand in that exact state and the window returned. A synthetic press
appears to reach only "activate", which is a no-op for an already-active app, so
it never reaches the delegate. Anything about Dock or reopen behaviour needs a
real click; do not conclude from a scripted one.

It is live in v0.3.0 and unreleased, so every public build still has the bug.
