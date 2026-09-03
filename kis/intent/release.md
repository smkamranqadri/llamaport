# Releases

Nine releases, all unsigned by Apple. Each entry below carries what shipped, the
artefact proof, and only the lessons that still bind. **Every release here was
downloaded back from GitHub and compared byte for byte with what was built**, so
that check is stated once rather than nine times.

## v0.6.1 — shipped 2026-09-02

https://github.com/smkamranqadri/llamaport/releases/tag/v0.6.1 — **packaging
only**, no functional change from v0.6.0, and the release that made the app
openable at all.

**The Gatekeeper check finally ran, and it failed.** Owed since v0.1.0 and never
performed, it was run for real on 2026-09-02: the author downloaded v0.6.0
through a browser, installed it, clicked it, and got **"Llamaport.app is damaged
and can't be opened."** Not the unidentified-developer prompt the README had
described for five releases, and there is no **Open Anyway** button for that
message — so the documented steps could never have worked on any release this
project had published.

Diagnosed rather than guessed: the shipped bundle carried only the ad-hoc
signature the linker puts on the arm64 executable — `flags=0x20002(adhoc,
linker-signed)`, `Info.plist=not bound`, `Sealed Resources=none` — because Tauri
runs `codesign` only when an identity is configured. macOS could not check the
app at all, so it refused instead of asking. `"signingIdentity": "-"` makes the
build sign ad-hoc properly; the shipped bundles verify at
`flags=0x10002(adhoc,runtime)`. The rule is in
[knowledge/technical.md](../knowledge/technical.md).

`Llamaport_0.6.1_aarch64.dmg`, 4,252,086 bytes, sha256
`6b7b8ae3f9ceaaed388c2aa6563575c678d1a9aa6ec5cb8db84d0fb5fb839749`.
`Llamaport_0.6.1_universal.dmg`, 8,714,320 bytes, sha256
`34baad55cd5e6bc08c31e88c46d4e10289859663aa1f9cf8961b9692afde384c`. The
universal one carries `x86_64 arm64`, and the aarch64 `.dmg` was mounted from
the *downloaded* file and the app inside verified — the artefact check this
release exists for.

**Every release from v0.1.0 to v0.6.0 carries a warning block** added the same
day — nine of them — naming the dialog, saying the app is not damaged, and
giving the `xattr` line, because each told readers to use Open Anyway.

**Confirmed end to end**: v0.6.1 was downloaded through a browser and opened,
macOS said it could not verify the developer, and Open Anyway let it through.
**The check owed since v0.1.0 is met** — it took two runs on one day, the first
to find that five releases were broken and the second to prove the fix.

## v0.7.0 — shipped 2026-09-04

https://github.com/smkamranqadri/llamaport/releases/tag/v0.7.0 — tagged at
`12de686`, published as a full release.

The first release to carry the redesign, Appearance, `tauri-plugin-dialog`,
Activity Monitor, Discover — a new capability and this app's first read-only
network client — the owner avatars, the translucent sidebar, a CSP, the
2026-09-04 code review's 22 fixes, and a **private API**. A minor rather than a
patch. Asked for 2026-09-04 with "ok release it".

**The private API is the one that changes what this project can do later.**
`macOSPrivateApi` is required for the translucent sidebar and bars the App Store
permanently ([appearance.md](appearance.md)). That was never a route this app was
taking, but it stops being one.

**The security review ran before the tag and changed what ships, as v0.2.0's
did.** A general-purpose reviewer over `v0.6.1..HEAD`, not the `/security-review`
skill. Two Medium, three Low, four Informational; areas checked clean were
traversal, host confusion on downloads, injection into the webview, the
planted-file cases and the async locks. Every finding but one was fixed in
`12de686`:

- **The allowlist stopped at `file_name_for`.** A browse cursor was fetched
  verbatim — a `Link` header or the webview could name `http://169.254.169.254/`
  and have it fetched and parsed as page two — and an `avatarUrl` was checked for
  `https://` and nothing else, with ureq following it and five redirects to any
  host. One rule now, `hub::on_hugging_face`: https, on `HOSTS` or a
  `.huggingface.co` subdomain; and the agent is `https_only`. A redirect can
  still reach another https host after the first check, for a GET with no
  credentials that becomes at most a 512 KB image; accepted.
