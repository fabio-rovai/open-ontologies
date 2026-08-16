---
name: your-skill-name
description: One sentence saying WHEN an agent should use this skill. Name the trigger conditions, domain, and tools involved.
---

# Your Skill Name

One paragraph: what job this skill accomplishes and what state it assumes (e.g. "an ontology is already loaded" or "starts from a Turtle file on disk").

## Workflow

### 1. First stage

- Call `onto_validate` on the input — if it fails, fix the reported syntax errors and re-validate before anything else.
- Call `onto_load`, then `onto_stats` to confirm counts match expectations.

### 2. Decision point

- If `onto_stats` shows N classes but the spec expects M, do X.
- If `onto_lint` reports missing labels, add them and reload — do not proceed with a dirty lint.

### 3. Verify and persist

- State the checks that must pass before the skill is done (lint clean, enforce clean, competency questions answerable via `onto_query`).
- Call `onto_save`, then `onto_version` — always version after save.

## Failure modes

Say what to do when things go wrong, not just the happy path:

- Fetch/parse failures: ...
- Validation that never converges: ...
- When to stop and ask the user: ...
