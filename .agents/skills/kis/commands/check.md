---
description: Audit KIS for contradictions, staleness, and bloat, then propose a cleanup.
argument-hint: [optional area to audit]
---

# KIS Check

Audit project memory. Report first, change nothing until the user approves.

Full instructions: `.agents/skills/kis/SKILL.md`. Cleanup rules: `.agents/skills/kis/references/file-hygiene.md`.

## Checks

1. Structure: does `kis/` hold `knowledge/`, `intent/`, and `state/`, and is every file in the right layer?
2. Truth: does State contradict Intent, Knowledge, the repository, or the current branch?
3. Duplication: is any fact stored in more than one place?
4. Staleness: resolved blockers, completed ledgers, superseded plans, obsolete next actions, dead file or command references.
5. Recovery: can State answer branch, task, command, blocker, and next without reading anything else?
6. Size: is `kis/state/current.md` past roughly 80 lines, or is any file hard to skim?
7. Proof: is anything marked done without recorded proof?

## Report

```text
Health: good | drifting | stale

Contradictions:
- <file> - <claim> vs <source of truth>

Misplaced facts:
- <fact> - in <layer>, belongs in <layer>

Stale content:
- <file> - <what to prune>

Missing:
- <what a fresh agent could not recover>

Proposed cleanup:
- <smallest set of edits>
```

Apply the cleanup only after the user approves. Then re-report the file sizes so the trend is visible.
