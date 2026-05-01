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

// Severity-adjustment thresholds live in `crate::runtime` (initialised from
// `[feedback]` in config.toml).

/// Check feedback history for a (tool, rule_id, entity) tuple.
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
    let suppress = crate::runtime::feedback_suppress_threshold();
    let downgrade = crate::runtime::feedback_downgrade_threshold();
    if dismiss_count >= suppress {
        return FeedbackAction::Suppress;
    }
    if dismiss_count >= downgrade {
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
