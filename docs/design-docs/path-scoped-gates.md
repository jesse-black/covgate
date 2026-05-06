# Design Doc: Path-Scoped Gates for File-Type Policy

## Context & Problem Statement

`covgate` currently applies each configured gate across the entire changed diff for a metric family. That is a good default when one threshold fits the whole repository, but JavaScript and TypeScript repositories often want different expectations for different file types.

A common policy split is:

- `*.ts` / `*.js` logic files should meet stricter gates because they contain business logic that is practical to exercise directly.
- `*.tsx` / `*.jsx` UI files may need a more relaxed gate because render wrappers, presentational branches, and event glue are often noisier to cover completely.

What is missing is a way to scope existing gate rules to subsets of files.

---

## Goals

- Let repositories configure different gates for different file sets, starting with JS/TS logic vs. TSX/JSX UI.
- Keep the existing metric model unchanged: `regions`, `lines`, `branches`, `functions`, and `named-functions` remain the only metric kinds.
- Reuse the existing diff-focused changed-opportunity model. A scoped gate should still evaluate only changed coverage opportunities, just within a narrower file set.
- Keep parser behavior format-agnostic. File-type policy should live in config and gate evaluation, not in LLVM/Coverlet/Istanbul adapters.

---

## Non-Goals

- Adding hardcoded language-specific categories such as “React mode” or “UI metric.”
- Inferring whether a file is “logic” or “UI” from AST or framework semantics.
- Requiring CLI parity for scoped gates in the first implementation slice.
- Preserving backward compatibility with the existing top-level `[gates]` table.

---

## Proposed Configuration Surface

Replace the current top-level `[gates]` table with a flattened `[[gates]]` array-of-tables in `covgate.toml`.

Each `[[gates]]` entry is one gate policy:

- if it has `include`, it is a scoped gate
- if it omits `include`, it is the fallback gate for unmatched changed files

This makes the old global gate just another gate entry instead of a separate config concept.

Example:

```toml
base = "origin/main"

[[gates]]
name = "js-logic"
include = ["**/*.js", "**/*.ts"]
exclude = ["**/*.jsx", "**/*.tsx", "**/*.d.ts", "**/*.test.*", "**/*.spec.*"]
fail-under-lines = 95
fail-under-branches = 90
fail-under-functions = 100
fail-under-named-functions = 100

[[gates]]
name = "js-ui"
include = ["**/*.jsx", "**/*.tsx"]
exclude = ["**/*.stories.*"]
fail-under-lines = 80
fail-under-branches = 70
fail-under-functions = 90

[[gates]]
fail-under-lines = 90
```

### Config Rules

- `name` is optional. If present, it must be unique.
- `include` is optional. When present, it contains one or more gitignore-style patterns and makes the entry scoped.
- `exclude` is optional and removes paths from the scope after `include` matching.
- `exclude` is invalid without `include`.
- The coverage rule keys live directly on each `[[gates]]` entry.
- At most one entry may omit `include`; that entry is the fallback gate.
- `[[gates]]` are TOML-only in the first slice. Existing CLI flags continue to configure one repository-wide fallback gate.
---

## Matching Semantics

- `include` and `exclude` use gitignore-style matching through the `ignore` crate.
- Patterns match normalized repo-relative paths, the same shape `covgate` already uses internally after coverage path normalization. Example: `src/profileCard.tsx`.
- Scope matching should also respect repository ignore rules through the same `ignore` crate machinery, so ignored files do not match scoped gates unless a gate pattern explicitly re-includes them.
- A scoped gate participates only when at least one changed file in the current diff matches it.
- Rules inside a scoped gate evaluate only coverage opportunities whose file path matches that gate.
- A changed file may match at most one scoped gate. If a file matches multiple scoped gates, configuration loading should fail with an error that names the overlapping gates and the file path.
- If a fallback gate exists, it applies only to changed files that match no scoped gate.
- If no fallback gate exists, `covgate` should fail when a changed file with supported coverage opportunities matches no scoped gate. This avoids accidentally leaving changed code ungated.

---

## Evaluation Model

1. One target per scoped `[[gates]]` entry whose matcher selects changed files.
2. The fallback `[[gates]]` entry, if present and if unmatched changed files remain.

Each target reuses the existing gate flow:

1. Filter changed files and coverage opportunities to the target's path set.
2. Compute the configured metric(s) for that subset.
3. Evaluate the existing `GateRule` values against those computed metrics.
4. Mark the overall run as passing only if every participating target passes.