- **The transfer engine followed a redirect to any scheme.** It may now change
  host — Hugging Face hands off to a CDN — and never scheme. The engine's own
  tests run against a local http server, which is why the rule is "same scheme"
  rather than "https", and why the check has a unit test and no engine test.
- **A file planted in `avatars/` went into an `<img src>` unread.** Only an
  empty file or a `data:image/` URI is served from the cache.
- **The Discover commands blocked the async workers.** `async fn` with a
  blocking body parks a worker; a cold page fires a dozen avatar fetches with
  20 s timeouts, and a network answering nothing would have parked every worker
  and with them every command, runner control included. The three run under
  `spawn_blocking`, with `Trees` behind an `Arc`.
- `opener:default` was granted and unused — dropped. The CSP gains `base-uri
  'none'; form-action 'none'`, which `default-src` does not cover.
- **Not fixed**: `image/svg+xml` passes the avatar's `image/` check. Inert inside
  an `<img>`, so left.

**The README screenshots came first this time**, by the author's choice: seven
of them, taken 2026-09-04 from the running app, in a new `assets/` folder rather
than `docs/`, which is documents. The three stills and the launch recording
from 2026-08-08 went with the move; they predated the redesign entirely.

`Llamaport_0.7.0_aarch64.dmg`, 4,570,488 bytes, sha256
`1b1cdd7991ce1bc1b06531e27b4fd956ce08b39645b3a341ab3e65526fb82caa`.
`Llamaport_0.7.0_universal.dmg`, 9,376,315 bytes, sha256
`1a779349dc167140771321957987110defa7b46d03b0a76aba9313cd640c4b02`, carrying
`x86_64 arm64`. Both `.app`s verify with `codesign --verify --deep --strict` at
`flags=0x10002(adhoc,runtime)`, `Sealed Resources version=2`. Proved on both
layers: `index-BluDBz9J.js` is embedded, once in the aarch64 build and once per
slice of the universal, and "the next page is not on Hugging Face" — a string
only `12de686` introduces — is in both and zero times in the installed 0.6.1.
Built with `CI=true`, downloaded back from GitHub and compared byte for byte.

**Checked by the author the same day, all good**: five launches from Finder,
the Dock click, a queued row with nothing on disk surviving a restart, the
folder picker on the signed bundle, the Discussions post brought up to date,
and Discover's avatars under the tightened rule. **Only the Intel Mac remains**,
as it has since v0.3.0, because nobody here has one.

## Unverified against v0.7.0

Moved out of [state/current.md](../state/current.md) on 2026-09-02, where it had
become a ledger. Four of the five were met by the author on 2026-09-04 against
v0.7.0 — five Finder launches with nothing fullscreen, the Dock click, a queued
row with nothing on disk coming back from a restart, and the folder picker on
the signed bundle. One remains:

- An **Intel Mac** running the universal build. Open since v0.3.0; nobody here
  has one, and the Rosetta run is the nearest thing to evidence.

## v0.6.0 — shipped 2026-09-02

https://github.com/smkamranqadri/llamaport/releases/tag/v0.6.0 — **the pi
button** ([pi.md](pi.md)): the app reaches outside itself for the first time,
writing the provider and enabled entry pi needs. A minor rather than a patch —
it is a new capability, and the first time this app writes a file it does not
own. Config stays at schema 7.

`Llamaport_0.6.0_aarch64.dmg`, 4,315,842 bytes, sha256
`7d77375594a5696bb032ca9c94a6f282b905a85a6c35d4da2e22bb41e65929f8`.
`Llamaport_0.6.0_universal.dmg`, 8,663,516 bytes, sha256
`d5b1b9a89a4b59ae21f7378a130ce285af8dbf5f13384ae4a9c720d53f545e3d`.

