# Figures

Planned 2026-08-31. Two numbers the app prints are wrong, and both were found by
reading someone else's source rather than by using the app — the first time on
this project that a defect has arrived that way ([gaps.md](gaps.md)).

Neither is a feature. That matters, because the roadmap's step 8 says not to
plan features against silence and nothing has converted yet
([roadmap.md](roadmap.md)). A wrong number is not a feature, so this phase does
not test that rule.

Live status is in [state/current.md](../state/current.md), not here.

## What is actually wrong

**The KV term over-counts, on the model the author runs.** `estimate.rs` charges
every layer a cache sized to the full context. Its own doc comment says the two
numbers are exact, and for a plain attention model they are. But layers are not
all alike, and a GGUF says so. `Qwen3.6-35B-A3B-UD-IQ4_NL.gguf`, on this disk,
declares `qwen35moe.full_attention_interval = 4` alongside `ssm.conv_kernel`,
`ssm.state_size`, `ssm.inner_size` and `ssm.group_count`: 10 of its 40 layers do
attention, and the other 30 are recurrent and hold a fixed state that does not
grow with context at all. The app charges all 40 the full cache — roughly four
times what the model will allocate, printed as an exact figure.

`libllama` reads the whole family of keys and builds two caches from them:
`attention.sliding_window`, `attention.sliding_window_pattern`,
`full_attention_interval`, `key_length_swa`, `value_length_swa`, and it logs
`creating SWA KV cache` beside `creating non-SWA KV cache`. Verified against
build b10360 by reading the strings in `libllama.dylib` and the header of the
file itself.

**The unit label disagrees with the arithmetic.** `format.ts` divided by
1024³ and printed GB. Every figure in the app is therefore 7.4% larger than its
own label claims: a 16.45 GiB file reads "16.5 GB" where Finder, which is
decimal, says 17.66 GB for the same bytes. One formatter currently serves file
sizes, disk free space, memory and transfer rates, which is the underlying
mistake — the right unit is not the same for all four.

## Decisions

- **Compute the layer kinds, do not retreat to a bound.** The data is in the
  file. The rule against forecasting is about what cannot be known, not about
  arithmetic that has not been done yet, and marking a computable figure as a
  ceiling would be the wrong lesson drawn from the right rule.
- **Revised 2026-08-31, after seeing it: count what the header does describe and
  mark the result a floor.** Withholding the figure entirely threw away one the
  file gives. Ten of Ornith's forty layers do full attention and can be sized
  exactly; only the other thirty cannot. The screen now reads "≥ 20.4 GB to
  allocate — weights 19.7 GB plus at least 0.7 GB of KV cache", with the missing
  term named. A floor is safe to print for the reason the earlier abstention was
  safe: everything left out can only add, so it can never say a model fits when
  it does not. This is the bounded-figure pattern recorded in
  [gaps.md](gaps.md) from the Unsloth read, arrived at here by the same route
  they arrived at it. The superseded reasoning follows.
- **Superseded — abstain on the recurrent term rather than model it.** A hybrid keeps its
  weights figure, which is exact regardless, and withholds the KV term with the
  reason on screen. Sizing a recurrent state from `conv_kernel`, `state_size`,
  `inner_size` and `group_count` would be a second arithmetic model nobody has
  measured, and getting it wrong would reintroduce exactly the defect being
  fixed. `estimate` already returns `None` where it cannot size a cache; this is
  that same stance, applied to a case it does not yet recognise.
- **Read both spellings.** Gemma-family models carry
  `attention.sliding_window_pattern` with `attention.sliding_window`; Qwen3.6
  carries `full_attention_interval`. Where neither is legible the code abstains,
  as it does today, rather than guessing a pattern.
- **The SWA layers get their own head widths.** `key_length_swa` and
  `value_length_swa` exist because they can differ from the full-attention ones,
  so the care `kv_bytes` already takes to sum K and V separately extends to them
  instead of reusing the wider pair.
- **The unit splits by what is being measured, not by one global choice.** File
  sizes, disk free space and transfer rates go decimal, so the Library,
  Downloads and ModelDetail agree with Finder and with Hugging Face. Memory
  stays binary and stays labelled GB, because that is what a 32 GB Mac has and
  what Activity Monitor reports. Two formatters, each named for which it is.
- **The rate limit moves with `formatRate` or not at all.** `toField`, `toRate`,
  the `MB` constant and the floor hint in `Downloads.tsx` are one closed system:
  a limit typed as 10 reads back as 9.5 if they disagree. Changing the formatter
  alone re-creates that bug pointing the other way. The screen has since been
  rewritten around a named ladder ([redesign.md](redesign.md)), and the rule
  survived it — the typed field is still parsed and printed by that same pair.
- **The exactness claim is not strengthened.** After this the arithmetic is
  right about what it models. Whether it equals llama.cpp's allocation to the
  byte is unproven — the server allocates in cells and pads them, and
  `kv_unified` is in play — so the doc comment says what was measured and no
  more.

## Parcel 1 — the KV term — DONE 2026-08-31

`gguf.rs` picks up the layer-kind keys into `GgufMetadata`; `types.ts` mirrors
them. `estimate.rs` sizes full-attention layers at the context and
sliding-window layers at `min(window, ctx)`, K and V summed separately as now,
and reports a hybrid's KV term as absent rather than zero. `ModelDetail.tsx`
says why when it is absent.

## Parcel 2 — the units — DONE 2026-08-31

