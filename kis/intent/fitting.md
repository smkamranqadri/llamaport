# Fitting

Planned 2026-08-31. The app names `-c` and `-ngl` on every launch, and both
overrule a llama.cpp default that is better than the value being passed.

Live status is in [state/current.md](../state/current.md), not here.

## What is actually wrong

**`--fit` is on by default and this app makes it inert.** It "adjusts unset
arguments to fit in device memory", defaults to `on`, and every argument the
launch fills in is one it may no longer size. Filling both in is not a
neutral act — it is switching the feature off, once per launch, silently.

It genuinely constrains rather than merely defaulting, which is the thing worth
measuring before building anything on it. Left alone, `llama-server` b10360 gave
`qwen2.5-0.5b` its whole 32,768-token context and `Qwen3.6-35B` its whole
262,144 on a 32 GB machine. Forced to reserve a 30 GiB margin with
`--fit-target`, it dropped the 0.5B to **4,096** — `--fit-ctx`'s floor. So the
fitter both grants and withholds ([knowledge/technical.md](../knowledge/technical.md)).

**`-ngl` is the same defect in the riskier field.** llama.cpp's own default is
`auto`; this app passes `all`. Where the weights do not fit, `auto` spills and
`all` insists.

The context number is also the one thing a user has to invent at every launch.
65,536 is a value the author picked once. Left to fit, llama.cpp chose 262,144
for the hybrid on this machine — four times more, and not a number anybody
would have guessed at.

## Decisions

- **Auto is stored as `ctx: 0`, and that is a borrowed vocabulary rather than an
  invented sentinel.** llama.cpp's own `--help` spells "loaded from model" as
  `0`. It costs no config schema bump and leaves every saved number alone. Say so
  in the code, so it is not later "fixed" into an `Option` and a schema 8.
- **Auto is gated on `--fit` being present in the probed `--help`, and this is
  the load-bearing decision.** Omitting `-c` on a build without the fitter does
  not fit anything — llama.cpp falls back to the model's *trained* context, which
  is 262,144 for the file on this disk, and allocates against it with nothing to
  stop it. Ungated, this phase is a way to exhaust a machine's memory. Gated, it
  is a convenience. If anything here is cut, cut everything else first.
- **Auto omits the flag rather than passing a word for it.** The fitter adjusts
  *unset* arguments, and `auto` is already llama.cpp's default for `-ngl`, so
  omitting is unambiguous where passing `-ngl auto` is a guess about how the
  fitter reads it. The shown command gets shorter, which is honest: the app
  really is saying less.
- **A never-launched model opens on Auto; a launched one keeps its number.** The
  existing rule stands unchanged — defaults seed a model nobody has launched and
  never overrule one that has ([knowledge/project.md](../knowledge/project.md)).
- **Auto gives up the pre-launch cache figure, and gets it back at Ready.**
  There is no context to size against until the server picks one. Before launch
  the panel shows weights as a marked floor; at Ready it sizes the cache at the
  `n_ctx` the server reports, which `runner.rs` already reads from `/props` and
  the screen already prints as a footnote. That footnote becomes the answer.
- **`bounded` will carry two reasons and needs two sentences.** "Some layers are
  not counted" and "no context has been chosen yet" are different facts, and one
  note for both would be a gentle lie.
- **The runner is not threaded into `build_plan`.** Plan-building knows nothing
  about what is running and should keep not knowing. The frontend holds both
  already; the seam belongs there or in an explicit context override.

## Parcel 1 — hand the flags back — DONE 2026-08-31

`profile.rs` defaults `ctx` to 0 and `ngl` to `auto`, and `args()` omits each
where that is what it holds. The probe decides whether Auto is offered at all.
`ProfileForm.tsx` grows an Auto position on the context control.

**Measure before relying on it:** that omitting `-ngl` is what the fitter means
by unset is assumed, not established. `-c` was measured; this was not. Settle it
the same way — a launch with the flag absent, read back from `/props` — before
the parcel closes rather than after.

## Parcel 2 — the panel under Auto — DONE 2026-08-31

`estimate.rs` gains the second bounded reason. Before launch, Auto shows weights
as a floor naming the context as what is missing. At Ready, the cache is sized
at the server's own `n_ctx` rather than at a profile field that holds 0.

## Proof — 2026-08-31

**The assumption the plan told itself to measure first, measured first.** Omitting
`-ngl` is what the fitter means by unset. Under a forced 31,500 MiB margin on the
0.5B, `-ngl` omitted put all 24 layers on **CPU** — the fitter spilling — while
`-ngl all` put all 24 on **MTL0**, the fitter overruled. So `all` was not a
neutral default; it was preventing the machine from being protected.

`--fit` also constrains rather than merely defaulting: the same forced margin
took that model's context from 32,768 to **4,096**, which is `--fit-ctx`'s floor.

