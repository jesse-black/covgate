---
name: description-frontmatter-authoring
description: Create or revise Markdown `description` frontmatter so `docgarden match` routes agents to the right repository documents. Use when adding a new routed Markdown document, updating existing frontmatter, fixing noisy or missing match results, tuning skills or docs discovery, or making descriptions more query-aligned without loading document bodies.
---

# Description Frontmatter Authoring

Use this skill to make frontmatter descriptions work as the repository's routing layer.

`docgarden match` is metadata-first discovery, not full-text search. It scores `name`, path context, and frontmatter `description`; it does not rely on the document body for normal routing. A good description tells an agent what the document is for and when to read it.

## Workflow

1. Identify the routing job.
   - Write down 2-5 realistic queries that should find the document.
   - Write down nearby documents that should not outrank it.

2. Inspect the current route.
   - Run `cargo run -- match <query>` for each realistic query.
   - Use `cargo run -- match --explain <query>` when ranking looks surprising.

3. Rewrite the description as a positive routing cue.
   - Lead with the document type or task family.
   - Include action words an agent or user would actually ask for.
   - Include the scope only when it helps separate this document from similar ones.
   - Keep it one sentence unless the document truly has multiple routing jobs.

4. Remove accidental attractors.
   - Avoid saying what the document is not. Negated phrases still add matchable tokens.
   - Avoid broad words that make the document appear for unrelated workflows.
   - Avoid duplicating another document's strongest trigger phrase unless both should route together.

5. Validate the route and the lint policy.
   - Rerun the same `cargo run -- match <query>` checks.
   - After modifying any Markdown file, run `cargo run -- lint <changed-files> --color never`.

## Description Pattern

Use this shape:

```text
<Document kind> for <specific task/scope>; read when <agent intent, workflow, or decision point>.
```

Examples:

```yaml
description: "Working design draft for `docgarden match` scoring, including the shipped BM25F model, stopword handling, and future tuning directions."
```

```yaml
description: "Create, update, revise, reopen, or rescope an ExecPlan using `docs/PLANS.md`; use when shaping plan requirements before implementation."
```

```yaml
description: "Follow-up tasks and cleanup items; read when looking for deferred implementation work, small backlog items, or candidate topics to promote into a future plan."
```

## Negative-Phrase Trap

Do not encode exclusions in the discovery description.

Bad:

```yaml
description: "Follow-up tasks and cleanup items that are not part of an active ExecPlan."
```

This can surface TODO for queries about active plans because `active` and `ExecPlan` are still strong tokens.

Better:

```yaml
description: "Follow-up tasks and cleanup items; read when looking for deferred implementation work, small backlog items, or candidate topics to promote into a future plan."
```

Move exclusion details into the body if humans need them. The description should describe the positive route.

## Heuristics

- Prefer verbs from expected user requests: create, update, review, implement, debug, validate, route, score.
- Prefer concrete nouns from the repository vocabulary: ExecPlan, frontmatter, match, lint, config, scoring, skills.
- Use path-like scope words only when useful: active plan, completed plan, design doc, skill.
- Keep stable context in descriptions; put volatile status or long rationale in the body.
- If two documents compete, make their descriptions contrast by job, not by negation.

## Done Criteria

A description is good enough when:

- expected queries route to the document near the top
- unrelated documents stop appearing because of accidental shared words
- the description remains true when read alone in `docgarden match` output
- `docgarden lint <changed-files> --color never` passes
