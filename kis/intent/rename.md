# Rename to Llamaport

Planned and done 2026-08-02. Proof in [state/current.md](../state/current.md).

The app is renamed from `llama-cpp-hub` to **Llamaport** across every surface,
including the bundle identifier and the config directory. Display casing is
`Llamaport`; every identifier is `llamaport`.

## Why now, and why not llama.cpp-led

**Now, because the identifier is free until it is signed.** Packaging is the next
roadmap item. After notarization, `com.mkamran.llama-cpp-hub` ->
`com.mkamran.llamaport` is a different application to macOS: new identity, new
Gatekeeper record, existing installs stranded. Nothing is signed and there is no
remote, so this is the last cheap moment. It goes before packaging, not with it.

**Not llama.cpp-led, because a shipped `llama.cpp hub` claims a project it is not
part of.** It reads as ggml-org's own companion app, and it is unfindable sitting
next to the upstream name in any search.

Three names were ruled out on collision, recorded so this is not re-litigated:

- **LlamaBarn** — a macOS menubar app for running local LLMs via llama.cpp with
  curated model downloads, shipped as a Homebrew cask. Nearest neighbour to this
  project on the same platform.
- **LlamaStation** — an open-source llama.cpp GUI for Windows. Same category.
- **LlamaHub** — LlamaIndex's connector registry at llamahub.ai.

`Llamaport` carries both halves of the app: models arrive at a port, and
`llama-server` binds a port — a number this app already puts in front of the user
on every launch.

`Llama` is Meta's mark. llama.cpp and Ollama both ship unchallenged so the
practical risk is low, but it is not zero. It is a consequence of the direction,
not of this word, and nothing follows from it today.

## Scope

- `package.json` name; `Cargo.toml` package name and lib name (`llamaport_lib`)
- `tauri.conf.json` productName, identifier, window title
- `main.rs` call site; the 12 test-file imports; the `App.tsx` sidebar title
- `index.html:7`, still the Tauri scaffold's `Tauri + React + Typescript`
- `README.md:1`
- `store.rs` config dir, plus the migration below

Out: icons, the folder on disk, a repo rename (there is no remote).

Also out, deliberately: `README.md:6` still claims downloading is "designed but
**not built**", which contradicts State. That is a docs-accuracy fix, and folding
it into a mechanical rename would hide a real correction inside a large diff.

## The migration is the only real work

Everything else is find-and-replace. `~/Library/Application Support/llama-cpp-hub`
exists on the author's machine with live settings: models directory,
`llama-server` path, per-model last-used launch settings, download options.

One-shot directory rename at startup — new absent and old present, then
`fs::rename`. Not a permanent dual-path read, which would leave two code paths
forever for an event that happens once. Written as a function over two paths
rather than over `$HOME`, so it is testable without mutating the environment;
`store.rs` already separates `load_from`/`save_to` from `config_path`, so this
follows the existing seam.

At startup, not lazily inside `load()`: `runner.rs` puts `runner.pid` and
`last-run.log` under the same directory, and `lib.rs` reads the pidfile through
`detect_orphans` in the same `setup` block that loads the config. A migration
hanging off `load()` alone would run after the pidfile had already been looked
for under the wrong name. It is the first statement in `setup`.

It never renames onto an existing target, and `fs::rename` is atomic on one
volume. An older build run afterwards recreates the old directory and reads an
empty config; accepted, one user and one machine, and it loses nothing already
written.

## Closing conditions — all met 2026-08-02

- No occurrence of `llama-cpp-hub`, `llama_cpp_hub` or `llama.cpp hub` outside
  `src-tauri/gen/` and `target/`. What remains is the `LEGACY_DIR` constant that
  has to name the old directory, its tests, and this file.
- A test proving both directions: old directory populated and new absent lands
  the config at the new path with every field intact; new directory already
  present leaves the old one untouched and clobbers nothing.
- Window title and sidebar both read `Llamaport`.
- The author's existing settings survived a real launch.

Proof is in [state/current.md](../state/current.md). One thing worth carrying:
the migration was verified by a `tauri dev` that was already running and rebuilt
on the edit, not by a deliberate launch. It is still real proof — the running app
moved the real directory — but it was luck that the check happened at all, and a
packaged build will not hot-reload into its own migration.

A rename that passes tests and loses the config is a failure, and only launching
it shows that. That is why the UI check was held as a blocker rather than waived
when screen access was declined.
