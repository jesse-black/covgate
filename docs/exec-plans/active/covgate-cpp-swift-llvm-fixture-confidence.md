# Build real-world C/C++ and Swift LLVM fixtures or narrow `covgate`'s support claims

Save this in-progress ExecPlan in `docs/exec-plans/active/covgate-cpp-swift-llvm-fixture-confidence.md`. Move it to `docs/exec-plans/completed/covgate-cpp-swift-llvm-fixture-confidence.md` only after implementation, validation, and documentation updates are complete.

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

Maintain this document in accordance with `docs/PLANS.md`. Re-read that file before revising this plan and keep this plan aligned with its rules.

## Purpose / Big Picture

`covgate` currently tells users that Rust, C, C++, and Swift are supported through LLVM JSON in [README.md](README.md). That claim is only trustworthy if the repository contains realistic fixtures and tests that exercise the parser behavior those ecosystems actually need. Right now the Rust path has a real-world reproduction for mangled LLVM function names, but the C/C++ and Swift LLVM fixtures are still tiny “basic pass/fail” scenarios. They are good smoke tests, not confidence-building compatibility tests.

This plan takes a strong stance. We will either build realistic C/C++ and Swift LLVM fixtures that can reproduce mangling-related function identity bugs and validate the corresponding parser behavior, or we will narrow the README support statement so it does not overclaim what the repository can prove. The work is complete only when a novice can regenerate the richer fixtures, run the parity and parser tests, and see why the support statement is justified. If we cannot reach that bar for either language family, the support statement must be reduced before shipping.

## Progress

- [x] (2026-03-18 00:00Z) Created this repository-specific ExecPlan and recorded the hard requirement that README support claims must be backed by realistic fixtures and parser validation.
- [x] (2026-03-18 00:10Z) Reviewed the current support claim in `README.md`, the fixture regeneration guidance in `docs/TESTING.md`, and the LLVM fixture-generation entry points in `xtask/src/main.rs`.
- [x] (2026-03-18 00:15Z) Confirmed that current C/C++ and Swift LLVM fixtures are still “basic-pass/basic-fail” shapes and do not demonstrate mangling-heavy, multi-function, real-world function identity behavior.
- [ ] Decide the fixture target shapes for C/C++ and Swift and document them in this plan before implementation begins.
- [ ] Add richer C/C++ LLVM fixtures that produce mangled function names from real language features such as namespaces, overloads, templates, lambdas, local statics, and out-of-line methods.
- [ ] Add richer Swift LLVM fixtures that produce mangled function names from real language features such as structs, generics, protocol conformances, nested functions, closures, and test targets.
- [ ] Extend fixture regeneration so those richer fixtures can be rebuilt with `cargo xtask regen-fixture-coverage <language>/<scenario>` and remain deterministic enough for checked-in JSON artifacts.
- [ ] Add fixture-inspection tests or helper scripts that prove the new C/C++ and Swift artifacts actually contain mangled function names and non-trivial function populations.
- [ ] Evaluate demangler strategy for LLVM languages and make the parser policy explicit: `rustc-demangle`, `cpp_demangle`, `symbolic-demangle`, `swift-demangle`, or a narrowly justified mix.
- [ ] Implement any needed C/C++ and Swift function normalization in `src/coverage/llvm_json.rs` only after a failing test reproduces the issue.
- [ ] Add parity and focused parser tests that fail before the demangling fix and pass after it.
- [ ] Revisit `README.md` and related docs. If the richer fixtures and tests land successfully, keep or strengthen the support statement. If they do not, narrow the language support claim to match what the repository can actually prove.
- [ ] Run `cargo xtask quick` during development and `cargo xtask validate` before considering the work complete.

## Surprises & Discoveries

- Observation: The repository already has end-to-end fixture regeneration paths for Rust, C/C++, and Swift LLVM coverage.
  Evidence: `xtask/src/main.rs` routes `FixtureToolchain::Rust`, `FixtureToolchain::Cpp`, and `FixtureToolchain::Swift` through `write_llvm_fixture_coverage`.

- Observation: The test and tooling docs already promise native LLVM fixture generation for C/C++ and Swift, which raises the bar for how representative those fixtures should be.
  Evidence: `docs/TESTING.md` explicitly instructs maintainers to regenerate C/C++ fixtures with Clang/LLVM and Swift fixtures with `swift test --enable-code-coverage` plus `llvm-cov export`.

- Observation: The README support claim is ecosystem-wide, not “best effort” or “experimental”.
  Evidence: `README.md` currently says `Rust / C / C++ / Swift (LLVM JSON): Region-aware gating from llvm-cov / cargo llvm-cov.`