Proved on both layers: the frontend bundle `index-Cdq2l2nV.js` is embedded, and
`enabledModels` and `pi_apply` are strings only this release introduces — zero
in the installed 0.5.0, once in the aarch64 build, once in *each slice* of the
universal one.

**A string missing from one slice is not evidence of a stub.** `llamaport.bak`
appears in the arm64 slice and not at all in the x86_64 one: a 14-byte literal
can be emitted as immediate stores rather than a rodata reference, which puts it
beyond `strings`. Prove a fat binary with more than one string, and prefer
longer ones.

## v0.5.0 — shipped 2026-09-01

https://github.com/smkamranqadri/llamaport/releases/tag/v0.5.0 — **Tune**, the
whole phase in one release ([tune.md](tune.md)): every run records what it did, a
ladder measures on request, and the app has one opinion where it had none.
Config stays at 7; `speeds.json` is a file of its own.

`Llamaport_0.5.0_aarch64.dmg`, 4,292,416 bytes, sha256
`0eff24e7886f8d9698f2c33be8a896b6a6e3f04f099eea17599c6c30a95dd009`.
`Llamaport_0.5.0_universal.dmg`, 8,608,076 bytes, sha256
`b3706b05805790945a249408ca275ea9c8aaaac4ffe7ee4b5e1d73411c0a50c7`. Proved by
`index-DsHm8385.js`, `speeds.json` and `tune:report`.

**Tag with `git tag -a`.** `git tag v0.5.0` makes a *lightweight* tag, and
`git describe --exact-match` without `--tags` only considers annotated ones — so
the check failed while the tag sat on HEAD exactly as intended. Every earlier
tag here is annotated. Left alone, this release would have shipped a tag unlike
every other and the check would have read as a failure on every future release.

**Installed and checked, 2026-09-01.** `/Applications` holds v0.5.0, installed
from the downloaded asset, and the installed binary carries this release's three
strings — what is installed is what was published, checked rather than assumed.
**Five launches, five usable windows**, each 1060x720, verified through
`CGWindowListCopyWindowInfo` rather than by eye — but `open -a` is not a Finder
double-click and nothing verified that no other app was fullscreen, so this is
strong evidence rather than the check discharged.

**The Dock click is met.** The author ran the installed build, closed its
window, clicked the Dock icon, and the window came back. A real press is the
only kind that settles it: a synthesized one reaches only "activate" and is a
no-op while the app is frontmost, so it never reaches the delegate. **Anything
about Dock or reopen behaviour needs a real click.**

## v0.4.0 — shipped 2026-08-31

https://github.com/smkamranqadri/llamaport/releases/tag/v0.4.0 — context and
layer offload can be left unset for llama.cpp to fit, which changes what a
never-launched model opens on, and the model screen is reorganised. Config stays
at 7.

`Llamaport_0.4.0_aarch64.dmg`, 4,226,049 bytes, sha256
`2a0c661626ddbf05ffe99f4054649e43979b0afe416840e1507b710e2ba2c4f1`.
`Llamaport_0.4.0_universal.dmg`, 8,489,627 bytes, sha256
`3244261b5748c68e9e19167abd43161ff1dc474309dee3e684ec2e5e72b84d36`. Proved by
`index-Cm_QE7SR.js` and `"list-devices"`.

**Three phases in one release** — Figures, Fitting and Screen — which is more
than the cadence intends. The cadence was agreed at midday and the phases were
finished the same afternoon; publishing after each would have been three
releases in six hours. The rule stays "publish when there is something worth
installing".

## v0.3.2 — shipped 2026-08-31

https://github.com/smkamranqadri/llamaport/releases/tag/v0.3.2 — three corrected
figures and one slider step.

`Llamaport_0.3.2_aarch64.dmg`, 4,222,157 bytes, sha256
`5a4c2356dbd44c2f9406b6a410950fe4be52f949f52fee62d27d38c100943d3e`.
`Llamaport_0.3.2_universal.dmg`, 8,472,231 bytes, sha256
`e9b6fd5e4df1bb375cfd07514badee8fc43ff69fd1be473abd28ee982ab3f141`.

