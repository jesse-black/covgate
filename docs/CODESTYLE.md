---
description: "Rust coding philosophy and conventions for the `covgate` crate. Six principles plus worked examples covering library leverage, struct shape, control flow, enum modeling, and CLI patterns; read when adding modules, designing data flow, choosing between wrappers and direct types, deciding between handwritten validators and `clap` features, or reviewing code changes and refactors."
---

# Code Style

This document is structured as a small philosophy followed by worked examples. The philosophy is durable; the rules in practice are illustrative. New entries must derive from an existing principle, or motivate a sharpening of one. The principle set is the spine of the document — read it first.

Rules about what tests should assert live in `docs/TESTING.md`; this document covers code shape.

## Philosophy

Six principles, ordered by how often they show up at review. Each rule below cites one or more of them.

1. **Trust the toolchain.** Rust and its ecosystem solve hard problems well. Reach for `clap`, `serde`, `toml`, `anyhow`, and the type system before writing parallel logic by hand. Custom code is for constraints the toolchain cannot express, not for places where reaching for the tool would have taken five minutes.

2. **One fact, one place.** A configuration field, a derived value, a rule identifier, a validation predicate — each lives in one canonical location. Parallel structs drift. Parallel collections drift in lockstep until they don't. String literals scattered across modules become typo bugs. Duplication is debt that compounds silently.

3. **Make invalid states unrepresentable.** If a value comes from a closed set, model it as an enum. If two flags must be mutually exclusive, encode that in the parser. If a derivation must be computed from another field, store it on the same struct so they cannot disagree. Push correctness into the type system; let the compiler refuse the bad program rather than asking the runtime to bail.

4. **Earn every layer.** Wrapper structs, helper functions, options bags, intermediate collections — each layer of indirection must have a reason that survives the next refactor. "Symmetry with another wrapper" is not a reason. "Future fields might be added" is not a reason unless this task adds them. YAGNI applies.

5. **The happy path reads top to bottom.** Functions reveal intent at one level of abstraction. Orchestration code looks like an outline; details live in helpers. Control flow does not zigzag through nested re-checks of premises the caller already established. A reader should be able to skim a function and know what it does.

6. **Tests assert behavior, not implementation.** A test that fails when refactoring rearranges internals — without changing what callers see — is rejected. Code shape is the same in test and release builds. Test rules expand on this; see `docs/TESTING.md`.

When a finding does not cleanly cite one of these, the philosophy is the lever, not the rule list.

## Rules in practice

Worked examples grouped by topic. Each rule tags the principle(s) it expresses. Rules are illustrative — when a rule's example pattern no longer appears in the codebase, the rule has done its job and can be deleted. The philosophy stays.

### Lean on libraries before writing custom logic

*Principles 1, 2.*

- ALWAYS prefer `clap` features (`ArgGroup`, `conflicts_with`, `default_value_t`, `value_enum`) over handwritten CLI validators or post-parse `bail!` checks. Clap exits at parse time with the offending flag named, makes the rule visible on the args type, and removes branching from `execute_*` handlers.
- ALWAYS prefer existing parsers (`toml`, `serde`, `serde_json`) over hand-rolled string scanning when one will do. A custom parser is appropriate when the existing tools cannot express the constraint (e.g., custom logic for merging LLVM segments); it is not appropriate for "I could just split on `:` here".
- NEVER duplicate library-provided validation in handwritten code. If a constraint can be expressed at parse time, express it there and delete the runtime check.

### Avoid parallel structures that drift in lockstep

*Principles 2, 4.*

- NEVER declare two structs whose only meaningful difference is a field rename. If a downstream module needs the same shape as an upstream policy minus one field, take the upstream struct directly and pass the missing field separately.
- NEVER carry two collections in lockstep when one struct field can hold the derived value. If a transformation of `SourceSpan` is needed for every opportunity, store the derivation on the opportunity type at construction.
- ALWAYS mutate an output struct in place when its fields mirror the local accumulators. Five `let mut` locals followed by a struct literal that names them all is a refactor that already exists — initialize the output and mutate it.

### Avoid wrappers that do not earn their abstraction

*Principle 4.*

- NEVER introduce a `Foo { only_field: T }` struct that callers immediately destructure. Return `T` directly. Same for `FooOptions` types that exist to bundle one collection — pass `&[T]` instead.
- ALWAYS justify a wrapper struct by either fields planned in the current task or a real semantic distinction between the wrapper and its field.
- NEVER bundle fields into a context struct only to re-explode them at the call site. If a state struct exists, downstream helpers should accept `&mut State`, not five individual references to its fields.

