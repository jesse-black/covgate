---
name: generator-execplan
description: Implement from an active plan or ExecPlan while keeping it up to date. Use when an agent needs to implement the active plan, continue plan-driven work, resume the current plan, or carry out an existing ExecPlan.
---

# Generator ExecPlan

Use this skill when the plan already exists and the task is to execute from it without losing the thread.

## Read first

MUST READ before implementing:

- `docs/PLANS.md`
- the active ExecPlan you are executing

`PLANS.md` defines the required ExecPlan structure, lifecycle, and maintenance rules.

## You own

As generator, you own:

- implementation from the current plan
- keeping the plan current while you work, not only at the end
- recording progress and requirement changes as they happen so the ExecPlan remains a live handoff artifact
- simplifying the solution after feedback changes the approach

You do not own:

- changing acceptance criteria
- redefining milestones or completion bars
- plan close-out (moving files to `completed/`) or completion review

## Role boundaries

Use the generator when the plan already exists and the task is to implement from it while keeping the living document current according to `PLANS.md`.

Update the ExecPlan as you go. Do not wait until the end of a long run to write progress back into the plan, and do not leave user-driven requirement changes only in chat context.

At every meaningful stopping point, leave the plan in a state that a fresh contributor can resume from after a crash, interruption, or handoff.

If requirements or success criteria change in a way that changes what the plan is asking for, stop and hand the plan back to `planner-execplan` before continuing.

When implementation is complete, you MUST check off the Generator items in the `Definition of Done` section and hand off to `evaluator-execplan` by spawning a subagent. If you cannot invoke subagents, STOP and ask the user to proceed with review. NEVER close the plan yourself.
