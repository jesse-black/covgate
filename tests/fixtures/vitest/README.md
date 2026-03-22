# Vitest fixtures

These fixtures exercise Istanbul native JSON emitted by Vitest's default v8 coverage provider:

```bash
vitest run --coverage --coverage.reporter=json --coverage.reporter=json-summary
```

Fixture coverage artifacts are checked in for deterministic integration tests. Repro fixtures
also check in `native-summary.json`, normalized from `coverage/coverage-summary.json`.
Regenerate with:

```bash
cargo xtask regen-fixture-coverage vitest/basic-fail
cargo xtask regen-fixture-coverage vitest/basic-pass
cargo xtask regen-fixture-coverage vitest/statement-line-divergence
cargo xtask regen-fixture-coverage vitest/empty-branch-locations
```
