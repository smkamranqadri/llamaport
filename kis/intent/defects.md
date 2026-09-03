# Defects found by looking

The record behind the rule in [knowledge/technical.md](../knowledge/technical.md)
Constraints: *a phase is not done when the suite is green; it is done when
somebody has looked.* Moved here 2026-09-04 from that file, where it had grown
to ninety lines. **Thirty-eight by 2026-09-03**, every one found by the author
at the built app, and two caught by the suite, both after the fact. The four
the method caught before he looked are named at the end. Nothing here is a
plan; new entries go at the bottom with their date.

**Twenty-four defects across seven phases by 2026-09-02, every one found by
looking and none by the suite.** Eleven on
2026-08-31, five in Tune's panel on 2026-09-01, three in the pi button on
2026-09-02 — a label that wrapped, a file mode read out of `ls -l`, and a
provider that turned out not to be selectable — and two in the redesign on
2026-09-02: every stray-server banner the app had ever shown named an unknown
model on an unknown port, and it took the author photographing one to notice;
and the sidebar unlit itself on a model screen, so a screen with no back
button said nothing about where you were. Two more in the redesign's last
item: the measurement row was pinned open, so a ladder that had answered kept
four tries on screen while the one preset it was measured for sat unchanged
above it — the author found it on the first look; and `.tune-head`,
`.tune-note` and `.tune-row.is-unranked` were painted with `--muted` and
`.tune-row.is-fastest` with `--surface-raised`, neither of which this
stylesheet has ever defined, so three greys rendered as body text and a
highlight as nothing. One more in Appearance on 2026-09-03: three of the
ported palettes are pale enough that the white label on the primary button
vanished into it, which is what `--on-accent` exists for. **A twenty-fourth was worse and took five releases**: a bundle macOS refuses to open, which only a browser download
exposed ([intent/release.md](../intent/release.md)). A phase is not done when
the suite is green; it is done when somebody has looked.

**Three more on 2026-09-03, from the author's first look at Discover, and one of
them is this list's own subject repeating.** The row printed "25.1 GB of 25.0 GB"
*while claiming to fit* — a decimal-GB file size beside a binary-GiB ceiling, both
labelled GB, the unit disagreement [knowledge/technical.md](../knowledge/technical.md)
carries a rule about. "Small & fast" listed a 229 GB model, because it ordered the
trending page by size and filtered nothing. And the stray-server banner had a top
margin and no bottom, so it sat four pixels off whatever followed it on every
screen. **Twenty-eight.**

**One was caught before it could be counted, on 2026-09-03**: Discover
painted its chips with `--muted`, and rendering the screen's own DOM against
`App.css` before the hand-over is what surfaced it. It is written here because
it is the same token as the redesign's, four months on, and because it is now
the one defect in this list that cannot recur silently — a test guards it.

**Four more from the second look, same day**: the app froze on opening Discover,
because a synchronous Tauri command holds the main thread; the download confirmation
named a quantisation and no model, on a screen the reader had just been thrown back
to; the sidebar showed nothing for a queued transfer, which the artboard had drawn;
and the detail page gave a name where nine repositories from nine owners publish the
same model. **Thirty-two.**

**Two more from the third**: a search left the old rows on screen underneath the
loading state, and the search box could not be cleared. **Thirty-four.**

**And a thirty-fifth of a kind not seen before: a feature that shipped dead.** The
MoE mark went out in `8c8db9d` doing nothing at all — `expand=gguf` never reached the
listing URL, so the architecture was always absent and every row came back unmarked.
It stayed green because an inline test asserted the URL must *not* carry
`expand=gguf`, which had been true one decision earlier. **A stale assertion is worse
than no assertion: it reports success for the thing it was written to forbid.**

**Two more in the same phase were the app's own guards being wrong rather than
the screen, and are not counted**, and both were caught by a test written the same hour: an avatar cache
whose containment check used `Path::starts_with`, and a size cap that had no test
until it was pulled out of the fetch. Neither reached the author. That is what the
method is for, and it is still the only way anything gets caught before he looks.

**Three more from the sidebar vibrancy, and they are a different kind again.** The
effect shipped defaulted off, which is the option the author had *not* chosen and
meant a transparent window painting over every pixel of itself; then at a 72% tint
that hid what was left; then flattening on every app switch, because the material
followed the window's focus. **Thirty-eight.**

A fourth was chased and **turned out not to be ours**: the vibrancy settles rather
than appearing when the window opens. The author found ChatGPT's app does the same,
which makes it what a transparent window on this platform does rather than a defect
to fix. **Compare a window effect against another app before treating its behaviour
as a bug** — there was no way to tell from inside this one.

**None of the three could have been caught by the usual render** — see the note on
window effects above. They are the first defects here where the author's look was the
only check available rather than the last one.

**The suite has caught two of the thirty-eight, both after the fact.**
`real_models.rs` went red on a speech model Discover had offered and the author had
downloaded; and a live test caught the dead MoE mark, but only once the author's own
find prompted someone to write it.

The sidebar one, the dead CSS tokens, the unreadable accent buttons and
Discover's chips are the only entries the *method* caught rather than the
author: one fell out of
rendering the artboard and putting it beside the app before anything was
changed, one out of reading the artboard's stylesheet against `App.css`, and
one out of rendering every palette before handing any of them over. That is the argument for doing the comparison first
rather than after a round of corrections.

**Added 2026-09-04, after the code review**: none from the author's look at the
four review parcels and the bundle — "all look good" — and one ask that was not a
defect but a gap, the model screen without its owner's picture. The count stays at
thirty-eight.
