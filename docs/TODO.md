# TODO

This file tracks follow-up cleanup ideas that came up during planning but are not currently part of an active exec plan.

- Consider extracting the repeated `totals_by_file` map assembly pattern from the coverage format adapters once the boundary-purification refactor is complete. `src/coverage/coverlet_json.rs`, `src/coverage/istanbul_json.rs`, and `src/coverage/llvm_json.rs` all build metric maps with the same “insert only when non-empty” shape.

- Consider consolidating the repeated test harness helpers used by coverage integration-style tests after the file-backed coverage tests move out of `src/coverage/mod.rs`. The cwd lock, temporary git-repo setup, and `run_git`/PATH override helpers are currently duplicated across coverage-focused tests.

- Revisit whether any deeper parser-internal deduplication is worth doing after the naming and boundary cleanup lands. For now, avoid broad parser rewrites that would obscure format-specific behavior, but a later pass may be worthwhile once the API and test boundaries settle.