- Observation: Rust already exposed one real LLVM function identity bug caused by mangled symbols, but C/C++ and Swift do not yet have equivalent real-world repro fixtures in-tree.
  Evidence: the Rust-side investigation produced `tests/llvm_real_parity.rs` and a real function normalization fix, while the C/C++ and Swift fixtures remain `basic-pass` and `basic-fail`.

## Decision Log

- Decision: Treat README support claims as promises that must be backed by checked-in fixtures and tests, not by anecdotal compatibility.
  Rationale: Users read the README before they read the code. If support claims are broader than the repository evidence, we create false confidence and make bug reports inevitable.
  Date/Author: 2026-03-18 / Codex

- Decision: Prefer real language features that naturally produce mangled LLVM function names over hand-authored JSON or synthetic post-processing.
  Rationale: The repository’s testing philosophy already requires native coverage artifacts. Real source programs are the only reliable way to exercise real toolchain mangling and coverage export behavior.
  Date/Author: 2026-03-18 / Codex

- Decision: Keep the demangler decision open until fixture evidence exists.
  Rationale: Dependency choice should follow reproduced parser needs. Pulling in `cpp_demangle`, `swift-demangle`, or `symbolic-demangle` before we have a failing C/C++ or Swift repro would be speculative.
  Date/Author: 2026-03-18 / Codex

- Decision: If Swift evaluation pushes us toward `symbolic-demangle`, explicitly evaluate whether it should replace the Rust- and C/C++-specific demanglers too.
  Rationale: Carrying a mixed demangler stack is only worth the complexity if fixture-backed evidence shows a real quality, compatibility, or maintenance advantage. If `symbolic-demangle` handles Rust, C/C++, and Swift LLVM symbols well enough, a single LLVM demangling layer may be simpler and easier to explain than language-by-language crates.
  Date/Author: 2026-03-18 / Codex

- Decision: If we cannot build convincing fixture-backed confidence for a language in this plan, narrow the README instead of leaving the claim ambiguous.
  Rationale: Underclaiming is safer than claiming support we cannot prove.
  Date/Author: 2026-03-18 / Codex

## Outcomes & Retrospective

Implementation has not started yet. The useful outcome so far is a clearer bar for what “supported language” must mean in this repository. The likely failure mode before this plan was to keep adding parser logic while the evidence stayed weak. This plan explicitly rejects that path.

The main lesson at this stage is that language support and parser correctness cannot be separated. If mangled function names are a normal part of LLVM coverage in C/C++ and Swift, then realistic fixtures are not optional polish; they are the proof that the support claim is honest.

## Context and Orientation

`covgate` is a Rust CLI in `src/` that parses native coverage reports into a shared internal model, computes changed and overall metrics, applies gates, and renders summaries. LLVM JSON parsing lives in `src/coverage/llvm_json.rs`. That parser now includes Rust-aware function-name normalization after a real Rust LLVM repro showed that source span alone can undercount functions when mangled symbol identity matters.

Fixture coverage generation lives in `xtask/src/main.rs`. The command `cargo xtask regen-fixture-coverage <language>/<scenario>` rebuilds checked-in artifacts for all current fixture families. C/C++ fixtures live under `tests/fixtures/cpp/`, Swift fixtures live under `tests/fixtures/swift/`, and both are currently simple scenarios intended to prove the basic pipeline works. Those fixtures are not yet rich enough to prove parser correctness for mangled function identity.

In this plan, a “realistic fixture” means a checked-in source program and exported LLVM JSON coverage artifact that together exercise language features normal users actually write, not just one top-level function with trivial coverage shape. A “mangled function identity issue” means a case where LLVM emits multiple function records whose symbol names carry meaningful distinctions that are not safely represented by source span alone.

The main files this plan will likely touch are:

- `README.md` for the public support statement.
- `docs/TESTING.md` and `docs/reference/coverage-parser-support-matrix.md` for fixture and parser expectations.
- `docs/reference/function-coverage-debugging.md` if the language-specific function identity notes expand beyond Rust.
- `xtask/src/main.rs` for richer fixture regeneration support.
- `src/coverage/llvm_json.rs` for any C/C++ or Swift function normalization added after a failing test exists.
- `tests/support/mod.rs`, `tests/overall_summary.rs`, and new focused integration tests for fixture-backed parity and parser behavior.
- `tests/fixtures/cpp/...` and `tests/fixtures/swift/...` for the new real-world source programs and exported coverage artifacts.

## Plan of Work

