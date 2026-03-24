# Coverlet method summary semantics

This document records the repository's decision and the evidence behind it for `.NET` function or method totals across:

- raw Coverlet JSON
- Cobertura XML exported by `coverlet.collector`
- downstream summary tools

## Decision

`covgate` will consume Coverlet and Cobertura method records as exported. It will not add ReportGenerator-style filtering for compiler-generated lambda helper methods.

This is an intentional product choice, not an unresolved bug.

## Why

The repository gathered enough evidence to make a clear decision:

- raw Coverlet JSON and Cobertura XML matched on method presence in the investigated real-world case
- the observed mismatch against ReportGenerator was small and additive: `covgate` reported `73/82`, while ReportGenerator reported `71/80`
- in that real-world case, the difference was `+2/+2`, not hidden uncovered noise
- ReportGenerator applies custom Cobertura-side method filtering
- Jenkins `coverage-model` appears much closer to raw Cobertura and does not appear to apply the same filter

So adding filtering in `covgate` would increase parser complexity, move the tool farther away from exported coverage data, and align it with one downstream consumer that does not appear to represent a broader consensus.

## Evidence summary

Real-project comparison:

    coverage.json method count:          82
    coverage.cobertura.xml <method>s:    82
    covgate function totals:             73 / 82
    ReportGenerator methods:             71 / 80

This showed:

- the JSON-to-Cobertura export was not dropping methods
- the downstream drift happened after Cobertura export

Open repros built during investigation:

- local-function repro
  - Coverlet JSON: 2 methods
  - Cobertura XML: 2 methods
  - ReportGenerator: 2 methods
- async-plus-lambda repro
  - Coverlet JSON: 2 methods
  - Cobertura XML: 2 methods
  - ReportGenerator: 1 method

This showed that ReportGenerator is not removing all compiler-generated methods. Its divergence is narrower and tool-specific.

## ReportGenerator-specific behavior

The repository inspected ReportGenerator source in [CoberturaParser.cs](/home/jesse/git/covgate/target/ReportGenerator/src/ReportGenerator.Core/Parser/CoberturaParser.cs).

In normal, non-raw mode, it filters methods whose normalized name still matches a lambda-helper pattern:

    if (!this.RawMode && methodName.Contains("__") && LambdaMethodNameRegex.IsMatch(methodName))
    {
        continue;
    }

with:

    private static readonly Regex LambdaMethodNameRegex = new Regex("<.+>.+__", RegexOptions.Compiled);

That means ReportGenerator is applying downstream, tool-specific method semantics rather than simply echoing raw Coverlet or Cobertura methods.

## Jenkins `coverage-model` findings

The repository also inspected Jenkins `coverage-model` in [CoberturaParser.java](/home/jesse/git/covgate/target/coverage-model/src/main/java/edu/hm/hafner/coverage/parser/CoberturaParser.java).

What we found:

- methods are created directly from Cobertura `name` and `signature`
- duplicate methods may be renamed in ignore-errors mode
- no ReportGenerator-style filtering or rewriting was found for:
  - `<>c`
  - `b__`
  - `MoveNext`
  - lambda helper methods

So Jenkins `coverage-model` appears much closer to raw Cobertura method semantics than ReportGenerator does.

## Consequence for `covgate`

`covgate` should treat raw Coverlet/Cobertura method records as the default truth source for `.NET` function totals.

If a future request wants ReportGenerator parity, that should be introduced as an explicit compatibility mode or explicit product decision, not as an undocumented parser tweak.

## Related files

- `src/coverage/coverlet_json.rs`
- `docs/reference/coverlet-method-to-function-normalization.md`
- `docs/exec-plans/completed/covgate-coverlet-function-summary-investigation.md`
