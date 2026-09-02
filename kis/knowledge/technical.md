# Technical

Tauri 2 desktop app. Rust backend, React 19 + TypeScript frontend built by Vite,
bun as the package manager. macOS only, either architecture.

**Nothing in the source is ARM-only** — there is no `cfg(target_arch)` anywhere,
and the shipped `.dmg` being `aarch64` was a fact about the build machine rather
than about the code. What is Darwin-bound is `sysmem.rs` entire (`sysctl` names,
`proc_pid_rusage`), the Trash through `NSFileManager`, `probe.rs`'s Homebrew
fallbacks, `~/Library/Application Support` and `open -R`. That list is what a
Linux port would cost, and it is why the answer is no rather than not yet.

## Layout

```text
src-tauri/src/
  lib.rs       tauri commands, AppState, tray, window; the seam to the UI
  catalog.rs   scan the models directory, group shard sets, disk space
  gguf.rs      GGUF header parser
  estimate.rs  weights and KV-cache arithmetic
  probe.rs     discover llama-server, probe its accepted flags
  runner.rs    spawn, supervise, telemetry, orphan detection
  health.rs    the ordered model test
  download.rs  the transfer engine: resolve, segments, resume, verify
  downloads.rs the job manager the commands drive; admission and settling
  store.rs     the single JSON config under Application Support
  speeds.rs    what a model did, and the settings it did it under
  tune.rs      the candidate ladder, the shared prompt, the measurement
  sysmem.rs    machine memory readings via libc
  profile.rs   launch settings -> argv
src/
  App.tsx, Library.tsx, ModelDetail.tsx, ProfileForm.tsx, SettingsScreen.tsx,
  HealthPanel.tsx, Memory.tsx, Downloads.tsx, api.ts, types.ts, format.ts
src-tauri/tests/   integration tests; real_* need a model, a binary or the network
                   common/ isolates the config directory; call it before any runner
```

Both long-running subsystems report through a trait they own rather than calling
Tauri: the runner through `EventSink`, the downloader through `ProgressSink`.
That is what makes spawn -> Ready -> telemetry -> stop, and resolve -> transfer
-> verify, testable against a stand-in with no window. Follow the pattern rather
than adding a third idiom.

Returning new state to a caller is not the same as announcing it. A command hands
its snapshot to the window, but the tray has no caller and learns everything from
the event stream, so a state change that only returns leaves the menu bar stale.
Assert on what was emitted, not only on the snapshot: the snapshot is right in
exactly the case this gets wrong.

Config is one JSON file at schema 7, every field `#[serde(default)]`, with
unknown keys preserved through a load/save round-trip. `migrate` strips keys the
app deliberately retired, so a new field must never reuse a retired name — serde
would claim it first and adopt settings from a build several schemas old.

Finished downloads live beside it in `downloads.json`, written atomically on
state transitions and never on a progress tick. A file of its own because a
transfer settles often and the config holds the models directory, the
llama-server path and every remembered launch — an unreadable history should cost
the user their history and nothing else.

It holds more than the history: **whatever nothing else on disk remembers.** A
transfer that moved bytes is described by the sidecar beside its `.part`, which
cannot go stale while it runs, and that account is left to own it. A queued row
has no `.part` and no sidecar, and neither does the Paused row a queued one comes
back as — so both are written here, and `only_record_of` is where that is
decided. Writing only the queued state made the queue survive one restart and not
the next.

What a model actually did is a third file, `speeds.json`, appended when a run that
reached Ready and generated something settles — on either path out, the user
stopping it or the process exiting. Its rows hold the run's *totals*, not a tick's
snapshot, and the rates are derived so nothing on disk can disagree with itself. A
row is keyed on everything that can move the number and stamped with the build.
Split out for the same reason as the history above.

**The suite writes none of this.** `store::use_config_dir` takes the directory
once, and the tests that can start a runner call `common::isolate_config_dir`
first. Before that existed, `cargo test` wrote `runner.pid` and `last-run.log`
into the directory the installed app was using, and the log sitting there was test
output.

Everything the app keeps lives in `~/Library/Application Support/llamaport`:
that config, `downloads.json`, `speeds.json`, the runner pidfile, the last run
log.
`store::adopt_legacy_dir`
takes over the directory left under the old `llama-cpp-hub` name, once, as the
first statement in `setup` — before the pidfile is read or the config is loaded,
both of which are in that same block.

## Run

```bash
bun install
bun run tauri dev
```

