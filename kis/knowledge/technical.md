# Technical

Tauri 2 desktop app. Rust backend, React 19 and TypeScript frontend built by
Vite, bun as the package manager. macOS only, either architecture: nothing in
the source is ARM-only, and the shipped `.dmg` being `aarch64` is a fact about
the build machine, not the code.

What is Darwin-bound is `sysmem.rs` entire (`sysctl` names,
`proc_pid_rusage`), the Trash through `NSFileManager`, `probe.rs`'s Homebrew
fallbacks, `~/Library/Application Support` and `open -R`. That is what a
Linux port would cost, and it is why the answer is no rather than not yet.

Working rules for whoever codes here are in [AGENTS.md](../../AGENTS.md).

## Layout

```text
src-tauri/src/
  lib.rs       tauri commands, AppState, tray, window
  catalog.rs   scan models directory, group shards, disk space
  gguf.rs      GGUF header parser
  estimate.rs  weights and KV-cache arithmetic
  probe.rs     discover llama-server, probe accepted flags
  runner.rs    spawn, supervise, telemetry, orphan detection
  activity.rs  every llama-server process, memory and CPU
  health.rs    the ordered model test
  download.rs  transfer engine: resolve, segments, resume, verify
  hub.rs       Hugging Face index behind a Transport trait
  quant.rs     which file in a repository to fetch
  discover.rs  browse listing, a tree per row over six lanes
  downloads.rs job manager: admission and settling
  store.rs     the JSON config under Application Support
  speeds.rs    what a model did, and under what settings
  tune.rs      candidate ladder, shared prompt, measurement
  sysmem.rs    machine memory readings via libc
  profile.rs   launch settings -> argv
src/
  App.tsx        shell: sidebar, routing, runner and orphan state
  Library.tsx    model list; FirstRun.tsx for the empty state
  ModelDetail.tsx  one model; Presets.tsx picks how it runs
  ProfileForm.tsx  ProfileFields, AdvancedFields, shared with Settings
  SettingsScreen.tsx, Downloads.tsx, HealthPanel.tsx
  Discover.tsx   browse screen; DiscoverDetail.tsx one repo's quants
  OwnerAvatar.tsx  publisher picture, one lookup per owner per process
  Disclosure.tsx a folding row; theme.ts sets data-theme/data-mode
  Memory.tsx     Stat, launchCost, MemoryBar: launch cost against ceilings
  Telemetry.tsx  Card, Sparkline, TelemetryPanel, shared with Activity.tsx
  TunePanel.tsx  measurement ladder; PiPanel.tsx writes pi's files
  icons.tsx      every SVG the UI draws
  api.ts, types.ts, format.ts, diff.ts
src-tauri/tests/   integration tests; real_* need a model, binary or network
                   common/ isolates the config directory
                   stylesheet.rs reads ../src/App.css, the only frontend test
```

## Architecture

The runner and the downloader report through a trait they own (`EventSink`,
`ProgressSink`) rather than calling Tauri directly, which makes spawn ->
Ready -> telemetry -> stop, and resolve -> transfer -> verify, testable
against a stand-in with no window. New subsystems should follow the
pattern. Returning new state to a caller is not the same as announcing it:
a command hands its snapshot to the window, but the tray has no caller and
learns everything from the event stream, so a state change that only
returns leaves the menu bar stale.

A synchronous Tauri command runs on the main thread, and the window cannot
paint while it does. `discover_browse` once held that thread for two and a
half seconds of network calls, so a loading state React had already been
told to render never appeared. `async fn` moves a command onto the async
runtime; anything touching the network is `async` for this reason, and so
is anything reaching `runner::inspect_port` or `capabilities()` on a miss
(three process spawns). An `async` command that borrows `State` must
return `Result`.

`write_atomic` renames a fresh temporary into place. It does not carry a
file's mode across, so a writer of a file the app does not own must read
the mode first and set it back. Its temporary name is
`path.with_extension("tmp")`, one per destination rather than per writer,
so two threads writing the same file race on it; a read-modify-write on
one of these files needs a mutex around the whole operation.

