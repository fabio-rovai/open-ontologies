# Feedback Framework + Infrastructure Positioning Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add self-calibrating feedback to lint and enforce tools (2 new MCP tools + CLI subcommands), fix benchmark CI to download Java reasoner JARs, run Pizza/LUBM benchmarks locally, and update README with real numbers.

**Architecture:** Track 1 adds a `tool_feedback` SQLite table and a `get_feedback_adjustment()` helper in a new `src/feedback.rs` module. Lint and enforce call this helper before emitting each issue, suppressing or downgrading based on history. Track 2 adds a JAR download script, fixes `benchmark.yml`, runs benchmarks, and commits results.

**Tech Stack:** Rust (rusqlite, serde_json, clap, rmcp/schemars), Java (OWL API 5.x + HermiT + Pellet), Python (matplotlib), Bash (benchmark harness), GitHub Actions.

---

## Track 1: Feedback Framework

### Task 1: Add tool_feedback table to SQLite schema

**Files:**
- Modify: `src/state.rs:74-85`

**Step 1: Write the table creation SQL**

Add this to the `SCHEMA` const in `src/state.rs`, after the `align_feedback` index (line 84) and before the closing `";` (line 85):

```sql
CREATE TABLE IF NOT EXISTS tool_feedback (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tool TEXT NOT NULL,
    rule_id TEXT NOT NULL,
    entity TEXT NOT NULL,
    accepted INTEGER NOT NULL,
    timestamp TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_tool_feedback ON tool_feedback(tool, rule_id, entity);
```

**Step 2: Run tests to verify schema migration works**

Run: `cargo test --lib -- state`
Expected: PASS (existing tests still work, new table is additive)

**Step 3: Commit**

```bash
git add src/state.rs
git commit -m "feat: add tool_feedback table for lint/enforce self-calibration"
```

---

### Task 2: Create feedback module with helper function

**Files:**
- Create: `src/feedback.rs`
- Modify: `src/lib.rs`

**Step 1: Write the failing test**

Create `src/feedback.rs` with:

