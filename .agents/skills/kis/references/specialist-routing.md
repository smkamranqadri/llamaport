# Specialist Routing

Use specialists when they reduce risk, improve quality, or provide a focused expert lens. Do not use them for tiny obvious work.

Specialists do not own project memory. They must not edit `kis/`. They may report KIS-relevant updates for the orchestrator to classify and write.

## Availability Rule

If a matched specialist skill or subagent tool is unavailable:

1. Say what is unavailable.
2. If installation or enablement is supported and the work is Standard/Phase risk, ask for approval or use the available install flow.
3. If unavailable tooling blocks the ideal path, create a checkpoint.
4. Continue only when the user enables the tool, reduces scope, switches to Fast Mode, or explicitly accepts fallback risk.

Do not silently pretend an unavailable specialist ran. Do not deadlock when a safe fallback is accepted.

## Common Routing

```text
Requirements or plan need structure? -> writing-plans, or a PRD/issue-planning skill
Domain or terminology unclear?       -> domain-modeling or grilling skill
Architecture or deep module design?  -> codebase-design or system-design skill
Architecture diagram?                -> Figma/FigJam diagram skill or Mermaid, depending on the requested output
Behavior changes?                    -> test-driven-development
Hard bug or regression?              -> systematic-debugging
End-to-end or browser test?          -> browser or Playwright skill/tooling
Explore a design fast?               -> prototype skill, or Fast Mode with accepted fallback risk
Auth, login, or accounts?            -> auth skill if installed; map session and security risks before implementing
Code or security review?             -> code-review, requesting-code-review, or a security scan skill
Hand off to another session?         -> thread/handoff tool
```

Use the skill names exposed in the current environment, whichever host you are running in. Discover what is installed through the session's own skill, agent, command, and tool surfaces rather than assuming a fixed set. If a routing example names a capability but the exact skill is unavailable, search installed skills and tools first, then use the host's install flow when supported.

Skip a matched skill only with a one-line reason.

## Agent Selection

Use one specialist when:

- task is small
- risk is low
- changes are localized
- orchestrator can review directly

Use implementer + reviewer when:

- code changes are non-trivial
- visible UI changes
- user behavior changes
- architecture/data flow changes
- regression risk exists

Use multiple agents when:

- feature spans frontend and backend
- mobile and web both change
- architecture is uncertain
- plan needs challenge before execution
- independent review would catch meaningful risk

## Dispatch Brief

Before launching a specialist, prepare:

```text
Role:
Skill:
Task:
Relevant KIS context:
Relevant files:
Constraints:
Acceptance criteria:
Commands to run:
Expected proof:
KIS boundary:
```

Required KIS boundary:

```text
Do not modify kis/.

Report any KIS-relevant updates under this heading:

KIS-relevant updates:
- Knowledge:
- Intent:
- State:

Only the KIS orchestrator updates KIS.
```

## Report Handling

Treat specialist reports as claims. Verify:

- changed files actually changed
- commands were actually run where possible
- test output is credible
- implementation matches the task
- no unrelated changes were made
- `kis/` was not modified by the specialist
- KIS updates are routed to the right layer

Preferred report:

```text
Summary:
- ...

Changed files:
- ...

Commands run:
- ...

Proof:
- ...

Issues / risks:
- ...

KIS-relevant updates:
- Knowledge:
- Intent:
- State:
```

If no KIS update is needed:

```text
KIS-relevant updates:
- none
```

## Install Notes

When the environment supports skill installation, prefer the host's supported install mechanism over raw shell commands, unless the user explicitly asks for a shell-based installer.

Before installing, confirm the exact skill/plugin exists and that the install target matches the current environment. If install support is unavailable, checkpoint and ask whether to reduce scope, switch to Fast Mode, or proceed with accepted fallback risk.
