---
name: planner-execplan
description: Create, update, revise, reopen, or rescope a plan or ExecPlan using `PLANS.md`. Use when an agent needs to create a plan, plan a change, update the plan with new requirements, or rewrite an active ExecPlan.
---

# Planner ExecPlan

Use this skill when the task is to shape the plan itself rather than to implement from it.

## Read first

MUST READ before drafting or substantially revising any ExecPlan:

- `docs/PLANS.md`

`PLANS.md` defines the required ExecPlan structure, lifecycle, and maintenance rules.

Then read the current ExecPlan, if one already exists, plus the nearby design docs and code context needed to make the plan self-contained.

## Required deliverable

The planner's final deliverable is a repository-local ExecPlan Markdown file, not only a chat summary.

When creating a new ExecPlan, write it to the repo's active ExecPlan location before ending the planner phase. If the repo follows the standard layout, use `docs/exec-plans/active/<descriptive-name>.md`.

Do not hand off to `generator-execplan` until the ExecPlan file exists in the working tree and contains the decision-complete plan.

In your planner response, explicitly name the ExecPlan path you created or updated so the next phase can pick up the right artifact.

## You own

As planner, you own:

- plan structure
- acceptance criteria
- major scope and sequencing decisions
- revising the plan when requirements change
- converting human conversation into updated plan requirements

Once implementation has started, only the planner may substantially rewrite those parts of the ExecPlan.

You do not own:

- implementation from the current plan
- review findings on the current branch
- declaring the work complete just because implementation seems close

## Role boundaries

Use the planner when the task is to create, rewrite, reopen, or rescope the plan itself.

Do not use the planner to continue implementation from an existing plan; use `generator-execplan` for that. Do not use the planner to perform completion review or close-out validation; use `evaluator-execplan` for that.

When requirements or success criteria change in a way that affects what "done" means, hand the plan back to the planner and revise the ExecPlan file before more implementation continues.

The planner's job is to make the plan decision-complete, persist it as the active ExecPlan artifact, and leave it ready for implementation and later evaluation, following `PLANS.md`.