```rust
use crate::state::StateDb;

/// What to do with an issue based on feedback history.
#[derive(Debug, PartialEq)]
pub enum FeedbackAction {
    /// Report at original severity
    Keep,
    /// Downgrade severity one level (warning → info)
    Downgrade,
    /// Suppress entirely (omit from output)
    Suppress,
}

const SUPPRESS_THRESHOLD: i64 = 3;
const DOWNGRADE_THRESHOLD: i64 = 2;

/// Check feedback history for a (tool, rule_id, entity) tuple.
/// Returns what action to take on the issue.
pub fn get_feedback_adjustment(db: &StateDb, tool: &str, rule_id: &str, entity: &str) -> FeedbackAction {
    let conn = db.conn();
    let dismiss_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM tool_feedback WHERE tool = ?1 AND rule_id = ?2 AND entity = ?3 AND accepted = 0",
            rusqlite::params![tool, rule_id, entity],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let accept_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM tool_feedback WHERE tool = ?1 AND rule_id = ?2 AND entity = ?3 AND accepted = 1",
            rusqlite::params![tool, rule_id, entity],
            |r| r.get(0),
        )
        .unwrap_or(0);

    if accept_count > 0 {
        return FeedbackAction::Keep;
    }
    if dismiss_count >= SUPPRESS_THRESHOLD {
        return FeedbackAction::Suppress;
    }
    if dismiss_count >= DOWNGRADE_THRESHOLD {
        return FeedbackAction::Downgrade;
    }
    FeedbackAction::Keep
}

/// Record feedback for a lint or enforce issue.
pub fn record_tool_feedback(db: &StateDb, tool: &str, rule_id: &str, entity: &str, accepted: bool) -> anyhow::Result<String> {
    let conn = db.conn();
    conn.execute(
        "INSERT INTO tool_feedback (tool, rule_id, entity, accepted) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![tool, rule_id, entity, accepted as i32],
    )?;
    Ok(serde_json::json!({
        "ok": true,
        "tool": tool,
        "rule_id": rule_id,
        "entity": entity,
        "accepted": accepted,
    })
    .to_string())
}

/// Downgrade a severity string by one level.
pub fn downgrade_severity(severity: &str) -> &str {
    match severity {
        "error" => "warning",
        "warning" => "info",
        _ => severity,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn test_db() -> StateDb {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();
        std::mem::forget(tmp);
        StateDb::open(&path).unwrap()
    }

    #[test]
    fn test_no_feedback_keeps() {
        let db = test_db();
        let action = get_feedback_adjustment(&db, "lint", "missing_label", "http://ex.org/Foo");
        assert_eq!(action, FeedbackAction::Keep);
    }

    #[test]
    fn test_two_dismissals_downgrades() {
        let db = test_db();
        record_tool_feedback(&db, "lint", "missing_label", "http://ex.org/Foo", false).unwrap();
        record_tool_feedback(&db, "lint", "missing_label", "http://ex.org/Foo", false).unwrap();
        let action = get_feedback_adjustment(&db, "lint", "missing_label", "http://ex.org/Foo");
        assert_eq!(action, FeedbackAction::Downgrade);
    }

    #[test]
    fn test_three_dismissals_suppresses() {
        let db = test_db();
        for _ in 0..3 {
            record_tool_feedback(&db, "lint", "missing_label", "http://ex.org/Foo", false).unwrap();
        }
        let action = get_feedback_adjustment(&db, "lint", "missing_label", "http://ex.org/Foo");
        assert_eq!(action, FeedbackAction::Suppress);
    }

    #[test]
    fn test_accept_overrides_dismissals() {
        let db = test_db();
        for _ in 0..5 {
            record_tool_feedback(&db, "lint", "missing_label", "http://ex.org/Foo", false).unwrap();
        }
        record_tool_feedback(&db, "lint", "missing_label", "http://ex.org/Foo", true).unwrap();
        let action = get_feedback_adjustment(&db, "lint", "missing_label", "http://ex.org/Foo");
        assert_eq!(action, FeedbackAction::Keep);
    }

    #[test]
    fn test_different_entities_independent() {
        let db = test_db();
        for _ in 0..3 {
            record_tool_feedback(&db, "lint", "missing_label", "http://ex.org/Foo", false).unwrap();
        }
        let action = get_feedback_adjustment(&db, "lint", "missing_label", "http://ex.org/Bar");
        assert_eq!(action, FeedbackAction::Keep);
    }

    #[test]
    fn test_different_tools_independent() {
        let db = test_db();
        for _ in 0..3 {
            record_tool_feedback(&db, "lint", "missing_label", "http://ex.org/Foo", false).unwrap();
        }
        let action = get_feedback_adjustment(&db, "enforce", "missing_label", "http://ex.org/Foo");
        assert_eq!(action, FeedbackAction::Keep);
    }

    #[test]
    fn test_downgrade_severity() {
        assert_eq!(downgrade_severity("error"), "warning");
        assert_eq!(downgrade_severity("warning"), "info");
        assert_eq!(downgrade_severity("info"), "info");
    }

    #[test]
    fn test_record_feedback() {
        let db = test_db();
        let result = record_tool_feedback(&db, "enforce", "orphan_class", "http://ex.org/Thing", true).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["ok"], true);
        assert_eq!(v["tool"], "enforce");
    }
}
```

**Step 2: Register the module**

Add `pub mod feedback;` to `src/lib.rs` (after `pub mod enforce;`).

**Step 3: Run tests to verify they pass**

Run: `cargo test --lib -- feedback`
Expected: ALL 8 tests PASS

**Step 4: Commit**

```bash
git add src/feedback.rs src/lib.rs
git commit -m "feat: add feedback module with adjustment logic and tests"
```

---

### Task 3: Integrate feedback into lint

**Files:**
- Modify: `src/ontology.rs:99-180`

**Step 1: Write a failing test**

Add to `tests/ontology_test.rs` (or the existing test file for ontology):

