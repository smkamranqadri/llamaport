# MoE

Written 2026-09-02, unplanned and unstarted. Found while building the redesign's
first-run screen: the app told the author that `Qwen3.6-35B-A3B` at 22.1 GB has
little room left for a conversation on a 32 GB Mac, which is true of the only
kind of launch this app can build and not true of the machine. The author's
question is what opened it — "why moe can't fit, its active param are less it
will take less memory, am i wrong?"

Live status is in [state/current.md](../state/current.md), not here.

## What is wrong

**Every launch this app builds is fully offloaded, and for a MoE that is the
expensive way to run it.** `profile.rs` owns eleven fields — alias, host, port,
ctx, ngl, parallel, flash attention, both cache types, jinja, raw args — and
none of them can say where the expert weights live. So the only lever the app
offers against a large MoE is a smaller quant, which costs quality on every
token, when the lever that costs almost nothing is available and unused.

**The verdict the app prints is therefore narrower than it sounds.** The
first-run card reads `22.1 GB · tight — little room for a conversation` against
the 25,559 MiB Metal working set. That is honest for a fully offloaded launch
and would be wrong on a build that could park the experts elsewhere.

## What was established, 2026-09-02

- **Active parameters buy arithmetic, not residency.** The router picks a
  different subset of experts for every token, so llama.cpp cannot know in
  advance which it will not need: all 35B of quantized weights sit in the
  Metal buffer. What "A3B" buys is roughly 3B of multiplication per token, so
  the model generates near a 3B model's speed while holding a 35B model's
  knowledge. The KV cache does not shrink either — it is set by layers and KV
  heads, not by which experts fired.
- **The binary already offers the lever.** `llama-server` build `10360`
  (`48d22e295`) at `/opt/homebrew/bin` lists `-cmoe, --cpu-moe` (all MoE
  weights on the CPU), `-ncmoe, --n-cpu-moe N` (the first N layers' worth), and
  the general `-ot, --override-tensor <pattern>=<buffer>`, plus draft-model
  variants of each.
- **Nothing in the app mentions any of them.** `grep -rn "cpu.moe\|override.tensor"
  src-tauri/src/` returns nothing.
- **On Apple silicon this moves a budget, not bytes.** CPU and GPU share one
  physical pool, but Metal may claim only part of it — 25,559 MiB of this
  machine's 34.36 GB ([knowledge/technical.md](../knowledge/technical.md)).
  Expert tensors held on the CPU come out of the capped working set and stay in
  the same RAM; attention and shared layers keep the GPU. The cost is that the
  expert matmuls run on CPU, which for a 3B-active model is a small slice of
  the arithmetic.
- **`gguf.rs` already knows a MoE when it sees one.** `expert_count` is parsed
  and `is_moe()` exists, used today only for the `MoE` badge. `estimate.rs`
  does not read it — line 176 passes `expert_count: None` — so the memory sum
  is blind to the distinction this plan turns on.

## What is not established, and blocks planning

**Whether it is actually faster, and by how much.** Everything above is
arithmetic about where bytes sit. This project's own rule is that a memory sum
says a launch is *allowed* and never that it is good, and only running it says
the second ([knowledge/technical.md](../knowledge/technical.md)). Nobody has
timed `--n-cpu-moe` on this machine against the smaller quant the author runs
today, so there is no basis yet for the app to prefer one.

The measurement that would settle it already has a home: `tune.rs` runs a
candidate ladder and `tools/fits.py --run` launches a winner and reports real
tokens per second. A ladder over `-ncmoe` values against the author's current
`UD-Q3_K_XL` is the experiment, and it is the first thing this plan needs.

## Open questions for the interview

Written down rather than answered, because the decisions are the author's.

- **Is it a preset, a field, or automatic?** A preset owns six fields today and
  never alias, port, jinja or raw args ([redesign.md](redesign.md)); adding a
  seventh changes how `selectedPreset` derives the highlight. "Fit a bigger
  model" as a fourth card is a different answer from a number under Advanced.
- **Does `estimate.rs` learn about experts?** It must, if the memory verdict is
  to change when the experts move. That is a real change to arithmetic the
  suite covers and `tools/fits.py` cross-checks, so both move together.
- **What does the first-run card say once this exists?** Its wording is written
  against a fully offloaded launch and would need to stop being.

## Risks named at writing

- **A flag the build may not have.** Gate it on the probe like every other
  flag, and never assume `10360`'s help text describes the user's binary
  ([knowledge/technical.md](../knowledge/technical.md)).
- **A second way to be slow.** Offering `-ncmoe` without the measurement above
  would hand the author a setting that trades speed for headroom in an unknown
  ratio, which is the same mistake as shipping the arithmetic-picked candidate
  Tune exists to correct ([tune.md](tune.md)).
