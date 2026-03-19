# .NET fixtures

These fixtures exercise Coverlet native JSON emitted by:

```bash
dotnet test --collect:"XPlat Code Coverage;Format=json,cobertura"
```

Fixture coverage artifacts are checked in for deterministic integration tests. Repro fixtures
also check in `native-summary.json`, normalized from Coverlet's Cobertura summary attributes.
Regenerate with:

```bash
cargo xtask regen-fixture-coverage dotnet/basic-fail
cargo xtask regen-fixture-coverage dotnet/basic-pass
cargo xtask regen-fixture-coverage dotnet/duplicate-lines
```