```rust
#[test]
fn test_lint_with_feedback_suppression() {
    use open_ontologies::ontology::OntologyService;
    use open_ontologies::state::StateDb;
    use open_ontologies::feedback::record_tool_feedback;

    let tmp = tempfile::NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();
    std::mem::forget(tmp);
    let db = StateDb::open(&path).unwrap();

    let ttl = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Dog a owl:Class .
    "#;

    // Without feedback, missing_label should appear
    let result = OntologyService::lint_with_feedback(ttl, Some(&db)).unwrap();
    let v: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(v["issue_count"].as_u64().unwrap() > 0);
    assert_eq!(v["suppressed_count"].as_u64().unwrap(), 0);

    // Dismiss 3 times
    for _ in 0..3 {
        record_tool_feedback(&db, "lint", "missing_label", "<http://example.org/Dog>", false).unwrap();
    }

    // Now the issue should be suppressed
    let result = OntologyService::lint_with_feedback(ttl, Some(&db)).unwrap();
    let v: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(v["suppressed_count"].as_u64().unwrap() > 0);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test ontology_test test_lint_with_feedback_suppression`
Expected: FAIL — `lint_with_feedback` doesn't exist yet

**Step 3: Add lint_with_feedback method to OntologyService**

In `src/ontology.rs`, add a new method `lint_with_feedback` that wraps the existing lint logic but filters issues through feedback. Keep the existing `lint()` method unchanged (it calls `lint_with_feedback(content, None)` internally):

```rust
/// Lint with optional feedback-based suppression.
pub fn lint_with_feedback(content: &str, db: Option<&crate::state::StateDb>) -> anyhow::Result<String> {
    let store = Store::new()?;
    let reader = Cursor::new(content.as_bytes());
    for quad in RdfParser::from_format(RdfFormat::Turtle).for_reader(reader) {
        store.insert(&quad?)?;
    }

    let mut issues: Vec<serde_json::Value> = Vec::new();
    let mut suppressed_count: u64 = 0;

    // Collect raw issues (same SPARQL queries as before)
    let raw_issues = Self::collect_lint_issues(&store)?;

    for issue in raw_issues {
        let rule_id = issue["type"].as_str().unwrap_or("");
        let entity = issue["entity"].as_str().unwrap_or("");
        let severity = issue["severity"].as_str().unwrap_or("warning");

        if let Some(db) = db {
            use crate::feedback::{get_feedback_adjustment, FeedbackAction, downgrade_severity};
            match get_feedback_adjustment(db, "lint", rule_id, entity) {
                FeedbackAction::Suppress => {
                    suppressed_count += 1;
                    continue;
                }
                FeedbackAction::Downgrade => {
                    let mut adjusted = issue.clone();
                    let new_sev = downgrade_severity(severity);
                    adjusted["severity"] = serde_json::json!(new_sev);
                    adjusted["adjusted_severity"] = serde_json::json!(new_sev);
                    adjusted["original_severity"] = serde_json::json!(severity);
                    issues.push(adjusted);
                    continue;
                }
                FeedbackAction::Keep => {}
            }
        }
        issues.push(issue);
    }

    Ok(serde_json::json!({
        "issues": issues,
        "issue_count": issues.len(),
        "suppressed_count": suppressed_count,
    })
    .to_string())
}
```

Extract the existing SPARQL queries from `lint()` into a helper `collect_lint_issues()` that returns `Vec<serde_json::Value>`. Then make `lint()` delegate:

```rust
pub fn lint(content: &str) -> anyhow::Result<String> {
    Self::lint_with_feedback(content, None)
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --test ontology_test test_lint_with_feedback_suppression`
Expected: PASS

Run: `cargo test`
Expected: ALL tests PASS (existing lint tests unchanged since `lint()` delegates)

**Step 5: Commit**

```bash
git add src/ontology.rs tests/ontology_test.rs
git commit -m "feat: integrate feedback suppression into lint"
```

---

### Task 4: Integrate feedback into enforce

**Files:**
- Modify: `src/enforce.rs:18-48`

**Step 1: Write a failing test**

Add to `tests/enforce_test.rs` (or the existing test file):

