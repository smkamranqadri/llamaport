---
description: Question, challenge, structure, and plan a task against KIS before any code is written.
argument-hint: [task or goal]
---

# KIS Plan

Run the QUESTION, CHALLENGE, STRUCTURE, and PLAN steps of the KIS loop for `$ARGUMENTS`. Do not implement anything in this command.

Full instructions: `.agents/skills/kis/SKILL.md`. Specialist routing: `.agents/skills/kis/references/specialist-routing.md`.

## Steps

1. Load the KIS context this task needs: State first, then relevant Intent, then only required Knowledge.
2. QUESTION: interview the user until you reach a shared understanding. One question at a time, waiting for each answer. Walk down each branch of the decision tree, resolving dependencies one by one. Give your recommended answer with every question. Look up any fact that KIS, the repository, or a tool can answer instead of asking it. Do not continue to the plan until the user confirms the understanding is shared.
3. CHALLENGE: name weak requirements, risky assumptions, missing edge cases, scope creep, and approaches heavier than the goal needs. Give the practical tradeoff and a clearer path.
4. STRUCTURE: classify every new fact into exactly one layer.

```text
Knowledge = what is true / how things work
Intent    = what we want / plan to do
State     = what is happening now
```

5. PLAN only enough to execute safely.

## Output

```text
Scope:
Out of scope:
Files likely involved:
Acceptance checks:
Risks and assumptions:
Verification method:
Specialist or review needed:
Work mode: Fast | Standard | Phase
KIS writes this plan implies:
- Knowledge:
- Intent:
- State:
```

## Stop Condition

For Fast Mode, offer to execute immediately. For Standard or Phase Mode, write the plan into `kis/intent/` only after the user approves the scope, then stop.
