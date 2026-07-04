//! Human-readable rendering of JSON result values for CLI output.

use serde_json::Value;

/// Render a JSON result value as human-readable text.
/// Falls back to pretty-printed JSON for shapes that have no specific rendering.
pub fn render_human(value: &Value) -> String {
    // Error
    if let Some(msg) = value.get("error").and_then(|v| v.as_str()) {
        return format!("Error: {}", msg);
    }

    // SPARQL query results
    if let Some(bindings) = value
        .get("results")
        .and_then(|r| r.get("bindings"))
        .and_then(|b| b.as_array())
    {
        let vars: Vec<&str> = value
            .get("variables")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
            .unwrap_or_default();
        return render_sparql_table(&vars, bindings);
    }

    // Stats shape: has "triples" field
    if value.get("triples").is_some() {
        return render_stats(value);
    }

    // Marketplace list
    if let Some(ontologies) = value.get("ontologies").and_then(|v| v.as_array()) {
        return render_marketplace_list(ontologies);
    }

    // Versions / history list
    if let Some(versions) = value.get("versions").and_then(|v| v.as_array()) {
        if versions.is_empty() {
            return "No saved versions.".into();
        }
        let mut out = String::from("Saved versions:\n");
        for v in versions {
            let label = v.get("label").and_then(|l| l.as_str()).unwrap_or("?");
            let ts = v.get("created_at").and_then(|t| t.as_str()).unwrap_or("");
            out.push_str(&format!("  {}", label));
            if !ts.is_empty() {
                out.push_str(&format!("  ({})", ts));
            }
            out.push('\n');
        }
        return out.trim_end().into();
    }

    // Lint issues
    if let Some(issues) = value.get("issues").and_then(|v| v.as_array()) {
        if issues.is_empty() {
            return "No issues found.".into();
        }
        let mut out = format!("{} issue(s):\n", issues.len());
        for issue in issues {
            let sev = issue.get("severity").and_then(|s| s.as_str()).unwrap_or("warning");
            let msg = issue.get("message").and_then(|m| m.as_str()).unwrap_or("?");
            out.push_str(&format!("  [{}] {}\n", sev, msg));
        }
        return out.trim_end().into();
    }

    // Enforce violations
    if let Some(violations) = value.get("violations").and_then(|v| v.as_array()) {
        if violations.is_empty() {
            return "No violations found.".into();
        }
        let mut out = format!("{} violation(s):\n", violations.len());
        for v in violations {
            let rule = v.get("rule").and_then(|r| r.as_str()).unwrap_or("?");
            let msg = v.get("message").and_then(|m| m.as_str()).unwrap_or("?");
            out.push_str(&format!("  [{}] {}\n", rule, msg));
        }
        return out.trim_end().into();
    }

    // Daemon status
    if let (Some(pid), Some(url)) = (value.get("pid"), value.get("url")) {
        let alive = value.get("alive").and_then(|a| a.as_bool()).unwrap_or(false);
        let status = if alive { "running" } else { "dead" };
        return format!(
            "Daemon {} — PID {} at {}",
            status,
            pid,
            url.as_str().unwrap_or("?")
        );
    }

    // Ok + specific known messages
    if value.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
        // marketplace install: {"ok":true, "installed":"...", "name":"...", "triples_loaded":N}
        if let Some(name) = value.get("name").and_then(|v| v.as_str()) {
            let n = value
                .get("triples_loaded")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            return format!("Installed \"{}\": {} triples loaded.", name, n);
        }

        // load: {"ok":true, "triples_loaded":N, "path":"..."}
        if let Some(n) = value.get("triples_loaded").and_then(|v| v.as_u64()) {
            if let Some(path) = value.get("path").and_then(|v| v.as_str()) {
                return format!("Loaded {} triples from {}.", n, path);
            }
            if let Some(src) = value.get("source").and_then(|v| v.as_str()) {
                return format!("Pulled {} triples from {}.", n, src);
            }
            return format!("Loaded {} triples.", n);
        }

        // save: {"ok":true, "path":"...", "format":"..."}
        if let (Some(path), Some(fmt)) = (
            value.get("path").and_then(|v| v.as_str()),
            value.get("format").and_then(|v| v.as_str()),
        ) {
            return format!("Saved to {} (format: {}).", path, fmt);
        }

        // message only: {"ok":true, "message":"..."}
        if let Some(msg) = value.get("message").and_then(|v| v.as_str()) {
            return msg.to_string();
        }

        // plan result
        if let Some(risk) = value.get("risk_score").and_then(|v| v.as_str()) {
            let added = value
                .get("added_classes")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            let removed = value
                .get("removed_classes")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            return format!(
                "Plan: risk={}, +{} classes, -{} classes.",
                risk, added, removed
            );
        }

        return "OK".into();
    }

    // Status command: {"status":"ok","version":"...","triples_loaded":N}
    if let Some(status) = value.get("status").and_then(|v| v.as_str()) {
        let version = value
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("?");
        let triples = value
            .get("triples_loaded")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        return format!(
            "Status: {} (v{}, {} triples in store)",
            status, version, triples
        );
    }

    // Diff / drift summary
    if let Some(added) = value.get("added") {
        let removed = value.get("removed");
        let a = added.as_array().map(|a| a.len()).unwrap_or(0);
        let r = removed
            .and_then(|v| v.as_array())
            .map(|a| a.len())
            .unwrap_or(0);
        return format!("Diff: +{} triples, -{} triples.", a, r);
    }

    // Count-only results: {"count":N}
    if let (Some(count), 1) = (value.get("count").and_then(|v| v.as_u64()), value.as_object().map(|o| o.len()).unwrap_or(0)) {
        return format!("{} item(s).", count);
    }

    // Fallback: pretty-print JSON
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}