```rust
#[test]
fn test_enforce_with_feedback_suppression() {
    use open_ontologies::enforce::Enforcer;
    use open_ontologies::graph::GraphStore;
    use open_ontologies::state::StateDb;
    use open_ontologies::feedback::record_tool_feedback;
    use std::sync::Arc;

    let tmp = tempfile::NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();
    std::mem::forget(tmp);
    let db = StateDb::open(&path).unwrap();
    let graph = Arc::new(GraphStore::new());

    // Load a class with no label (triggers missing_label rule)
    let ttl = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Dog a owl:Class .
    "#;
    graph.load_turtle(ttl).unwrap();

    let enforcer = Enforcer::new(db.clone(), graph.clone());

    // Without feedback, violations should appear
    let result = enforcer.enforce_with_feedback("generic", Some(&db)).unwrap();
    let v: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(v["violations"].as_array().unwrap().len() > 0);
    assert_eq!(v["suppressed_count"].as_u64().unwrap(), 0);

    // Dismiss missing_label for Dog 3 times
    for _ in 0..3 {
        record_tool_feedback(&db, "enforce", "missing_label", "<http://example.org/Dog>", false).unwrap();
    }

    // Now that violation should be suppressed
    let result = enforcer.enforce_with_feedback("generic", Some(&db)).unwrap();
    let v: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(v["suppressed_count"].as_u64().unwrap() > 0);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test enforce_test test_enforce_with_feedback_suppression`
Expected: FAIL — `enforce_with_feedback` doesn't exist

**Step 3: Add enforce_with_feedback method**

In `src/enforce.rs`, add a new method that wraps `enforce()`:

```rust
/// Run enforcement with optional feedback-based suppression.
pub fn enforce_with_feedback(&self, rule_pack: &str, feedback_db: Option<&StateDb>) -> anyhow::Result<String> {
    let mut violations = Vec::new();
    let mut total_rules = 0u32;
    let mut passed_rules = 0u32;

    match rule_pack {
        "generic" => self.run_generic_rules(&mut violations, &mut total_rules, &mut passed_rules),
        "boro" => self.run_boro_rules(&mut violations, &mut total_rules, &mut passed_rules),
        "value_partition" => self.run_value_partition_rules(&mut violations, &mut total_rules, &mut passed_rules),
        _ => {}
    }
    self.run_custom_rules(rule_pack, &mut violations, &mut total_rules, &mut passed_rules);

    let mut suppressed_count: u64 = 0;
    let filtered: Vec<serde_json::Value> = if let Some(db) = feedback_db {
        use crate::feedback::{get_feedback_adjustment, FeedbackAction, downgrade_severity};
        violations.into_iter().filter_map(|mut v| {
            let rule = v["rule"].as_str().unwrap_or("").to_string();
            let entity = v["entity"].as_str().unwrap_or("").to_string();
            let severity = v["severity"].as_str().unwrap_or("warning").to_string();
            match get_feedback_adjustment(db, "enforce", &rule, &entity) {
                FeedbackAction::Suppress => {
                    suppressed_count += 1;
                    None
                }
                FeedbackAction::Downgrade => {
                    let new_sev = downgrade_severity(&severity);
                    v["severity"] = serde_json::json!(new_sev);
                    v["adjusted_severity"] = serde_json::json!(new_sev);
                    v["original_severity"] = serde_json::json!(&severity);
                    Some(v)
                }
                FeedbackAction::Keep => Some(v),
            }
        }).collect()
    } else {
        violations
    };

    let compliance = if total_rules > 0 {
        passed_rules as f64 / total_rules as f64
    } else {
        1.0
    };

    let result = serde_json::json!({
        "rule_pack": rule_pack,
        "violations": filtered,
        "total_rules": total_rules,
        "passed_rules": passed_rules,
        "compliance": compliance,
        "suppressed_count": suppressed_count,
    });

    Ok(result.to_string())
}
```

Make existing `enforce()` delegate:

```rust
pub fn enforce(&self, rule_pack: &str) -> anyhow::Result<String> {
    self.enforce_with_feedback(rule_pack, None)
}
```

**Step 4: Run tests**

Run: `cargo test --test enforce_test test_enforce_with_feedback_suppression`
Expected: PASS

Run: `cargo test`
Expected: ALL tests PASS

**Step 5: Commit**

```bash
git add src/enforce.rs tests/enforce_test.rs
git commit -m "feat: integrate feedback suppression into enforce"
```

---

### Task 5: Register MCP tools for lint_feedback and enforce_feedback

**Files:**
- Modify: `src/server.rs` (add input structs after line ~268, add tool methods after line ~1025)

**Step 1: Add input structs**

