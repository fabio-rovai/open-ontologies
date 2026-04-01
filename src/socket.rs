//! Unix domain socket adapter for the Tardygrada language.
//!
//! Speaks newline-delimited JSON over `SOCK_STREAM`.
//! Supported actions:
//!   - `ground`            — check if triples exist in the graph store
//!   - `check_consistency` — add triples temporarily and look for contradictions

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tracing::{error, info, warn};

use crate::graph::GraphStore;
use crate::reason::Reasoner;

// ── Wire types ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct Request {
    action: String,
    triples: Vec<Triple>,
}

#[derive(Debug, Deserialize)]
struct Triple {
    s: String,
    p: String,
    o: String,
}

#[derive(Debug, Serialize)]
struct GroundResult {
    status: &'static str,
    confidence: u32,
    evidence_count: u64,
}

#[derive(Debug, Serialize)]
struct GroundResponse {
    results: Vec<GroundResult>,
}

#[derive(Debug, Serialize)]
struct ConsistencyResponse {
    consistent: bool,
    contradiction_count: u64,
    explanation: String,
}

// ── Public entry point ───────────────────────────────────────────────

/// Start listening on the given unix socket path.
///
/// Each accepted connection is handled in a spawned task.  The listener
/// runs until the process is killed or the socket is removed.
pub async fn serve(socket_path: &str, graph: Arc<GraphStore>) -> anyhow::Result<()> {
    // Clean up stale socket file if it exists.
    let _ = std::fs::remove_file(socket_path);

    let listener = UnixListener::bind(socket_path)?;
    info!("Unix socket listening on {socket_path}");

    loop {
        match listener.accept().await {
            Ok((stream, _addr)) => {
                let g = graph.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(stream, &g).await {
                        warn!("Connection error: {e}");
                    }
                });
            }
            Err(e) => {
                error!("Accept error: {e}");
            }
        }
    }
}

// ── Per-connection handler ───────────────────────────────────────────

async fn handle_connection(
    stream: tokio::net::UnixStream,
    graph: &Arc<GraphStore>,
) -> anyhow::Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();

    while let Some(line) = lines.next_line().await? {
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }

        let response = match serde_json::from_str::<Request>(&line) {
            Ok(req) => dispatch(graph, &req),
            Err(e) => serde_json::json!({"error": format!("bad request: {e}")}),
        };

        let mut out = serde_json::to_string(&response)?;
        out.push('\n');
        writer.write_all(out.as_bytes()).await?;
    }

    Ok(())
}

// ── Dispatch ─────────────────────────────────────────────────────────

fn dispatch(graph: &Arc<GraphStore>, req: &Request) -> serde_json::Value {
    match req.action.as_str() {
        "ground" => handle_ground(graph, &req.triples),
        "check_consistency" => handle_check_consistency(graph, &req.triples),
        other => serde_json::json!({"error": format!("unknown action: {other}")}),
    }
}

// ── Ground ───────────────────────────────────────────────────────────

fn handle_ground(graph: &Arc<GraphStore>, triples: &[Triple]) -> serde_json::Value {
    let mut results = Vec::new();

    for t in triples {
        let obj_is_iri = t.o.starts_with("http://")
            || t.o.starts_with("https://")
            || t.o.starts_with("urn:");

        // Count evidence: try IRI form (if valid) and literal form
        let count_query = if obj_is_iri {
            format!(
                "SELECT (COUNT(*) AS ?c) WHERE {{ \
                    {{ <{s}> <{p}> <{o}> }} \
                    UNION \
                    {{ <{s}> <{p}> \"{o}\" }} \
                }}",
                s = t.s, p = t.p, o = t.o,
            )
        } else {
            format!(
                "SELECT (COUNT(*) AS ?c) WHERE {{ <{s}> <{p}> \"{o}\" }}",
                s = t.s, p = t.p, o = t.o,
            )
        };

        let evidence_count = run_count_query(graph, &count_query);

        // Check for contradictions: different value for the same s+p
        let contra_query = if obj_is_iri {
            format!(
                "SELECT (COUNT(*) AS ?c) WHERE {{ \
                    <{s}> <{p}> ?val . \
                    FILTER(?val != <{o}> && ?val != \"{o}\") \
                }}",
                s = t.s, p = t.p, o = t.o,
            )
        } else {
            format!(
                "SELECT (COUNT(*) AS ?c) WHERE {{ \
                    <{s}> <{p}> ?val . \
                    FILTER(?val != \"{o}\") \
                }}",
                s = t.s, p = t.p, o = t.o,
            )
        };

        let contra_count = run_count_query(graph, &contra_query);

        let (status, confidence) = if evidence_count > 0 && contra_count == 0 {
            ("grounded", std::cmp::min(50 + (evidence_count as u32) * 15, 100))
        } else if evidence_count > 0 && contra_count > 0 {
            // Evidence exists but so do contradictions — partial
            ("grounded", std::cmp::min(30 + (evidence_count as u32) * 10, 70))
        } else if contra_count > 0 {
            ("contradicted", 0)
        } else {
            ("unknown", 0)
        };

        results.push(GroundResult {
            status,
            confidence,
            evidence_count,
        });
    }

    serde_json::to_value(GroundResponse { results }).unwrap_or_default()
}