Start by defining the target fixture shapes in plain language inside this plan before any code changes are made. For C/C++, the fixture should include multiple distinct callable forms that share nearby or overlapping spans in ways that could confuse a span-only deduplication strategy: free functions, overloaded functions, class methods, template instantiations, lambdas, and namespace-qualified functions. The source should be small enough to understand but rich enough to produce mangled names that matter. The resulting LLVM export must contain a non-trivial function population and clearly mangled symbol names.

For Swift, build an equally realistic fixture around language features that generate distinctive mangled function names in LLVM exports: structs, protocol conformances, generic helpers, nested closures, top-level helper functions, and test-target calls. As with C/C++, the goal is not a giant sample project. The goal is a compact source program whose exported LLVM function records look like real Swift compiler output instead of toy one-function examples.

Once the fixture source programs are chosen, extend the existing fixture regeneration path only as much as needed to support those richer scenarios. Prefer adding new fixture scenarios such as `cpp/mangled-functions` and `swift/mangled-functions` instead of overloading the current `basic-pass` and `basic-fail` fixtures with too many responsibilities. The existing basic fixtures should remain fast smoke tests; the new ones should become the compatibility proofs.

After richer artifacts exist, add tests that inspect them before changing parser logic. Those tests should prove that the artifacts contain mangled LLVM function names and enough function records to make identity bugs plausible. Then add parity or targeted parser tests that intentionally fail if function identity is computed incorrectly. Only after those tests fail should the parser gain C/C++ or Swift demangling behavior.

Demangler evaluation must stay evidence-driven. For C/C++, prefer `cpp_demangle` first if the failing fixture shows Itanium ABI mangling that this crate can normalize safely. For Swift, compare `swift-demangle` and `symbolic-demangle` against the checked-in Swift fixture symbols before choosing a dependency. `symbolic-demangle` is broader and more mature as a demangling project, but it is heavier and may bring more surface area than `covgate` needs. `swift-demangle` is lighter but appears younger.

If Swift evaluation points toward `symbolic-demangle`, do not stop at “Swift uses symbolic.” Also compare `symbolic-demangle` against the existing Rust and any new C/C++ fixture symbols to decide whether `covgate` should consolidate all LLVM demangling on one library. A single-library policy is preferable if it preserves correct fixture-backed behavior while reducing parser-policy complexity. A mixed policy is acceptable only if the fixture comparison shows a clear reason to keep language-specific crates. The chosen policy must be recorded in this plan and in a reference doc, including why we chose a crate instead of adapting demangling logic ourselves. If none of the options is convincing enough, this plan must narrow the README support statement rather than ship an unproven parser path.

Finish by bringing the docs back into alignment. If the new fixtures and tests give strong confidence, keep the README support statement and add a short note that C/C++ and Swift LLVM compatibility is backed by checked-in native fixture coverage. If the work falls short for one ecosystem, rewrite the README to describe that language more cautiously or remove it from the supported list until fixture-backed confidence exists.

## Concrete Steps

Run all commands from the repository root, the directory containing `Cargo.toml`.

1. Inspect the current fixture landscape and pick candidate source features for the new scenarios.

    rg -n "basic-pass|basic-fail|FixtureToolchain::Cpp|FixtureToolchain::Swift" xtask tests/fixtures docs
    find tests/fixtures/cpp -maxdepth 3 -type f | sort
    find tests/fixtures/swift -maxdepth 4 -type f | sort

    Expected result: a short list of current source files and scenario directories, plus a concrete understanding of which new fixture ids should be added.

2. Add new source fixtures first, then verify they regenerate cleanly.

    cargo xtask regen-fixture-coverage cpp/mangled-functions
    cargo xtask regen-fixture-coverage swift/mangled-functions

    Expected result: new `coverage.json` files appear under the new fixture directories, and regeneration succeeds without hand-editing the exported JSON.

3. Prove the artifacts are worth keeping before touching parser logic.

    rg -n "\"name\":" tests/fixtures/cpp/mangled-functions/coverage.json
    rg -n "\"name\":" tests/fixtures/swift/mangled-functions/coverage.json

    Expected result: the exported LLVM JSON contains multiple mangled function names for each new fixture, not just one trivial callable record.

4. Add failing tests that expose the parser gap, then implement the smallest parser fix that makes them pass.

    cargo test llvm_json -- --nocapture
    cargo test overall_summary -- --nocapture

    Expected result: at least one new test fails before demangling support is added and passes after the correct normalization is implemented.

5. Re-run the repository’s standard development and validation loops.

    cargo xtask quick
    cargo xtask validate

    Expected result: all repository checks pass. If they do not, keep the plan active and record the exact blocker here.

