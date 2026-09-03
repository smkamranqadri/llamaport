# Figures

Planned and completed 2026-08-31, shipped in v0.3.2 and v0.4.0. Two numbers the
app printed were wrong.

## Purpose

`estimate.rs` charged every layer a cache sized to the full context and called
the result exact. `Qwen3.6-35B-A3B-UD-IQ4_NL.gguf` declares
`full_attention_interval` alongside `ssm.conv_kernel`, `ssm.state_size`,
`ssm.inner_size` and `ssm.group_count`: 10 of its 40 layers do attention, and
the other 30 hold a fixed state that does not grow with context. The app
charged all 40 the full cache, roughly four times what the model allocates.

`format.ts` divided by 1024 cubed and printed GB. Every figure in the app was
7.4% larger than its own label claimed.

A wrong number is not a feature.

## Decisions

- **Count what the header describes, and mark the result a floor.** Revised
  the same day from abstaining on the KV term entirely: withholding threw away
  a figure the file gives, and everything left out can only add, so a floor
  can never say a model fits when it does not.
- **Read both pattern spellings.** Gemma-family headers carry
  `attention.sliding_window_pattern` with `attention.sliding_window`; Qwen3.6
  carries `full_attention_interval`. Where neither is legible, the code
  abstains.
- **Sliding-window layers get their own head widths**, from `key_length_swa`
  and `value_length_swa`, rather than reusing the full-attention pair.
- **The unit splits by what is measured, not by one global choice.** Files,
  disk space and rates go decimal, matching Finder and Hugging Face. Memory
  stays binary and stays labelled GB, matching Activity Monitor.
- **The rate-limit constants move with `formatRate`.** Changing the formatter
  alone would re-create the same mismatch pointing the other way.
- **The exactness claim is not strengthened.** The arithmetic is now right
  about what it models. Whether it matches llama.cpp's own allocation to the
  byte is unproven.
- **The abstain rule is a pattern key with no window size**, not the presence
  of `ssm.*` keys as first planned. It catches the same model without the code
  needing to know what a recurrent layer is.
- **The withheld-cache sentence names the numbers it read**, such as "one
  layer in 4, the other 30", rather than describing the shape of such a file
  in general.
- **The unit split stands even though the same file reads 18.0 GB in the
  Library and 16.8 GB as weights.** Each figure agrees with the tool it is
  meant to be compared against. The author ruled on 2026-09-04 that the screen
  stays without a hint ([review.md](review.md)).

## What was built

1. The KV term: `estimate.rs` sizes full-attention layers at the context and
   sliding-window layers at `min(window, ctx)`, and reports a hybrid's KV term
   as absent rather than zero, with the reason shown.
2. The units: `format.ts` split into a decimal formatter and a binary one,
   chosen per call site.

## Acceptance

Met 2026-08-31.

- A model with only full-attention layers estimates the same as before.
- A sliding-window model's KV term is full layers at the context plus
  sliding-window layers at `min(window, ctx)`, using the `*_swa` widths where
  the header carries them.
- `Qwen3.6-35B-A3B` shows a weights figure, no KV term, and the reason why.
- A model with neither pattern key legible abstains, as before.
- The Library's figure for a real file matches what Finder shows for it.
- A 32 GB Mac reports 32 GB of memory, not 34.4.
- A rate limit typed as 10 reads back as 10, and the floor hint matches what
  the engine applies.

## Verified

Verified 2026-08-31: all four checks passed. The formatters, run against real
byte counts, agree with Finder. `llama-server` ran `Qwen3.6-35B` at full
context on a 32 GB machine where the old estimate said it could not fit. The
author confirmed the wording on screen.

## Out of scope

Everything in [gaps.md](gaps.md). The recurrent-state
arithmetic. The sampling defaults, already correct because the app passes
nothing ([knowledge/technical.md](../knowledge/technical.md)).

## Risks

- The two pattern spellings are assumed to cover what is in reach. Anything
  else abstains, which is safe rather than wrong.
- `min(window, ctx)` cells is modelled from what `libllama` reads, not
  measured against what it allocates.
