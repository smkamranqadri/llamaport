---
description: Bootstrap the smallest useful KIS memory for this project and stop before implementation.
argument-hint: [optional project description]
---

# KIS Init

Run the KIS bootstrap flow. Stop before implementation.

Read `.agents/skills/kis/references/bootstrap.md` and follow it.

## Steps

1. Check whether `kis/` already exists.
   - If it exists and is healthy, stop and tell the user to run the start command instead.
   - If it exists but is incomplete, repair only the missing parts.
2. Decide whether this is a new project or an existing codebase.
   - New project: interview the user through what, who, problem, success, constraints, and stage. One question at a time, each with your recommended answer. Skip anything `$ARGUMENTS` or the repository already answers.
   - Existing codebase: inspect the repository, derive what you can, and interview only on what inspection cannot answer.
3. Challenge weak or contradictory answers before writing long-lived memory.
4. Create only the useful parts of:

```text
kis/
  knowledge/
  intent/
  state/
```

5. Write the smallest useful files. Do not create empty files or placeholder templates.
6. Put each fact in exactly one layer.

## Report

List the files created, one line each, with what each one owns. Then propose the first work mode and first task, and wait for approval.