6. Reconcile the public docs with the result.

    rg -n "Supported Ecosystems|LLVM JSON|Swift|C / C\\+\\+ / Swift" README.md docs

    Expected result: the support statement and reference docs clearly match the evidence produced by the new fixtures and tests.

## Validation and Acceptance

The work is accepted only if a novice can do all of the following and observe the expected result:

Regenerate the new C/C++ and Swift fixtures with `cargo xtask regen-fixture-coverage ...` and obtain checked-in LLVM JSON artifacts with multiple mangled function names.

Run parser or parity tests that would have failed without the new normalization behavior and now pass because `covgate` computes function identity correctly for the affected language.

Read the README support statement and find matching evidence in the fixture directories and tests, rather than vague claims.

Acceptance also requires an explicit outcome for each LLVM ecosystem claim:

- Rust remains supported and continues using `rustc-demangle`.
- C/C++ is either supported with realistic fixture-backed confidence and a chosen demangler strategy, or it is narrowed in the README.
- Swift is either supported with realistic fixture-backed confidence and a chosen demangler strategy, or it is narrowed in the README.

It is not acceptable to leave C/C++ or Swift in the README as fully supported LLVM ecosystems if this plan ends without realistic fixtures and passing tests that exercise mangled function identity.

## Idempotence and Recovery

Fixture regeneration must remain safe to rerun. The new scenarios should rebuild their exported `coverage.json` from the checked-in source fixtures without requiring manual editing. If a fixture shape turns out to be unstable across toolchain versions, prefer simplifying the source until the exported artifact is stable enough to check in rather than hand-normalizing the JSON.

If a demangler dependency is tried and proves unsuitable, recover by reverting only the dependency addition and parser wiring while keeping the richer fixtures and failing tests. Those richer fixtures are valuable evidence even if the first dependency choice is wrong.

If we cannot produce a convincing fixture or parser fix for one ecosystem within reasonable complexity, recovery is to narrow the README support claim for that ecosystem and record the reasoning here. That is a successful outcome compared with shipping an unproven claim.

## Artifacts and Notes

Representative evidence this plan should eventually capture:

    $ cargo xtask regen-fixture-coverage cpp/mangled-functions
    wrote tests/fixtures/cpp/mangled-functions/coverage.json

    $ cargo xtask regen-fixture-coverage swift/mangled-functions
    wrote tests/fixtures/swift/mangled-functions/coverage.json

    $ cargo test llvm_json -- --nocapture
    test coverage::llvm_json::tests::normalizes_cpp_llvm_function_names_for_identity ... ok
    test coverage::llvm_json::tests::normalizes_swift_llvm_function_names_for_identity ... ok

Representative fallback if confidence cannot be earned:

    README.md:
    - Rust / C / C++ / Swift (LLVM JSON): Region-aware gating from llvm-cov / cargo llvm-cov.
    + Rust (LLVM JSON): Region-aware gating from cargo llvm-cov.
    + C/C++ and Swift LLVM support remain experimental until richer fixture-backed parity coverage lands.

The exact wording can change, but the document must make the public claim match the repository evidence.

## Interfaces and Dependencies

Use the existing fixture generation and Rust test stack. Do not introduce a second fixture pipeline.

The final implementation should leave the repository with:

- new LLVM fixture ids for C/C++ and Swift, likely under `tests/fixtures/cpp/` and `tests/fixtures/swift/`, each containing source code, overlay content if needed, and checked-in `coverage.json`;
- one or more helper tests that prove those fixtures contain mangled function names and non-trivial callable populations;
- language-specific function normalization in `src/coverage/llvm_json.rs` only where failing tests prove it is necessary;
- a documented demangler policy for LLVM languages.

Dependency policy for this plan is prescriptive:

- keep `rustc-demangle` for Rust;
- evaluate `cpp_demangle` first for C/C++ because it is purpose-built and narrower than a multi-language demangling bundle;
- evaluate `swift-demangle` and `symbolic-demangle` against the checked-in Swift fixture symbols before choosing one;
- if `symbolic-demangle` emerges as the best Swift option, also evaluate whether it can become the single demangling layer for Rust, C/C++, and Swift LLVM parsing;
- prefer pulling a crate over adapting demangling code from another project unless crate quality or coverage is demonstrably insufficient;
- do not add any new demangler dependency without a failing fixture-backed test that justifies it.

Plan revision note: created this ExecPlan to turn the README support claim for C/C++ and Swift LLVM coverage into an explicit fixture-and-tests accountability plan, with a documented fallback of narrowing the public claim if confidence cannot be earned.
