# Coverlet, Cobertura, and ReportGenerator method semantics

This document records what the repository has learned so far about `.NET` function or method totals across Coverlet JSON, Cobertura XML, and ReportGenerator.

It exists to answer a narrower question than "what should `covgate` do?" The first job is to describe the evidence clearly enough that future changes can make an explicit product decision instead of silently treating a downstream summary as if it were raw Coverlet truth.

## Big picture

For `.NET` coverage, the repository now has strong evidence that line and branch totals are stable:

- raw Coverlet JSON detail
- Cobertura XML exported by `coverlet.collector`
- downstream summaries such as ReportGenerator
- `covgate`'s current line and branch parsing

all line up closely enough for practical parity.

Function or method totals are different.

On a real project artifact, the three layers do not mean the same thing:

- Coverlet JSON counted `82` methods
- Cobertura XML also contained `82` `<method>` elements
- ReportGenerator reported `80` total methods

That means ReportGenerator is not simply echoing raw Coverlet method totals. It is applying its own normalization.

## Main finding

The current evidence says:

- Coverlet JSON method totals are raw export data
- Cobertura XML preserves those raw method records closely enough that the same total can still be observed
- ReportGenerator filters some methods before computing its method summary

So ReportGenerator is a downstream summary oracle, not a native Coverlet method oracle.

If the repository chooses to match ReportGenerator in the future, that should be described as:

- "ReportGenerator-compatible method totals"

not as:

- "raw Coverlet function semantics"

## Real-project evidence

The motivating closed-source project comparison had:

    covgate line totals:      228 / 255
    native line totals:       228 / 255
    covgate branch totals:     37 / 52
    native branch totals:      37 / 52
    covgate function totals:   73 / 82
    ReportGenerator methods:   71 / 80

The important narrowing result is that the mismatch is function-specific.

Additional inspection of the local Voicer artifacts showed:

    coverage.json method count:          82
    coverage.cobertura.xml <method>s:    82
    ReportGenerator total methods:       80

That proves the drift is not caused by `covgate` inventing extra methods and is not caused by the JSON-to-Cobertura export dropping methods. The change happens later, inside downstream summary normalization.

## Open repros built during investigation

The repository investigation produced two especially useful open probes.

### Local function repro

An open fixture with a local function produced:

- Coverlet JSON: 2 methods
- Cobertura XML: 2 methods
- ReportGenerator: 2 methods

Representative raw method names:

    System.Int32 CovgateDemo.MathOps::Add(System.Int32)
    System.Int32 CovgateDemo.MathOps::<Add>g__Duplicate|0_0(System.Int32)

This showed that ReportGenerator does not blindly remove all compiler-generated methods. Local-function helper methods can still count.

### Async plus lambda repro

A temporary open repro with an async method that used a LINQ projection produced:

- Coverlet JSON: 2 methods
- Cobertura XML: 2 methods
- ReportGenerator: 1 method

Representative raw method names:

    System.String CovgateDemo.Shapes/<>c::<HandleAsync>b__0_0(System.String)
    System.Void CovgateDemo.Shapes/<HandleAsync>d__0::MoveNext()

ReportGenerator counted only one method in that case. The filtered method was the generated lambda helper in the `<>c` class, while the async state-machine `MoveNext()` method remained counted.

This is the strongest open evidence for the current downstream normalization rule shape.

## Exact ReportGenerator filter logic

The repository now has the exact Cobertura-side filter logic from ReportGenerator source, not just an inference from artifacts.

In `target/ReportGenerator/src/ReportGenerator.Core/Parser/CoberturaParser.cs`, ReportGenerator:

1. rewrites some compiler-generated names with `ExtractMethodName(...)`
2. then skips methods that still look like compiler-generated lambda helpers

The filter condition is:

    if (!this.RawMode && methodName.Contains("__") && LambdaMethodNameRegex.IsMatch(methodName))
    {
        continue;
    }

The lambda regex is:

    private static readonly Regex LambdaMethodNameRegex = new Regex("<.+>.+__", RegexOptions.Compiled);

So in normal, non-raw mode, ReportGenerator drops Cobertura methods whose post-normalization name still matches the lambda-helper pattern.

### What `ExtractMethodName(...)` does first

Before the filter runs, ReportGenerator rewrites some names:

- local functions are rewritten to the nested function name and are therefore kept
- compiler-generated `MoveNext()` methods are rewritten back to a source-style method name and are therefore kept
- lambda helper methods such as `<HandleAsync>b__0_0(...)` still match the lambda regex and are dropped

This means the real rule is narrower than "drop compiler-generated methods."

The exact practical rule for Cobertura in default mode is:

- drop compiler-generated lambda helper methods that still match `<...>...__`
- keep rewritten local functions
- keep rewritten async or iterator `MoveNext()` methods

There is also class interpretation logic in `CoberturaClassNameParser.cs`: nested or compiler-generated classes are folded into their parent class in non-raw mode. That affects class presentation, but the actual method drop comes from the explicit lambda-method `continue` in `CoberturaParser.cs`.

## What ReportGenerator filters in practice

The source-backed interpretation matches the open repros and the Voicer artifact:

- lambda helper methods in `<>c` classes are filtered
- async state-machine `MoveNext()` methods are still counted
- iterator `MoveNext()` methods are still counted
- local-function helper methods can still be counted

## Why this matters for `covgate`

This distinction changes the meaning of any future parity target.

If `covgate` counts raw Coverlet methods, it is doing something stricter and closer to native export detail.

If `covgate` matches ReportGenerator, it is intentionally adopting a downstream, presentation-oriented method summary that excludes some implementation-detail methods.

That choice affects user-visible behavior:

- a filtered generated lambda helper method could be uncovered
- ReportGenerator could still report `100%` method coverage because that method is excluded from both numerator and denominator
- a raw-method interpretation would count it and lower coverage

So this is not just a formatting difference. It is a semantic product choice.

## Current recommendation

The repository should not treat ReportGenerator as automatically authoritative for Coverlet function semantics.

The safer framing is:

1. Coverlet JSON is the raw method export.
2. Cobertura XML preserves those raw methods closely enough for the observed cases.
3. ReportGenerator applies extra method filtering.
4. If `covgate` wants ReportGenerator parity, it should say so explicitly and test against that behavior deliberately.

At the moment, the evidence supports using ReportGenerator as a possible oracle for a user-facing `.NET` method-summary mode, but not as a factual description of raw Coverlet method totals.

## Practical implications

Future changes should keep these distinctions explicit:

- "raw Coverlet methods" means method records visible in `coverage.json`
- "Cobertura methods" means `<method>` elements in `coverage.cobertura.xml`
- "ReportGenerator methods" means downstream filtered method totals from ReportGenerator summaries

Any repository test or documentation that says "native method totals" should name which of those three it means.

## Related files

Implementation and harness:

- `src/coverage/coverlet_json.rs`
- `tests/support/mod.rs`
- `xtask/src/main.rs`

Related reference and plan documents:

- `docs/reference/coverlet-method-to-function-normalization.md`
- `docs/exec-plans/active/covgate-coverlet-function-summary-investigation.md`