Config is one JSON file at schema 8, every field `#[serde(default)]`, with
unknown keys preserved through a load/save round trip. `migrate` strips
keys the app deliberately retired, so a new field must never reuse a
retired name, or serde would adopt settings from a build several schemas
old. A retired key must be checked for shape, not only name: `lastRun` on
real v1 configs was a map of model id to timestamp, the shape a later
launch-time field wanted.

## Storage

Everything the app keeps lives in `~/Library/Application Support/llamaport`:
the config file, `downloads.json`, `speeds.json`, the runner pidfile, the
last run log, and `avatars/`.

- **`config.json`** holds the models directory, the llama-server path and
  every remembered launch profile.
- **`downloads.json`** holds finished and in-progress transfer state,
  written atomically on state transitions, never on a progress tick, so an
  unreadable history costs the user only their history. It also holds
  whatever nothing else on disk remembers: a running transfer is described
  by the sidecar beside its `.part`, but a queued row (and the Paused row
  it becomes on restart) has neither, so both live here. `only_record_of`
  decides which account owns a row.
- **`speeds.json`** is appended when a run that reached Ready and
  generated something settles, on either exit path. Its rows hold the
  run's totals, not a tick's snapshot, and are keyed on everything that
  can move the number and stamped with the build.
- **`avatars/`** is one small file per owner rather than one shared map,
  because several threads can write it at once and `write_atomic`'s
  temporary is named per destination, not per writer. An empty file is a
  remembered miss worth as much as a hit, since owners with no picture
  publish the most repositories. Kept a month, since an avatar changing is
  cosmetic and no row can show that it has.
- `store::adopt_legacy_config_dir` takes over the directory left under the
  old `llama-cpp-hub` name, once, as the first statement in `setup`.

The test suite writes none of this: `store::use_config_dir` takes the
directory once, and any test that can start a runner calls
`common::isolate_config_dir` first.

## pi

The app's only outward integration, read off the author's own config on
2026-09-02.

`~/.pi/agent/models.json` is hand-maintained, one top-level key
`"providers"`. Each provider is `{ baseUrl, api, apiKey, models[] }`,
optionally `compat` or `authHeader`; each model is `{ id, name,
contextWindow, maxTokens, reasoning, input[], cost{} }`. Five local
providers: `local-llama` and `unsloth` on 8888, `mlx-lm` and `omlx` on 8080,
`ollama` on 11434, with `omlx` the default.

- A provider is not enough to reach a model: `~/.pi/agent/settings.json`
  holds `enabledModels`, a list of `"<provider>/<model id>"` strings, and pi
  will not offer a model until it is named there.
- pi re-reads both files live, with no restart needed.
- A provider carries exactly one `baseUrl`, shared by every model under it,
  so models cannot be accumulated under one provider without redirecting
  the older ones wherever the newest points. A `baseUrl` is a declaration,
  not evidence anything is bound there: only one server can hold a port,
  and two entries naming the same port are a naming ambiguity, not a
  conflict.
- `--alias` is the id an OpenAI-compatible client addresses; `default_alias`
  turns a display name into it, and already produces `qwen3.6-35b-a3b`.
- Anything the app writes outside its own directory needs the taken-once
  override that `store::use_config_dir` gives Application Support, since the
  file at risk belongs to another tool.

## The Hugging Face API

Measured against the live API on 2026-09-03 while planning
[intent/discover.md](../intent/discover.md), and cross-read against
Unsloth's shipped hub, which calls the same API from their frontend.

- Sorts: `downloads` (30-day), `likes`, `trendingScore`, `lastModified`,
  `createdAt`. `downloadsAllTime` is rejected as a sort, accepted only as
  an `expand`. `lastModified` and `createdAt` return sludge (zero-download
  repos at the top), unusable without a popularity floor. `trendingScore`
  is descending only.
- `expand=gguf` on the listing returns `total` (parameter count),
  `architecture` and `context_length` per repo with no per-repo call, but
  drags in the whole `chat_template` (kilobytes a row) and is read off one
  file, so it is wrong when that file is a sidecar:
  `HauhauCS/...-27B-...-MTP-GGUF` reports 1.86B parameters for a 27B model
  because its first GGUF is the MTP drafter.