**The artefact gap v0.3.1 recorded is closed here**: provable on both layers
independently — `index-g7XWERza.js` embedded, and the Rust string "does full
attention" in the binary, twice in the universal, once per slice.

**Why it shipped now rather than at some later "end".** The plan had been to
gather an audience first and build after. No audience arrived, and meanwhile the
author uses the app daily and asks for things. Batching optimises for readers who
are not there yet at the cost of the one user who is. **Publish whenever there is
something worth installing.**

## v0.3.1 — shipped 2026-08-08

https://github.com/smkamranqadri/llamaport/releases/tag/v0.3.1 — the first
published as **Latest rather than pre-release**. One fix.

`Llamaport_0.3.1_aarch64.dmg`, 4,221,604 bytes, sha256
`848b8d837da822321262c14629547e2af253293e169a3ef5b3596f5f6e88063a`.
`Llamaport_0.3.1_universal.dmg`, 8,469,496 bytes, sha256
`bd73ca4a6182cb5df89c60c8748157c9fbf0d30fa9cc59efdac4e40954bb4a0d`. The frontend
digests are unchanged from v0.3.0, which is the correct result: this release
changes six lines of Rust and no frontend.

**Two closing conditions were not met and it shipped anyway — a decision, not an
oversight.** The fix adds no new string, so its presence in the artefact was
inferred rather than seen; and the five-launch check could not be read, because
**a fullscreen app owned the display** and every reading taken in that state was
noise — v0.3.1 showed no window on 4 of 5 launches and the published v0.3.0 did
the same on 3 of 3, which proves the measurement rather than the build was at
fault. **Never run a launch check while anything is fullscreen.**

## v0.3.0 — shipped 2026-08-08

https://github.com/smkamranqadri/llamaport/releases/tag/v0.3.0 — Last used plus
the row fixes it turned up. A minor because the config schema moves 6 → 7.

`Llamaport_0.3.0_aarch64.dmg`, 4,221,465 bytes, sha256
`7bbb75f7479291031b3b86ad5c9122a71d8530c8ec554cf94ac29c81272babba`.
`Llamaport_0.3.0_universal.dmg`, 8,469,810 bytes, sha256
`bf0ce9392bc9bb9b122cb042556744f209acd8394a13a378185adc0515725051`, attached
alongside rather than replacing it.

**How to prove a frontend-heavy release is in the artefact.** Tauri compresses
the embedded assets, so `strings` finds neither the new text nor the old. Vite
content-hashes its filenames instead, so the name is a digest of the contents:
`index-R_7rLVpB.js` and `index-BaiJagCa.css` are embedded and no name from any
earlier build is.

**"Apple Silicon only" was never true of the code**, only of the artefact. Two
lines of build recipe closed it: `rustup target add x86_64-apple-darwin`, then
`--target universal-apple-darwin`. Both slices are present, and the account-name
count doubles for exactly that reason — same cargo paths, twice.

**The x86_64 half was run, not merely inspected**, under Rosetta against a
throwaway `HOME`: it registered with LaunchServices as `Arch=x86_64`, so the
slice is not a stub. Rosetta is not Intel hardware; an actual Intel Mac remains
untested and nobody here has one. **The Intel audience is fixed and dated**:
macOS 26 Tahoe is the last major release for Intel, so the reachable machines
are four 2019–2020 models with security updates until roughly 2028. The cost
stays two lines of recipe, which is why the build continues — and why nothing
further is spent on Intel.

## v0.2.1 — shipped 2026-08-04

https://github.com/smkamranqadri/llamaport/releases/tag/v0.2.1 — the download
queue plus a path traversal fix, 4.0 MB, sha256
`e8d5f70988b2db16007d39ebe468dab7cf9432c539363de25e03d7f89e792e95`.

**It is owed rather than optional**: v0.2.0 carries a path traversal — `restore`
rebuilt a row's path as `models_dir.join(file_name)` without checking the name,
so `../` in `downloads.json` reached out of the models directory and a resume
wrote there. Found by reading the file the app wrote.

