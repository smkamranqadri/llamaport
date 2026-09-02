# Appearance

Asked for 2026-09-03: "let's add theme and mode", pointing at the light-mode
artboard and at a sibling project to read the themes from —
`/Users/mkamran/Repositores/side-projects/hermes-hq`. Built and shipped the
same day in `d15f9ad` and `d8f9d9b`.

Live status is in [state/current.md](../state/current.md), not here.

## What was wrong

The app followed macOS silently and had no way to say otherwise. Both
appearances existed — the redesign drew them, and the artboards include a light
variant — but nothing on any screen let a person choose, and `App.css` held two
hand-written lists of fifteen values with no name between them.

## Decisions, all four put to the author

- **Seven palettes, then five.** The built-in one plus six ported from
  hermes-hq; Nous Light and Hermes were removed on sight the same day, leaving
  Violet, Nous, Bronze and Slate. Their ids, names, descriptions and colours are
  that project's own (`frontend/src/theme.ts`, `frontend/src/index.css`); its
  `working` and `error` stand in for this app's `running` and `danger`.
- **Mode drives the built-in palette alone.** A ported palette is one
  appearance and its row says so, rather than the Mode control pretending to
  apply. The alternative offered was deriving a light set for each dark palette,
  refused because those variants would have been invented here and attributed
  to hermes-hq.
- **The choice lives in `config.json`**, beside the models directory and the
  launch defaults, at schema 8 — not in the webview, where a cleared store would
  lose it and no file would show it.
- **The picker is a section on the Settings screen**, not a menu in the sidebar
  where hermes-hq puts its own.

## What the shape cost

- **A palette is seven anchors, not fifteen values.** Ground, text, muted text,
  line, accent, running and danger; the surfaces between them — sidebar, card,
  card2, hover, badge, input, code, faint — are mixed from those once, in
  `:root`. A ported palette is seven declarations rather than fifteen guesses.
  The built-in one still names all fifteen, because they were drawn against the
  artboards rather than derived.
- **`theme.ts` writes `data-theme` and `data-mode` on the root before the first
  render.** The two selectors never both match, so what wins is what the user
  picked rather than what comes later in the file. Every
  `@media (prefers-color-scheme: dark)` is gone from the stylesheet with it:
  System is resolved in one place, in JavaScript, and macOS switching under a
  window set to System re-applies through a `matchMedia` listener.
- **`--on-accent`, because white is not always readable on an accent.** Nous is
  amber, Bronze is bronze and Slate is pale blue; a white "Use in pi" on any of
  the three is a button nobody can read. Those three carry their own ink.
- **Theme and mode are stored as strings and not enums.** A name written by a
  later build must cost the screen its highlight, not fail the parse and take
  the models directory with it. The screen falls back to the built-in palette
  and the config keeps the name it did not understand.
- **A localStorage copy, deliberately a cache.** Reading the config is a round
  trip to Rust, so without one the window opens light and corrects itself a
  moment later. The config is the truth: `App` reconciles the two at boot.

## Deviations worth knowing

- **This screen has no artboard.** The Settings artboard draws Models folder,
  llama-server and Launch defaults and nothing else, so the Appearance section
  was designed here. Offered to the author as such.
- **Some colours still do not follow a palette**: the memory-safety badges, the
  Starting pill and the warning badge keep fixed ambers and greens. Recorded
  rather than fixed, because nobody has asked what they should be under Nous.

## Proof

```text
build: 0
cargo test status: 0        (258 passed, 5 ignored — one new)
clippy status: 0
fmt status: 0
```

The new test is `an_appearance_survives_a_round_trip_and_an_unknown_name_is_kept`:
a saved appearance comes back, and a theme name from a later build survives a
load without costing the config its other fields. **Mutation-checked** — with
`#[serde(skip)]` on the field it fails at the round trip.

**Seen before it was handed over, and by the author after.** Each palette was
rendered against `App.css` in headless Chrome — one document per palette,
because the derived tokens resolve on the element that declares them and a first
attempt putting `data-theme` on a `div` rendered every ported theme with
near-white cards. That is what found the unreadable accent buttons. The author's
word on the result: "all good".
