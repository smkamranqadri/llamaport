# MoE

Written 2026-09-02. Open, blocked on a measurement.

## Purpose

Every launch this app builds is fully offloaded. For a mixture-of-experts (MoE)
model that is the expensive way to run it: a 35B-A3B model costs its whole
size against the Metal working set, even though only a fraction of its
parameters are used per token.

## What is established

- **Active parameters buy arithmetic, not residency.** The router picks a
  different subset of experts per token, so llama.cpp cannot know in advance
  which weights it will not need. All of the quantised weights sit in memory;
  active parameters only shrink the multiplication per token, not the bytes
  held.
- **`llama-server` already offers the lever.** `-cmoe`/`--cpu-moe` puts all
  expert weights on the CPU; `-ncmoe`/`--n-cpu-moe N` puts the first N layers'
  worth there; `-ot`/`--override-tensor` places tensors by pattern. Nothing in
  the app names any of them.
- **On Apple silicon this moves a budget, not bytes.** CPU and GPU share one
  physical memory pool, but Metal can claim only part of it. Expert tensors
  held on the CPU come out of the capped working set and stay in the same
  RAM; attention and shared layers keep the GPU.
- **`gguf.rs` already detects MoE; `estimate.rs` ignores it.** The memory
  estimate is blind to the distinction this plan turns on.
- **Discover marks MoE repositories since 2026-09-03**, from the uploader's
  tag and the file's architecture together; see [discover.md](discover.md).
  That says which models this matters for. It says nothing about the
  measurement below, and a mark on a model nobody has downloaded is not a
  launch setting.

## What blocks planning

Nobody has timed `--n-cpu-moe` against the smaller quant the author actually
runs, so there is no basis yet for the app to prefer one setting over another.
A memory sum can say a launch is allowed; only a real run says it is good.
The ladder in `tune.rs` is where that measurement belongs; see
[tune.md](tune.md).

## Open questions

- Preset, field, or automatic: how does the author choose an `-ncmoe` value,
  if at all.
- Does `estimate.rs` learn about experts, so the memory verdict changes when
  experts move off the GPU.
- What the first-run card says once this exists; its wording today assumes a
  fully offloaded launch.

## Risks

- Offering `-ncmoe` without the ladder measurement would hand the author a
  setting that trades speed for headroom in an unknown ratio.
- The flag must be gated on the probe, like every other flag; a build may not
  have it.
