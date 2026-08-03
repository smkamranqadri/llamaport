# Current

```text
Branch:   main, clean and pushed. `v0.1.0` is tagged back at `05c3a21`, so the
          sixteen commits since it are on `main` but in no release — including
          the whole Persistence phase.
Task:     none in progress. The Persistence phase is finished — all three
          parcels committed and confirmed in the running app.
          [intent/persistence.md](../intent/persistence.md).
Mode:     Phase, three parcels
Blocker:  none. Unproved rather than blocking: the README's "Open Anyway" steps
          have never met a real Gatekeeper prompt, and Discard and the Downloads
          History pages have not been looked at.
Next:     decide whether the tree past `v0.1.0` wants a `v0.1.1`. The published
          `.dmg` has none of the Persistence phase, so anyone installing from
          the release page gets a build that loses download history. Discard
          and the Downloads History pages are still unlooked-at.
```

Run and verify commands: [knowledge/technical.md](../knowledge/technical.md).
Plans and decisions: [intent/roadmap.md](../intent/roadmap.md).

## Where the project actually is

The runner lists, launches, supervises and tests models. The downloader fetches
from Hugging Face in four ranged segments, survives a kill, resumes from its
sidecar, verifies sha256 and lands the file in the models directory. Its speed
limit and its time-remaining estimate are both reachable from the Downloads
screen, and a limit changed there reaches the transfer already running.

Downloads now outlive the app. A transfer is paused or discarded rather than
cancelled, an interrupted one comes back from the `.part` on disk and resumes,
and finished ones are kept in `downloads.json` and paged on the screen
([intent/persistence.md](../intent/persistence.md)).

The app is now Llamaport, identifier `com.mkamran.llamaport`, config under
`Application Support/llamaport` ([intent/rename.md](../intent/rename.md)).

A ready model offers **Web UI**, which opens `llama-server`'s own interface in a
second app window titled "llama.cpp — Web UI". An explicit stop closes it; a
Reload does not, because the server returns on the same port. The app still has
no chat of its own and is not getting one
([knowledge/project.md](../knowledge/project.md)).

**v0.1.0 is published**, unsigned, as a GitHub pre-release with the `.dmg`
attached: https://github.com/smkamranqadri/llamaport/releases/tag/v0.1.0. The tag
sits at `05c3a21`; `main` has moved eight commits past it and is pushed, so the
released build and the tree are not the same thing.

Discover was planned and then dropped ([intent/roadmap.md](../intent/roadmap.md)).

Apart from the two files named above, `git log` is the record.

## Proof

The four commands were last run green over the working tree, uncommitted changes
included: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
`cargo test`, `bun run build` — all exit 0, statuses captured directly rather
than through a pipe. 153 tests, up 8.

Launch defaults (parcel 3), 2026-08-03:

- Precedence is one pure function, `profile::seed`: draft, then `last_used`,
  then the configured defaults, then the built-in ones. It was in `lib.rs`, where
  nothing can reach it; moving it is what made it testable. The test fails
  against an order that puts defaults ahead of `last_used`, which is the mistake
  that would quietly overwrite a tuned model.
- The retired-key trap is pinned by a test that fails when the field is named
  `defaultProfile`: a v1 config on this machine carries one from the profile
  system v3 removed, and serde would claim it before `extra` ever saw it.
- 161 tests, up 2. Both new ones checked against a gutted implementation.
- **`clippy` failed and I read it as passing** — `echo $?` after a `grep` reports
  the grep. That is the exact trap
  [knowledge/technical.md](../knowledge/technical.md) already warns about, and it
  caught a second reader within the day. Statuses are captured on their own line
  now.
- **Confirmed by the author in the running app**: the Settings panel works.

Favourites and delete (parcel 2), 2026-08-03:

- Trash goes through `osascript -l JavaScript` to `NSFileManager.trashItemAtURL`,
  which involves Finder in nothing and so raises no Automation prompt. That was
  the parcel's open risk and it is closed by a third option neither the plan nor
  the roadmap had considered.
- Real files were really trashed by the test, including one named
  `Odd "quoted" \name.gguf` — the path reaches an interpreter, so a quote in a
  model name must not end the string it sits in.
- **Confirmed by the author in the running app**: stars, the sort, and a real
  delete that really landed in the Trash. So the API contract is no longer being
  taken on faith — the file was seen in the Trash.
- **A bug the suite could not have caught, found by the author clicking.**
  Delete did nothing at all: the confirmation was `window.confirm`, which returns
  no usable answer in this webview, so `if (!window.confirm(...)) return` refused
  every delete before it started. `tsc` was content — `confirm` is typed as
  returning `boolean`, so the code was type-correct and behaviourally wrong. It
  now asks in the row itself and depends on no host dialog. This is the second
  defect in this project found by using the app rather than by testing it.
- One test was wrong and the code was right: the ordering test fed `arrange`
  unsorted entries and expected them sorted. `arrange` partitions and
  deliberately does not re-sort — `scan` is what orders. The test was fixed, not
  the function.
