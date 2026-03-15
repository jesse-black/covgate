# Vitest fixtures

These fixtures exercise Istanbul native JSON emitted by Vitest's default v8 coverage provider:

```bash
vitest run --coverage
```

Fixture coverage artifacts are checked in for deterministic integration tests.
Regenerate with:

```bash
cargo xtask regen-fixture-coverage vitest/basic-fail
cargo xtask regen-fixture-coverage vitest/basic-pass
```