### Prefer enums to stringly-typed identifiers

*Principles 2, 3.*

- ALWAYS model closed sets of identifiers (metric kinds, diff sources, gate rules) as enums. Free-form string matching against a hand-maintained literal list is a refactor away from a typo bug, and the literal list is duplicated wherever the set is referenced.
- ALWAYS let serde reject unknown variants at parse time when the identifier appears in user-facing config. A runtime predicate that scans for unknown rule names duplicates work the deserializer can do for free.
- ALWAYS attach `as_str` (or `Display`) to the enum so emit-sites read `MetricKind::Region.as_str()` rather than reintroducing string literals.

### Use kebab-case for user-facing identifiers

*Principles 1, 2.*

- ALWAYS spell user-facing identifiers in kebab-case: diagnostic output, TOML config keys, and CLI flag long names. This matches `cargo`, `clippy`, and gives every user-typed identifier in the project one canonical shape.
- ALWAYS keep Rust struct field names snake_case (idiomatic Rust); bridge to kebab TOML keys with `#[serde(rename = "...")]` or `#[serde(rename_all = "kebab-case")]`. The Rust type system stays in its convention; the user-facing schema stays in its convention; serde resolves the gap.
- ALWAYS spell flag identifiers consistently across CLI and TOML. A second accepted spelling is two facts in two places — use `rename_all = "kebab-case"` to maintain parity between `--fail-under-lines` and `fail-under-lines`.

### Keep control flow flat

*Principle 5.*

- NEVER pattern-match on a variant a caller already filtered. If a dispatcher matches on `DiffSource::GitBase(_)` before calling a helper, the helper should accept the inner `String` directly or destructure with `let DiffSource::GitBase(base) = source else { ... };`.
- ALWAYS use `match` (or `unwrap_or_else`) when the fallback computation has cost. `.unwrap_or(git::discover_base_ref()?)` evaluates the Git command on every call.
- ALWAYS collapse `Option<Vec<T>>` to `Vec<T>` with `#[serde(default)]` when "absent" and "empty" mean the same thing. The `Option` only adds an `unwrap_or_default()` at every consumer and lies about absence carrying meaning.

### Keep production and test type shapes identical

*Principle 6.*

- NEVER fence production fields with `#[cfg(test)]` to expose internals to tests. The struct then has different layouts in test vs release builds and the test is implicitly checking implementation.
- ALWAYS expose deliberately-test-visible state through `pub(crate) fn` accessors that exist in both profiles. The corresponding rule for what tests should assert lives in `docs/TESTING.md`.

### Keep orchestration functions readable

*Principles 4, 5.*

- ALWAYS extract sub-steps from any `fn` longer than ~50 lines whose body is sequential ("first do A, then B, then C"). Long orchestration functions read as checklists; they obscure the dependency between steps and grow without resistance. `src/lib.rs::run` is the canonical example of a clean orchestration function.
- ALWAYS prefer pulling private helpers into the same file before splitting modules. The lighter-friction first cut is usually enough.

## Growing this document

This document is intended to stabilize, not accumulate. When a code review surfaces a smell that is not covered:

1. **Find the principle that catches it.** If one of the six principles, taken seriously, would have prevented the smell, the principle has done its job and the rule list does not need a new entry. Cite the principle in the review and move on.
2. **If no principle fits, sharpen one before adding a new rule.** A novel finding usually means an existing principle is not crisply stated. Sharpening is preferred to expansion; the principle set should grow only when the existing six genuinely cannot generate the new rule.
3. **If a rule is required, write it as a worked example.** Tag the principle it expresses. Keep it concrete: name the file, name the pattern, name the alternative. Avoid restating the principle as a rule — that is what the philosophy section is for.
4. **Prune aggressively.** When a rule's example pattern no longer appears in the codebase, the rule has stopped earning its place. Delete it. The principles persist; the examples are scaffolding.
5. **Cap the principle count.** Aim for five to seven. A growing principle list is a sign the principles have stopped being generative.

The test for whether this document is healthy is not "does it cover everything?" but "can a reviewer derive the right answer from the philosophy alone?" When the answer is yes, the rule list is doing what it should: illustrating, not legislating.