After `OntoAlignFeedbackInput` (around line 268), add:

```rust
#[derive(Deserialize, JsonSchema)]
pub struct OntoLintFeedbackInput {
    /// The lint rule ID (e.g. "missing_label", "missing_comment", "missing_domain", "missing_range")
    pub rule_id: String,
    /// The entity IRI that triggered the lint issue
    pub entity: String,
    /// true = this is a real issue, false = dismiss/ignore
    pub accepted: bool,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoEnforceFeedbackInput {
    /// The enforce rule ID (e.g. "orphan_class", "missing_domain", "missing_range", "missing_label", or custom rule ID)
    pub rule_id: String,
    /// The entity IRI that triggered the violation
    pub entity: String,
    /// true = this is a real violation, false = dismiss/override
    pub accepted: bool,
}
```

**Step 2: Add tool methods**

After the `onto_align_feedback` method (around line 1025), add:

```rust
#[tool(name = "onto_lint_feedback", description = "Accept or dismiss a lint issue to improve future lint runs. Dismissed issues are suppressed after 3 dismissals. Stores feedback for self-calibrating severity.")]
async fn onto_lint_feedback(&self, Parameters(input): Parameters<OntoLintFeedbackInput>) -> String {
    match crate::feedback::record_tool_feedback(&self.db, "lint", &input.rule_id, &input.entity, input.accepted) {
        Ok(result) => {
            self.lineage().record(&self.session_id, "LF", "lint_feedback", if input.accepted { "accepted" } else { "dismissed" });
            result
        }
        Err(e) => format!(r#"{{"error":"{}"}}"#, e),
    }
}

#[tool(name = "onto_enforce_feedback", description = "Accept or dismiss an enforce violation to improve future enforce runs. Dismissed violations are suppressed after 3 dismissals. Stores feedback for self-calibrating compliance.")]
async fn onto_enforce_feedback(&self, Parameters(input): Parameters<OntoEnforceFeedbackInput>) -> String {
    match crate::feedback::record_tool_feedback(&self.db, "enforce", &input.rule_id, &input.entity, input.accepted) {
        Ok(result) => {
            self.lineage().record(&self.session_id, "EF", "enforce_feedback", if input.accepted { "accepted" } else { "dismissed" });
            result
        }
        Err(e) => format!(r#"{{"error":"{}"}}"#, e),
    }
}
```

**Step 3: Update onto_lint and onto_enforce to use feedback**

In `onto_lint` (around line 412-423), change the call from `OntologyService::lint(&content)` to `OntologyService::lint_with_feedback(&content, Some(&self.db))`.

In `onto_enforce` (around line 770-776), change from `enforcer.enforce(&input.rule_pack)` to `enforcer.enforce_with_feedback(&input.rule_pack, Some(&self.db))`.

**Step 4: Run tests**

Run: `cargo test`
Expected: ALL tests PASS

**Step 5: Commit**

```bash
git add src/server.rs
git commit -m "feat: register onto_lint_feedback and onto_enforce_feedback MCP tools"
```

---

### Task 6: Add CLI subcommands for lint-feedback and enforce-feedback

**Files:**
- Modify: `src/main.rs`

**Step 1: Add enum variants**

After the `AlignFeedback` variant (around line 194), add in a new section:

```rust
// ─── Feedback ────────────────────────────────────────────────
/// Accept or dismiss a lint issue
LintFeedback {
    /// Lint rule ID (e.g. "missing_label", "missing_comment")
    #[arg(long)]
    rule_id: String,
    /// Entity IRI that triggered the issue
    #[arg(long)]
    entity: String,
    /// Accept the issue as valid
    #[arg(long, default_value_t = false)]
    accept: bool,
    /// Dismiss/ignore the issue
    #[arg(long, default_value_t = false)]
    dismiss: bool,
},
/// Accept or dismiss an enforce violation
EnforceFeedback {
    /// Enforce rule ID (e.g. "orphan_class", "missing_domain")
    #[arg(long)]
    rule_id: String,
    /// Entity IRI that triggered the violation
    #[arg(long)]
    entity: String,
    /// Accept the violation as valid
    #[arg(long, default_value_t = false)]
    accept: bool,
    /// Dismiss/override the violation
    #[arg(long, default_value_t = false)]
    dismiss: bool,
},
```

