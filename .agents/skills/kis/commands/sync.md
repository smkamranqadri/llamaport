---
description: Synchronize KIS after work by classifying what changed and updating only the affected layers.
argument-hint: [optional summary of what changed]
---

# KIS Sync

Run the SYNCHRONIZE step of the KIS loop. Update memory only. Do not implement anything in this command.

Full instructions: `.agents/skills/kis/SKILL.md`. File rules: `.agents/skills/kis/references/file-hygiene.md`.

## Steps

1. Establish what actually changed since the last sync. Use `$ARGUMENTS`, the session's changed files, and `git status` / `git diff` rather than memory.
2. Ask the three questions:

```text
Did Knowledge change?  new stable fact, rule, constraint, or durable decision
Did Intent change?     goal, plan, acceptance criteria, or approved direction
Did State change?      branch, task, status, blocker, command, proof, next action
```

3. Write each fact into exactly one layer. Prefer updating an existing file over creating one.
4. Rewrite State in place. Do not append new truth under old truth.
5. Move stable facts out of State into Knowledge, and plans out of State into Intent.
6. Mark superseded plans `SUPERSEDED - see <new plan>`.
7. Grep `kis/` for terms that changed and remove stale references and contradictions.
8. Record proof in State when it helps recovery. Do not paste implementation history into KIS.

## Report

```text
Updated:
- <file> - <what changed>

Unchanged layers:
- <layer> - no change

Contradictions fixed:
- ...
```

If nothing changed, say so and write nothing.
