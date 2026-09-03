# Defects found by looking

This is a record of defects found by looking at the built app rather than by
the automated test suite. It supports the rule in
[knowledge/technical.md](../knowledge/technical.md): a phase is not done when
the suite is green; it is done when somebody has looked.

## Tally

| Date | Phase | Count | Examples | How found |
|---|---|---|---|---|
| 2026-08-31 | figures, fitting, screen | 11 | a raw `0` shown for Auto, two plans on one panel, the wrong reason on a cache floor, a form naming a context its command contradicted, a green "fits" with no memory free | looking |
| 2026-09-01 | tune | 5 | no column headers, an unlabelled figure, the suggested row unmarked, an explainer that denied existing measurements, a duplicated empty state | looking |
| 2026-09-02 | pi | 3 | a wrapped label, a file mode dropped from 600 to 644, an unselectable provider | looking |
| 2026-09-02 | redesign | 2 | stray-server banner named the wrong model, sidebar gave no back cue | looking (sidebar caught by render check) |
| 2026-09-02 | redesign | 2 | measurement row pinned open, undefined CSS tokens rendered as body text | looking (tokens caught by render check) |
| 2026-09-02 | packaging | 1 | bundle refused as damaged | a real download through a browser |
| 2026-09-03 | appearance | 1 | white label vanished into a pale palette | render check |
| 2026-09-03 | discover | 3 | GB/GiB unit clash, an oversized model in a size filter, banner margin off by four pixels | looking |
| 2026-09-03 | discover | 4 | frozen main thread, wrong confirmation copy, missing queue indicator, ambiguous model name | looking |
| 2026-09-03 | discover | 2 | stale search results under the loading state, search box would not clear | looking |
| 2026-09-03 | discover | 1 | the MoE mark shipped dead | looking |
| 2026-09-03 | vibrancy | 3 | defaulted off, tint too strong, flattened on every app switch | looking |

## Summary

The total reached 38 by 2026-09-03. The suite caught two of them after the
fact: a live test against a downloaded speech model, and a live test for the
dead MoE mark, both written only after the author's own find prompted them.
Four were caught by rendering before the author looked at the app at all: the
redesign's sidebar, the redesign's undefined CSS tokens, Appearance's
unreadable accent buttons, and a set of Discover's chips that was caught
before it could be added to the count. Two further issues that same week, both
in the app's own guards rather than the screen, are excluded from the count:
they were caught by a test written the same hour and never reached the author.

## Rules

- A stale assertion is worse than no assertion: it reports success for the
  exact thing it was written to forbid.
- Compare a window effect against another app before treating its behaviour as
  a bug; a vibrancy effect that settles rather than appears may be the
  platform, not a defect.
- Do the render comparison first, not after a round of corrections.
