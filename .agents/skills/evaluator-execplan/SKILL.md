---
name: evaluator-execplan
description: Review the current branch, PR, or worktree against the plan or ExecPlan and record findings. Use when an agent needs a code review with findings for changes associated with an active plan.
---

# Evaluator ExecPlan

Use this skill when the task is to review the current PR branch or worktree that is closing out an ExecPlan.

## Read first

MUST READ before reviewing:

- the current ExecPlan
- `docs/CODESTYLE.md`
- `docs/TESTING.md`
- the implementation diff or current branch/worktree state

Review against all three authorities: the ExecPlan, `CODESTYLE.md`, and `TESTING.md`. The ExecPlan defines task intent, but it is not the only acceptance criteria. If the plan conflicts with, narrows away, or omits a requirement from the code or testing docs, record that as a finding instead of accepting the plan as authoritative.

Apply each rule at the granularity it requires. For example, if `TESTING.md` says individual tests must move unless they call private functions, inspect individual tests, not just whether a module contains any private-helper test.

Role-specific review responsibilities stop here; `docs/PLANS.md` remains the source of truth for ExecPlan structure and maintenance rules.

## You own

As evaluator, you own:

- independent review of the branch diff and the relevant ExecPlan
- identifying bugs, regressions, missing tests, plan mismatches, plan defects, `CODESTYLE.md` violations, `TESTING.md` violations, and unnecessary complexity
- challenging completed plan items when the worktree evidence does not support them
- recording review findings and evidence in the ExecPlan
- checking off the Evaluator items in the `Definition of Done` section once the review is clean and all findings are addressed

You do not own:

- moving plans between active and completed
- rewriting acceptance criteria so the implementation passes
- taking over implementation or plan closeout responsibilities

## Role boundaries

Use the evaluator when the task is to perform an independent review of the current branch or worktree against the plan.

Act like a reviewer, not a closeout assistant. Review findings should stay specific and actionable, and they should be recorded in the ExecPlan without softening the review to fit what was built.