// ── Consistency check ────────────────────────────────────────────────

fn handle_check_consistency(
    graph: &Arc<GraphStore>,
    triples: &[Triple],
) -> serde_json::Value {
    // Build INSERT DATA body
    let mut insert_body = String::new();
    for t in triples {
        insert_body.push_str(&format!("<{}> <{}> ", t.s, t.p));
        // Heuristic: if object looks like a URI, use <>, otherwise literal
        if t.o.starts_with("http://") || t.o.starts_with("https://") || t.o.starts_with("urn:") {
            insert_body.push_str(&format!("<{}> .\n", t.o));
        } else {
            insert_body.push_str(&format!("\"{}\" .\n", t.o));
        }
    }

    let insert_sparql = format!("INSERT DATA {{ {} }}", insert_body);

    // Insert the triples temporarily
    if let Err(e) = graph.sparql_update(&insert_sparql) {
        return serde_json::json!({
            "consistent": false,
            "contradiction_count": 0,
            "explanation": format!("failed to insert triples: {e}")
        });
    }

    // Run OWL-RL reasoning (non-materialising probe)
    let reason_result = Reasoner::run(graph, "owl-rl", false);

    // Count contradictions: owl:sameAs loops, disjointWith violations, etc.
    let contra_query = "SELECT (COUNT(*) AS ?c) WHERE { \
        ?a <http://www.w3.org/2002/07/owl#differentFrom> ?b . \
        ?a <http://www.w3.org/2002/07/owl#sameAs> ?b . \
    }";
    let contradictions = run_count_query(graph, contra_query);

    // Also check for disjointWith violations
    let disjoint_query = "SELECT (COUNT(*) AS ?c) WHERE { \
        ?cls1 <http://www.w3.org/2002/07/owl#disjointWith> ?cls2 . \
        ?x a ?cls1 . \
        ?x a ?cls2 . \
    }";
    let disjoint_violations = run_count_query(graph, disjoint_query);

    let total_contradictions = contradictions + disjoint_violations;

    let explanation = if total_contradictions > 0 {
        let mut parts = Vec::new();
        if contradictions > 0 {
            parts.push(format!("{contradictions} sameAs/differentFrom conflict(s)"));
        }
        if disjoint_violations > 0 {
            parts.push(format!("{disjoint_violations} disjointWith violation(s)"));
        }
        parts.join("; ")
    } else {
        match reason_result {
            Ok(_) => String::new(),
            Err(e) => format!("reasoning error: {e}"),
        }
    };

    // Remove the temporarily inserted triples
    let delete_sparql = format!("DELETE DATA {{ {} }}", insert_body);
    if let Err(e) = graph.sparql_update(&delete_sparql) {
        warn!("Failed to clean up temporary triples: {e}");
    }

    serde_json::to_value(ConsistencyResponse {
        consistent: total_contradictions == 0,
        contradiction_count: total_contradictions,
        explanation,
    })
    .unwrap_or_default()
}

// ── Helpers ──────────────────────────────────────────────────────────

/// Run a SPARQL SELECT that returns a single `?c` count value.
fn run_count_query(graph: &Arc<GraphStore>, query: &str) -> u64 {
    match graph.sparql_select(query) {
        Ok(json_str) => {
            // sparql_select returns {"variables":["c"],"results":[{"c":"\"3\"^^…"}]}
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&json_str) {
                if let Some(row) = v["results"].as_array().and_then(|a| a.first()) {
                    if let Some(val) = row["c"].as_str() {
                        // Oxigraph wraps the value like  "\"3\"^^<…integer>"
                        return parse_sparql_integer(val);
                    }
                }
            }
            0
        }
        Err(e) => {
            warn!("Count query failed: {e}");
            0
        }
    }
}

/// Parse a SPARQL integer result.  Oxigraph returns values like
/// `"3"^^<http://www.w3.org/2001/XMLSchema#integer>`.
fn parse_sparql_integer(raw: &str) -> u64 {
    // Strip surrounding quotes and datatype suffix
    let s = raw
        .trim_start_matches('"')
        .split('"')
        .next()
        .unwrap_or("0");
    s.parse::<u64>().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sparql_integer_variants() {
        assert_eq!(parse_sparql_integer("\"3\"^^<http://www.w3.org/2001/XMLSchema#integer>"), 3);
        assert_eq!(parse_sparql_integer("\"0\"^^<http://www.w3.org/2001/XMLSchema#integer>"), 0);
        assert_eq!(parse_sparql_integer("\"42\""), 42);
        assert_eq!(parse_sparql_integer("bad"), 0);
    }
}