- `full=true` carries `siblings`, filenames with no sizes. Sizes come only
  from `/api/models/{repo}/tree/main?recursive=true` (required, since
  quants live in subdirectories like `BF16/` as often as at the root), so a
  size on screen costs a call per repo. A tree holds traps beside the
  quants too: MTP drafters, `mmproj-*` projectors, and shard halves, so a
  rule that takes the largest file that fits can pick the wrong one. `lfs`
  on an entry is what says the size headers will exist.
- `gated` is `false`, `"auto"` or `"manual"`, returned only by `full=true`.
  Four of the top 50 trending GGUF repos are gated; the tree call returns
  200 for a gated repo while `resolve` returns 401, so a screen that
  ignores the flag will offer a download that fails. Tags do not carry
  task domain either: `code` appears 0 times and `conversational` 46 over
  the same top 50, so a Coding filter has no backing (Unsloth files
  models by modality instead). No rate-limit headers come back
  unauthenticated; the budget is unadvertised, not absent.
- A mixture of experts needs two signals, neither complete. Over 300 GGUF
  repositories sampled 2026-09-03: the uploader's `moe` tag covers 35, an
  architecture name containing `moe` covers 34, only 13 carry both, and the
  union is 56. Certainty for a model on disk stays
  `gguf::Metadata::is_moe`, which reads a real expert count the index does
  not carry.
- An owner's picture is two endpoints (`/api/organizations/{owner}/overview`
  for an org, `/api/users/{owner}/overview` for a person, the org one
  404s for a person); fifteen distinct owners appeared in a page of
  twenty-four, so this caches well. `num_parameters=min:20B,max:40B` is a
  real filter and more reliable than `gguf.total`, since the `HauhauCS`
  repository above still lands correctly in the 20-40B band at the
  server: filter by parameters there, never by arithmetic over
  `gguf.total`.
- One GGUF repository in six is not a language model. Over 300 sampled
  2026-09-03: 104 `image-text-to-text`, 97 `text-generation`, 48 with no
  `pipeline_tag` at all, and 51 that `llama-server` cannot serve. The
  filter has to be a denylist: known-good tags alone would hide the 48
  untagged, some among the best models on the site.
- The listing paginates by an opaque cursor in a `Link: <...>; rel="next"`
  header, not by offset, and already encodes the sort, filter, expands and
  search, so it must be followed verbatim.
- One call gives a repository everything a detail page needs:
  `?expand=downloads&likes&lastModified&gated&gguf&cardData`, affordable
  for one repository but not for twenty-four at once, which is why a tree
  per row is fanned out instead: twenty-four trees take 13.7 seconds in a
  queue and 2.3 seconds across six lanes, measured 2026-09-03.
- `catalog::quant_from_name` is the one spelling of a quantisation,
  handling the `UD-` prefix and `TQ` quants; `catalog::parse_shard` splits
  `-00001-of-00002` (all 461 real shard files sampled use five digits).
- A fit badge computed from file size over-claims. Unsloth's
  `classifyGgufFit` scores `size × 1.15 + 1 GB` against 97% of a device's
  memory in five classes; their own comments record that badge and their
  memory bar disagreeing on 11 of 19 sizes on the same row, and the
  residual after fixing a shared constant is the estimator itself. This is
  the same finding as "fit does not mean it works", and it is why nothing
  in this app prints "fits" beside a file size it has not opened.

## Constraints

- **Installed memory is the wrong ceiling, and "fits" is not "works".** On
  an M2 Pro, `llama-server -lv 10` reports a Metal working set of 25,559 MiB
  (26.80 GB) against 34.36 GB installed; read that log line rather than
  computing a fraction of RAM. Ornith at its full 262,144 context needs
  23,931 MiB with a `q8_0` cache and fits, 26,335 MiB with `f16` and does
  not. A memory sum says a launch is allowed, never that it is good.

- **A green suite says nothing about the sentence beside a figure.** Figures
  and Fitting each shipped arithmetic that was tested and correct under
  wrong captions, every one found by looking at the built app, none by the
  suite ([intent/defects.md](../intent/defects.md) has the record). A
  phase is not done when the suite is green, but when somebody has looked.