## Verify

**A UI change is checked against the mockup rendered, never against the code
that generated it.** Established 2026-09-02 after four passes of the redesign
were handed over as matching and none did; the fifth rendered the artboard,
looked at it, and found a missing control in a minute
([intent/redesign.md](../intent/redesign.md)).

Both halves are scriptable, and neither needs the author:

- **The mockup.** The canvas artboards under the scratchpad are `.dc.html` —
  plain markup inside `<x-dc>`/`<helmet>` wrappers. Strip the wrappers, hoist
  the `<style>` into `<head>`, serve the directory with `python3 -m
  http.server`, and open it in Chrome to screenshot.
- **The app.** `screencapture -x -o -l <window id>` takes the window alone,
  with no desktop around it and without raising it. The id comes from
  `CGWindowListCopyWindowInfo` in a one-file Swift script — the app's webview
  exposes no accessibility tree, and System Events refuses to focus it
  (`-25208`), so this is the only route.
- **A window that will not photograph.** `screencapture -l` answers "could not
  create image from window" when the window is hidden or on another Space,
  which `kCGWindowIsOnscreen` reports as `on=false`. `osascript -e 'tell
  application "System Events" to set visible of process "llamaport" to true'`
  makes it capturable — and unlike `frontmost`, that call is permitted here.
  Waiting for `on=true` does not work; the window can sit that way for
  minutes.
- **Reaching a screen behind a click.** The webview forgets its React state on
  every hot reload, so the model screen cannot be photographed by opening it
  by hand and then editing. Patch a temporary `useEffect` into `App.tsx` that
  selects a model on mount, capture, revert it, and prove the revert with
  `git diff --stat src/App.tsx` before the four commands run again.

`tools/fits.py MODEL.gguf` is a **second opinion on `estimate.rs`**, not a
convenience. The suite only ever sizes synthetic headers; the script sizes a
real file in the models directory by an independent route, so a disagreement
between them is a finding rather than a nuisance. Checked 2026-08-31 and they
agree: 204 MiB for `qwen2.5-0.5b` at 32,768 with all 24 layers charged, 680 MiB
for Ornith at 65,536 with 10 of 40 charged.

It reads the two ceilings the same way the app now does — `llama-server
--list-devices` for the GPU working set, `vm_stat` for what is free — so a
disagreement there is a finding too. `--run` launches the winner and reports real
tokens per second, which `tune.rs` is the Rust port of and is checked against.


Also in the README. Both copies are deliberate: the session anchor loads this
one, contributors read that one. Not a duplication to clean up.

```bash
bun run build
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
```

`cargo` is not on the PATH of a non-interactive shell here — it lives in
`~/.cargo/bin`, which only a login shell picks up. Without it all three cargo
commands exit **127**, which is "command not found" wearing the shape of a
failing check. Export it first: `export PATH="$HOME/.cargo/bin:$PATH"`.

## pi

The app's only outward integration, read off the author's own config 2026-09-02.

`~/.pi/agent/models.json` is hand-maintained and has exactly one top-level key,
`"providers"`. Each provider is `{ baseUrl, api, apiKey, models[] }`, optionally
with `compat` or `authHeader`; each model is `{ id, name, contextWindow,
maxTokens, reasoning, input[], cost{} }`. Five local providers: `local-llama` and
`unsloth` on 8888, `mlx-lm` and `omlx` on 8080, `ollama` on 11434. `omlx` is the
default. Sibling `.bak`, `.save` and `.backup` files already accumulate beside it.

- **A provider is not enough to reach a model.** `~/.pi/agent/settings.json` holds
  `enabledModels`, a list of `"<provider>/<model id>"` strings, and pi will not offer
  a model until it is named there. That file also holds `defaultModel`,
  `defaultProvider`, `theme` and the rest of pi's settings. Established 2026-09-02
  by the author, who wrote a provider and still had to enable it by hand.
- **pi re-reads both files live.** A write to `models.json` or `settings.json` while pi
  is running is picked up with no restart: proved 2026-09-02 by writing both from the
  app with pi open and finding the model immediately selectable. So nothing the app
  writes here needs to tell the user to restart anything.
- **A provider carries exactly one `baseUrl`, shared by every model under it.**
  So models cannot be accumulated under one provider without redirecting the
  older ones wherever the newest points.
- **A `baseUrl` is a declaration, not evidence that anything is bound there.**
  Only one server can hold a port; whoever holds it answers to every provider
  entry naming it. Two entries on 8080 are a naming ambiguity, not a conflict.
