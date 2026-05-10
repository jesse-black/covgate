---
description: "Design doc for token-efficient console output; read when working on compact output format, understanding the minimal display mode, or implementing agent-friendly coverage summaries."
---

# Design Doc: Token-Efficient Console Output

## Context & Design Goals

`covgate` is often used by AI agents (like Gemini CLI, Jules, or Codex Cloud) to validate code changes in isolated sandboxes. The console output is intentionally compact to minimize token usage in agent contexts.

- **Minimal token usage**: On a passing run, only rule outcomes are printed. Per-file details are omitted when every file passes.
- **Actionable precision**: On a failing run, uncovered spans include row and column numbers so agents can locate issues immediately.
- **Readability**: Output is grouped by filename on failure.

## Current Design

### 1. Compact Success Output (PASS)
When the quality gate passes, `covgate` prints only the rule outcomes.

```
PASS Lines: 100.00% (10/10) ≥ 90.00%
PASS Branches: 85.00% (17/20) ≥ 80.00%
PASS Functions: 0 uncovered ≤ 0
```
*Note: The per-file list (`file1.ts (100.00%)`) and the `Diff Coverage: PASS` header are intentionally dropped. When every file passes, listing them adds tokens without actionable information — agents only need to act on failures. PASS/FAIL leads each line so the verdict is visible without scanning across. Minimal console output is intentionally not table-aligned: fixed-width padding spends tokens, makes snapshots brittle, and encourages unlike rule families to share one visual column model. Percent rules render a percent plus covered/total counts. Uncovered-count rules render the observed uncovered count directly (for example, `0 uncovered`) and must not borrow the percent/count display from the corresponding changed metric.*

### 2. Focused Failure Output (FAIL)
When the gate fails, `covgate` only prints details for files that have uncovered changes. Context about the diff range is included.
```
Diff: origin/main...HEAD, staged and unstaged changes

web/src/features/chat/Chat.tsx (81.48% line, 84.48% branch)
  lines: 89, 90-92, 180
  branches: 52:10-52:15, 88:4-92:10
  functions: 171:15-171:30, 180:5-180:45

FAIL Lines: 81.48% (164/201) ≥ 90.00%
FAIL Branches: 75.58% (164/217) ≥ 80.00%
PASS Functions: 0 uncovered ≤ 0
```
*Note: Rulers are replaced with single blank lines. The `Diff Coverage: FAIL` header is omitted (same reasoning as PASS). All configured metrics are shown — passing and failing — so agents see the complete picture in one scan. The diff description string comes from `DiffSource::describe()` in `src/diff.rs` and is passed through unchanged.*


### 3. Row and Column Precision
`SourceSpan` includes `start_col` and `end_col`.
- **Why**: AI agents can jump directly to the exact expression or branch within a line. This is particularly useful for complex lines (e.g., `if (a && b) || c`) where line-level coverage is ambiguous. It is also **essential for function metrics**, as multiple lambdas or anonymous functions can exist on a single line; column precision allows unique identification of which function is uncovered.
- **Format support**:
    - **LLVM** and **Istanbul** both expose full column data (start/end) for all span types.
    - **Coverlet** does not expose source column positions — its JSON format only provides line numbers and IL byte offsets (`Offset`, `EndOffset`). Coverlet spans will always use the line-only fallback.
- **Formatting**:
    - Single point: `line:col`
    - Range on same line: `line:start_col-end_col`
    - Range across lines: `start_line:start_col-end_line:end_col`
    - If columns are unknown (Coverlet, or any parser that can't populate them): Fall back to `line` or `start_line-end_line`.

### 4. Console Output
Console output summarizes rule outcomes and, on failure, lists uncovered file details. Markdown is the format for detailed inspection.
