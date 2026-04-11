//! Batch mode — run multiple CLI commands against a single shared graph store.
//!
//! Reads commands from a file or stdin (one per line, or JSON array),
//! executes them sequentially with shared state, and outputs NDJSON.

use std::sync::Arc;
use serde_json::{json, Value};

use crate::graph::GraphStore;
use crate::state::StateDb;

/// Holds shared state across batch commands.
pub struct BatchRunner {
    db: StateDb,
    graph: Arc<GraphStore>,
    pretty: bool,
}

/// A parsed batch command with its arguments.
struct BatchCmd {
    name: String,
    args: Vec<String>,
}

impl BatchRunner {
    pub fn new(db: StateDb, graph: Arc<GraphStore>, pretty: bool) -> Self {
        Self { db, graph, pretty }
    }

    /// Parse input (auto-detect line vs JSON format) and run all commands.
    /// Returns the process exit code (0 = success, 1 = at least one error).
    pub async fn run(&self, input: &str, bail: bool) -> i32 {
        let commands = match parse_input(input) {
            Ok(cmds) => cmds,
            Err(e) => {
                let err = json!({"seq": 0, "command": "parse", "error": e});
                self.print_json(&err);
                return 1;
            }
        };

        let mut exit_code = 0;
        for (seq, cmd) in commands.iter().enumerate() {
            let result = self.execute(cmd).await;
            let has_error = result.get("error").is_some();
            let line = json!({
                "seq": seq,
                "command": cmd.name,
                "result": result,
            });
            self.print_json(&line);

            if has_error {
                exit_code = 1;
                if bail {
                    break;
                }
            }
        }
        exit_code
    }

    fn print_json(&self, value: &Value) {
        if self.pretty {
            println!("{}", serde_json::to_string_pretty(value).unwrap());
        } else {
            println!("{}", value);
        }
    }

    async fn execute(&self, cmd: &BatchCmd) -> Value {
        match cmd.name.as_str() {
            "load" => self.exec_load(&cmd.args),
            "save" => self.exec_save(&cmd.args),
            "clear" => self.exec_clear(),
            "stats" => self.exec_stats(),
            "query" => self.exec_query(&cmd.args),
            "validate" => self.exec_validate(&cmd.args),
            "lint" => self.exec_lint(&cmd.args),
            "reason" => self.exec_reason(&cmd.args),
            "shacl" => self.exec_shacl(&cmd.args),
            "diff" => self.exec_diff(&cmd.args),
            "convert" => self.exec_convert(&cmd.args),
            "enforce" => self.exec_enforce(&cmd.args),
            "plan" => self.exec_plan(&cmd.args),
            "apply" => self.exec_apply(&cmd.args),
            "version" => self.exec_version(&cmd.args),
            "history" => self.exec_history(),
            "rollback" => self.exec_rollback(&cmd.args),
            "status" => self.exec_status(),
            "pull" => self.exec_pull(&cmd.args).await,
            "ingest" => self.exec_ingest(&cmd.args),
            "drift" => self.exec_drift(&cmd.args),
            "lock" => self.exec_lock(&cmd.args),
            "monitor" => self.exec_monitor(),
            "monitor-clear" => self.exec_monitor_clear(),
            _ => json!({"error": format!("unknown batch command: '{}'", cmd.name)}),
        }
    }

    // ─── Command implementations ─────────────────────────────────────

    fn exec_load(&self, args: &[String]) -> Value {
        let path = match args.first() {
            Some(p) => p,
            None => return json!({"error": "load requires a file path"}),
        };
        match self.graph.load_file(path) {
            Ok(count) => json!({"ok": true, "triples_loaded": count, "path": path}),
            Err(e) => json!({"error": e.to_string()}),
        }
    }

    fn exec_save(&self, args: &[String]) -> Value {
        let path = match args.first() {
            Some(p) => p,
            None => return json!({"error": "save requires a file path"}),
        };
        let format = Self::flag_value(args, "--format").unwrap_or("turtle".to_string());
        match self.graph.save_file(path, &format) {
            Ok(_) => json!({"ok": true, "path": path, "format": format}),
            Err(e) => json!({"error": e.to_string()}),
        }
    }

