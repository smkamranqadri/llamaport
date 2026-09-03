# Project

**Llamaport.** A macOS app for running local GGUF models under `llama-server`, and
downloading them from Hugging Face with working resume. Named 2026-08-02: models arrive at a
port, and `llama-server` binds one.

**For:** the author first, other local-LLM users on macOS second. Stage is a public beta,
eleven releases in, so packaging and other people's machines are real scope, not
hypothetical.

**Problem:** the `llama-server` launch command is stable except for three values, yet gets
retyped or hunted from shell history every time, and nothing records what context a model
supports or what a given `-c` costs. Separately, `curl -L -C -` against Hugging Face does
not resume reliably and is slow on one connection, so downloads go through an external
manager.

**In scope**, rewritten 2026-08-31 by the author as its only user, see
[intent/direction.md](../intent/direction.md), which supersedes the two paragraphs above
where they disagree: list models with real GGUF metadata, launch one, report what it costs,
stop it, say whether it works; download with resume; take a position on the settings rather
than offering knobs; point pi at what is running; and help find a model worth downloading,
which Discover does since 2026-09-03.

**Out of scope:** a chat UI built by this app. `llama-server` ships one, enabled by default, and
a ready model opens it in a second app window labelled Web UI. That window is the decision
holding, not an exception to it. Also out: non-macOS platforms, managing the llama.cpp
installation, a saved-profile system beyond the three presets, API keys, binding anywhere
but loopback.

## How llama.cpp's Web UI actually works

Checked against the installed build, because the design depends on it and the obvious guess
is wrong:

- It is served by `llama-server` itself, not by any always-on service. With no model running
  the port refuses connections, so a permanent Chat item in the sidebar cannot work.
- Conversations and settings live in the client, not the server, keyed by origin
  (`http://127.0.0.1:<port>`). History survives a restart but a different port starts a
  different, empty history, so the fixed port is load-bearing.
- `--help` calls it the Web UI, flags `--ui`/`--webui`, enabled by default. `--no-webui` in
  `rawArgs` is deliberately left unguarded and would leave the window on a 404.

## Durable decisions

Each is argued in the specs; this is the index, not a second copy.

- Report memory, never forecast a total; a forecast wrong by 2x gets believed.
- Build argv, never a shell string; display a shell-quoted rendering only.
- Probe `llama-server --help` for accepted flags; never assume a build's flags.
- Read GGUF headers directly, and walk the entire KV block.
- One model at a time; a busy port refuses the launch instead of moving it.
- Find stray servers by scanning processes, not by reading a pidfile.
- Three presets exist since the redesign ([intent/redesign.md](../intent/redesign.md)).
  The earlier no-profile-system rule was reversed on 2026-08-31
  ([intent/direction.md](../intent/direction.md)): fields nobody had changed across 21
  launches became the optimizer's business. No merging, below, still holds.
- No merging. A form opens on the most specific whole profile there is: what is being
  edited, else the model's last successful launch, else Settings' defaults, else the
  built-in values. Defaults never overrule a launched model.
- Extra arguments may not set a field's own value: `--host`, `--port` and `--alias` are
  refused, since a duplicate would disagree with the form.
- Loopback only, no authentication.
- Downloads re-resolve the redirect on every resume, because the CDN signature expires and
  re-requesting the original URL is where `curl -C -` fails.
- A failure is transient, an expired signature, or fatal, each answered differently;
  treating them alike abandons transfers or hammers a wall.
- One rate limit applies across all segments of a transfer, not one per segment, and is read
  live as the transfer runs so a change mid-run takes effect.
- Where a forecast cannot be avoided, time remaining, it is smoothed, withheld until it
  settles, and worded as an approximation.
- An interrupted transfer is described by the disk: a `.part`'s sidecar cannot go stale
  while it runs, and `downloads.json` holds complete and failed rows with no other trace.
- Stopping keeps the bytes and says so: Pause is what cancel always did, Discard is the one
  that deletes, and only after the engine has returned.
- A resume continues the job it belongs to rather than opening a second one; a paused row
  holds its file against a fresh start of the same URL.
- The models directory is scanned for `.part` files, not only for models, since without that
  scan their bytes accumulate unseen.
- What the app reads off its own disk is not trusted because it wrote it: a path in the
  history is rebuilt from the models directory, and a URL in a sidecar is checked against
  the same rule a pasted one meets.
- History is never trimmed; the screen pages it. A cap would be a number nobody has evidence
  for.
- The Library orders on recency: the last launch when there has been one, else the file's
  mtime, favourites partitioned above, undated models last. A list may reorder only for a
  reason the reader can see.
- A model is dated by a run that reached Ready, never one that merely spawned, since a start
  can return before the process fails to load its weights.
- A Library row's border, hover and running tint belong to the outer row, not a middle
  element; the trailing button is Stop while running, Delete otherwise, at a fixed width.
- A server that will not serve ranges is refused: no ranges means no resume, and an
  unresumable multi-gigabyte transfer is a trap, not a convenience.
- llama.cpp's UI opens in a second app window, not an iframe or a browser tab, since an
  embedded pane needs Tauri's unstable feature. A second window keeps the webview's own
  persistent store, and only the main window hides on close.
- What is borrowed is labelled as borrowed: the button reads Web UI, the window is titled
  `llama.cpp — Web UI`, never "Open Web UI", the name of a different, well-known project.
- The name does not lead with llama.cpp: a shipped `llama.cpp hub` would claim a project it
  is not part of. Names ruled out on collision are in
  [intent/rename.md](../intent/rename.md).

Detail: [runner spec](../../docs/runner-spec.md),
[downloader spec](../../docs/downloader-spec.md).
