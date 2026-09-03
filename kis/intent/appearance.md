# Appearance

Asked for and shipped 2026-09-03, pointing at the light-mode artboard and at
the hermes-hq project to read palettes from.

## What was wrong

The app followed macOS silently and had no way to say otherwise. Both
appearances existed (the redesign drew them, and the artboards include a
light variant), but nothing on any screen let a person choose, and `App.css`
held two hand-written lists of fifteen values with no name between them.

## Decisions

- **Five palettes.** The app's built-in one, plus four ported from the
  hermes-hq project (Violet, Nous, Bronze, Slate), with attribution to that
  project for their ids, names and colours.
- **Mode drives the built-in palette only.** A ported palette is one fixed
  appearance, not something Light or Dark mode can switch between.
- **The choice lives in `config.json`**, at schema 8, alongside the models
  directory and the launch defaults.
- **The picker is a section on the Settings screen**, not a sidebar menu.

## What the shape cost

- A palette is seven anchors, not fifteen values: ground, text, muted text,
  line, accent, running and danger. The remaining surfaces (sidebar, card,
  hover, input, and so on) are mixed from those once, in `:root`.
- `theme.ts` writes `data-theme` and `data-mode` on the root before the
  first render, so the user's choice always wins over the system default.
- `--on-accent` exists because white is not always readable on an accent
  colour; three of the palettes carry their own ink instead.
- Theme and mode are stored as strings, not enums, so a name written by a
  later build does not fail the whole config, only its own highlight.
- A `localStorage` copy exists as a cache, since reading `config.json` is a
  round trip to Rust. The config remains the source of truth.

## Deviations

- No artboard covers this screen; it was designed directly against the app.
- Some colours still do not follow a palette: the memory-safety badges, the
  Starting pill and the warning badge keep fixed ambers and greens.

## Vibrancy

Asked for 2026-09-03: a blurred, semi-transparent sidebar. Shipped the same
evening.

### Decisions

- The window is transparent at every launch; a Settings toggle decides
  whether the effect shows.
- Tinted, not clear, so the active palette's colour still comes through.
- An absent key in `config.json` means the effect is on, which covers
  configs written before the setting existed.

### Cost

- The private API the effect needs bars distribution through the App
  Store, permanently. Accepted, since the app already ships unsigned
  through GitHub.
- A brief flash on window open is confirmed to be platform behaviour, not a
  defect here, by checking another app that uses the same effect.
- Two tuning numbers, the tint level and the underlying material, remain
  untuned against each other.

## Verified

Verified 2026-09-03 for the appearance system and 2026-09-04 for vibrancy:
all four checks passed, and the author confirmed every palette and the
vibrancy effect on the running app.