`format.ts` splits into a decimal formatter and a binary one. Call sites choose:
`Library.tsx` and `Downloads.tsx` decimal for sizes and free space,
`Memory.tsx` and the system rows of `ModelDetail.tsx` binary, `formatRate`
decimal with the three rate-limit constants moving with it.

## Out of scope

Any release. Everything in [gaps.md](gaps.md) — none of its items belong to
this phase. The recurrent-state
arithmetic. The sampling defaults, which are already correct precisely because
the app passes nothing ([knowledge/technical.md](../knowledge/technical.md)).

## Acceptance

- `qwen2.5-0.5b-instruct-q8_0.gguf`, all full attention: the estimate is
  byte-identical to today's.
- A sliding-window model: the KV term is full layers at the context plus
  sliding-window layers at `min(window, ctx)`, using the `*_swa` widths where
  the header carries them.
- `Qwen3.6-35B-A3B`: the weights figure still shows, the KV term does not, and
  the screen says why.
- Neither pattern key legible: the same `None` as today.
- The Library's figure for a real file matches what Finder shows for that file.
- A 32 GB Mac still reports 32 GB of memory, not 34.4.
- A rate limit typed as 10 reads back as 10, and the floor hint matches what the
  engine applies.

## Verification

The four commands, each status captured on its own line and never after a pipe:
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`,
`bun run build`.

Every new test is gutted and watched to fail before it is trusted
([knowledge/technical.md](../knowledge/technical.md)).

A green suite does not finish either parcel. Three things are proved on screen
in the built app: the Library's figure against Finder for one real file, the
withheld-KV wording on `Qwen3.6-35B`, and the rate-limit round trip typed by
hand.

## Proof — 2026-08-31

The four commands green over the working tree, each status captured on its own
line: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
`cargo test`, `bun run build`. **190 tests**, up from 180 — seven new in
`estimate`, three in `gguf`.

Ten of those are trusted because they were watched to fail. `layer_split` gutted
to ignore the interval failed exactly the three that describe layer kinds;
`window.min(ctx)` gutted to `window` failed exactly the one about a window wider
than the context; dropping the second interval spelling and never reading the
window size each failed the key-reading test; and softening the withheld-cache
sentence back to "one layer in every few" failed the test that demands it name
the numbers it read.

The formatters were run against the real byte counts rather than eyeballed.
`Qwen3.6-35B-A3B-UD-IQ4_NL.gguf` is 18,040,888,288 bytes and the Library
formatter renders 18.0 GB, against Finder's 18.04 GB. A 32 GiB machine still
reports 32.0 GB. A rate limit typed as 10 reads back as 10, and 2.5 and 0.5
likewise; the floor hint moved from 64 KB/s to 66 KB/s, the same constant
rendered decimal.

The defect was also confirmed against llama.cpp itself, which is stronger than
the arithmetic. Left to size the context on its own, `llama-server` b10360 ran
`Qwen3.6-35B` at its full 262,144 tokens on this 32 GB machine at 6.7 GB
resident. The estimate this app shipped until today charged all 40 layers a full
cache and claimed 39.52 GB against 34.36 GB installed — it called a model
unfittable while the server was running it at maximum context. The attention
layers alone come to 23.41 GB.

**Seen on screen 2026-08-31**, on `ornith-1.0-35b-Q4_K_M.gguf`, which is the
same architecture and so takes the same withheld path. Size reads 21.2 GB for
21,166,757,760 bytes, installed memory reads 32.0 GB, the Context panel's cache
row reads Unavailable against "the header does not size these layers", and the
memory panel prints the sentence with the numbers it read — one layer in 4, the
other 30.

The screenshot also caught a defect in this phase's own work. The note
explaining which counting the memory figures use rendered only in the branch
that HAS a cache figure; the withheld branch replaced the whole hint with the
reason. That is exactly backwards. On that screen the Model panel's 21.2 GB and
the memory panel's 19.7 GB of weights sit one panel apart — closer than the two
figures were assumed to be when the split was argued — and the explanation was
the thing missing. Both branches now carry it.

## Decided while building

- The abstain rule is **a pattern key with no window size**, not the presence of
  `ssm.*` keys the plan named. It catches the same model without the code
  needing to know what a recurrent layer is, and catches others besides.
- The withheld-cache sentence carries the numbers it read — "one layer in 4 …
  the other 30" — rather than describing the shape of such a file in general. A
  test holds it to that.
- **The unit split stands**, including its consequence: the same file reads
  18.0 GB in the Library and 16.8 GB as weights in the memory bar. Each agrees
  with the tool it is meant to be compared against, the seam belongs to the
  domain rather than to this app, and macOS splits in the same place. The memory
  panel carried a hint naming its counting and the Model panel's figure as the
  same bytes counted the other way; the redesign dropped it with the three rows
  it removed, and the author ruled on 2026-09-04 that the screen stays as it is
  ([review.md](review.md)).
- `predicted_base` in `lib.rs` became weights-plus-cache-when-known rather than
  failing to compile. It is written to the launch spec and never read; removing
  it belongs to no phase yet.

## Risks

- **Both fixes land unreleased, by decision.** Every public build keeps the
  over-count and the mislabelled figures until something else cuts a release.
  This has happened before — the Dock fix sat unreleased through v0.3.0 — so the
  phase ends with the tree green and ready to tag, leaving the release a
  decision rather than a project.
- The two pattern spellings are assumed to cover what is in reach. Anything else
  abstains, which is safe rather than wrong.
- `min(window, ctx)` cells is modelled from what `libllama` reads, not measured
  against what it allocates. Recorded as an assumption, not stated as fact.
- The abstain wording is new prose on a screen and is settled by the author's
  eye, not by a test.
