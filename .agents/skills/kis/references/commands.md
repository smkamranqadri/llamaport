# Commands And Re-Anchoring

Commands are explicit entry points into the KIS loop. The re-anchor layer keeps KIS present when nobody types a command.

## Command Files

`commands/` holds host-neutral bodies, one per loop step, each with `description` and `argument-hint` frontmatter that Claude, Pi, and Codex all read.

| File | Step |
| --- | --- |
| `start.md` | LOAD: recover context, report current reality, propose a work mode. |
| `init.md` | Bootstrap `kis/` for a project that has none. |
| `plan.md` | QUESTION, CHALLENGE, STRUCTURE, PLAN: interview, then produce an approved scope. |
| `act.md` | ACT: execute the current task with proof. |
| `sync.md` | SYNCHRONIZE: write what changed into the right layer. |
| `check.md` | Maintenance: audit for contradictions, staleness, and bloat. |

Keep each command pointed at one step. A command that restates the whole skill becomes a second source of truth.

## Host Adapters

Install with `scripts/install-commands.sh --host project|claude|pi|codex|all`. The default is `project`, which covers Claude and Pi and writes nothing outside the project.

| Host | Adapter | Invocation |
| --- | --- | --- |
| Claude | `.claude/commands/kis` symlinked to `commands/` | `/kis:start` |
| Pi | `.pi/prompts/kis-*.md` symlinked per file, since Pi's prompt discovery is not recursive | `/kis-start` |
| Codex | `${CODEX_HOME:-~/.codex}/prompts/kis-*.md` copies | `/prompts:kis-start` |

Claude and Pi links follow package updates. Codex prompts are user-level, cannot be shared through a repository, and are copies, so re-run the installer after an update. OpenAI marks Codex custom prompts deprecated in favor of skills, so treat them as a convenience layer over the skill.

Pi discovers the skill itself from `.agents/skills/`, so `/skill:kis` works with no adapter. On any host, the command files are also plain instructions: an agent can be told to follow `.agents/skills/kis/commands/sync.md` directly.

## Re-Anchor Layer

Commands only fire when someone types one. Drift happens when a long session buries the skill and the agent stops loading State, stops interviewing, stops requiring proof, or stops synchronizing.

`scripts/install-anchor.sh` installs two counterweights:

- A Claude `SessionStart` hook running `hooks/session-anchor.sh` on startup, resume, clear, and compact. It prints the KIS rules, the command list, and the current State file, so every session and every post-compaction context starts from current operational reality.
- A marked KIS block in `AGENTS.md` and `CLAUDE.md`, between `<!-- kis:anchor:start -->` and `<!-- kis:anchor:end -->`. Pi and Codex load these files automatically and have no hook equivalent.

Both are idempotent. `--check` reports status, `--remove` reverses them, and existing settings, hooks, and instructions are preserved.