**Step 2: Add match arms**

After the `Commands::AlignFeedback` match arm (around line 897), add:

```rust
Commands::LintFeedback { rule_id, entity, accept, dismiss } => {
    let (db, _graph) = setup(&cli.data_dir)?;
    let accepted = accept || !dismiss;
    let result = open_ontologies::feedback::record_tool_feedback(&db, "lint", &rule_id, &entity, accepted)
        .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
    output_json_str(&result, cli.pretty);
}
Commands::EnforceFeedback { rule_id, entity, accept, dismiss } => {
    let (db, _graph) = setup(&cli.data_dir)?;
    let accepted = accept || !dismiss;
    let result = open_ontologies::feedback::record_tool_feedback(&db, "enforce", &rule_id, &entity, accepted)
        .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
    output_json_str(&result, cli.pretty);
}
```

**Step 3: Update the existing Lint match arm to use feedback**

Change `Commands::Lint` (around line 399-417) to use `lint_with_feedback`:

```rust
Commands::Lint { input } => {
    use open_ontologies::ontology::OntologyService;
    let (db, _graph) = setup(&cli.data_dir)?;
    let content = if input == "-" {
        let mut buf = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf)?;
        buf
    } else {
        std::fs::read_to_string(&input)?
    };
    let result = OntologyService::lint_with_feedback(&content, Some(&db)).unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
    output_json_str(&result, cli.pretty);
}
```

**Step 4: Update the existing Enforce match arm to use feedback**

Change `Commands::Enforce` (around line 748-757) to use `enforce_with_feedback`:

```rust
Commands::Enforce { pack } => {
    let (db, graph) = setup(&cli.data_dir)?;
    let enforcer = open_ontologies::enforce::Enforcer::new(db.clone(), graph);
    let result = enforcer.enforce_with_feedback(&pack, Some(&db))
        .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
    output_json_str(&result, cli.pretty);
}
```

Note: Check if `output_json_str` exists as a helper — if not, it's the inline `if cli.pretty { ... }` pattern used elsewhere. Use whichever pattern the file uses.

**Step 5: Run tests**

Run: `cargo test`
Expected: ALL tests PASS

**Step 6: Commit**

```bash
git add src/main.rs
git commit -m "feat: add lint-feedback and enforce-feedback CLI subcommands"
```

---

### Task 7: Add CLI integration tests for feedback tools

**Files:**
- Modify: `tests/cli_test.rs`

**Step 1: Add tests**

After the `test_cli_align_feedback` test, add:

```rust
#[test]
fn test_cli_lint_feedback() {
    let dir = tempfile::tempdir().unwrap();
    let out = oo_isolated(&dir)
        .args(["lint-feedback", "--rule-id", "missing_label", "--entity", "<http://example.org/Dog>", "--dismiss"])
        .output().unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("ok"));
    assert!(stdout.contains("lint"));
}

#[test]
fn test_cli_enforce_feedback() {
    let dir = tempfile::tempdir().unwrap();
    let out = oo_isolated(&dir)
        .args(["enforce-feedback", "--rule-id", "orphan_class", "--entity", "<http://example.org/Thing>", "--accept"])
        .output().unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("ok"));
    assert!(stdout.contains("enforce"));
}

#[test]
fn test_cli_lint_suppression_end_to_end() {
    let dir = tempfile::tempdir().unwrap();
    let ttl_path = dir.path().join("test.ttl");
    std::fs::write(&ttl_path, r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Dog a owl:Class .
    "#).unwrap();

    // Lint should report issues initially
    let out = oo_isolated(&dir)
        .args(["lint", ttl_path.to_str().unwrap()])
        .output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("missing_label") || stdout.contains("missing_comment"));

    // Dismiss 3 times
    for _ in 0..3 {
        let out = oo_isolated(&dir)
            .args(["lint-feedback", "--rule-id", "missing_label", "--entity", "<http://example.org/Dog>", "--dismiss"])
            .output().unwrap();
        assert!(out.status.success());
    }

    // Lint should now show suppressed_count > 0
    let out = oo_isolated(&dir)
        .args(["lint", ttl_path.to_str().unwrap()])
        .output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("suppressed_count"));
}
```