- **`--alias` is the id an OpenAI-compatible client addresses.** `default_alias`
  turns a display name into it, and already produces `qwen3.6-35b-a3b` — the id
  the author wrote by hand under `local-llama`.
- **Anything the app writes outside its own directory needs the taken-once
  override** that `store::use_config_dir` gives Application Support, and a test
  that cannot run without it. The file at risk here belongs to another tool.

## Constraints

- **Installed memory is the wrong ceiling, and "fits" is not "works".** On this
  M2 Pro `llama-server -lv 10` reports `MTL0 : Apple M2 Pro (25559 MiB, 25558
  MiB free)` against `CPU : 32768 MiB` — the Metal working set is **25,559 MiB
  (26.80 GB)**, not the 34.36 GB installed, and llama.cpp keeps `--fit-target`'s
  1,024 MiB below it. Read the real figure from that log line rather than
  computing a fraction of RAM.

  Two ceilings bite, and arithmetic against either can still be wrong. Ornith at
  its full 262,144 context needs 23,931 MiB with a `q8_0` cache and fits, and
  26,335 MiB with `f16` and does not. Separately, what is *available* is far
  below both: 14.83 GB free with 6.26 of 7.17 GB of swap already used, on a
  machine whose spec sheet says 32 GB.

  This corrected a claim made in this project on 2026-08-31 — that the inherited
  `q8_0` cache was buying memory nobody needed at a cost to quality. Against the
  real ceiling it is what buys the full context. The author's objection, "fit
  does not mean it works", is the rule: a memory sum says a launch is *allowed*,
  never that it is good, and only running it says the second.

- **A green suite says nothing about the sentence beside the number.** Figures
  and Fitting each shipped arithmetic that was tested, mutation-checked and
  correct, under captions that were wrong: a cache stat reading "≥ 0 MB" hinted
  "some layers are not counted" on a dense model where every layer was counted,
  a panel reading two different plans at once, a raw sentinel printed as `0`, and
  a form naming a context its own command contradicted. Seven defects across the
  two phases, every one found by the author looking at the built app and none by
  the suite. Anything that puts prose next to a figure earns a look before it is
  called done.

  **The running tally, because it is the argument: twenty defects across six
  phases, every one found by looking and none by the suite.** Eleven on
  2026-08-31, five in Tune's panel on 2026-09-01, three in the pi button on
  2026-09-02 — a label that wrapped, a file mode read out of `ls -l`, and a
  provider that turned out not to be selectable — and one in the redesign on
  2026-09-02: every stray-server banner the app had ever shown named an unknown
  model on an unknown port, and it took the author photographing one to notice.
  **A twenty-first was worse and took five releases**: a bundle macOS refuses to
  open, which only a browser download exposed
  ([intent/release.md](../intent/release.md)). A phase is not done when the suite
  is green; it is done when somebody has looked.

- **`--fit` is on by default, and this app suppresses it by naming every value.**
  It "adjusts unset arguments to fit in device memory" (`--fit [on|off]`,
  default `on`; `--fit-target` a per-device margin, default 1024 MiB;
  `--fit-ctx` a floor, default 4096). Every argument the launch fills in is one
  it may no longer size. This app always passes `-c` and `-ngl`, so on every
  launch it makes the feature inert. Measured 2026-08-31 on build b10360 with
  nothing set: `qwen2.5-0.5b` came up at 32,768 and `Qwen3.6-35B-A3B` at
  262,144, each the model's whole trained context, the second on a 32 GB
  machine at 6.7 GB resident.

- **A model's recommended sampling settings are already applied, and the app
  gets them by passing nothing.** `libllama` reads a `general.sampling.*` block
  out of the GGUF header — `sequence`, `top_k`, `top_p`, `min_p`, `temp`,
  `penalty_last_n`, `penalty_repeat`, `xtc_probability`, `xtc_threshold`,
  `mirostat`, `mirostat_tau`, `mirostat_eta` — and uses each as the server
  default. Proved 2026-08-30 on build b10360 against `/props`: the same binary
  and flags reported `temp 0.8, top_k 40` for a model without the block
  (`qwen2.5-0.5b`) and `temp 1.0, top_k 20` for one with it
  (`Qwen3.6-35B-A3B`, whose header declares exactly those). So adding `--temp`
  or `--top-k` to the launch would **overrule the model's own recommendation**,
  which is the opposite of what a sampling form would be for. A request field
  still beats the server default, so any client that sends one wins over both.
  What the app is missing is not the values but the display: nothing on screen
  says 1.0 was the model's choice rather than a fallback, and a `--temp` typed
  into extra arguments silently replaces it with no indication.

