# Fitting

Planned and completed 2026-08-31. Shipped in v0.4.0. The app stopped passing
`-c` and `-ngl` on every launch, so llama.cpp's own fitter can size them.

## Purpose

llama.cpp's `--fit` option is on by default. It adjusts any argument the launch
leaves unset so the model fits device memory. The app passed both `-c` and
`-ngl` on every launch, which switched the fitter off each time without saying
so.

Two measurements made this worth fixing:

- Left to itself, `llama-server` gave `qwen2.5-0.5b` its full 32,768-token
  context and `Qwen3.6-35B` its full 262,144 on a 32 GB machine. Forced to
  reserve a large margin, it dropped the small model to 4,096, which is the
  `--fit-ctx` floor. The fitter both grants and withholds.
- The app passed `-ngl all`. llama.cpp's default is `auto`. Where the weights
  do not fit, `auto` spills layers to the CPU and `all` insists on the GPU.

The context value was also the one number a user had to invent at every launch.

## Decisions

- **Auto is stored as `ctx: 0`.** llama.cpp's own help uses `0` for "loaded
  from model", so this borrows an existing meaning. It needs no config schema
  change and leaves every saved number alone.
- **Auto is offered only when the probed `--help` lists `--fit`.** Without the
  fitter, omitting `-c` makes llama.cpp allocate the model's trained context,
  which can exhaust a machine. This gate is the safety of the whole phase.
- **Auto omits the flag rather than passing a word for it.** The fitter adjusts
  unset arguments, and `auto` is already llama.cpp's default for `-ngl`. The
  shown command gets shorter, which reflects what the app is doing.
- **A never-launched model opens on Auto. A launched model keeps its number.**
  This follows the seeding rule in [knowledge/project.md](../knowledge/project.md).
- **Under Auto, the memory panel shows weights as a floor before launch.** At
  Ready it sizes the cache at the `n_ctx` the server reports, which the runner
  already reads from `/props`.
- **A bounded figure carries its reason.** "Some layers are not counted" and
  "no context has been chosen yet" are different facts and get different
  sentences.
- **Plan-building does not know what is running.** The frontend joins the plan
  and the runner state.

## What was built

1. `profile.rs` defaults `ctx` to 0 and `ngl` to `auto`, and `args()` omits
   each where that is what it holds. The probe decides whether Auto is offered.
   The context control gains an Auto position.
2. `estimate.rs` gains the second bounded reason. Before launch, Auto shows
   weights as a floor and names the context as the missing term. At Ready, the
   cache is priced at the server's own `n_ctx`.

## Acceptance

All met 2026-08-31.

- With `--fit` present and context on Auto, the argv carries no `-c`, and the
  shown command agrees with the argv.
- With `ngl` on auto, the argv carries no `-ngl`. With `all` or a number, it
  does.
- With `--fit` absent, Auto is not offered and nothing omits `-c`. A test
  covers this case because it is the one that can exhaust a machine.
- A remembered profile of 65,536 still launches with `-c 65536`.
- A config missing `ctx` deserializes to Auto. One carrying a number keeps it.
- Before launch under Auto, the weights show as a floor with a reason that names
  the context, worded differently from the uncounted-layers reason.
- At Ready under Auto, the cache figure is sized at the server's `n_ctx`.
- Ornith launched under Auto comes up with a larger context than the 65,536
  the author had been typing by hand.

## Verified

Verified 2026-08-31: all four checks passed. Launches against the real server
showed that omitting `-ngl` lets the fitter spill layers to the CPU, and that
Ornith with neither flag came up at 262,144 tokens. The author confirmed the
Auto wording, the shorter command, and both memory-panel states on screen.

Five defects were found on screen and fixed the same day: a raw `0` shown in
the profile panel, two plans shown at once, the wrong reason on the cache
floor, an unclamped 65,536 when Auto was switched off, and a hard-coded copy of
the built-in profile in the Settings screen. Rust now sends
`built_in_defaults` and the screen renders what it is told.

## Out of scope

Auto for cache types, parallel slots or batch sizes. `--fit-target` and
`--fit-ctx` as controls. A config schema change, which `ctx: 0` exists to
avoid.

## Risks

- The gate on `--fit` is the whole safety story. The test for the absent case
  matters more than any other in this phase.
- `--fit` floors at 4,096, so Auto can hand back far less context than expected
  on a loaded machine. Showing the server's own figure at Ready makes that
  visible.
- The change assumes no config on disk omits `ctx`. That is true of every
  profile the app has written, and a test guards it.
