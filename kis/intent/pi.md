# pi

Item 6 of what was asked for ([direction.md](direction.md)): **one click to point
pi at the running model.** Planned and built 2026-09-02, unreleased. The last of the three
remaining pieces of the shape direction implies that could be planned at all —
search is blocked on what "best model" means, and the launch form is blocked on
what the named choices are called.

## Why this exists

pi can reach 2 of the 19 models this app has launched. Its `local-llama`
provider points at 8888 while 17 of 21 launches were on 8080, and the file is
hand-maintained and has fallen behind. The app knows the port, the alias and the
context the server actually accepted. It is the only party in the room with the
facts.

## What it does

A button in the Running panel beside Web UI and Test model
(`src/ModelDetail.tsx`), which exists only at Ready. It opens a panel showing the `llamaport` provider block as it stands and as it
would be, one checkbox, and a confirm. On confirm the app backs the file up and
writes one provider, leaving every other byte alone.

Written from what the runner already holds: `baseUrl` from the bound port, `id`
from the launch alias, `contextWindow` from the server's own `n_ctx`. The rest
mirrors the author's hand-written `local-llama` block, which is the convention
this file already has.

## Decisions

**A diff, then a write on confirm — not a write on the click.** direction.md
says the file is hand-edited and shared with four other providers, so writing
behind the author's back is wrong. The recommendation here was that the click is
itself the consent and one press should be enough; the author chose the panel.
Two clicks against a file the app does not own is the right trade, and the diff
is also what makes the port overlap visible.

**One provider, one model, replaced on every confirm.** Reversed twice during
planning and worth recording as such. Accumulating models was chosen first, then
reversed once the consequence was put: a pi provider carries exactly one
`baseUrl`, so every model accumulated under `llamaport` points at whichever port
the newest launch bound. Accumulation does not preserve old entries; it
silently redirects them. That is precisely how `local-llama` came to point at
models nobody runs.

**The port overlap is named, never refused.** `mlx-lm` and `omlx` already point
at 8080, `omlx` being the default provider. The author's correction: an entry in
that file is a declaration, not evidence that anything is bound to the port.
Only one server can hold 8080, and whoever holds it answers to all three names.
So the overlap is a naming ambiguity worth stating above the diff, not a
conflict to prevent — and refusing would block 17 of 21 launches.

**The reasoning flag is a checkbox.** Everything else is derived and read-only.
Nothing in a GGUF reliably states whether a model reasons, the author set it
true by hand for Qwen3.6, and defaulting it false would write the opposite of
what they wrote, every time. The checkbox seeds from the `llamaport` entry
already in the file, so a second click on the same model needs no thought and no
new storage. That memory does not survive writing a different model, which is
the price of keeping one entry and of not bumping the config past schema 7.

**A missing file is created; an unparseable one is refused.** Creating it is
cheap and not a guess: `models.json` has exactly one top-level key,
`"providers"`. Refusing the unparseable one is the whole point — overwriting a
file the app failed to read is how a hand-maintained config gets destroyed.

## Constraints this inherits

- **The suite may not touch `~/.pi/agent/models.json`.** The pi path gets the
  taken-once override `store::use_config_dir` already provides for Application
  Support ([knowledge/technical.md](../knowledge/technical.md)). This is Tune's
  first parcel again, against a file that is not even the app's.
- **A read-modify-write on one file needs a mutex around the whole pair**, not
  faith in the rename: `write_atomic`'s temporary is one name per destination,
  so two writers race on it. This feature is a read-modify-write by definition.
- **The built app, not `tauri dev`.** Sixteen defects across four phases were
  found by looking at the app and none by the suite.

## Enabling, found by using it

**A provider is not enough.** The author wrote the entry, and the model still did
not appear in pi — it had to be enabled by hand first. `enabledModels` in
`~/.pi/agent/settings.json` is a list of `"<provider>/<model id>"` strings, and pi
offers nothing that is not named there. So "one click to point pi at the running
model" was one click plus a trip through pi's UI, and item 6 was half delivered.

**The button now writes both files.** Ours is appended to `enabledModels` if it is
not already present, and **every other `llamaport/` entry is dropped**: the
provider lists exactly one model, so any other line of ours names a model it no
longer has. The author's list already held two of them — a dead entry we created
without noticing, the same rot that put `local-llama` on 8888, reappearing in a
file we did not think we were touching. Entries belonging to other providers are
never removed.

**Both files are read and checked before either is written.** An unparseable
settings file discovered after `models.json` had changed would leave pi holding a
provider it will not offer — a half-applied state, and the worst of the outcomes
available.

**The panel diffs rather than displays.** Two JSON blobs side by side asked the
reader to find the difference themselves, which is not what a confirm is for. It
now shows a line diff per file with added and removed lines marked and counted,
what would be pruned named in a sentence, and a line saying the provider alone is
not enough.

