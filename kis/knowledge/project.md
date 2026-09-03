# Project

**Llamaport.** A macOS app for running local GGUF models under `llama-server`,
and downloading them from Hugging Face with working resume. Named 2026-08-02,
before packaging: models arrive at a port, and `llama-server` binds one.

**For:** the author first, other local-LLM users on macOS second. Stage is a
public beta, eleven releases in, so packaging and other people's machines are
real scope, not hypothetical.

**Problem:** the `llama-server` launch command is stable except for three values
yet gets retyped or hunted out of shell history every time; nothing records what
context a model supports or what a given `-c` costs. Separately, `curl -L -C -`
against Hugging Face does not resume reliably and is slow on one connection,
so downloads currently go through an external download manager.

**In scope**, rewritten 2026-08-31 by the author as its only user — see
[intent/direction.md](../intent/direction.md), which supersedes the two lines
below where they disagree: list models with real GGUF metadata, launch one,
report what it costs, stop it, say whether it works; download with resume; have
an opinion about the settings rather than offering knobs; point pi at what is
running; and help find a model worth downloading, which Discover does since
2026-09-03.

**Out of scope:** a chat UI of our own — `llama-server` ships one, enabled by
default, and a ready model opens it in a second app window labelled Web UI. That
window is the decision holding, not an exception to it. Also out: non-macOS platforms,
managing the llama.cpp installation itself, saved profiles or presets, API keys,
binding anywhere but loopback.

## How llama.cpp's Web UI actually works

Checked against the installed build, because the design depends on it and the
obvious guess is wrong:

- It is served by `llama-server` itself, not by any always-on service. With no
  model running the port refuses connections outright. It exists only between
  Ready and Stop, which is why a permanent Chat item in the sidebar cannot work.
- Conversations and settings live in the **client**, not the server: IndexedDB
  (`LlamacppWebui`, a `Conversations` store) plus `localStorage`. That is why
  history survives a restart, and why it is keyed by origin — `http://127.0.0.1:<port>`.
  A different port is a different, empty history. The fixed port is what makes
  the persistence usable, so it is load-bearing, not incidental.
- `--help` calls it the **Web UI**, flags `--ui`/`--webui`, enabled by default.
  `--no-webui` in `rawArgs` would leave the window on a 404; deliberately
  unguarded, since `Capabilities.flags` lists the flag either way.

## Durable decisions

Each is argued in the specs; this is the index, not a second copy.

- Report memory, never forecast a total — a forecast wrong by 2x gets believed.
- Build argv, never a shell string; display a shell-quoted rendering only.
- Probe `llama-server --help` for accepted flags; never assume a build's flags.
- Read GGUF headers directly, and walk the entire KV block.
- One model at a time; a busy port refuses the launch instead of moving.
- Find stray servers by scanning processes, not by reading a pidfile.
- ~~No profile system~~ — **reversed 2026-08-31**
  ([intent/direction.md](../intent/direction.md)). Not into presets: the seven
  fields the author has never once changed across 21 launches become the
  optimizer's business, and the app takes a position instead of offering knobs.
  The rule was right that a form of knobs nobody sets is bad and wrong about the
  remedy. What survives unchanged is the no-merging rule below.
- No merging. A form opens on the most specific whole
  profile there is: what is being edited, else this model's last **successful**
  launch, else the launch defaults set in Settings, else the built-in values.
  Defaults seed a model nobody has launched; they never overrule one that has.
- Extra arguments may not set what a field already owns. `--host` and `--port`
  for safety — the app must know where the server is. `--alias` for a different
  reason: the field exists, so a second one only makes the launch disagree with
  what the form shows. The refusal names the field that owns the flag.
- Loopback only, no authentication.
- Downloads re-resolve the redirect on every resume, because the CDN signature
  expires and re-requesting the original URL is exactly where `curl -C -` fails.
- A failure is transient, an expired signature, or fatal, and each gets a
  different answer; treating them alike either abandons recoverable transfers or
  hammers a wall.
- One rate limit across all segments, not one per segment.
- The rate is read from `Control` as the transfer runs rather than fixed when it
  starts, so a limit changed mid-download applies to the one being watched. What
  a user may ask for is bounded in the app; the engine honours what it is told.
- Where a forecast cannot be avoided — time remaining — it is smoothed, withheld
  until it settles, and worded as an approximation. Same reasoning as the memory
  rule above.
- An interrupted transfer is described by the disk, and only what finished is
  written down. The sidecar beside a `.part` is the one account that cannot go
  stale while a transfer runs; `downloads.json` holds complete and failed rows,
  which have no on-disk trace to recover from. Neither store guesses at the
  other's business.
- Stopping keeps the bytes and says so: **Pause** is what cancel always did,
  **Discard** is the one that deletes. A discard removes the files on the settle
  path, after the engine has returned — `Control::cancel` returns while the
  transfer is still writing.
- A resume continues the job it belongs to rather than opening a second one, and
  a paused row holds its file against a fresh start of the same URL.
- The models directory is scanned for `.part` files, not only for models. They
  are invisible to the catalog, so without it they accumulate unseen and their
  bytes are unreachable — which is how 5.66 GiB of a 16.45 GiB quant sat
  stranded until 2026-08-03.
- What the app reads off its own disk is not trusted because it wrote it. A path
  in the history decides what a resume writes and a discard deletes, so it is
  rebuilt from the models directory rather than believed; a URL in a sidecar is
  checked against the same rule a pasted one meets.
- History is never trimmed; the screen pages it. A cap would be a number nobody
  has evidence for.
- The Library orders on **recency** — the last launch when there has been one, the
  file's mtime otherwise — favourites partitioned above, undated models last. A
  list may reorder only for a reason the reader can see, so this holds exactly as
  long as the row shows the value it sorts on. The two meanings share one cell and
  are told apart by weight, a launch in normal text against a muted mtime; the
  cell has no header to label it, and "today" otherwise means two things.
- A model is dated by a run that reached **Ready**, never by one that merely
  spawned. `runner.start` returns while the process may still fail to load its
  weights, and a model that never served was not used.
- A Library row is a star, a row button and a trailing button in a flex line, and
  **what dresses the row belongs to the row, not to the middle child**. Its
  border, its hover and its running tint are all on `.model-item`; putting any of
  them on `.model-row` paints the middle and leaves both ends bare. The trailing
  button is Stop while that model runs and Delete otherwise, at one fixed width —
  the row button is `flex: 1`, so a shorter word there drags that row's figures
  out of line with every other row.
- A server that will not serve ranges is refused: no ranges means no resume, and
  an unresumable 20 GB transfer is a trap rather than a convenience.
- llama.cpp's UI is reached by a second app window, not an iframe and not a
  browser tab. An iframe would need an http subresource inside a custom-scheme
  document, which nobody has verified WKWebView allows; a true embedded pane
  needs Tauri's `unstable` feature. A second window needs neither and keeps the
  webview's own persistent store. Only the main window hides on close — any
  other window closes for real, or it would sit invisible and stale.
- What is borrowed is labelled as borrowed: the button reads **Web UI** and the
  window **llama.cpp — Web UI**. Never "Open Web UI" — Open WebUI is a different,
  well-known project, and the phrasing would claim it.
- The name does not lead with llama.cpp. A shipped `llama.cpp hub` claims a
  project it is not part of, and sits unfindable next to it in any search. The
  names ruled out on collision are in [rename.md](../intent/rename.md), so this
  is not reopened a third time.

Detail: [runner spec](../../docs/runner-spec.md),
[downloader spec](../../docs/downloader-spec.md).
