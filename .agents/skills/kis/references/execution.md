# KIS Execution

Use this for written plans, Standard Mode with meaningful risk, and Phase Mode.

## Setup

Before execution:

1. Confirm or create a feature branch when the repo uses branches.
2. Keep a short progress ledger in `kis/state/current.md`.
3. Pre-flight the plan for contradictions.
4. Raise blocking contradictions before starting.
5. Choose the smallest capable agent/model for each role.
6. Record the execution approach in State when useful.
7. Select specialist skills for the current phase.

## Plan Self-Review

Before coding, check:

- Does every requirement map to a task?
- Are commands exact?
- Are file paths clear where needed?
- Are acceptance checks clear?
- Are risky assumptions called out?
- Is the task small enough to execute safely?
- Can a fresh agent understand it without the full conversation?
- Does the plan avoid unrelated changes?
- Is the right specialist selected, if any?

If the plan is weak, improve it before implementation.

## Per-Task Loop

For each task or cohesive task group:

```text
PICK -> IMPLEMENT -> VERIFY/REVIEW -> RESOLVE -> INTEGRATION CHECK -> FINISH
```

### PICK

Select the next unfinished task from the plan. Keep it small enough to review.

### IMPLEMENT

Use a fresh implementer when available and useful. Otherwise implement directly only when the user accepted fallback risk or the work is small enough for Fast Mode.

The implementer should receive:

- assigned task
- relevant files
- interfaces it needs
- acceptance criteria
- commands to run
- Knowledge/Intent constraints
- specialist skill, if needed
- instruction not to modify `kis/`

### VERIFY / REVIEW

Use a separate reviewer when changes are non-trivial, risky, or visible to users.

The reviewer checks:

- diff against the plan/spec
- correctness
- missing edge cases
- code quality
- maintainability
- tests
- regressions

Tag findings:

```text
Critical
Important
Minor
```

Resolve Critical and Important findings or get explicit user acceptance of the risk.

### UI / UX Review

For visible UI, user flow, interaction, hierarchy, forms, navigation, or empty/error/loading states, evaluate the actual UI.

Acceptable proof:

- live preview inspected
- screenshots captured
- responsive/device checks where relevant
- accessibility basics checked

Text descriptions are not enough for visual UI acceptance.

### INTEGRATION CHECK

Verify behavior that unit tests cannot prove:

- UI rendering
- browser flow
- animation
- deployment behavior
- auth callback
- webhook/payment behavior
- mobile/responsive behavior
- generated files

### FINISH

Only when review and verification are clean:

- mark the task done in the plan
- append a short State ledger line when useful
- record proof
- synchronize KIS
- move to the next task

## After All Tasks

When all tasks are complete:

1. Run final review.
2. Run final verification commands.
3. Confirm acceptance criteria.
4. Clean stale State.
5. Update Intent if the plan outcome changed.
6. Update Knowledge only for stable new understanding.
7. Prepare a concise final summary.
