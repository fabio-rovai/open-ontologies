use crate::state::StateDb;

/// What to do with an issue based on feedback history.
#[derive(Debug, PartialEq)]
pub enum FeedbackAction {
    /// Report at original severity
    Keep,
    /// Downgrade severity one level (warning → info)
    Downgrade,
    /// Move out of the issue list into `suppressed[]`, carrying the number of
    /// dismissals that put it there.
    Suppress { dismissals: i64 },
}

// Severity-adjustment thresholds live in `crate::runtime` (initialised from
// `[feedback]` in config.toml).

/// Number of times a (tool, rule_id, entity) tuple has been dismissed.
pub fn dismissal_count(db: &StateDb, tool: &str, rule_id: &str, entity: &str) -> i64 {
    db.conn()
        .query_row(
            "SELECT COUNT(*) FROM tool_feedback WHERE tool = ?1 AND rule_id = ?2 AND entity = ?3 AND accepted = 0",
            rusqlite::params![tool, rule_id, entity],
            |r| r.get(0),
        )
        .unwrap_or(0)
}

/// Check feedback history for a (tool, rule_id, entity) tuple reported at
/// `severity`.
///
/// An `error` is never suppressed and never downgraded. The caller that
/// dismisses issues is the same orchestrator that loops until lint and
/// enforce are clean, so a feedback channel that can waive an error lets the
/// gate weaken itself (issue #325: three dismissals of an error produced
/// "0 issues, 1 suppressed"). Downgrading was rejected as well: error to
/// warning is the same waiver one step slower, and it turns an error into
/// something the next two dismissals can suppress. Removing an error takes
/// changing the ontology or the rule, both of which leave a diff. Feedback
/// keeps calibrating the warning and info tiers, which is what it was built
/// for (missing labels on utility classes, intentional root classes).
pub fn get_feedback_adjustment(db: &StateDb, tool: &str, rule_id: &str, entity: &str, severity: &str) -> FeedbackAction {
    if severity == "error" {
        return FeedbackAction::Keep;
    }
    let dismiss_count = dismissal_count(db, tool, rule_id, entity);
    let accept_count: i64 = db
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM tool_feedback WHERE tool = ?1 AND rule_id = ?2 AND entity = ?3 AND accepted = 1",
            rusqlite::params![tool, rule_id, entity],
            |r| r.get(0),
        )
        .unwrap_or(0);

    if accept_count > 0 {
        return FeedbackAction::Keep;
    }
    let suppress = crate::runtime::feedback_suppress_threshold();
    let downgrade = crate::runtime::feedback_downgrade_threshold();
    if dismiss_count >= suppress {
        return FeedbackAction::Suppress { dismissals: dismiss_count };
    }
    if dismiss_count >= downgrade {
        return FeedbackAction::Downgrade;
    }
    FeedbackAction::Keep
}

