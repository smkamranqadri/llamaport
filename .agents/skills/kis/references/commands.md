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

Install with `scripts/install-commands.sh --host project|claude|pi|opencode|codex|all`. The default is `project`, which covers Claude and Pi and writes nothing outside the project.

| Host | Adapter | Invocation |
| --- | --- | --- |
| Claude | `.claude/commands/kis` symlinked to `commands/` | `/kis:start` |
| Pi | `.pi/prompts/kis-*.md` symlinked per file, since Pi's prompt discovery is not recursive | `/kis-start` |
| OpenCode | `.opencode/command/kis-*.md` symlinked per file, since OpenCode command names come from the filename | `/kis-start` |
| Codex | `${CODEX_HOME:-~/.codex}/prompts/kis-*.md` copies | `/prompts:kis-start` |
| Antigravity | none: it supports rules, skills, plugins, hooks, and MCP, but not custom slash commands | name the step, or point it at the command file |

Claude, Pi, and OpenCode links follow package updates. Codex prompts are user-level, cannot be shared through a repository, and are copies, so re-run the installer after an update. OpenAI marks Codex custom prompts deprecated in favor of skills, so treat them as a convenience layer over the skill.

Pi, OpenCode, and Antigravity discover the skill itself from `.agents/skills/`, so no adapter is needed to reach it. Claude does not, which is why the installer also links `.claude/skills/kis`. On any host, the command files are also plain instructions: an agent can be told to follow `.agents/skills/kis/commands/sync.md` directly.

In Claude the skill and the commands are separate entries with the same name. `/kis` runs the skill, which is the whole loop and the path the model takes when it triggers KIS on its own. `/kis:start` and its siblings enter one step. Both are expected; neither replaces the other.

## Re-Anchor Layer

Commands only fire when someone types one. Drift happens when a long session buries the skill and the agent stops loading State, stops interviewing, stops requiring proof, or stops synchronizing.

`scripts/install-anchor.sh` installs three counterweights:

- A Claude `SessionStart` hook running `hooks/session-anchor.sh` on startup, resume, clear, and compact. It prints the KIS rules, the command list, and a short State summary: Branch, Task, Blocker and Next, with indented continuation lines joined and the value cut at `KIS_ANCHOR_MAX_FIELD_CHARS` (default 200), beside a pointer to the file, all within the `KIS_ANCHOR_MAX_LINES` line budget (default 20). Blocker and Next fall back to the first `## Open` and `## Next` item when their bullet labels are absent, so every session and every post-compaction context starts from current operational reality without re-reading the whole file.
- An Antigravity `PreInvocation` hook in `.agents/hooks.json` running `hooks/antigravity-anchor.sh`, which wraps the same anchor text in an `ephemeralMessage`. Antigravity has no session-start event, so the hook fires before every model call and a marker keyed on `conversationId` holds the injection to once per conversation. Antigravity runs lifecycle hooks in an interactive session only, not under `agy --print`.
- A marked KIS block in `AGENTS.md` and `CLAUDE.md`, between `<!-- kis:anchor:start -->` and `<!-- kis:anchor:end -->`. Pi, OpenCode, Codex, and Antigravity all load these files automatically.

Both are idempotent. `--check` reports status, `--remove` reverses them, and existing settings, hooks, and instructions are preserved.