- **Tauri does not sign the bundle unless an identity is configured, and an unsigned
  bundle is not "unsigned" to macOS — it is broken.** Without
  `bundle.macOS.signingIdentity`, the `.app` carries only the ad-hoc signature the
  linker puts on the arm64 executable: `flags=0x20002(adhoc,linker-signed)`,
  `Info.plist=not bound`, `Sealed Resources=none`. `codesign --verify` then says
  *code has no resources but signature indicates they must be present*, and a
  quarantined copy is refused outright as **"is damaged and can't be opened. You
  should move it to the Trash"** — with no **Open Anyway** button anywhere, because
  macOS never got far enough to ask. Every release from v0.1.0 to v0.6.0 shipped
  this, and the README documented a flow that could not work.

  `"signingIdentity": "-"` fixes it: `flags=0x10002(adhoc,runtime)`, `Info.plist
  entries=14`, `Sealed Resources version=2`, and `codesign --verify --deep --strict`
  reports valid on disk. `spctl` still rejects it, which is correct — ad-hoc is not
  notarization, so the user meets the ordinary unidentified-developer dialog, and
  that one **Open Anyway** does dismiss.

  **Check the artefact with `codesign --verify --deep --strict`, not by eye.** The
  `.dmg` mounts, the app looks fine, and it launches perfectly from a local copy —
  quarantine is the only thing that exposes it, which is why five releases went out
  with it.

- **`write_atomic` does not carry a file's mode across.** It renames a fresh temporary
  into place, and a fresh file is born at the process umask rather than at the mode of
  the file it replaces. Writing pi's `models.json` this way took it from `600` to `644` —
  a file holding five API keys, made world-readable on the author's own machine, and
  found by looking at `ls -l` after the first real write rather than by any test. Any
  writer of a file the app does not own must read the mode first and set it back.

- **`write_atomic` cannot take two writers at once.** Its temporary is
  `path.with_extension("tmp")` — one name per destination, not per writer — so two
  threads writing the same file race on it and the loser's rename fails with
  `NotFound`. Every caller is serialised today, and any new one must be: a
  read-modify-write on one of these files needs a mutex around the whole pair, not
  just faith in the rename. Found by removing `store::append_speed`'s lock and
  watching a concurrent-append test fail on the collision rather than on the lost
  row it was written for. Losing a row silently is the failure it prevents in
  production, where writers are serialised — the collision is what happens when
  they are not.
- **`sysinfo`'s plain `refresh_processes` leaves `cmd()` empty.** The argv is only
  populated by `refresh_processes_specifics` with a `ProcessRefreshKind` that asks
  for it. Nothing errors: the process list is complete and correct, every entry
  simply has no command line, so a parser over it returns `None` forever. That is
  what made `detect_orphans` name an unknown model on an unknown port for the
  life of the feature, with the parser and its tests entirely correct. Proved
  2026-09-02 by a throwaway test printing `cmd=[]` for a live `llama-server`.
- HTTP is `ureq` with `default-features = false, features = ["tls"]`, which is
  rustls plus webpki-roots. Blocking, one thread per connection — there is no
  async runtime and nothing needs one.
- Rust edition 2021; deps are deliberately few (`serde`, `sha2`, `sysinfo`,
  `ureq`, `libc`).
- No comments that narrate what code does; the codebase keeps them for
  non-obvious why only.
- The shell here is **zsh**, so `$PIPESTATUS` is not the array you want —
  bash's name for it. It expands to nothing, `echo "x: ${PIPESTATUS[0]}"` prints
  an empty status, and the command reads as though it reported success. zsh's own
  is `$pipestatus` (lowercase). The rule below is the reliable answer either way.
- **Tag a release with `git tag -a`.** `git tag` alone makes a lightweight tag,
  and `git describe --exact-match` only considers annotated ones — so the check
  that guards against building something other than the tag fails while nothing
  is wrong. Every tag in this repository is annotated; v0.5.0 was created
  lightweight, caught by that check, and recreated.