- Six new tests, each checked against a gutted implementation and failed.
- Still not looked at: Discard and the Downloads History pages, from parcel 1.

Download persistence (parcel 1), 2026-08-03 — committed `33d492e`:

- **Proved against the real CDN, not only the stand-in.** 676 MB from Hugging
  Face, stopped at 135,397,705 bytes, and then a `Downloads` built from scratch —
  holding nothing from the first, which is what the app is on a restart — found
  the partial on disk at 135,496,009 bytes, resumed it and delivered the file at
  the expected sha256. The adopted figure being ahead of the reported one is the
  sidecar being the better record, which is the reason it is read rather than
  `downloads.json`.
- A real Discard against a live transfer, 8.5 MB in: the row went, the `.part`
  and the sidecar went, and the directory was left as it was found.
- Each new test was checked against a gutted implementation and failed:
  `partial_at`'s resumable judgement, the resume guard, `adopt`'s scan,
  `restore`'s finished-only filter, and the settle-path delete.
- **One test was corrected rather than trusted.** The discard test's comment
  claimed it caught deleting the files too early; the gut check showed it did
  not, because an eager delete is followed by the settle-path delete anyway and
  the end state is identical. What it actually pins is that the engine's parting
  write does not survive — that is now what it says.
- **Confirmed by the author in the running app**, which is the proof this parcel
  was waiting on: the unfinished download was listed and Resume worked. Screen
  recording is denied to the agent, so this is the author looking, as it was for
  the Web UI window and the tray.
- **The case was real, not seeded.** The models directory already held an orphan
  from before any of this existed: `Qwen3-Coder-30B-A3B-Instruct-UD-Q4_K_XL.gguf`,
  16.45 GiB declared, 5.66 GiB on disk, `.part` at full preallocated length and
  four segments tiling cleanly. Those bytes were unreachable until this parcel —
  nothing in the app could see a `.part`, because the catalog scans for `.gguf`.
  That is a partial written by an older binary and adopted by a newer one, which
  was listed as unproved an hour ago and no longer is.
- Still unlooked-at: Pause, Discard, and the History pages. Discard deletes real
  bytes and has only been proved in Rust.

Extra arguments, 2026-08-03:

- Found by the author using the app: `--alias qwen-2.5-0.5b` in Extra arguments
  reached `llama-server` as `—alias` and the launch exited 1. The rendered
  command showed it quoted, which `shell_quote` only does for a non-ASCII
  character — macOS had substituted the dash. Two defects in one field: that,
  and the field being a controlled input over `rawArgs.join(" ")`, so a space
  could not be typed at all and only a paste ever worked. Both are now
  constraints in [knowledge/technical.md](../knowledge/technical.md).
- `--alias` joins `--host`/`--port` in `OWNED_FLAGS`, so the duplicate is
  refused by name instead of launching under a name the form does not show.
- 145 tests, four commands green, each status captured directly. Weaker than
  usual on one point: the dash fix is TypeScript, and there is no frontend test
  framework, so it is covered by `tsc` and by reading it, not by a test. The
  Rust half is covered. Nobody has yet typed `--threads 8` into the running app
  and watched it survive.

Release, 2026-08-03:

- The `.dmg` was built with `CI=true`, mounted, and checked: `Llamaport.app` at
  0.1.0, the `/Applications` symlink present, and `icon.icns` byte-identical to
  the source by sha256.
- The published asset was downloaded back from GitHub and is byte-identical to
  what was built, mounts, and holds the same app.
- Not proved, and the plan asked for it: the download was `curl`, which sets no
  quarantine attribute — checked, there is none on the file. So the Gatekeeper
  wall was never hit and the README's "Open Anyway" steps remain untested. That
  needs a browser download by a human.

Blockers, 2026-08-02:

- 144 tests, up 7. The guard's tests fail against a gutted `check_raw_args`; the
  two asserting that unrelated flags still pass survive it, as a permissive stub
  should.
- `--no-host` and `--reuse-port` were checked against the installed
  `llama-server --help` rather than assumed: both are real flags, neither is
  blocked, and neither `--host` nor `--port` has a short alias in that build.
- The window fix is confirmed by the author looking at it, not by this session.
  Screen recording and Apple events are both denied here, so window geometry
  could not be read. Five launches of the bundled `.app` each survived, which
  proves the process starts and nothing more.

Downloader, 2026-08-02:

- The live-rate test was checked against a gutted `set_rate_limit` and failed, so
  it detects the absence of what it claims to prove.
- Real transfer: 676 MB from Hugging Face killed mid-flight, resumed, sha256
  verified, landed in the models directory, offered in Library.
- UI, screenshots of one running 656 MB transfer. The observed rate
  followed the limit from unlimited to 1.0 to 1.5 MB/s on the same job with
  progress climbing, so the change reached a transfer already running. The
  estimate read "about 5 min left" against a smoothed rate rather than the last
  sample.