/// Split `items` into the issues that stay reported and the ones feedback
/// suppressed. `rule_key` names the field holding the rule id ("type" for
/// lint, "rule" for enforce).
///
/// A suppressed issue is moved, not dropped: its entry in the second list
/// keeps rule, entity, message, original severity and the dismissal count,
/// so a report that reads clean after waivers still says what was waived.
/// Before #325 the only trace was `suppressed_count`. An error that has been
/// dismissed stays in the first list at its own severity and gains a
/// `dismissals` field, so a reviewer can see the orchestrator contested it.
pub fn apply_feedback(
    db: Option<&StateDb>,
    tool: &str,
    rule_key: &str,
    items: Vec<serde_json::Value>,
) -> (Vec<serde_json::Value>, Vec<serde_json::Value>) {
    let Some(db) = db else {
        return (items, Vec::new());
    };
    let mut kept = Vec::new();
    let mut suppressed = Vec::new();
    for mut item in items {
        let rule = item[rule_key].as_str().unwrap_or("").to_string();
        let entity = item["entity"].as_str().unwrap_or("").to_string();
        let severity = item["severity"].as_str().unwrap_or("info").to_string();
        match get_feedback_adjustment(db, tool, &rule, &entity, &severity) {
            FeedbackAction::Suppress { dismissals } => {
                suppressed.push(serde_json::json!({
                    "rule": rule,
                    "entity": entity,
                    "original_severity": severity,
                    "dismissals": dismissals,
                    "message": item["message"].clone(),
                }));
            }
            FeedbackAction::Downgrade => {
                let downgraded = downgrade_severity(&severity);
                item["original_severity"] = serde_json::json!(severity);
                item["adjusted_severity"] = serde_json::json!(downgraded);
                item["severity"] = serde_json::json!(downgraded);
                kept.push(item);
            }
            FeedbackAction::Keep => {
                if severity == "error" {
                    let dismissals = dismissal_count(db, tool, &rule, &entity);
                    if dismissals > 0 {
                        item["dismissals"] = serde_json::json!(dismissals);
                    }
                }
                kept.push(item);
            }
        }
    }
    (kept, suppressed)
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

/// Downgrade a severity string by one level. Errors never reach here (see
/// `get_feedback_adjustment`), so the ladder is warning to info only.
pub fn downgrade_severity(severity: &str) -> &str {
    match severity {
        "warning" => "info",
        _ => severity,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> StateDb {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();
        std::mem::forget(tmp);
        StateDb::open(&path).unwrap()
    }

    #[test]
    fn test_no_feedback_keeps() {
        let db = test_db();
        let action = get_feedback_adjustment(&db, "lint", "missing_label", "http://ex.org/Foo", "warning");
        assert_eq!(action, FeedbackAction::Keep);
    }

    #[test]
    fn test_two_dismissals_downgrades() {
        let db = test_db();
        record_tool_feedback(&db, "lint", "missing_label", "http://ex.org/Foo", false).unwrap();
        record_tool_feedback(&db, "lint", "missing_label", "http://ex.org/Foo", false).unwrap();
        let action = get_feedback_adjustment(&db, "lint", "missing_label", "http://ex.org/Foo", "warning");
        assert_eq!(action, FeedbackAction::Downgrade);
    }

    #[test]
    fn test_three_dismissals_suppresses() {
        let db = test_db();
        for _ in 0..3 {
            record_tool_feedback(&db, "lint", "missing_label", "http://ex.org/Foo", false).unwrap();
        }
        let action = get_feedback_adjustment(&db, "lint", "missing_label", "http://ex.org/Foo", "warning");
        assert_eq!(action, FeedbackAction::Suppress { dismissals: 3 });
    }

    #[test]
    fn test_accept_overrides_dismissals() {
        let db = test_db();
        for _ in 0..5 {
            record_tool_feedback(&db, "lint", "missing_label", "http://ex.org/Foo", false).unwrap();
        }
        record_tool_feedback(&db, "lint", "missing_label", "http://ex.org/Foo", true).unwrap();
        let action = get_feedback_adjustment(&db, "lint", "missing_label", "http://ex.org/Foo", "warning");
        assert_eq!(action, FeedbackAction::Keep);
    }

    #[test]
    fn test_different_entities_independent() {
        let db = test_db();
        for _ in 0..3 {
            record_tool_feedback(&db, "lint", "missing_label", "http://ex.org/Foo", false).unwrap();
        }
        let action = get_feedback_adjustment(&db, "lint", "missing_label", "http://ex.org/Bar", "warning");
        assert_eq!(action, FeedbackAction::Keep);
    }

    #[test]
    fn test_different_tools_independent() {
        let db = test_db();
        for _ in 0..3 {
            record_tool_feedback(&db, "lint", "missing_label", "http://ex.org/Foo", false).unwrap();
        }
        let action = get_feedback_adjustment(&db, "enforce", "missing_label", "http://ex.org/Foo", "warning");
        assert_eq!(action, FeedbackAction::Keep);
    }

    #[test]
    fn test_error_is_never_suppressed_or_downgraded() {
        let db = test_db();
        for _ in 0..10 {
            record_tool_feedback(&db, "lint", "bad_axiom", "http://ex.org/Foo", false).unwrap();
        }
        let action = get_feedback_adjustment(&db, "lint", "bad_axiom", "http://ex.org/Foo", "error");
        assert_eq!(action, FeedbackAction::Keep);
    }

    #[test]
    fn test_apply_feedback_moves_suppressed_and_keeps_errors() {
        let db = test_db();
        for _ in 0..3 {
            record_tool_feedback(&db, "lint", "missing_label", "http://ex.org/W", false).unwrap();
            record_tool_feedback(&db, "lint", "bad_axiom", "http://ex.org/E", false).unwrap();
        }
        let items = vec![
            serde_json::json!({"type": "missing_label", "entity": "http://ex.org/W", "severity": "warning", "message": "w"}),
            serde_json::json!({"type": "bad_axiom", "entity": "http://ex.org/E", "severity": "error", "message": "e"}),
        ];
        let (kept, suppressed) = apply_feedback(Some(&db), "lint", "type", items);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0]["entity"], "http://ex.org/E");
        assert_eq!(kept[0]["severity"], "error");
        assert_eq!(kept[0]["dismissals"], 3);
        assert!(kept[0].get("adjusted_severity").is_none());
        assert_eq!(suppressed.len(), 1);
        assert_eq!(suppressed[0]["rule"], "missing_label");
        assert_eq!(suppressed[0]["entity"], "http://ex.org/W");
        assert_eq!(suppressed[0]["original_severity"], "warning");
        assert_eq!(suppressed[0]["dismissals"], 3);
    }

    #[test]
    fn test_downgrade_severity() {
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