- **A tag at the release commit is not a tag at HEAD.** Memory commits land after
  it, so `git describe --exact-match` on HEAD is expected to find nothing once
  syncing has happened. Before building an artefact, check out the tag or verify
  the distance — assuming they were identical is what put a build of HEAD in the
  v0.3.0 release ([intent/release.md](../intent/release.md)).
- Capture a command's exit status directly. `cmd | tail -3; echo $?` reports
  `tail`'s status, which reported a failing clippy as clean twice in one session.
- **A new test is not trusted until the code it covers has been gutted and the
  test watched to fail.** Used on every phase since the downloader, and it has
  paid twice: two tests that passed for the wrong reason, and a guard nothing
  would have noticed the loss of. A green suite is not the claim; a suite that
  fails when the code is wrong is.
- **Read the file the app actually wrote.** Four defects in this project were
  found by looking — at the screen, or at `downloads.json` between runs — and
  none of them by the suite. Anything that persists state earns this check.
- macOS substitutes an em dash for `--` inside the webview's text fields, so a
  flag typed into any field is corrupted before it reaches Rust and
  `llama-server` rejects it. A field that takes flags undoes the substitution on
  input. Only a token's leading dash can be one — substitution needs two
  hyphens, so hyphens inside `--cache-type-k` are the user's own.
- A field whose value is a parsed list rendered back as text cannot be typed
  into: the separator is dropped the instant it is typed. Such a field keeps its
  own text and re-seeds from props only when they disagree.
- `on_window_event` fires for every window, so any handler there must check
  `window.label()`. The close-hides-instead rule is the main window's alone.
- A `.part` and its sidecar are named from the destination — `{dest}.part` and
  `{dest}.part.json` — so a partial is found by scanning the models directory for
  the sidecar suffix and stripping it. `catalog::scan` filters on the `.gguf`
  extension and will never see either.
- The models directory and the config directory hold **untrusted input**. A `.part`,
  a `.part.json` and `downloads.json` are files anything with write access can
  create, and each of them names something the app then acts on — a URL to fetch,
  a path to write, a path to delete. Every one is re-validated on the way in.
  Two live holes came from forgetting this.
- A `.part` is opened with `O_NOFOLLOW` (`custom_flags`), never a plain `open`.
  `lstat`-then-open is two syscalls with a window between them, which is not a
  guard against a link planted on purpose.
- `admit` is not the only gate a transfer passes. Anything that starts one —
  today `start` and `resume` — validates the URL itself. Resume once did not, and
  that asymmetry was the whole vulnerability.
- A restored row's **file name** is validated too, not only its path. `path` is
  rebuilt as `models_dir.join(file_name)`, and `join("../evil")` lands outside
  the directory the rebuild was meant to confine it to. Rebuilding a path from an
  untrusted name is not a check.
- One transfer at a time is enforced by **queueing**, not by refusing. The
  invariant is one line — nothing `Active` and something `Queued` means the head
  of the queue starts — and every path a transfer can settle on runs it, on the
  finishing transfer's own thread after the jobs lock is released. Promoting from
  inside the settle path takes the same mutex and deadlocks.
- A queued job carries the `Options` it was admitted on, because it starts on a
  thread with no caller to ask. The rate limit is the one term that stays live:
  `set_rate_limit` rewrites it on waiting rows as well as on the running one.
- What `admit` checked can be hours stale by the time a queued job starts. The
  destination is re-checked at the moment of promotion, and a file that landed
  meanwhile fails that row rather than being written over.
- `AppState::save_config` takes the config lock itself, and a `std::sync::Mutex` is
  not reentrant. Anything that edits the config must drop its guard — a scoped
  block — before saving. Calling it while holding the guard deadlocks that path
  outright, and the paths that edit the config are launching and settling.
- **`Ready` is announced on every telemetry tick, not once.** The `runner:state`
  stream carries the current state, so a listener that acts on Ready acts dozens
  of times per run. Anything it does must be idempotent or guarded against the
  repeat; `store::stamp_if_newer` is the guard, keyed on `started_secs`, which
  moves only when a process is spawned.
- A retired key must be checked for *shape*, not only for name. `lastRun` on real
  v1 configs is a map of model id to timestamp — the shape a launch-time field
  wants — so naming that field `lastRun` would have serde adopt five-year-old
  values as this build's. Proved by naming it that and watching the test fail.
- There is no frontend test framework. Every test is Rust, in `src-tauri/tests/`
  or an inline `#[cfg(test)]` module; TypeScript is covered by `tsc` and by
  looking at the screen. Logic worth testing belongs in Rust until that changes.