- **An artboard is the spec for the shape, not for the facts.** Downloads'
  own mockup caption claimed downloads survive quitting the app; a
  relaunch actually restores them Paused with Resume live
  ([intent/downloader.md](../intent/downloader.md)). Every claim a mockup
  makes about behaviour is checked against the code or phase file that
  proved it, and corrected in place when wrong.

- **macOS vibrancy costs a private API, and the config flag alone does not
  enable it.** `transparent: true` needs `macOSPrivateApi: true` in
  `tauri.conf.json` and the `macos-private-api` Cargo feature on `tauri`,
  which the Tauri CLI adds on its own the next time it runs the app. It
  bars the App Store permanently, accepted since this app has always
  shipped unsigned through GitHub. `state: "followsWindowActiveState"`
  flattens the material whenever the app goes to the back, which reads as
  a glitch on a fixed chrome surface, so it is pinned to `active`.
  Headless Chrome will not draw an `NSVisualEffectView`, so a render
  proves only that the CSS stops painting, nothing about the blur.

- **Every request this app makes goes through Rust; the webview loads
  nothing remote.** Avatars are fetched in Rust and handed over as
  `data:` URIs, at 5-15 KB almost free. `bundle.security.csp` (set since
  2026-09-04) omits `script-src`, since Tauri adds `'self'` and a hash per
  bundled script, but Tauri injects the CSP only into assets served over
  `tauri://localhost`, so `bun run tauri dev` runs with no policy at all;
  only a built bundle enforces it.

- **The probe cache is stamped on the binary's mtime and size, taken before
  the probe runs.** A Homebrew upgrade replaces the file in place; stamped
  after, a file replaced mid-probe would be remembered under the old flags
  for good. An `Err` stays cached until the path is set again.

- **`select` is `width: 100%` across this stylesheet**, for the forms the
  app is mostly made of. A select that sits beside something has to opt out
  and re-state the right padding, since the shared rule paints the chevron
  there.

- **A `var(--x)` with nothing behind it is invisible.** CSS does not error;
  the property simply inherits, and the result looks nearly right.
  `src-tauri/tests/stylesheet.rs` fails on any bare `var(--x)` the
  stylesheet does not define; it is the only test this project has of the
  frontend. A `var(--x, fallback)` is deliberately not a defect, which is
  how the fixed ambers and greens in
  [intent/appearance.md](../intent/appearance.md) are written.

- **A palette is seven anchors; the surfaces are mixed from them.**
  `App.css` holds ground, text, muted text, line, accent, running and danger
  per theme, and derives sidebar, card, card2, hover, badge, input, code and
  faint from those once in `:root`. `theme.ts` writes `data-theme` and
  `data-mode` on the root before the first render, so there is no
  `prefers-color-scheme` query in the stylesheet. Anything painted on the
  accent uses `--on-accent`, never `#fff`: three of the palettes are pale
  enough that white is unreadable on them.

- **`--fit` is on by default, and this app used to suppress it by naming
  every value.** llama.cpp's `--fit` adjusts unset arguments to fit device
  memory (`--fit-target` default 1024 MiB, `--fit-ctx` floor default
  4,096 tokens). Measured 2026-08-31 with nothing set: `qwen2.5-0.5b` came up
  at its full 32,768 context and `Qwen3.6-35B-A3B` at 262,144, on a 32 GB
  machine.

- **A model's recommended sampling settings are already applied, and the
  app gets them by passing nothing.** `libllama` reads a
  `general.sampling.*` block out of the GGUF header and uses each field as
  the server default: the same binary reported `temp 0.8, top_k 40` for a
  model without the block and `temp 1.0, top_k 20` for one with it. Adding
  `--temp` or `--top-k` to the launch would overrule the model's
  recommendation, though a request field still beats the server default.
  Nothing on screen shows a value was the model's choice rather than a
  fallback.