Proved in the artefact by a string that arrived (`is already in the queue`) and
one that left (`this app downloads one file at a time`).

## v0.2.0 — shipped 2026-08-03

https://github.com/smkamranqadri/llamaport/releases/tag/v0.2.0 — the whole
Persistence phase plus three security fixes.

**A security review is now part of shipping, because this one changed the
release.** It found a symlinked `.part` being written through — live in v0.1.0,
predating everything the phase touched — and a resume that never re-validated
its URL, which the phase had just introduced. Both confirmed in the code before
being acted on, both fixed, and the `.dmg` rebuilt. Had it not run, the second
would now be in a public unsigned build. The reviewer was a general code-review
subagent given the `v0.1.0..main` diff, **not** the `/security-review` skill.

**That skill still has not reviewed anything here.** It diffs
`origin/HEAD...`, which needed `git remote set-head origin -a` to resolve at
all, and `origin/HEAD...main` is an empty diff whenever `main` is pushed.
Reviewing a *release* means diffing the previous tag, which is not what the
skill does.

## v0.1.0 — first beta

Planned 2026-08-02, shipped 2026-08-03.
https://github.com/smkamranqadri/llamaport/releases/tag/v0.1.0 — a public,
unsigned, MIT-licensed macOS beta, marked pre-release. Four phases: the window
blockers, the porthole icon, a README written for a stranger with a LICENSE and
screenshots, and the ship itself.

Two things from it that still bind:

- **`CI=true` is mandatory for `tauri build`.** `bundle_dmg.sh` drives Finder
  through Apple events and fails without it ("Not authorized to send Apple
  events to System Events (-1743)"), leaving only the `rw.*` intermediate. The
  command lives in the README's build section; KIS does not keep a second copy.
- **The version is read from the bundle**, not `CARGO_PKG_VERSION`, so what a
  tester quotes always matches the `.dmg` filename. The app had exposed its
  version nowhere.

The README's "Open Anyway" steps were written here and never met a real
Gatekeeper prompt, because the verification download used `curl`, which sets no
quarantine attribute. **That sentence stood for five releases and cost them
all** — see v0.6.1.

## Decisions

**Do not write "Latest" into a release entry.** It is a state GitHub owns and
moves on the next release. Record what is durable — full release or pre-release
— and let `gh release list` answer which is current.

- **Public GitHub release, not a private handout**, accepting that it pulls in a
  licence, a README for strangers, and an issue tracker.
- **Unsigned, and said out loud.** No Apple Developer Program membership; $99/yr
  before anything says the app is wanted. This loses a real share of testers, and
  the Gatekeeper wall is the largest single drop-off between a visitor and a
  star. Accepted: the audience compiles llama.cpp already, and notarization can
  be added later without invalidating anything a tester did.
- **MIT**, which is what this ecosystem runs on — llama.cpp itself is MIT.
- **No auto-updater, no CI.** The updater needs signing keys, a hosted manifest
  and a maintenance commitment; CI needs runner config and secrets for a single
  target that builds on the Mac in front of us. Both are right after the beta
  says whether anyone cares.
- **`rawArgs` may not set what the app owns.** `check_raw_args` is a blocklist
  and llama.cpp adds flags faster than this app tracks them, so the app's
  `--host` and `--port` are appended *after* `rawArgs` and win by being last.
  Where the server binds no longer depends on the blocklist being complete.

## Closing conditions

**Split 2026-08-31, after the same two went unmet across two releases.** They
had been written as gates on publishing, and half of them cannot be performed
until a release exists — the Gatekeeper one names a download *through a
browser*. A condition that cannot be met does not get met, and then reads as a
lapse.

### Before the tag — these gate the build

- `rawArgs` containing `--host` or `--port` is refused with a message naming the
  field that owns it, covered by a test.
- No Tauri logo anywhere in the bundle; the Dock icon is the new mark.
- LICENSE present; the README answers what, who and how without assuming the
  reader has seen the code.
- The change is demonstrable **in the artefact**, not inferred from the tree: a
  frontend change by its new bundle digest, a Rust change by a string only it
  introduces. v0.3.1 could do neither and had to infer; v0.3.2 has both.
- `codesign --verify --deep --strict` on the built `.app` reports **valid on
  disk**. Added 2026-09-02, after five releases shipped a bundle macOS refused
  to open as damaged.
- The published asset is downloaded back and compared byte for byte.

### After the tag — these need a published asset or a human at the machine

Owed against the *next* release if they go unmet, and recorded as owed rather
than quietly carried. The live list is "Unverified against v0.6.1" above.

- Launching the installed `.app` five times from Finder shows a usable window
  every time. **Never with anything fullscreen.**
- Closing the window and clicking the Dock icon brings it back. Needs a real
  click.
- No `llama-server` found, and no models directory, both say what to do.
- The `.dmg` is downloaded *through a browser* and opened following only what
  the README says. Copying the file locally sets no quarantine and proves
  nothing. ~~Owed since v0.1.0.~~ **Met 2026-09-02** — see v0.6.1.

## Distribution — from 2026-08-08

Everything here is about being found. None of it changes the app.

**Nothing has converted, and nobody has looked.** Stars, forks and watchers were
0 after the 2026-08-08 push and have not been checked since. Show HN is drafted
and unposted, deferred by the author rather than blocked.

**A Show and tell post is live in llama.cpp's own Discussions:**
https://github.com/ggml-org/llama.cpp/discussions/26772 — the category that
replaced the third-party UI list the README used to carry, so there is no longer
a PR to send. **The post is a standing obligation, not a one-off**: it claims
what the app does, that it has no chat of its own, that it is an unsigned beta,
and that no Intel Mac has run the universal build. Edit it whenever a release
changes what it claims.

**Four list submissions are open**, all disclosing authorship:

- `jaywcjlove/awesome-mac`, 109k stars — https://github.com/jaywcjlove/awesome-mac/pull/2526
- `serhii-londar/open-source-mac-os-apps`, 50k — https://github.com/serhii-londar/open-source-mac-os-apps/pull/1252
- `rafska/awesome-local-llm`, 2.5k — https://github.com/rafska/awesome-local-llm/pull/171
- `vince-lam/awesome-local-llms` — https://github.com/vince-lam/awesome-local-llms/issues/66

Two large lists were skipped as dead rather than missed: `Hannibal046/Awesome-LLM`
(27k stars, no push in a year) and `underlines/awesome-ml` (16 months).

**What each channel will and will not accept:**

- **r/LocalLLaMA** forbids primarily LLM-generated copy outright, requires
  disclosed affiliation with a 1-in-10 self-promotion guideline, and needs karma
  this account does not have. Any post has to be the author's own words. Reddit
  blocks every automated route to its rules, so it has to be checked in a
  browser.
- **Hacker News** has no karma gate, which makes Show HN the one channel open
  today. Never solicit upvotes — HN penalises voting rings, which is the usual
  way a good Show HN dies. It also needs answering for its first three hours.
- **Homebrew is deferred.** `homebrew/cask` requires notability this repository
  does not have, and an own tap quarantines every download with no way to opt
  out, so an unsigned build would install and then refuse to open.

**The repository's description, topics and social preview** were empty six days
after it went public and are now set; `assets/social-preview.png` is the committed
source, and the upload is browser-only, so `usesCustomOpenGraphImage` is the only
way to verify it.

**Screenshots and recordings are the author's**, with one exception: on
2026-08-08 screen recording and accessibility were granted and the agent drove
the app directly to record a launch recording, since deleted with the 0.7.0
screenshots. That grant is
not the current arrangement — see
[knowledge/technical.md](../knowledge/technical.md) under Verify, which now rules
the app off limits to the session entirely. The README's images are due a retake
against the finished UI, which is phase 3's standing instruction. All three date
from 2026-08-08 and predate the entire redesign, so every one is owed rather than
some of them.
