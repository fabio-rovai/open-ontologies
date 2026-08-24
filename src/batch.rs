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
#[derive(Debug)]
struct BatchCmd {
    name: String,
    args: Vec<String>,
}

impl BatchRunner {
    pub fn new(db: StateDb, graph: Arc<GraphStore>, pretty: bool) -> Self {
        Self { db, graph, pretty }
    }

    /// Parse input (auto-detect line vs JSON format) and run all commands,
    /// printing each result as it completes.
    /// Returns the process exit code (0 = success, 1 = at least one error).
    pub async fn run(&self, input: &str, bail: bool) -> i32 {
        self.run_each(input, bail, |line| self.print_json(line)).await
    }

    /// Run all commands and collect results into a Vec instead of printing.
    /// Returns (results, exit_code).
    pub async fn run_collect(&self, input: &str, bail: bool) -> (Vec<Value>, i32) {
        let mut results = Vec::new();
        let code = self
            .run_each(input, bail, |line| results.push(line.clone()))
            .await;
        (results, code)
    }

    /// The one driver behind `run` and `run_collect`. They differ only in what
    /// they do with each result, so that is the only thing they pass in: keeping
    /// two copies of the loop meant keeping the envelope, the bail handling and
    /// the exit code in step by hand. A sink rather than a returned Vec because
    /// `run` prints as it goes, and a batch of a hundred loads should not go
    /// silent until the last one finishes.
    async fn run_each<F: FnMut(&Value)>(&self, input: &str, bail: bool, mut on_result: F) -> i32 {
        let commands = match parse_input(input) {
            Ok(cmds) => cmds,
            Err(e) => {
                on_result(&json!({"seq": 0, "command": "parse", "error": e}));
                return 1;
            }
        };

        let mut exit_code = 0;
        for (seq, cmd) in commands.iter().enumerate() {
            let result = self.execute(cmd).await;
            let has_error = result.get("error").is_some();
            on_result(&json!({
                "seq": seq,
                "command": cmd.name,
                "result": result,
            }));

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
            "vocab_check" => self.exec_vocab_check(&cmd.args),
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
            "push" => self.exec_push(&cmd.args).await,
            "ingest" => self.exec_ingest(&cmd.args),
            "drift" => self.exec_drift(&cmd.args),
            "lock" => self.exec_lock(&cmd.args),
            "monitor" => self.exec_monitor(),
            "monitor-clear" => self.exec_monitor_clear(),
            "marketplace" => self.exec_marketplace(&cmd.args).await,
            _ => json!({"error": format!("unknown batch command: '{}'", cmd.name)}),
        }
    }

    // ─── Command implementations ─────────────────────────────────────

