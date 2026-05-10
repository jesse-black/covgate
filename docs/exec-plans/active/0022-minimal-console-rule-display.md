---
description: "ExecPlan for making minimal console rule summaries unaligned and rendering uncovered-count gates with uncovered-count observations."
---

# Minimal Console Rule Display

## Goal
- Minimal console output uses compact, unaligned rule summary lines, and uncovered-count gates display their observed uncovered count instead of percent metric evidence.

## Scope
- In: normal/minimal console rule summary formatting, scoped gate labels in minimal console output, uncovered-count gate observed-value display, focused renderer tests, and CLI output expectations affected by the new minimal format.
- Out: verbose console table-style output, Markdown output, gate semantics, parser metric definitions, config schema, and threshold defaults.

## Relevant Areas
- `docs/design-docs/token-efficient-output.md` — source design decision for unaligned minimal output and uncovered-count display.
- `src/render/console.rs` — minimal console rule summary formatting and failure-focused output.
- `tests/render_console.rs` — renderer-level output shape and regression coverage.
- `tests/cli_interface.rs`, `tests/cli_metrics.rs` — end-to-end expectations that may assert minimal console output.
- `docs/exec-plans/active/0021-split-changed-metrics-from-gates.md` — nearby model split that should remain compatible with rule-observation rendering.

## Open Questions
- None yet.

## Steps
- [ ] Add failing tests that prove minimal console output has no fixed-width table padding for metric labels, observations, counts, or rules.
- [ ] Add failing tests that prove uncovered-count gates render as `<N> uncovered` with their uncovered-count comparator and threshold, not as percent coverage plus covered/total counts.
- [ ] Cover scoped minimal output so optional gate labels remain compact, for example `[frontend] PASS Lines: ...`, without reintroducing column alignment.
- [ ] Update `src/render/console.rs` so `render_rule_summary` builds compact record-like lines from the `RuleOutcome` family rather than a shared padded table format.
- [ ] Update stale CLI/interface expectations that intentionally snapshot minimal console output.
- [ ] Run focused tests during the edit loop, then run full validation because this changes Rust output behavior.

## Validation
- `cargo test --test render_console`
- `cargo test --test cli_interface`
- `cargo test --test cli_metrics`
- `cargo xtask validate`

## Discoveries
- The token-efficient output design now explicitly rejects fixed-width table alignment in minimal console output.
- Uncovered-count gates should display observed uncovered count directly; percent evidence belongs to percent rules or changed metric evidence, not uncovered-count rule summaries.

## Review
- None yet.

## Definition of Done

### Planner
- [x] Plan is consistent, up to date, decision-complete, and ready to hand off.

### Generator
- [ ] Goal achieved: minimal console rule summaries are compact and uncovered-count gates display uncovered-count observations.
- [ ] All planned steps are complete.
- [ ] All validation commands pass.
- [ ] Handed off to an independent reviewer (MUST use the `evaluator-execplan` skill via a subagent or separate agent, not the generator agent).

### Evaluator
- [ ] Standard review posture applied.
- [ ] Adheres to the principles of `docs/CODESTYLE.md`.
- [ ] Adheres to the principles of `docs/TESTING.md`.
- [ ] All review findings have been addressed.