fn render_stats(value: &Value) -> String {
    let triples = value.get("triples").and_then(|v| v.as_u64()).unwrap_or(0);
    let classes = value.get("classes").and_then(|v| v.as_u64()).unwrap_or(0);
    let props = value.get("properties").and_then(|v| v.as_u64()).unwrap_or(0);
    let individuals = value.get("individuals").and_then(|v| v.as_u64()).unwrap_or(0);
    format!(
        "Triples:     {}\nClasses:     {}\nProperties:  {}\nIndividuals: {}",
        triples, classes, props, individuals
    )
}

fn render_sparql_table(vars: &[&str], bindings: &[Value]) -> String {
    if bindings.is_empty() {
        return "No results.".into();
    }
    // Collect column widths
    let mut widths: Vec<usize> = vars.iter().map(|v| v.len()).collect();
    let rows: Vec<Vec<String>> = bindings
        .iter()
        .map(|row| {
            vars.iter()
                .enumerate()
                .map(|(i, var)| {
                    let cell = row
                        .get(var)
                        .and_then(|v| {
                            v.get("value")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string())
                        })
                        .unwrap_or_default();
                    if cell.len() > widths[i] {
                        widths[i] = cell.len();
                    }
                    cell
                })
                .collect()
        })
        .collect();

    let mut out = String::new();
    // Header
    let header: Vec<String> = vars
        .iter()
        .enumerate()
        .map(|(i, v)| format!("{:<width$}", v, width = widths[i]))
        .collect();
    out.push_str(&header.join("  "));
    out.push('\n');
    // Separator
    let sep: Vec<String> = widths.iter().map(|&w| "-".repeat(w)).collect();
    out.push_str(&sep.join("  "));
    out.push('\n');
    // Rows
    for row in &rows {
        let cells: Vec<String> = row
            .iter()
            .enumerate()
            .map(|(i, cell)| format!("{:<width$}", cell, width = widths[i]))
            .collect();
        out.push_str(&cells.join("  "));
        out.push('\n');
    }
    out.trim_end().into()
}

fn render_marketplace_list(ontologies: &[Value]) -> String {
    if ontologies.is_empty() {
        return "No ontologies found.".into();
    }
    let mut out = String::new();
    for o in ontologies {
        let id = o.get("id").and_then(|v| v.as_str()).unwrap_or("?");
        let name = o.get("name").and_then(|v| v.as_str()).unwrap_or("?");
        let desc = o.get("description").and_then(|v| v.as_str()).unwrap_or("");
        let domain = o.get("domain").and_then(|v| v.as_str()).unwrap_or("");
        out.push_str(&format!("  {:20} {}  [{}]\n", id, name, domain));
        if !desc.is_empty() {
            out.push_str(&format!("                       {}\n", desc));
        }
    }
    out.trim_end().into()
}
