# Project

A macOS app for running local GGUF models under `llama-server`, and (not yet
built) downloading them from Hugging Face with working resume.

**For:** the author first, other local-LLM users on Apple Silicon second. Stage
is MVP heading for release, so packaging and other people's machines are real
scope, not hypothetical.

**Problem:** the `llama-server` launch command is stable except for three values
yet gets retyped or hunted out of shell history every time; nothing records what
context a model supports or what a given `-c` costs. Separately, `curl -L -C -`
against Hugging Face does not resume reliably and is slow on one connection,
so downloads currently go through an external download manager.

**In scope:** list models with real GGUF metadata, launch one, report what it
costs, stop it, say whether it works; download with resume.

**Out of scope:** a chat UI (`llama-server` ships one), non-macOS platforms,
managing the llama.cpp installation itself, saved profiles or presets, API keys,
binding anywhere but loopback.

## Durable decisions

Each is argued in the specs; this is the index, not a second copy.

- Report memory, never forecast a total — a forecast wrong by 2x gets believed.
- Build argv, never a shell string; display a shell-quoted rendering only.
- Probe `llama-server --help` for accepted flags; never assume a build's flags.
- Read GGUF headers directly, and walk the entire KV block.
- One model at a time; a busy port refuses the launch instead of moving.
- Find stray servers by scanning processes, not by reading a pidfile.
- No profile system: a model's form opens with its last **successful** launch.
- Loopback only, no authentication.

Detail: [runner spec](../../docs/runner-spec.md),
[downloader spec](../../docs/downloader-spec.md).