    fn exec_clear(&self) -> Value {
        match self.graph.clear() {
            Ok(_) => json!({"ok": true, "message": "Store cleared"}),
            Err(e) => json!({"error": e.to_string()}),
        }
    }

    fn exec_stats(&self) -> Value {
        match self.graph.get_stats() {
            Ok(s) => serde_json::from_str(&s).unwrap_or(json!({"raw": s})),
            Err(e) => json!({"error": e.to_string()}),
        }
    }

    fn exec_query(&self, args: &[String]) -> Value {
        let query = match args.first() {
            Some(q) => q,
            None => return json!({"error": "query requires a SPARQL string"}),
        };
        match self.graph.sparql_select(query) {
            Ok(s) => serde_json::from_str(&s).unwrap_or(json!({"raw": s})),
            Err(e) => json!({"error": e.to_string()}),
        }
    }

    fn exec_validate(&self, args: &[String]) -> Value {
        let input = match args.first() {
            Some(p) => p,
            None => return json!({"error": "validate requires a file path"}),
        };
        match GraphStore::validate_file(input) {
            Ok(count) => json!({"ok": true, "triples": count}),
            Err(e) => json!({"error": e.to_string()}),
        }
    }

    fn exec_lint(&self, args: &[String]) -> Value {
        use crate::ontology::OntologyService;
        let input = match args.first() {
            Some(p) => p,
            None => return json!({"error": "lint requires a file path"}),
        };
        match std::fs::read_to_string(input) {
            Ok(content) => {
                let result = OntologyService::lint_with_feedback(&content, Some(&self.db))
                    .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
                serde_json::from_str(&result).unwrap_or(json!({"raw": result}))
            }
            Err(e) => json!({"error": e.to_string()}),
        }
    }

    fn exec_reason(&self, args: &[String]) -> Value {
        use crate::reason::Reasoner;
        let profile = Self::flag_value(args, "--profile")
            .or_else(|| args.first().cloned())
            .unwrap_or("rdfs".to_string());
        let result = Reasoner::run(&self.graph, &profile, true)
            .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
        serde_json::from_str(&result).unwrap_or(json!({"raw": result}))
    }

    fn exec_shacl(&self, args: &[String]) -> Value {
        use crate::shacl::ShaclValidator;
        let shapes_path = match args.first() {
            Some(p) => p,
            None => return json!({"error": "shacl requires a shapes file path"}),
        };
        match std::fs::read_to_string(shapes_path) {
            Ok(shapes_content) => {
                let result = ShaclValidator::validate(&self.graph, &shapes_content)
                    .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
                serde_json::from_str(&result).unwrap_or(json!({"raw": result}))
            }
            Err(e) => json!({"error": e.to_string()}),
        }
    }

    fn exec_diff(&self, args: &[String]) -> Value {
        use crate::ontology::OntologyService;
        if args.len() < 2 {
            return json!({"error": "diff requires two file paths"});
        }
        let old = match std::fs::read_to_string(&args[0]) {
            Ok(s) => s,
            Err(e) => return json!({"error": format!("reading {}: {}", args[0], e)}),
        };
        let new = match std::fs::read_to_string(&args[1]) {
            Ok(s) => s,
            Err(e) => return json!({"error": format!("reading {}: {}", args[1], e)}),
        };
        let result = OntologyService::diff(&old, &new)
            .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
        serde_json::from_str(&result).unwrap_or(json!({"raw": result}))
    }

