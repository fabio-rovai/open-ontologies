// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

use std::collections::HashMap;

use open_ontologies::sql_sync::extract_max_watermark;

fn row(key: &str, value: &str) -> HashMap<String, String> {
    HashMap::from([(key.to_string(), value.to_string())])
}

#[test]
fn extract_max_watermark_compares_offset_timestamps_by_instant() {
    let rows = vec![
        row("updated_at", "2026-11-01T01:30:00-04:00"),
        row("updated_at", "2026-11-01T01:15:00-05:00"),
    ];
    // During a daylight-saving fall-back, 01:15 at -05:00 is 06:15Z,
    // later than 01:30 at -04:00 (05:30Z), despite sorting first as text.
    assert_eq!(
        extract_max_watermark(&rows, "updated_at").as_deref(),
        Some("2026-11-01T01:15:00-05:00")
    );
}
