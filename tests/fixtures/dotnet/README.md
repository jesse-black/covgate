# .NET fixtures

These fixtures exercise Coverlet native JSON emitted by:

```bash
dotnet test --collect:"XPlat Code Coverage;Format=json"
```

Fixture coverage artifacts are checked in for deterministic integration tests.
Regenerate with:

```bash
cargo xtask regen-fixture-coverage dotnet/basic-fail
cargo xtask regen-fixture-coverage dotnet/basic-pass
```