    fn exec_convert(&self, args: &[String]) -> Value {
        if args.len() < 2 {
            return json!({"error": "convert requires: <path> --to <format> [--output <path>]"});
        }
        let path = &args[0];
        let to = Self::flag_value(args, "--to").unwrap_or_else(|| {
            if args.len() > 1 { args[1].clone() } else { "turtle".to_string() }
        });
        let output = Self::flag_value(args, "--output");
        let store = GraphStore::new();
        match store.load_file(path) {
            Ok(_) => match store.serialize(&to) {
                Ok(content) => {
                    if let Some(out_path) = output {
                        match std::fs::write(&out_path, &content) {
                            Ok(_) => json!({"ok": true, "path": out_path, "format": to}),
                            Err(e) => json!({"error": e.to_string()}),
                        }
                    } else {
                        json!({"ok": true, "format": to, "content_length": content.len()})
                    }
                }
                Err(e) => json!({"error": e.to_string()}),
            },
            Err(e) => json!({"error": e.to_string()}),
        }
    }

    fn exec_enforce(&self, args: &[String]) -> Value {
        let pack = args.first().map(|s| s.as_str()).unwrap_or("generic");
        let enforcer = crate::enforce::Enforcer::new(self.db.clone(), self.graph.clone());
        let result = enforcer.enforce_with_feedback(pack, Some(&self.db))
            .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
        serde_json::from_str(&result).unwrap_or(json!({"raw": result}))
    }

    fn exec_plan(&self, args: &[String]) -> Value {
        let file = match args.first() {
            Some(p) => p,
            None => return json!({"error": "plan requires a file path"}),
        };
        match std::fs::read_to_string(file) {
            Ok(turtle) => {
                let planner = crate::plan::Planner::new(self.db.clone(), self.graph.clone());
                let result = planner.plan(&turtle)
                    .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
                serde_json::from_str(&result).unwrap_or(json!({"raw": result}))
            }
            Err(e) => json!({"error": e.to_string()}),
        }
    }

    fn exec_apply(&self, args: &[String]) -> Value {
        let mode = args.first().map(|s| s.as_str()).unwrap_or("safe");
        let planner = crate::plan::Planner::new(self.db.clone(), self.graph.clone());
        let result = planner.apply(mode)
            .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
        serde_json::from_str(&result).unwrap_or(json!({"raw": result}))
    }

    fn exec_version(&self, args: &[String]) -> Value {
        use crate::ontology::OntologyService;
        let label = match args.first() {
            Some(l) => l,
            None => return json!({"error": "version requires a label"}),
        };
        let result = OntologyService::save_version(&self.db, &self.graph, label)
            .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
        serde_json::from_str(&result).unwrap_or(json!({"raw": result}))
    }

    fn exec_history(&self) -> Value {
        use crate::ontology::OntologyService;
        let result = OntologyService::list_versions(&self.db)
            .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
        serde_json::from_str(&result).unwrap_or(json!({"raw": result}))
    }

    fn exec_rollback(&self, args: &[String]) -> Value {
        use crate::ontology::OntologyService;
        let label = match args.first() {
            Some(l) => l,
            None => return json!({"error": "rollback requires a label"}),
        };
        let result = OntologyService::rollback_version(&self.db, &self.graph, label)
            .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
        serde_json::from_str(&result).unwrap_or(json!({"raw": result}))
    }

    fn exec_status(&self) -> Value {
        json!({
            "status": "ok",
            "version": env!("CARGO_PKG_VERSION"),
            "triples_loaded": self.graph.triple_count(),
        })
    }

    async fn exec_pull(&self, args: &[String]) -> Value {
        let url = match args.first() {
            Some(u) => u,
            None => return json!({"error": "pull requires a URL"}),
        };
        let is_sparql = args.iter().any(|a| a == "--sparql");
        let content = if is_sparql {
            let q = Self::flag_value(args, "--query")
                .unwrap_or("CONSTRUCT { ?s ?p ?o } WHERE { ?s ?p ?o }".to_string());
            match GraphStore::fetch_sparql(url, &q).await {
                Ok(c) => c,
                Err(e) => return json!({"error": e.to_string()}),
            }
        } else {
            match GraphStore::fetch_url(url).await {
                Ok(c) => c,
                Err(e) => return json!({"error": e.to_string()}),
            }
        };
        match self.graph.load_turtle(&content, None) {
            Ok(count) => json!({"ok": true, "triples_loaded": count, "source": url}),
            Err(e) => json!({"error": format!("Parse error: {}", e)}),
        }
    }

