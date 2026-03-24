# Close out Coverlet function-summary investigation with a documented raw-method decision

Save this completed ExecPlan in `docs/exec-plans/completed/covgate-coverlet-function-summary-investigation.md`.

Maintain this document in accordance with `docs/PLANS.md`.

## Purpose

Decide whether `covgate` should add downstream-style filtering to Coverlet method totals or continue consuming Coverlet and Cobertura method records as exported.

## Outcome

This work is complete.

The repository decision is:

- keep consuming Coverlet and Cobertura method records as exported
- do not add ReportGenerator-style filtering for compiler-generated lambda helper methods

## What we learned

- Line and branch totals were already stable for the investigated `.NET` path.
- The remaining mismatch was function-specific.
- In the real-world example:

      coverage.json method count:          82
      coverage.cobertura.xml <method>s:    82
      covgate function totals:             73 / 82
      ReportGenerator methods:             71 / 80

- That showed the drift was not introduced by `covgate` inventing methods and was not introduced by the JSON-to-Cobertura export dropping methods.
- ReportGenerator applies custom Cobertura-side filtering for generated lambda helper methods.
- Jenkins `coverage-model` appears much closer to raw Cobertura and does not appear to apply that same filtering.

## Decision log

- Decision: do not treat ReportGenerator as the default method-summary oracle for Coverlet.
  Rationale: it applies downstream, tool-specific filtering and does not appear to represent a broader consensus.
  Date/Author: 2026-03-24 / Codex

- Decision: keep raw exported method semantics in `covgate`.
  Rationale: the observed difference was small and additive, the raw data is easier to explain, and avoiding extra filtering keeps the parser simpler and more faithful to exported coverage.
  Date/Author: 2026-03-24 / Codex

## Artifacts

- Reference record: `docs/reference/coverlet-method-summary-semantics.md`
- Related parser: `src/coverage/coverlet_json.rs`

## Closeout

No code change is required from this investigation.

Future work should only reopen this area if the product explicitly wants compatibility with a specific downstream reporter such as ReportGenerator. If that happens, the compatibility target should be named explicitly rather than described as native Coverlet semantics.

Revision note: Simplified and closed after the repository chose raw Coverlet/Cobertura method semantics over downstream ReportGenerator-style filtering.