**Parcel 1 proved by launching, which is the only thing that could.** Ornith run
with no `-c` and no `-ngl` — the argv the app now builds — came up at
**262,144** tokens, four times the 65,536 that had been typed by hand, at 4.8 GB
resident. An explicit `-c 8192` still yields 8,192, so the remembered path is
untouched.

The four commands green, **197 tests**, up from 191. Four mutations watched to
fail: pretending every build can fit (which is the dangerous one — it failed the
absent-fitter test), never omitting `-ngl`, defaulting back to a guessed number,
and pricing Auto like an ordinary context, which would put "plus 0 B of KV
cache" on the screen.

Three existing tests asserted the old default of 65,536 and `all`. They were
changed rather than worked around: each encoded the decision this phase
reverses, and one of them — that a field falls back within its own profile
rather than across to another — keeps its point intact and only changes the
value it falls back to.

**Seen on screen 2026-08-31, and looking is what finished it.** Under Auto the
context field reads "fitted to memory", the command names neither `-c` nor
`-ngl`, and the memory panel reads "≥ 644 MB to allocate — weights 644 MB, and
the cache is not counted here." Once Ready the same panel reads "848 MB to
allocate — weights 644 MB plus 204 MB of KV cache at 32,768 tokens", priced at
the context the server fitted, with no `≥` because at a known context it is
exact. That second plan fetch was the part most likely to fail and it did not.

**Four defects the suite could not reach, all found by looking:**

- `Current profile` printed the raw `0` that carries the Auto meaning.
- That panel read two plans at once — `Current profile` from the live form and
  the cache from the plan fetched at mount — so a cache priced at 32,768 sat
  beside a profile reading 0. Every figure on the screen now comes from one
  source.
- The cache stat said "≥ 0 MB" hinted "a floor — some layers are not counted",
  which is **the wrong reason**: on a dense 0.5B every layer is counted, and the
  real reason is that no context has been chosen. The decision above says in as
  many words that `bounded` carries two facts and needs two sentences; it was
  honoured in the memory panel and forgotten in the stat beside it. A wrong
  explanation beside a correct number is worse than no explanation, and no test
  can see it.
- Switching Auto off set a flat 65,536 without clamping to the model's maximum,
  so the form read 65,536 while the command under it read `-c 32768`. The slider
  clamps what it displays, which is what let the mismatch look settled.

A fifth was mine from further back: `SettingsScreen.tsx` kept its own hardcoded
copy of the built-in profile, so changing `Profile::default()` in Rust left the
Settings form still offering 65,536 and `all` — and saving anything there would
have frozen the old values into the config. The copy is deleted; Rust now sends
`built_in_defaults` and the screen renders what it is told.

## Out of scope

Everything else in [gaps.md](gaps.md). Auto for cache types, parallel slots or
batch sizes. `--fit-target` and `--fit-ctx` as controls — their defaults stand,
and the second is why Auto can hand back 4,096 rather than nothing. A config
schema bump, which `ctx: 0` exists to avoid.

## Acceptance

- `--fit` present and ctx Auto: argv carries no `-c`, and the shown command
  agrees with the argv.
- `ngl` auto: argv carries no `-ngl`. `all` or a number: it does, as today.
- **`--fit` absent: Auto is not offered and nothing omits `-c`.** Covered by a
  test, because this is the case that can exhaust a machine.
- A remembered profile of 65,536 still launches with `-c 65536`.
- A config missing `ctx` deserializes to Auto; one carrying a number keeps it.
- Before launch under Auto: weights shown as a floor, the reason naming the
  context, worded differently from the uncounted-layers reason.
- At Ready under Auto: the cache figure is sized at the server's `n_ctx`.
- Ornith launched under Auto comes up, and the context the server reports is
  larger than the 65,536 that had been typed by hand.

## Verification

The four commands, each status captured on its own line and never after a pipe.
Every new test gutted and watched to fail before it is trusted
([knowledge/technical.md](../knowledge/technical.md)).

A green suite finishes neither parcel. What `--fit` does is llama.cpp's
behaviour and not this app's, so the outcome is settled by launching: once under
Auto, reading back what the server chose, and once with an explicit number to
show the remembered path is untouched.

## Risks

- **The gate is the whole safety story.** Without `--fit`, omitting `-c` asks a
  262,144-token cache of a 32 GB machine. The test for the absent case matters
  more than any other test in this phase.
- That omitting a flag is what the fitter means by unset is measured for `-c`
  and assumed for `-ngl`. Parcel 1 measures it.
- `--fit` floors at 4,096, so Auto can hand back far less context than expected
  on a loaded machine. Showing the server's own figure at Ready is what makes
  that visible instead of mysterious.
- Assumes no config on disk omits `ctx`. True of every profile this app has
  written; the test is the guard rather than the assumption.
