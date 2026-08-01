---
description: Execute the current KIS task with the per-task loop and require proof before done.
argument-hint: [optional task id or scope]
---

# KIS Act

Run the ACT step of the KIS loop for `$ARGUMENTS`, or for the current task in State when no argument is given.

Full instructions: `.agents/skills/kis/SKILL.md`. Execution detail: `.agents/skills/kis/references/execution.md`.

## Before Implementing

1. Read `kis/state/*` and the approved plan in `kis/intent/*`.
2. If there is no approved scope for this work, stop and run the plan command first.
3. Pre-flight the plan for contradictions. Raise blocking ones before starting.
4. Update State with the current task, work mode, status, and verification plan.

## Per-Task Loop

```text
PICK -> IMPLEMENT -> VERIFY/REVIEW -> RESOLVE -> INTEGRATION CHECK -> FINISH
```

- Implement only the current scope. No unrelated changes.
- Keep each change small enough to review.
- Use a specialist when it reduces real risk and one is available. If a matched specialist is unavailable, say so and checkpoint instead of pretending it ran.
- For visible UI, inspect the actual UI. A text description is not acceptance.

## Proof Before Done

Never mark a task done from intuition, code reading, or an agent report. Record what actually ran, in the Proof shape from `SKILL.md`.

If verification fails, record the failure in State as the current blocker rather than marking the task done.

## Finish

Only when verification is clean:

- mark the task done in the plan
- record proof in State
- rewrite `Now`, `Next`, and `Status` in State
- report the changed files, commands run, and proof
- name the next task

Then run the sync command, or synchronize the changed KIS layers inline.