**pi re-reads both files live** — no restart for either, which closes the question
the plan could not answer.

## Placement, corrected on the screen

**Built into the sidebar's `NowRunning` strip first, and moved.** Planning offered
"the running-status area" against "the model screen", and the sidebar strip was
the wrong reading of it: the author's answer meant the Running panel, beside the
live figures and the two buttons already there. Looking at it settled it twice
over — "Use in pi" wrapped onto three lines in a 200px sidebar and collided with
Stop, and the author said plainly it belonged with Web UI and Test model. The
sidebar is back exactly as it was.

## What looking at it found

Two defects, neither reachable by the suite, which is this project's pattern
rather than a surprise.

- **`apply` reported having pruned nothing.** It returned a preview read back from
  disk, and by then the dead entries were gone — so the panel would have removed
  lines from the user's settings and told them it had removed none. The pruned
  list is now carried from before the write. Found by a test that asserted what
  the panel would show rather than what the file ended up holding.
- **The label wrapped onto three lines** in the sidebar and ran into Stop. Fixed
  by moving the button, not by styling around it.
- **The file's mode dropped from `600` to `644`.** `write_atomic` renames a fresh
  temporary into place, and a fresh file does not inherit the mode of the one it
  replaces — so the first real write published five API keys to every account on
  the machine. Found in `ls -l`, not by a test. The mode is now read before the
  write and set back after it, a created file is born `600`, and both are covered
  by tests watched to fail. The general rule is in
  [knowledge/technical.md](../knowledge/technical.md).

## Proof

The four commands green, each status captured on its own line. **256 tests**, up
from 234 at v0.5.0; the twenty-two are this feature's. Four were watched to fail
against a mutation: the parse guard, the `preserve_order` feature, the file mode,
and the rule that neither file is written when the other cannot be read.

`~/.pi/agent/models.json` was byte-identical across a full run of the suite, with
no new files in that directory — the seam holds.

**Written for real against the author's own file**, by the author at the machine
rather than from here. Six providers where there were five, `llamaport` appended
last, the other five and their keys untouched, the hand-written order intact, and
`models.json.llamaport.bak` holding the previous 3,528 bytes. That write is what
exposed the mode defect.

## Acceptance

1. Running the suite leaves `~/.pi/agent/models.json` byte-identical.
2. A confirm on a real launch adds the provider and changes nothing else, proved
   by diffing the file across the write.
3. The four other providers, their models and their keys survive a confirm.
4. A file that will not parse is refused, not overwritten.
5. A missing file is created holding only `{"providers": {...}}`.
5b. ~~The model is named in `enabledModels`; other providers' entries, `defaultModel`,
   `defaultProvider` and the rest of the settings survive; our own dead entries are
   dropped and nobody else's ever are; writing the same model twice does not
   double it; and an unparseable settings file leaves `models.json` untouched.~~
   **Met 2026-09-02**, and confirmed end to end: written from the panel with pi
   open, the model was selectable with no trip through pi's UI and no restart.
   Item 6 is one click.
6. **pi lists the model and answers a prompt through it.** The author's to run;
   nothing in this repository can prove it. **Still owed** — the entry is written
   and correct on disk, and nobody has asked pi to use it.

## Open when work starts

- ~~**Does pi re-read its files while running?**~~ **Closed 2026-09-02**: both of
  them, with no restart. Written from the panel with pi open, and the model was
  selectable straight away. Recorded in
  [knowledge/technical.md](../knowledge/technical.md). The panel says nothing
  about restarting, which turns out to be right rather than merely cautious.

- **`apiKey`.** Every provider carries one and `llama-server` ignores it, so the
  app must write something and copying another provider's is wrong. Read
  `local-llama`'s value at the time and follow its convention, or write a literal
  placeholder if that value looks like a secret.
- **`maxTokens`** is not derivable; mirror `local-llama`.
- The backup is a single `models.json.llamaport.bak`, overwritten each confirm.
  That directory already collects backup files; this feature should not add to
  the pile on every press.

## Files

`src-tauri/src/pi.rs` (new), `src-tauri/src/lib.rs` (`pi_preview`, `pi_apply`),
`src-tauri/src/store.rs` (the path seam), `src-tauri/tests/common/mod.rs`,
`src/PiPanel.tsx` (new), `src/ModelDetail.tsx`, `src/api.ts`, `src/types.ts`,
`src/App.css`. `serde_json` gains `preserve_order`, which adds nothing to the
build: indexmap is already in the tree through tauri.

## Out of scope

Search and item 7. The launch form and per-field override. Reading anything back
out of pi. Restarting or signalling pi. Multiple models under the provider.
