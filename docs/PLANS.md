---
description: "ExecPlan authoring contract to create, write, revise, review, and validate repository execution plans; read when shaping plan requirements, required sections, formatting rules, or self-contained planning expectations."
---

# Agent Execution Plans

This document defines the minimum required format for an execution plan ("ExecPlan"). An ExecPlan is a compact, executable specification for a task.

Assume the reader has only:
- the current working tree
- this document
- the active ExecPlan file

Do not assume memory of prior work.

## Locations

Store ExecPlans in:
- Active: `docs/exec-plans/active/`
- Completed: `docs/exec-plans/completed/`

Name ExecPlan files `NNNN-<plan-title>.md`, where `NNNN` is the next zero-padded sequence number and `<plan-title>` is a concise kebab-case slug. Keep the same filename when moving a plan from `active/` to `completed/`.

To print the next sequence number:

```sh
./scripts/next-plan-number.sh
```

## Rules

- Prefer bullets over prose.
- Keep plans compact and concrete.
- Use exact file paths and commands when known.
- Validation commands must be copy-paste runnable; for `cargo test`, use one positional filter per command.
- Do not repeat unchanged content when updating a plan.
- Do not add background or narrative unless required for execution.
- Do not restate repository context already obvious from the code or file paths.
- Use append-only bullets for discoveries and review notes.

## Template

```md
---
description: "<plan-specific routing cue; read when implementing, reviewing, or continuing this task>"
---

# <Short task title>

## Goal
- <desired end state>

## Scope
- In: <what is included>
- Out: <what is excluded>

## Relevant Areas
- `<path-or-component>` — <why it matters>

## Open Questions
- <unknown that may affect implementation>
- None yet

## Steps
- [ ] <concrete step>
- [ ] <concrete step>

## Validation
- `<command>`
- <manual check>

## Discoveries
- <append-only discovery>
- None yet

## Review
- [ ] <append-only review note>
- [ ] None yet
```

## Section Guidance

- `Goal`: state the end state only.
- `Scope`: define boundaries briefly.
- `Relevant Areas`: list only files, modules, or systems likely to matter.
- `Open Questions`: include only unresolved items that could affect implementation. Use `None yet` if there are none.
- `Steps`: use actionable checkboxes.
- `Validation`: list exact commands and checks.
- `Discoveries`: append-only findings that affect implementation, scope, or validation. Use `None yet` if there are none.
- `Review`: append-only review findings. Use `None yet` if there are none.

## Update Rules

When updating an ExecPlan:
- change only the sections that changed
- mark completed steps with `[x]`
- add discoveries only to `Discoveries`
- add review notes only to `Review`

## Principle

The plan should contain only the detail needed for a fresh agent to execute correctly.