    fn exec_ingest(&self, args: &[String]) -> Value {
        use crate::ingest::DataIngester;
        use crate::mapping::MappingConfig;
        let path = match args.first() {
            Some(p) => p,
            None => return json!({"error": "ingest requires a data file path"}),
        };
        let base = Self::flag_value(args, "--base-iri")
            .unwrap_or("http://example.org/data/".to_string());
        let mapping_path = Self::flag_value(args, "--mapping");

        let rows = match DataIngester::parse_file(path) {
            Ok(r) => r,
            Err(e) => return json!({"error": e.to_string()}),
        };
        if rows.is_empty() {
            return json!({"ok": true, "triples_loaded": 0, "warnings": ["No data rows found"]});
        }

        let mapping_config = if let Some(ref mp) = mapping_path {
            match std::fs::read_to_string(mp) {
                Ok(content) => match serde_json::from_str::<MappingConfig>(&content) {
                    Ok(mc) => mc,
                    Err(e) => return json!({"error": format!("bad mapping: {}", e)}),
                },
                Err(e) => return json!({"error": e.to_string()}),
            }
        } else {
            let headers = DataIngester::extract_headers(&rows);
            MappingConfig::from_headers(&headers, &base, &format!("{}Thing", base))
        };

        let ntriples = mapping_config.rows_to_ntriples(&rows);
        match self.graph.load_ntriples(&ntriples) {
            Ok(count) => json!({"ok": true, "triples_loaded": count, "rows": rows.len()}),
            Err(e) => json!({"error": e.to_string()}),
        }
    }

    fn exec_drift(&self, args: &[String]) -> Value {
        if args.len() < 2 {
            return json!({"error": "drift requires two file paths"});
        }
        let v1 = match std::fs::read_to_string(&args[0]) {
            Ok(s) => s,
            Err(e) => return json!({"error": format!("reading {}: {}", args[0], e)}),
        };
        let v2 = match std::fs::read_to_string(&args[1]) {
            Ok(s) => s,
            Err(e) => return json!({"error": format!("reading {}: {}", args[1], e)}),
        };
        let detector = crate::drift::DriftDetector::new(self.db.clone());
        let result = detector.detect(&v1, &v2)
            .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
        serde_json::from_str(&result).unwrap_or(json!({"raw": result}))
    }

    fn exec_lock(&self, args: &[String]) -> Value {
        if args.is_empty() {
            return json!({"error": "lock requires at least one IRI"});
        }
        let reason = Self::flag_value(args, "--reason").unwrap_or("locked".to_string());
        let planner = crate::plan::Planner::new(self.db.clone(), self.graph.clone());
        let iris: Vec<&str> = args.iter()
            .filter(|a| !a.starts_with("--") && *a != &reason)
            .map(|s| s.as_str())
            .collect();
        for iri in &iris {
            planner.lock_iri(iri, &reason);
        }
        json!({"ok": true, "locked": iris, "reason": reason})
    }

    fn exec_monitor(&self) -> Value {
        let monitor = crate::monitor::Monitor::new(self.db.clone(), self.graph.clone());
        let result = monitor.run_watchers();
        serde_json::to_value(&result).unwrap_or(json!({"error": "serialization failed"}))
    }

    fn exec_monitor_clear(&self) -> Value {
        let monitor = crate::monitor::Monitor::new(self.db.clone(), self.graph.clone());
        monitor.clear_blocked();
        json!({"ok": true, "message": "Monitor block cleared"})
    }

    // ─── Helpers ─────────────────────────────────────────────────────

    /// Extract --flag value from args (e.g. --format turtle → Some("turtle"))
    fn flag_value(args: &[String], flag: &str) -> Option<String> {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1).cloned())
    }
}

