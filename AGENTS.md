# Working in this repository

These rules apply to any coding agent working here. Product facts, decisions
and plans live under `kis/`; this file holds only how to work.

## Checks

Run all four before calling any change done. Capture each exit status on its
own line, never after a pipe. In zsh, `$PIPESTATUS` is empty, and
`cmd | tail; echo $?` reports the status of `tail`.

```bash
export PATH="$HOME/.cargo/bin:$PATH"   # cargo is not on a non-login shell's PATH
bun run build
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
```

Without the PATH export, every cargo command exits 127, which looks like a
failing check.

## The app belongs to the author

Do not launch, capture, restore from the tray, resize, or drive the app unless
the author asks. Ask the author for a screenshot and wait. When the author
asks for a launch, run `bun run tauri dev` in the background and stop it once
they say the captures are in. Editing a Rust file restarts the dev app under
them and stops whatever model is running.

## Verifying UI changes

- Compare a UI change against the mockup **rendered**, never against the code
  that generated the mockup. Canvas artboards are `.dc.html` files: strip the
  `<x-dc>` and `<helmet>` wrappers, move the `<style>` into `<head>`, serve the
  directory with `python3 -m http.server`, and open it in Chrome.
- Render what can be rendered without the app. The panel's own DOM against
  `src/App.css` in headless Chrome is a real check of metrics, tokens and both
  appearances. Do this before asking the author to look, not instead.
- A rendered mock proves the stylesheet, never the data behind it. A window
  effect (vibrancy) cannot be rendered at all; the author's look is the first
  check there.
- Every claim a mockup makes about behaviour is checked against the code or the
  phase file that proved it. Artboards are the spec for the shape, not for the
  facts.
- To reach a screen behind a click during a capture, patch a temporary
  `useEffect` into `App.tsx` that selects a model on mount, capture, revert,
  and prove the revert with `git diff --stat src/App.tsx` before running the
  checks.

### Capturing a window, when asked

- `screencapture -x -o -l <window id>` takes the window alone. The window id
  comes from `CGWindowListCopyWindowInfo` in a one-file Swift script. The
  webview exposes no accessibility tree, and System Events refuses to focus it.
- If `screencapture` reports "could not create image from window", the window
  is hidden or on another Space. `osascript -e 'tell application "System
  Events" to set visible of process "llamaport" to true'` makes it capturable.
- Wait for the window's bounds to stop moving before capturing. A window
  restored from the tray animates open, and a capture taken mid-animation has
  sloping lines that look like a layout defect.
- Never resize the window through System Events. `set size of window 1`
  collapses it.

## Editing

Assert that every edit anchor is present and unique before replacing it, and
read back what landed. An anchor that `cargo fmt` or an earlier edit has
already reformatted changes nothing and reports nothing.

## Tests

A new test is not trusted until the code it covers has been gutted and the
test watched to fail. For anything that persists state, read the file the app
actually wrote rather than trusting that it was written.

## Writing KIS

The repository is public, and `kis/` is read by contributors as documentation.
Write it in plain language (ISO 24495-1:2023) and keep it small.

- One idea per sentence, about 20 words or fewer. Active voice. Common words.
  Headings say what the section contains.
- Refer to the repository owner as the author and to the AI session as the
  agent, only where it matters who did or checked something. No first person,
  no quoted chat messages, no commentary on the process or on KIS itself.
- Dates as `YYYY-MM-DD`. No em-dashes or en-dashes, no strikethrough, no
  shouted status words. A reversed decision is stated as the current fact with
  the date and a one-line reason.
- Record decisions with their reason, acceptance criteria as met facts, open
  items, risks, and links. Do not record test counts, mutation records,
  checksums, commit hashes, timings, or how a check was run.
- Proof is one line under `## Verified`: what passed, what was confirmed, by
  whom, and when. Release checksums live on the GitHub release page.
- Working rules for agents belong in this file, never in `kis/`.
- A phase file follows: purpose, decisions, what was built, acceptance,
  verified, open, out of scope. Keep it to roughly one screen per section.

<!-- kis:anchor:start -->
## KIS Project Memory

This project uses KIS memory under `kis/`. Knowledge = stable facts. Intent = goals and plans. State = current reality.

- Read `kis/state/` before planning or implementing, then only the Intent and Knowledge the task needs.
- Put each fact in exactly one layer, and update an existing file instead of creating a new one.
- Prove work with real command or verification output before marking anything done.
- Synchronize the KIS layers that changed when work finishes.
- Full instructions: `.agents/skills/kis/SKILL.md`.
- Commands live in `.agents/skills/kis/commands/`: start, plan, act, sync, check, init.
  Claude `/kis:start`, Pi `/kis-start`, Codex `/prompts:kis-start`.
<!-- kis:anchor:end -->
