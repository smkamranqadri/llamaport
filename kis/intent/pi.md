# pi

Item 6 of [direction.md](direction.md): one click to point pi, the author's coding
agent, at the running model. Planned, built and shipped in `v0.6.0` on 2026-09-02.

## Purpose

pi's `local-llama` provider is hand-maintained and had fallen behind: it pointed at
a port most launches did not use. The app knows the port, the alias and the context
a running model actually accepted, and nothing else does.

## What it does

A button on the running model, beside Web UI and Test model, opens a panel that
shows the `llamaport` provider block as it stands and as it would be, with a line
diff per file. A checkbox controls the reasoning flag. Confirm backs up the file
and writes one provider and one `enabledModels` entry.

## Decisions

- **A diff, then a write on confirm, not a write on the click.** The file is
  hand-edited and shared with four other providers, so writing behind the author's
  back is wrong. Two clicks against a file the app does not own is the right trade,
  and the diff also makes a port overlap visible.
- **One provider, one model, replaced on every confirm.** A pi provider carries
  exactly one `baseUrl`, so accumulating models under it would silently redirect
  old entries to whatever port the newest launch bound. That is how `local-llama`
  came to point at models nobody runs.
- **A port overlap is named, never refused.** Two other providers already point at
  the same port. An entry is a declaration, not evidence of what is bound; only one
  server can hold a port, and refusing would block most launches.
- **The reasoning flag is a checkbox, seeded from the existing entry.** Nothing in
  a GGUF states whether a model reasons, so the value cannot be derived. Seeding it
  from the current `llamaport` entry means a second click needs no new thought.
- **A missing file is created; an unparseable one is refused.** Creating it is
  cheap: `models.json` has exactly one top-level key. Refusing guards against
  overwriting a file the app failed to read. Both files are checked before either
  is written, so a bad settings file found late cannot leave pi holding a provider
  it will not offer.
- **The app's own dead `enabledModels` entries are dropped; nobody else's are.**
  The provider lists exactly one model, so any other `llamaport/` line names a
  model it no longer has. Entries belonging to other providers are never touched.
- **The file's mode is read before the write and restored; a created file is
  `600`.** The first real write dropped the mode from `600` to `644` and briefly
  published five API keys, because a fresh temporary file does not inherit the
  mode of the file it replaces. See [knowledge/technical.md](../knowledge/technical.md).

## Constraints

- The test suite may not touch `~/.pi/agent/models.json`. It uses the taken-once
  path override `store::use_config_dir` instead.
- A read-modify-write on one file needs a mutex around the whole pair; a rename
  alone does not stop two writers racing on the same temporary name.

## Acceptance

Met 2026-09-02, unless noted:

- Running the suite leaves `~/.pi/agent/models.json` byte-identical. A confirm on
  a real launch adds the provider and changes nothing else, and the four other
  providers, their models and their keys survive it.
- A file that will not parse is refused, not overwritten; a missing file is
  created holding only `{"providers": {...}}`.
- The model is named in `enabledModels`; the rest of the settings survive; the
  app's own dead entries are dropped; writing the same model twice does not
  duplicate it; an unparseable settings file leaves `models.json` untouched.
- **Open:** pi answering a prompt through the entry. That step is the author's to
  run; nothing in this repository can prove it.

## Verified

Verified 2026-09-02: all four checks passed, and the suite left the real pi file
byte-identical. The author wrote the entry for real, and pi listed the model with
no restart.

## Open

- `apiKey` and `maxTokens` are not derivable and mirror the existing `local-llama`
  provider. The backup is a single file, overwritten on each confirm.

## Files

`src-tauri/src/pi.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/store.rs`,
`src/PiPanel.tsx`, `src/ModelDetail.tsx`, `src/api.ts`, `src/types.ts`, `src/App.css`.

## Out of scope

Search and item 7. The launch form and per-field override. Reading anything back
out of pi. Restarting or signalling pi. Multiple models under the provider.