Important behavior:

- Percent rules keep the current zero-total semantics: if a participating scope has zero changed opportunities for a configured metric, the observed percent is `100.0`.
- Uncovered-count rules keep the current zero-total semantics: zero uncovered opportunities passes a `<= N` rule.
- Unsupported metrics remain errors. For example, a scope that configures `fail-under-regions` against an Istanbul-only report should fail with a scope-aware version of the current “metric not supported by the loaded report” error.

---

## Output Behavior

Console and Markdown output should identify which gate a rule belongs to by prefixing each rule or section with a gate label.

If `name` is present, use it. If `name` is absent, derive a stable label:

- scoped gate: join the `include` patterns, or render as `gate #N` if that is too long
- fallback gate: render as `default`

Example minimal output:

```text
PASS  [js-logic] Lines:      100.00% (12/12) >= 95.00%
PASS  [js-logic] Branches:    90.00% (9/10)  >= 90.00%
FAIL  [js-ui]    Lines:       75.00% (3/4)   >= 80.00%
PASS  [js-ui]    Branches:   100.00% (2/2)   >= 70.00%
```

Unnamed fallback gate when multiple gates participate:

```text
PASS  [default] Lines:      95.00% (19/20) >= 90.00%
PASS  [default] Branches:   80.00% (8/10)  >= 80.00%
```

Unnamed fallback gate when it is the only participating gate:

```text
PASS  Lines:      95.00% (19/20) >= 90.00%
PASS  Branches:   80.00% (8/10)  >= 80.00%
```

In verbose mode, group output by gate.

Markdown should keep the existing table-based structure.

When multiple gates participate, add a `Gate` column to the existing Markdown tables:

```md
### Diff Coverage

| Gate | Result | Rule | Observed | Configured |
| --- | --- | --- | ---: | ---: |
| `js-logic` | ✅PASS | `fail-under-lines` | 100.00% | ≥ 95.00% |
| `js-logic` | ✅PASS | `fail-under-branches` | 90.00% | ≥ 90.00% |
| `js-ui` | ❌FAIL | `fail-under-lines` | 75.00% | ≥ 80.00% |
| `js-ui` | ✅PASS | `fail-under-branches` | 100.00% | ≥ 70.00% |
```

Per-metric tables should follow the same pattern:

```md
#### Line

| Gate | File | Covered Changed Lines | Changed Lines | Coverage | Missed Changed Spans |
| --- | --- | ---: | ---: | ---: | --- |
| `js-logic` | `src/math.ts` | 12 | 12 | 100.00% 🟢 |  |
| `js-ui` | `src/profileCard.tsx` | 3 | 4 | 75.00% 🟡 | `42-44` |
| **js-logic Total** |  | **12** | **12** | **100.00% 🟢** |  |
| **js-ui Total** |  | **3** | **4** | **75.00% 🟡** |  |
```

If an unnamed fallback gate participates alongside named gates, render it as `default` in the `Gate` column.

If the unnamed fallback gate is the only participating gate, keep the current Markdown shape with no `Gate` column.

---

## Implementation Notes

- Parse `[[gates]]` entries in `src/config.rs` and compile their matchers with the `ignore` crate.
- Merge CLI gate flags into the fallback gate only. Scoped gates come from config.
- Restrict metric computation and per-file totals to the files matched by each gate.
- Evaluate each gate independently and aggregate the results into one overall pass/fail.
- Update console and Markdown output to label results by gate.

---

## Testing Plan

- Config parsing tests for valid `[[gates]]` TOML.
- Config validation tests for duplicate explicit names, invalid `exclude` without `include`, multiple fallback gates, and overlapping scopes.
- Integration tests using Vitest fixtures that mix `*.ts` and `*.tsx` changes in one diff.
- Regression test for the “unmatched changed file with no fallback gate” error path.
- Rendering tests that assert scope names appear in console and Markdown output.

The existing `vitest/tsx-line-summary` fixture is a good starting point for the UI side, but this feature will probably need a mixed fixture that changes both a logic file and a UI file in the same run.

---

## Summary

Move to a single flattened `[[gates]]` model in `covgate.toml` and evaluate each gate against its matching file set. That covers the immediate JS/TS need for stricter `ts/js` logic gates and more relaxed `tsx/jsx` UI gates without changing the coverage model, parser contracts, or metric definitions.
