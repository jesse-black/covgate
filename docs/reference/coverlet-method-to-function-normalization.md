# Coverlet method-to-function normalization in `covgate`

This document defines how `covgate` converts Coverlet-native method records into the public `functions` metric.

## Why this exists

Coverlet reports coverage at method granularity. `covgate` exposes one public cross-format callable metric named `functions`. To keep CLI and config stable across ecosystems, Coverlet methods are normalized into that shared function metric.

## Input shape (`coverlet_json`)

For each parsed method object, `covgate` reads:

- `Lines` map (`line_number -> hit_count`)
- `Branches` array (still used for branch metric, not function span shape)

Methods with invalid line keys are skipped by existing parser behavior. Methods with empty `Lines` do not produce function opportunities.

## Normalization algorithm

For each method:

1. Determine method span from line keys:
   - `start_line = min(Lines.keys)`
   - `end_line = max(Lines.keys)`
2. Determine covered-state:
   - covered if **any** line hit count is `> 0`
3. Emit one `OpportunityKind::Function` for that method span.

After method normalization within a file:

- `MetricKind::Function` per-file totals are computed from emitted function opportunities.
- `covered` is count of covered method-derived opportunities.
- `total` is count of method-derived opportunities.

## Important semantics

- Public vocabulary remains `functions` even when source data is methods.
- Function opportunities are diff-filtered later by the shared metric engine using span overlap with changed lines.
- Coverlet function normalization is independent from line/branch opportunity construction, except it reuses method line data as the span source.

## Edge cases

- Empty method `Lines`: method is ignored for function metric.
- Duplicate lines across methods: does not merge methods for function totals; each method remains one function opportunity.
- Non-object method/class payloads: ignored by parser.

## Source pointers

Implementation lives in:

- `src/coverage/coverlet_json.rs`

Related gate/model code:

- `src/model.rs`
- `src/metrics.rs`
- `src/gate.rs`
