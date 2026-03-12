# Feedback Framework + Infrastructure Positioning — Design

**Date:** 2026-03-12
**Goal:** Two parallel tracks — (1) self-calibrating feedback for lint and enforce, (2) working benchmarks with real HermiT/Pellet comparisons and CI integration.

## Track 1: Feedback Framework

### Problem

Lint and enforce produce the same warnings every run. Users who repeatedly dismiss "missing label" on utility classes or override "orphan class" violations on intentional root classes have no way to teach the system. Drift and align already have self-calibrating feedback — lint and enforce don't.

### Data Model

One unified table in `state.rs` (shared by lint and enforce):

```sql
CREATE TABLE IF NOT EXISTS tool_feedback (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tool TEXT NOT NULL,          -- 'lint' or 'enforce'
    rule_id TEXT NOT NULL,       -- e.g. 'missing_label', 'orphan_class', custom rule ID
    entity TEXT NOT NULL,        -- the IRI that triggered the issue
    accepted INTEGER NOT NULL,   -- 1 = real issue, 0 = dismissed
    timestamp TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_tool_feedback ON tool_feedback(tool, rule_id, entity);
```

Align and drift keep their existing tables (they store signal-specific columns that don't fit a generic schema).

### Severity Adjustment Logic

When lint or enforce runs, for each candidate issue:

1. Query `tool_feedback` for `(tool, rule_id, entity)`
2. Count accepts and dismissals
3. Apply thresholds:
   - **3+ dismissals, 0 accepts** → suppress (omit from output)
   - **2 dismissals, 0 accepts** → downgrade severity (warning → info)
   - **Otherwise** → report at original severity

Thresholds are constants (`SUPPRESS_THRESHOLD = 3`, `DOWNGRADE_THRESHOLD = 2`), not configurable.

### New Tools (2)

| Tool | Input | Effect |
|------|-------|--------|
| `onto_lint_feedback` | `{ rule_id, entity, accepted }` | Insert into tool_feedback with tool='lint' |
| `onto_enforce_feedback` | `{ rule_id, entity, accepted }` | Insert into tool_feedback with tool='enforce' |

### Changes to Existing Tools

**`onto_lint`:**
- Before reporting issues, check feedback history for each `(rule_id, entity)` pair
- Suppress or downgrade as described above
- Add `suppressed_count` to output JSON (top level)
- Add `adjusted_severity` field to each issue when it differs from original

**`onto_enforce`:**
- Same pattern — check feedback before reporting violations
- Add `suppressed_count` to output JSON
- Add `adjusted_severity` field to each violation when it differs from original

### No Breaking Changes

Existing output fields remain unchanged. New fields (`suppressed_count`, `adjusted_severity`) are additive. Clients that don't use feedback see identical output (suppressed_count: 0, no adjusted_severity fields).

### Architecture

No trait/abstraction needed. The feedback lookup is a single helper function:

```rust
fn get_feedback_adjustment(db: &StateDb, tool: &str, rule_id: &str, entity: &str) -> FeedbackAction {
    // query tool_feedback, count accepts/dismissals, return Suppress/Downgrade/Keep
}
```

Called inline by lint and enforce. Simple function, not a framework.

## Track 2: Infrastructure

### Problem

CI workflows and benchmark scripts exist but benchmarks have never actually run against HermiT/Pellet. The README has no real benchmark numbers. The `benchmark.yml` workflow doesn't download Java JARs.

### JAR Setup

New script `benchmark/reasoner/setup_jars.sh`:
- Downloads OWL API 5.1.20, HermiT 1.4.5.901, Pellet 2.4.0 into `benchmark/reasoner/lib/`
- Uses Maven Central URLs (stable, versioned)
- Idempotent (skips if JARs already exist)
- `lib/` is .gitignored

### benchmark.yml Fixes

- Add JAR download step (runs `setup_jars.sh`)
- Add Pizza correctness benchmark step (currently only runs LUBM)
- Upload both result sets as artifacts

### Run Benchmarks Locally

1. Execute Pizza correctness → capture `results/pizza_comparison.json`
2. Execute LUBM performance at 1K/5K/10K/50K → capture `results/lubm_results.json` + chart
3. Commit results to `benchmark/reasoner/results/`

### README Update

- Tool count 37 → 39
- Add benchmark results table (Pizza pass/fail, LUBM timing)
- Add feedback tools to tools table
- Mention feedback framework in lifecycle section

## File Impact

### Track 1 (Rust — `src/`)
- `src/state.rs` — add tool_feedback table to SCHEMA
- `src/ontology.rs` — modify lint() to check feedback, add suppressed_count
- `src/enforce.rs` — modify enforce() to check feedback, add suppressed_count
- `src/server.rs` — add OntoLintFeedbackInput, OntoEnforceFeedbackInput structs + tool methods
- `src/main.rs` — add LintFeedback, EnforceFeedback CLI subcommands
- `tests/cli_test.rs` — add integration tests for new subcommands

### Track 2 (CI/benchmarks — no Rust changes)
- `benchmark/reasoner/setup_jars.sh` — new
- `benchmark/reasoner/.gitignore` — add lib/
- `.github/workflows/benchmark.yml` — fix JAR download + add Pizza
- `benchmark/reasoner/results/` — committed benchmark results
- `README.md` — benchmark numbers + tool count + feedback mention
- `CLAUDE.md` — add new tools to reference table

### No Overlap
Track 1 touches `src/`. Track 2 touches `benchmark/`, `.github/`, `README.md`, `CLAUDE.md`. Zero file conflicts — safe to implement in parallel or interleaved batches.

## Execution Order

Interleaved batches:

1. **Batch 1:** Track 1 — SQLite table + feedback helper + lint integration + tests
2. **Batch 2:** Track 1 — enforce integration + MCP/CLI tools + tests
3. **Batch 3:** Track 2 — JAR setup + benchmark.yml fix + run benchmarks locally
4. **Batch 4:** Track 2 — README/CLAUDE.md update with results + tool count