- **The models and config directories hold untrusted input.** A `.part`, a
  `.part.json` and `downloads.json` are files anything with write access
  can create, and each names something the app then acts on: a URL to
  fetch, a path to write, a path to delete. Every one is re-validated on
  the way in; two live security holes came from forgetting this. A `.part`
  is opened with `O_NOFOLLOW`, never a plain `open`. `admit` is not the
  only gate a transfer passes: anything that starts one validates the URL
  itself, and `resume` once did not. A restored row's file name is
  validated too, not only its path: `join("../evil")` on
  `models_dir.join(file_name)` lands outside the intended directory, and
  `Path::starts_with` is not a guard either, since it compares components
  without resolving them (`avatars/..` starts with `avatars`). Validate
  the name, never the joined path; `hub::valid_segment` is the one rule,
  shared with repository ids. A `.part` and its sidecar are named from the
  destination, `{dest}.part` and `{dest}.part.json`, found by scanning for
  the sidecar suffix; `catalog::scan` filters on `.gguf` and never sees
  either.

- **One transfer at a time is enforced by queueing, not by refusing.** The
  invariant is one line: nothing `Active` and something `Queued` means the
  head of the queue starts, on the finishing transfer's own thread after
  the jobs lock is released; promoting from inside the settle path takes
  the same mutex and deadlocks. A queued job carries the `Options` it was
  admitted on, since it starts with no caller to ask. What `admit` checked
  can be hours stale by the time a queued job starts, so the destination
  is re-checked at the moment of promotion.

- **`AppState::save_config` takes the config lock itself, and a
  `std::sync::Mutex` is not reentrant.** Anything that edits the config must
  drop its guard before saving; calling it while holding the guard deadlocks
  that path outright.

- **`Ready` is announced on every telemetry tick, not once**, so a listener
  that acts on it acts dozens of times per run and must be idempotent;
  `store::stamp_if_newer`, keyed on `started_secs`, is the guard.

- **There is no frontend test framework.** Every test is Rust; TypeScript is
  covered by `tsc` and by looking at the screen.

- **macOS substitutes an em dash for `--` inside the webview's text
  fields**, corrupting any flag typed into a field before it reaches Rust.
  A field that takes flags undoes the substitution; only a leading dash
  can be one, since substitution needs two hyphens. A field whose value is
  a parsed list rendered back as text cannot be typed into either, since
  the separator is dropped as it is typed; such a field keeps its own text
  and re-seeds from props only when they disagree.

- **`on_window_event` fires for every window**, so any handler there must
  check `window.label()`; the close-hides-instead rule belongs to the main
  window alone.

- **`sysinfo`'s plain `refresh_processes` leaves `cmd()` empty**, populated
  only by `refresh_processes_specifics` with a `ProcessRefreshKind` that
  asks for it. Nothing errors; a parser over the empty command line
  returns `None` forever, which made `detect_orphans` name an unknown
  model on an unknown port for the life of the feature.

- HTTP is `ureq` with `default-features = false, features = ["tls"]`
  (rustls plus webpki-roots), blocking, one thread per connection, no
  async runtime. Rust edition 2021; dependencies are deliberately few
  (`serde`, `sha2`, `sysinfo`, `ureq`, `libc`).

## Releases

- Tauri signs nothing without `bundle.macOS.signingIdentity` configured, and
  the ad-hoc signature the linker leaves is not "unsigned" to macOS, it is
  broken: a quarantined copy is refused as "is damaged and can't be opened",
  with no **Open Anyway** button. `"signingIdentity": "-"` fixes it: `spctl`
  still rejects the result correctly, since ad-hoc is not notarization, so a
  user meets the ordinary unidentified-developer dialog, whose **Open
  Anyway** does work.
- Check a release artefact with `codesign --verify --deep --strict`, not by
  eye. The `.dmg` mounts and the app launches fine from a local copy either
  way; quarantine is the only thing that exposes a broken signature.
- Tag a release with `git tag -a`. A lightweight tag is invisible to `git
  describe --exact-match`, so a check meant to guard against building the
  wrong commit passes while nothing is right.
- A tag at the release commit is not a tag at HEAD: memory commits land
  after it. Check out the tag, or verify the distance, before building an
  artefact ([intent/release.md](../intent/release.md)).
- `CI=true` is mandatory for `tauri build`. Without it, the disk image step
  drives Finder through Apple events and fails. The command is in the
  README's build section.