**Step 2: Run tests**

Run: `cargo test --test cli_test test_cli_lint_feedback test_cli_enforce_feedback test_cli_lint_suppression_end_to_end`
Expected: ALL PASS

**Step 3: Commit**

```bash
git add tests/cli_test.rs
git commit -m "test: add CLI integration tests for lint/enforce feedback"
```

---

## Track 2: Infrastructure

### Task 8: Create JAR setup script for benchmarks

**Files:**
- Create: `benchmark/reasoner/setup_jars.sh`
- Create: `benchmark/reasoner/.gitignore`

**Step 1: Write the setup script**

```bash
#!/bin/bash
# Download OWL API, HermiT, and Pellet JARs for benchmark comparisons.
# Idempotent — skips if JARs already present.
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
LIB_DIR="$SCRIPT_DIR/lib"
mkdir -p "$LIB_DIR"

MAVEN="https://repo1.maven.org/maven2"

download() {
    local url="$1"
    local dest="$2"
    if [ -f "$dest" ]; then
        echo "  EXISTS: $(basename "$dest")"
        return
    fi
    echo "  DOWNLOAD: $(basename "$dest")"
    curl -fsSL -o "$dest" "$url"
}

echo "=== Setting up Java reasoner JARs ==="

# OWL API 5.1.20
download "$MAVEN/net/sourceforge/owlapi/owlapi-distribution/5.1.20/owlapi-distribution-5.1.20.jar" \
    "$LIB_DIR/owlapi-distribution-5.1.20.jar"

# HermiT 1.4.5.456 (from OWL API-compatible build on Maven Central)
download "$MAVEN/net/sourceforge/owlapi/org.semanticweb.hermit/1.4.5.456/org.semanticweb.hermit-1.4.5.456.jar" \
    "$LIB_DIR/HermiT-1.4.5.456.jar"

# Pellet — openllet fork (actively maintained, Maven Central)
download "$MAVEN/com/github/galigator/openllet/openllet-owlapi/2.6.5/openllet-owlapi-2.6.5.jar" \
    "$LIB_DIR/openllet-owlapi-2.6.5.jar"
download "$MAVEN/com/github/galigator/openllet/openllet-core/2.6.5/openllet-core-2.6.5.jar" \
    "$LIB_DIR/openllet-core-2.6.5.jar"

# SLF4J (required by HermiT/Pellet)
download "$MAVEN/org/slf4j/slf4j-api/2.0.9/slf4j-api-2.0.9.jar" \
    "$LIB_DIR/slf4j-api-2.0.9.jar"
download "$MAVEN/org/slf4j/slf4j-simple/2.0.9/slf4j-simple-2.0.9.jar" \
    "$LIB_DIR/slf4j-simple-2.0.9.jar"

# Guava (Pellet dependency)
download "$MAVEN/com/google/guava/guava/33.0.0-jre/guava-33.0.0-jre.jar" \
    "$LIB_DIR/guava-33.0.0-jre.jar"

echo ""
echo "All JARs in $LIB_DIR"
ls -la "$LIB_DIR"
```

**Step 2: Write .gitignore**

```
lib/
results/
*.class
```

**Step 3: Make setup script executable and test it**

Run: `chmod +x benchmark/reasoner/setup_jars.sh && bash benchmark/reasoner/setup_jars.sh`
Expected: JARs downloaded to `benchmark/reasoner/lib/`

**Step 4: Commit**

```bash
git add benchmark/reasoner/setup_jars.sh benchmark/reasoner/.gitignore
git commit -m "infra: add JAR setup script for HermiT/Pellet benchmarks"
```

---

### Task 9: Fix benchmark.yml to download JARs and run Pizza

**Files:**
- Modify: `.github/workflows/benchmark.yml`

**Step 1: Update the workflow**

Replace the contents of `.github/workflows/benchmark.yml`:

