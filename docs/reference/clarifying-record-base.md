## Recommendations for clarifying `record-base` guidance

## Problem statement

The current `covgate` README and `covgate --help` make `covgate record-base` sound like the normal workflow for any agent-driven usage. That guidance is too broad and can lead users in standard local or devcontainer checkouts to run `record-base` unnecessarily.

This creates confusion because there are really two different environments:
- normal local/devcontainer checkouts, where `main` / `origin/main` is available and `covgate check <coverage-report>` should work without extra setup
- cloud sandbox environments such as Codex Cloud and Jules, where base refs may be unavailable or unstable and `record-base` is the right escape hatch

Because the current wording does not clearly separate those cases, users can infer that “agent” means “always use `record-base`,” when the intended guidance is narrower: use it for constrained cloud-agent environments, not ordinary local agent workflows.

The wording is also muddied by describing automatic discovery of `main` / `origin/main` when `--base` is omitted and the recorded `record-base` ref does not exist as a "fallback." That label is misleading because in ordinary checkouts automatic base-ref discovery is the standard behavior and `record-base` is the exceptional path.

The current wording also obscures when `record-base` is supposed to happen. The intended workflow is to run `record-base` at the beginning of a task before the agent makes Git changes, then perform the task, then run `covgate check`. Running `record-base` immediately before `check` is pointless because it records the post-change `HEAD` instead of the pre-change base commit.

### Tighten the scope of the recommendation
The current README / `--help` wording makes `covgate record-base` sound like the default workflow for any agent usage. That overreaches.

Suggested clarification:
- `record-base` is primarily for *agent cloud / sandboxed worktree* environments such as Codex Cloud and Jules.
- It is not the recommended default for ordinary local clones, devcontainers, or other environments where `main` / `origin/main` is already available.
- “Agent” alone is too broad; a normal Codex agent running in a full local repo should usually rely on the standard automatic base-ref discovery behavior.

Suggested wording:
> Use `covgate record-base` in cloud agent environments where the default branch refs are unavailable. In normal local clones and devcontainers, omit `record-base` and let `covgate` resolve the base ref normally.

### Make the normal local workflow more prominent
The README currently emphasizes the cloud-agent workflow, but the common workflow in a normal checkout is simpler.

Suggested explicit guidance:
- In a normal local/devcontainer environment, run:
  `covgate check coverage.json`
- If `origin/main` or `main` is available, no `record-base` step is needed.
- If a team wants a non-default base, they can still pass `--base`.

Suggested wording:
> In a standard local checkout, the normal workflow is simply `covgate check <coverage-report>`. If `--base` is omitted, `covgate` will auto-discover the base branch. No `record-base` step is needed in that case.

### Clarify fallback precedence and intent
The current docs mention the automatically discovered refs, but they should state the intent more directly without calling that normal path a "fallback."

Suggested clarification:
- `record-base` is an escape hatch for environments missing a usable branch ref.
- It should not replace normal automatic base resolution when that resolution already works.

Suggested wording:
> `record-base` is intended as an escape hatch for constrained environments, not as a replacement for normal branch-based diffing in a standard repository checkout.

### Consider making `record-base` more defensive
Your idea makes sense: `record-base` could detect whether a normal base ref already resolves and either no-op or emit a message explaining that recording is unnecessary.

Possible behavior:
- If `origin/HEAD`, `origin/main`, or `main` resolves cleanly:
  - no-op with a message like:
    > Base ref `origin/main` is available; `record-base` is unnecessary in this environment.
- Otherwise:
  - write `refs/worktree/covgate/base` as it does today.

That would reduce accidental overuse and reinforce the intended model.

### Update `covgate --help`
The current help text:
> Agent workflow:
> `covgate record-base`
> `covgate check <coverage-report>`

This reads like the generic recommended workflow.

Suggested replacement:
> Standard workflow:
> `covgate check <coverage-report>`
>
> Cloud-agent workflow (when `main` / `origin/main` is unavailable):
> `covgate record-base`
> ...make changes...
> `covgate check <coverage-report>`

### Add one explicit environment split example
A short side-by-side example would likely prevent this confusion.

Suggested structure:
- Local/devcontainer:
  `covgate check coverage.json`
- Codex Cloud / Jules:
  `covgate record-base`
  ...make changes...
  `covgate check coverage.json`

That makes it obvious that `record-base` is conditional, not universal.
