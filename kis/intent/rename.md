# Rename to Llamaport

Planned and done 2026-08-02. The app is renamed from `llama-cpp-hub` to
**Llamaport** across every surface, including the bundle identifier and the
config directory. Display casing is `Llamaport`; every identifier is
`llamaport`.

## Why then

The identifier is free until it is signed. Packaging is the next roadmap item,
and after notarization a changed identifier (`com.mkamran.llama-cpp-hub` to
`com.mkamran.llamaport`) is a different application to macOS: new identity,
new Gatekeeper record, existing installs stranded. Nothing is signed and there
is no remote, so this is the last cheap moment to rename.

## Why not llama.cpp-led

A shipped `llama.cpp hub` claims a project it is not part of. It reads as
ggml-org's own companion app, and it is unfindable sitting next to the
upstream name in any search.

Three names were ruled out on collision:

- **LlamaBarn**: a macOS menubar app for running local LLMs via llama.cpp
  with curated model downloads, shipped as a Homebrew cask.
- **LlamaStation**: an open-source llama.cpp GUI for Windows.
- **LlamaHub**: LlamaIndex's connector registry at llamahub.ai.

`Llamaport` carries both halves of the app: models arrive at a port, and
`llama-server` binds a port, a number the app already puts in front of the
user on every launch.

`Llama` is Meta's mark. llama.cpp and Ollama both ship unchallenged, so the
practical risk is low but not zero.

## Migration

The config directory moves with a one-shot rename at startup: if the new
directory is absent and the old one is present, `fs::rename` moves it. It
never renames onto an existing target, and `fs::rename` is atomic on one
volume. It runs as the first statement in `setup`, before anything else reads
the config or the pidfile under the old or new name.

## Verified

Verified 2026-08-02: a test covers both directions of the migration, the
window title and sidebar both read `Llamaport`, and the author's real settings
survived a launch of the renamed app.