    /// `push` was serialized by the proxy and had no arm here, so it was the one
    /// proxy-able command that could not run: it fell through to
    /// `unknown batch command` and exited 1 whenever a daemon was up. Running it
    /// locally instead would be worse than an error, because the store holding
    /// the triples worth pushing is the daemon's.
    async fn exec_push(&self, args: &[String]) -> Value {
        let endpoint = match args.first() {
            Some(e) => e,
            None => return json!({"error": "push requires an endpoint"}),
        };
        let content = match self.graph.serialize("ntriples") {
            Ok(c) => c,
            Err(e) => return json!({"error": e.to_string()}),
        };
        match GraphStore::push_sparql(endpoint, &content).await {
            Ok(msg) => json!({"ok": true, "message": msg}),
            Err(e) => json!({"error": e.to_string()}),
        }
    }

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
        if args.is_empty() {
            return json!({"error": "query requires a SPARQL string. Accepted forms: \
                a line such as `query SELECT ?s WHERE { ?s ?p ?o }`, or JSON \
                {\"command\":\"query\",\"args\":\"SELECT ...\"}, or \
                {\"command\":\"query\",\"args\":[\"SELECT ...\"]}"});
        }
        // Defensive: if a caller supplies a query already split across arguments,
        // rejoin it rather than silently running the first fragment.
        let joined;
        let query = if args.len() == 1 {
            &args[0]
        } else {
            joined = args.join(" ");
            &joined
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
            Ok(counts) => json!({
                "ok": true,
                "triples": counts.triples,
                "statements": counts.statements,
            }),
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

    fn exec_vocab_check(&self, args: &[String]) -> Value {
        let data_path = match args.first() {
            Some(p) => p,
            None => return json!({"error": "vocab_check requires a data file path"}),
        };
        match std::fs::read_to_string(data_path) {
            Ok(data) => {
                let result = crate::vocab_check::check_data_vocab(&self.graph, &data, &[])
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
        let plan_id = Self::flag_value(args, "--plan-id");
        let mode = args
            .iter()
            .find(|a| !a.starts_with("--") && Some(a.as_str()) != plan_id.as_deref())
            .map(|s| s.as_str())
            .unwrap_or("safe");
        let planner = crate::plan::Planner::new(self.db.clone(), self.graph.clone());
        let result = planner.apply_plan(plan_id.as_deref(), mode)
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

    async fn exec_marketplace(&self, args: &[String]) -> Value {
        use crate::marketplace;
        let action = match args.first() {
            Some(a) => a.as_str(),
            None => return json!({"error": "marketplace requires 'list' or 'install'"}),
        };
        match action {
            "list" => {
                let domain = Self::flag_value(args, "--domain");
                let (items, community_error) = marketplace::cli_list(domain.as_deref()).await;
                json!({
                    "count": items.len(),
                    "ontologies": items,
                    "community_registry_error": community_error,
                })
            }
            "install" => {
                let id = match Self::flag_value(args, "--id") {
                    Some(id) => id,
                    None => return json!({"error": "marketplace install requires --id"}),
                };
                let pack = match marketplace::cli_resolve(&id).await {
                    Ok(p) => p,
                    Err(e) => return json!({"error": e}),
                };
                let content = match crate::graph::GraphStore::fetch_url(&pack.url).await {
                    Ok(c) => c,
                    Err(e) => return json!({"error": e.to_string()}),
                };
                match self.graph.load_content_with_base(&content, pack.format, Some(&pack.url)) {
                    Ok(count) => {
                        let stats = self.graph.get_stats().unwrap_or_default();
                        json!({
                            "ok": true,
                            "installed": pack.id,
                            "name": pack.name,
                            "triples_loaded": count,
                            "stats": serde_json::from_str::<serde_json::Value>(&stats).unwrap_or_default(),
                        })
                    }
                    Err(e) => json!({"error": format!("Parse error: {}", e)}),
                }
            }
            _ => json!({"error": format!("Unknown marketplace action: '{}'. Use 'list' or 'install'.", action)}),
        }
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
        // A bare string is the single positional argument. Without this the only
        // object form available was the flag-flattening branch below, which turns
        // {"args":{"path":"x.ttl"}} into ["--path","x.ttl"] and hands "--path" to a
        // command expecting a path. That broke every positional-argument command,
        // not only query.
        let args = if let Some(s) = item["args"].as_str() {
            vec![s.to_string()]
        } else if let Some(obj) = item["args"].as_object() {
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

/// Split a batch line into a command and its arguments.
///
/// Deliberately not `shell_words::split`. POSIX escaping treats `\` as an
/// escape character everywhere, which silently consumed every separator in a
/// Windows path: `plan C:\onto\proposed.ttl` arrived as `C:ontoproposed.ttl`,
/// the file was never found, and the failure surfaced somewhere else entirely
/// — `apply` reporting "No plan found" because the `plan` before it could not
/// read its input.
///
/// In a batch file a backslash is a path separator far more often than an
/// escape, so here it is literal outside quotes, and an escape only inside
/// double quotes and only before `"` or `\`. Single quotes are literal
/// throughout. Quoting still groups arguments containing spaces.
fn split_command_line(line: &str) -> Result<Vec<String>, String> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut started = false;
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        if c.is_whitespace() {
            if started {
                words.push(std::mem::take(&mut word));
                started = false;
            }
            continue;
        }
        started = true;
        match c {
            '\'' => loop {
                match chars.next() {
                    Some('\'') => break,
                    Some(c) => word.push(c),
                    None => return Err("unterminated single quote".to_string()),
                }
            },
            '"' => loop {
                match chars.next() {
                    Some('"') => break,
                    Some('\\') => match chars.peek() {
                        Some('"') | Some('\\') => word.push(chars.next().unwrap()),
                        _ => word.push('\\'),
                    },
                    Some(c) => word.push(c),
                    None => return Err("unterminated double quote".to_string()),
                }
            },
            c => word.push(c),
        }
    }

    if started {
        words.push(word);
    }
    Ok(words)
}

/// Commands whose argument is free text rather than a path or a flag. When such an
/// argument is not quoted it is taken from the line verbatim, because tokenising it
/// destroys it: an unquoted SPARQL query split on whitespace into a dozen fragments
/// and only the first, the bare word "SELECT", ever reached the engine. A quoted
/// argument still goes through the tokeniser, which has always handled it correctly.
const VERBATIM_ARG_COMMANDS: &[&str] = &["query"];

fn parse_lines(input: &str) -> Result<Vec<BatchCmd>, String> {
    let mut cmds = Vec::new();
    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // Verbs whose single argument is free text must not be tokenised. A SPARQL
        // query splits on whitespace into a dozen useless fragments, and taking the
        // first of them handed the engine the bare word "SELECT". For these, the
        // remainder of the line after the verb is the argument, verbatim.
        let (verb, rest) = match line.find(char::is_whitespace) {
            Some(i) => (&line[..i], line[i..].trim()),
            None => (line, ""),
        };
        // A quoted argument was always handled correctly by the tokeniser, and
        // several tests pin that behaviour, so only an UNQUOTED remainder takes the
        // verbatim path. That is the case that was broken: an unquoted query was
        // split on whitespace and only its first fragment reached the engine.
        let rest_is_quoted = rest.starts_with('"') || rest.starts_with('\'');
        if VERBATIM_ARG_COMMANDS.contains(&verb) && !rest.is_empty() && !rest_is_quoted {
            cmds.push(BatchCmd {
                name: verb.to_string(),
                args: vec![rest.to_string()],
            });
            continue;
        }

        let words = split_command_line(line)
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
    fn a_windows_absolute_path_keeps_its_separators() {
        // `shell_words::split` applies POSIX escaping, so every backslash in a
        // Windows path was consumed before the file was ever opened: `oo batch`
        // could not load, plan against, or apply any absolute path on Windows.
        let cmds = parse_lines(r"plan C:\Users\runneradmin\Temp\ttl\proposed.ttl").unwrap();
        assert_eq!(cmds[0].name, "plan");
        assert_eq!(
            cmds[0].args,
            vec![r"C:\Users\runneradmin\Temp\ttl\proposed.ttl"]
        );
    }

    #[test]
    fn a_quoted_argument_keeps_its_spaces() {
        let cmds = parse_lines(r#"load "my ontology.ttl""#).unwrap();
        assert_eq!(cmds[0].args, vec!["my ontology.ttl"]);
    }

    #[test]
    fn a_quoted_windows_path_with_spaces_survives_both_hazards() {
        let cmds = parse_lines(r#"load "C:\Program Files\onto\my file.ttl""#).unwrap();
        assert_eq!(cmds[0].args, vec![r"C:\Program Files\onto\my file.ttl"]);
    }

    #[test]
    fn a_backslash_escapes_a_quote_inside_a_quoted_argument() {
        let cmds = parse_lines(r#"query "he said \"hi\"""#).unwrap();
        assert_eq!(cmds[0].args, vec![r#"he said "hi""#]);
    }

    #[test]
    fn single_quotes_take_their_contents_literally() {
        let cmds = parse_lines(r#"query 'C:\x\y "quoted"'"#).unwrap();
        assert_eq!(cmds[0].args, vec![r#"C:\x\y "quoted""#]);
    }

    #[test]
    fn an_unterminated_quote_is_an_error() {
        let err = parse_lines(r#"load "unfinished"#).unwrap_err();
        assert!(err.contains("quot"), "unexpected error: {err}");
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

#[cfg(test)]
mod batch_query_parsing_tests {
    use super::*;

    // Regression tests for issue #100: a SPARQL query could not be executed in any
    // accepted batch input form.

    #[test]
    fn line_form_keeps_the_query_verbatim() {
        let cmds = parse_lines("query SELECT (COUNT(?s) AS ?n) WHERE { ?s ?p ?o }").unwrap();
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].name, "query");
        assert_eq!(cmds[0].args.len(), 1, "the query must arrive as one argument, not tokenised");
        assert_eq!(cmds[0].args[0], "SELECT (COUNT(?s) AS ?n) WHERE { ?s ?p ?o }");
    }

    #[test]
    fn line_form_preserves_quotes_and_braces_inside_a_query() {
        let q = r#"SELECT ?s WHERE { ?s <http://example.org/p> "a literal with spaces" }"#;
        let cmds = parse_lines(&format!("query {}", q)).unwrap();
        assert_eq!(cmds[0].args[0], q);
    }

    #[test]
    fn json_string_args_is_a_single_positional() {
        let cmds = parse_json(r#"[{"command":"query","args":"SELECT ?s WHERE { ?s ?p ?o }"}]"#).unwrap();
        assert_eq!(cmds[0].args, vec!["SELECT ?s WHERE { ?s ?p ?o }".to_string()]);
    }

    #[test]
    fn json_string_args_works_for_paths_too() {
        // The object form flattens to --key value, which handed "--path" to load.
        let cmds = parse_json(r#"[{"command":"load","args":"data/graph.ttl"}]"#).unwrap();
        assert_eq!(cmds[0].args, vec!["data/graph.ttl".to_string()]);
    }

    #[test]
    fn json_array_args_still_works() {
        let cmds = parse_json(r#"[{"command":"query","args":["SELECT ?s WHERE { ?s ?p ?o }"]}]"#).unwrap();
        assert_eq!(cmds[0].args, vec!["SELECT ?s WHERE { ?s ?p ?o }".to_string()]);
    }

    #[test]
    fn non_verbatim_commands_are_still_tokenised() {
        // Only free-text verbs bypass tokenisation; paths and flags must not.
        let cmds = parse_lines("lint ontology.ttl --strict").unwrap();
        assert_eq!(cmds[0].name, "lint");
        assert_eq!(cmds[0].args, vec!["ontology.ttl".to_string(), "--strict".to_string()]);
    }

    #[test]
    fn windows_paths_survive_as_before() {
        // Guards the backslash behaviour documented on split_command_line.
        let cmds = parse_lines(r"plan C:\onto\proposed.ttl").unwrap();
        assert_eq!(cmds[0].args[0], r"C:\onto\proposed.ttl");
    }

    #[test]
    fn bare_query_verb_yields_a_helpful_error_not_a_panic() {
        let cmds = parse_lines("query").unwrap();
        assert_eq!(cmds[0].name, "query");
        assert!(cmds[0].args.is_empty());
    }
}