```yaml
name: Benchmark
on:
  workflow_dispatch:

jobs:
  benchmark:
    runs-on: ubuntu-latest
    env:
      FORCE_JAVASCRIPT_ACTIONS_TO_NODE24: true
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - uses: actions/setup-java@v4
        with:
          distribution: 'temurin'
          java-version: '21'
      - uses: actions/setup-python@v5
        with:
          python-version: '3.12'
      - name: Install Python deps
        run: pip install matplotlib
      - name: Build Open Ontologies
        run: cargo build --release
      - name: Download reasoner JARs
        run: bash benchmark/reasoner/setup_jars.sh
      - name: Run Pizza correctness
        run: |
          export OO_BIN=./target/release/open-ontologies
          cd benchmark/reasoner && bash run_pizza_correctness.sh
      - name: Run LUBM performance
        run: |
          export OO_BIN=./target/release/open-ontologies
          cd benchmark/reasoner && bash run_lubm_performance.sh
      - name: Upload results
        uses: actions/upload-artifact@v4
        with:
          name: benchmark-results
          path: benchmark/reasoner/results/
```

**Step 2: Commit**

```bash
git add .github/workflows/benchmark.yml
git commit -m "ci: fix benchmark workflow to download JARs and run Pizza correctness"
```

---

### Task 10: Run benchmarks locally and commit results

**Step 1: Build release binary**

Run: `cd /Users/fabio/projects/open-ontologies && cargo build --release`

**Step 2: Download JARs**

Run: `bash benchmark/reasoner/setup_jars.sh`

**Step 3: Compile Java wrapper**

Run: `cd benchmark/reasoner && javac -cp "lib/*" JavaReasoner.java`

**Step 4: Run Pizza correctness**

Run: `export OO_BIN=./target/release/open-ontologies && cd benchmark/reasoner && bash run_pizza_correctness.sh`
Expected: Comparison output showing pass/fail for each reasoner

**Step 5: Run LUBM performance**

Run: `export OO_BIN=./target/release/open-ontologies && cd benchmark/reasoner && bash run_lubm_performance.sh`
Expected: Timing data for each scale + chart PNG

**Step 6: Record results**

Save the comparison output and timing data. Note the exact numbers — they go in the README.

**Step 7: Commit results**

```bash
git add benchmark/reasoner/results/
git commit -m "bench: add Pizza correctness and LUBM performance results"
```

---

### Task 11: Update README and CLAUDE.md

**Files:**
- Modify: `README.md`
- Modify: `CLAUDE.md`

**Step 1: Update tool count**

In `README.md`, change `37 tools` to `39 tools` in the intro paragraph.

**Step 2: Add feedback tools to tools table**

In the README tools table, add rows:

```markdown
| `onto_lint_feedback` | Accept/dismiss lint issues for self-calibrating severity |
| `onto_enforce_feedback` | Accept/dismiss enforce violations for self-calibrating compliance |
```

**Step 3: Add benchmark results**

Add a Benchmark Results section with the actual numbers from Task 10.

**Step 4: Update CLAUDE.md tool reference**

Add to the tool reference table in `CLAUDE.md`:

```markdown
| `onto_lint_feedback` | To accept/dismiss a lint issue for self-calibrating severity — dismissed issues are suppressed after 3 dismissals |
| `onto_enforce_feedback` | To accept/dismiss an enforce violation for self-calibrating compliance — dismissed violations are suppressed after 3 dismissals |
```

**Step 5: Commit**

```bash
git add README.md CLAUDE.md
git commit -m "docs: update README with 39 tools, feedback tools, and benchmark results"
```

---

## Summary

| Task | Track | What |
|------|-------|------|
| 1 | Feedback | SQLite table |
| 2 | Feedback | Feedback module + 8 unit tests |
| 3 | Feedback | Lint integration |
| 4 | Feedback | Enforce integration |
| 5 | Feedback | MCP tool registration |
| 6 | Feedback | CLI subcommands |
| 7 | Feedback | CLI integration tests |
| 8 | Infra | JAR setup script |
| 9 | Infra | Fix benchmark.yml |
| 10 | Infra | Run benchmarks locally |
| 11 | Both | README + CLAUDE.md update |

**Batches:**
- Batch 1 (Tasks 1-3): SQLite + feedback module + lint integration
- Batch 2 (Tasks 4-7): Enforce integration + MCP/CLI tools + tests
- Batch 3 (Tasks 8-10): Benchmarks
- Batch 4 (Task 11): Documentation
