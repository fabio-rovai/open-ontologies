# Community Skills

Skills are the zero-code extension tier: markdown workflow recipes that teach a connected agent (Claude or any MCP client) how to chain the `onto_*` tools for a specific job. The bundled [`ontology-engineering`](../ontology-engineering/SKILL.md) skill is the reference — community skills live here, one directory per skill.

If you have a tool-chaining pattern that works — a validation gauntlet for a specific domain, a migration recipe, an alignment review loop — you can contribute it without writing a line of Rust.

## Layout

```
skills/community/
  your-skill-name/
    SKILL.md        # required — frontmatter + instructions
    references/     # optional — supporting files the skill points at
```

`SKILL.md` starts with YAML frontmatter:

```yaml
---
name: your-skill-name
description: One sentence saying WHEN an agent should use this skill, not just what it does.
---
```

Copy [`TEMPLATE.md`](TEMPLATE.md) to `your-skill-name/SKILL.md` to start.

## What makes a good skill

1. **Trigger-first description.** The frontmatter description is what an agent reads to decide relevance. "Use when validating a clinical ontology against SNOMED crosswalks" beats "clinical utilities".
2. **Tool sequences with decision points**, not fixed pipelines. Say what to call next *based on what the previous tool returned* — that is the project's core orchestration principle.
3. **Real tool names.** Every `onto_*` reference must exist in the current tool set (see the [Tool Reference](../../CLAUDE.md)). CI reviewers will check.
4. **MCP-native.** A skill instructs the *orchestrating agent*; it must not tell the server to call an LLM. Judgment happens in the conversation, verdicts flow back through the `*_feedback` tools.

## Submitting

Open a PR adding your skill directory. Acceptance criteria: frontmatter parses, referenced tools exist, the workflow is honest about failure modes (what to do when `onto_validate` fails, not just the happy path).

Skills here are documentation, not executable code — the same trust model as the community pack registry.
