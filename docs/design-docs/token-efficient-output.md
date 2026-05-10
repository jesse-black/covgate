# Design Doc: Token-Efficient Console Output

## Context & Problem Statement
`covgate` is often used by AI agents (like Gemini CLI, Jules, or Codex Cloud) to validate code changes in isolated sandboxes. Standard coverage output can be verbose, listing every file and metric even when they are 100% covered. This consumes unnecessary context tokens in the agent's window, leading to higher costs and potentially pushing important history out of context.

## Goals
- **Minimize Token Usage**: Reduce the output size significantly, especially in successful "PASS" scenarios.
- **Actionable Precision**: Provide precise row and column numbers for uncovered spans so agents can immediately locate and fix issues.
- **Improved Readability**: Group output by filename rather than metric for a more intuitive, tree-like structure.

## Proposed Design

### 1. Minimal Success Output (PASS)
When the quality gate passes, `covgate` should print only a minimal summary. Dash-based rulers are replaced with blank lines, and redundant headers are removed.

**Current (Verbose):**
```
-------------
Diff Coverage: PASS
Diff: origin/HEAD...HEAD, staged and unstaged changes
-------------
file1.ts (100.00%) [line]
file2.ts (100.00%) [line]
-------------
Line Coverage: 100.00%
Rule fail-under-lines: PASS (100.00% ≥ 90.00%)
-------------
```

**Proposed (Minimal):**
```
PASS Lines: 100.00% (10/10) ≥ 90.00%
PASS Branches: 85.00% (17/20) ≥ 80.00%
PASS Functions: 0 uncovered ≤ 0
```
*Note: The per-file list (`file1.ts (100.00%)`) and the `Diff Coverage: PASS` header are intentionally dropped. When every file passes, listing them adds tokens without actionable information — agents only need to act on failures. PASS/FAIL leads each line so the verdict is visible without scanning across. Minimal console output is intentionally not table-aligned: fixed-width padding spends tokens, makes snapshots brittle, and encourages unlike rule families to share one visual column model. Percent rules render a percent plus covered/total counts. Uncovered-count rules render the observed uncovered count directly (for example, `0 uncovered`) and must not borrow the percent/count display from the corresponding changed metric.*

### 2. Focused Failure Output (FAIL)
When the gate fails, `covgate` should only print details for files that have uncovered changes. Context about the diff range is kept but modernized.

**Proposed (Focused):**
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
`SourceSpan` will be updated to include `start_col` and `end_col`.
- **Why**: AI agents can jump directly to the exact expression or branch within a line. This is particularly useful for complex lines (e.g., `if (a && b) || c`) where line-level coverage is ambiguous. It is also **essential for function metrics**, as multiple lambdas or anonymous functions can exist on a single line; column precision allows unique identification of which function is uncovered.
- **Format support**:
    - **LLVM** and **Istanbul** both expose full column data (start/end) for all span types and will be updated to populate the new fields.
    - **Coverlet** does not expose source column positions — its JSON format only provides line numbers and IL byte offsets (`Offset`, `EndOffset`). Coverlet spans will always use the line-only fallback.
- **Formatting**:
    - Single point: `line:col`
    - Range on same line: `line:start_col-end_col`
    - Range across lines: `start_line:start_col-end_line:end_col`
    - If columns are unknown (Coverlet, or any parser that can't populate them): Fall back to `line` or `start_line-end_line`.

### 4. Console Output
Console output summarizes rule outcomes and, on failure, lists uncovered file details. Markdown is the format for detailed inspection.