/// Parse batch input — auto-detects JSON array vs line-per-command format.
fn parse_input(input: &str) -> Result<Vec<BatchCmd>, String> {
    let trimmed = input.trim();
    if trimmed.starts_with('[') {
        parse_json(trimmed)
    } else {
        parse_lines(trimmed)
    }
}

fn parse_json(input: &str) -> Result<Vec<BatchCmd>, String> {
    let arr: Vec<Value> = serde_json::from_str(input)
        .map_err(|e| format!("invalid JSON: {}", e))?;
    let mut cmds = Vec::new();
    for item in arr {
        let name = item["command"].as_str()
            .ok_or_else(|| "each JSON object must have a \"command\" field".to_string())?
            .to_string();
        let args = if let Some(obj) = item["args"].as_object() {
            let mut flat = Vec::new();
            for (k, v) in obj {
                if v.is_boolean() {
                    if v.as_bool().unwrap_or(false) {
                        flat.push(format!("--{}", k));
                    }
                } else if let Some(s) = v.as_str() {
                    flat.push(format!("--{}", k));
                    flat.push(s.to_string());
                } else {
                    flat.push(format!("--{}", k));
                    flat.push(v.to_string());
                }
            }
            flat
        } else if let Some(arr) = item["args"].as_array() {
            arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()
        } else {
            Vec::new()
        };
        cmds.push(BatchCmd { name, args });
    }
    Ok(cmds)
}

fn parse_lines(input: &str) -> Result<Vec<BatchCmd>, String> {
    let mut cmds = Vec::new();
    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let words = shell_words::split(line)
            .map_err(|e| format!("bad quoting on line '{}': {}", line, e))?;
        if words.is_empty() {
            continue;
        }
        cmds.push(BatchCmd {
            name: words[0].clone(),
            args: words[1..].to_vec(),
        });
    }
    Ok(cmds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_lines() {
        let input = r#"
# comment
load my-ontology.ttl
stats
reason --profile owl-rl
query "SELECT (COUNT(*) AS ?c) WHERE { ?s ?p ?o }"
"#;
        let cmds = parse_lines(input).unwrap();
        assert_eq!(cmds.len(), 4);
        assert_eq!(cmds[0].name, "load");
        assert_eq!(cmds[0].args, vec!["my-ontology.ttl"]);
        assert_eq!(cmds[1].name, "stats");
        assert!(cmds[1].args.is_empty());
        assert_eq!(cmds[2].name, "reason");
        assert_eq!(cmds[2].args, vec!["--profile", "owl-rl"]);
        assert_eq!(cmds[3].name, "query");
        assert_eq!(cmds[3].args[0], "SELECT (COUNT(*) AS ?c) WHERE { ?s ?p ?o }");
    }

    #[test]
    fn test_parse_json() {
        let input = r#"[
            {"command": "load", "args": {"path": "test.ttl"}},
            {"command": "stats"},
            {"command": "reason", "args": {"profile": "owl-rl"}}
        ]"#;
        let cmds = parse_json(input).unwrap();
        assert_eq!(cmds.len(), 3);
        assert_eq!(cmds[0].name, "load");
        assert_eq!(cmds[1].name, "stats");
        assert_eq!(cmds[2].name, "reason");
    }

    #[test]
    fn test_auto_detect_json() {
        let json_input = r#"[{"command": "stats"}]"#;
        let line_input = "stats\nquery \"SELECT * WHERE { ?s ?p ?o }\"";
        assert!(parse_input(json_input).unwrap()[0].name == "stats");
        assert!(parse_input(line_input).unwrap()[0].name == "stats");
    }

    #[test]
    fn test_flag_value() {
        let args: Vec<String> = vec!["file.ttl", "--format", "ntriples", "--output", "out.nt"]
            .into_iter().map(String::from).collect();
        assert_eq!(BatchRunner::flag_value(&args, "--format"), Some("ntriples".to_string()));
        assert_eq!(BatchRunner::flag_value(&args, "--output"), Some("out.nt".to_string()));
        assert_eq!(BatchRunner::flag_value(&args, "--missing"), None);
    }
}
