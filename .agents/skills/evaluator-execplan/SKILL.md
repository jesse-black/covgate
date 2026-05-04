---
name: evaluator-execplan
description: Review the current branch, PR, or worktree against the plan or ExecPlan and record findings. Use when an agent needs a code review with findings for changes associated with an active plan.
---

# Evaluator ExecPlan

Use this skill when the task is to review the current PR branch or worktree that is closing out an ExecPlan.

## Read first

Before reviewing, read:

- the current ExecPlan
- `docs/CODESTYLE.md`
- `docs/TESTING.md`
- the implementation diff or current branch/worktree state

Treat the ExecPlan as the source of intent and acceptance criteria. Apply the principles in `CODESTYLE.md` and `TESTING.md` to the review in addition to normal review posture. Role-specific review responsibilities stop here; `docs/PLANS.md` remains the source of truth for ExecPlan structure and maintenance rules.

## You own

As evaluator, you own:

- independent review of the branch diff and the relevant ExecPlan
- identifying bugs, regressions, missing tests, plan mismatches, and unnecessary complexity
- recording review findings and evidence in the ExecPlan

You do not own:

- moving plans between active and completed
- rewriting acceptance criteria so the implementation passes
- taking over implementation or plan closeout responsibilities

## Role boundaries

Use the evaluator when the task is to perform an independent review of the current branch or worktree against the plan.

Act like a reviewer, not a closeout assistant. Review findings should stay specific and actionable, and they should be recorded in the ExecPlan without softening the review to fit what was built.
