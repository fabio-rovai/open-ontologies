//! PyWhy/DoWhy backdoor identification subprocess wrapper (#48).
//!
//! Scaffold implementation of the Causal flagship's substantive v0.5 work:
//! a Python subprocess around DoWhy v0.13 that:
//!
//!   1. Builds a causal DAG from the caller-supplied `nodes` + `edges`.
//!   2. Runs DoWhy's `identify_effect` to check whether the do-effect of
//!      `treatment` on `outcome` is identifiable via backdoor adjustment.
//!   3. Returns the adjustment set (the variables to condition on) and an
//!      estimand expression suitable for the certificate's
//!      `identification_proof` field.
//!
//! Per the May 2026 roadmap memo, **we do not port Pearl–Shpitser ID to
//! Rust** — DoWhy is a 15-year-stable Python implementation; wrap it via
//! subprocess like `plan_classical.rs` does for Fast Downward.
//!
//! ## Bounded scope (v0.4 scaffold)
//!
//! - Backdoor identification + adjustment set extraction only (frontdoor /
//!   IV defer to v0.5.x).
//! - Subprocess wrapper + parser + clean errors on missing Python or DoWhy.
//! - **NOT** yet integrated into `certify_action`. The integration point is
//!   marked in `src/civex.rs` and slated for v0.5; for now the structural
//!   proxy (`"structural_only"` assumption) remains the only identifier in
//!   shipped certificates.
//!
//! ## Behaviour when Python or DoWhy is missing
//!
//! Returns a structured error with `kind = "python_unavailable"` or
//! `"pywhy_unavailable"`. CIVeX's calling code is expected to fall back to
//! the structural proxy rather than propagate the failure as a tool error.

use serde::{Deserialize, Serialize};
use std::io::Write;
use std::process::{Command, Stdio};

/// Result of a successful PyWhy identification run.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PyWhyEstimate {
    /// Whether the do-effect `P(outcome | do(treatment))` is identifiable
    /// from the supplied graph via backdoor adjustment.
    pub identifiable: bool,
    /// Variables that constitute a valid backdoor adjustment set. Empty
    /// when `identifiable == false`.
    pub adjustment_set: Vec<String>,
    /// Estimand expression in DoWhy's printed form. Used as the certificate's
    /// `identification_proof` when this scaffold is integrated into CIVeX.
    pub estimand_expression: String,
    /// Raw JSON output from the Python subprocess, retained for audit.
    pub raw_output: String,
}

/// Subprocess input shape — pure JSON; the embedded Python script reads
/// this from stdin.
#[derive(Clone, Debug, Serialize)]
pub struct PyWhyInput<'a> {
    pub nodes: &'a [String],
    pub edges: &'a [(String, String)],
    pub treatment: &'a str,
    pub outcome: &'a str,
}

/// Embedded Python driver. Runs under any `python3 >= 3.9` interpreter that
/// has `dowhy`, `networkx`, `pandas`, `numpy` installed.
pub const PYWHY_PYTHON_DRIVER: &str = r#"
import json
import sys

try:
    inp = json.loads(sys.stdin.read())
except Exception as e:
    print(json.dumps({"error": f"invalid json input: {e}", "kind": "input_parse_failed"}))
    sys.exit(0)

try:
    import networkx as nx  # noqa: F401
    import pandas as pd
    import numpy as np
    from dowhy import CausalModel
except ImportError as e:
    print(json.dumps({"error": str(e), "kind": "pywhy_unavailable"}))
    sys.exit(0)

try:
    nodes = inp.get("nodes", [])
    edges = inp.get("edges", [])
    treatment = inp.get("treatment")
    outcome = inp.get("outcome")
    if not (treatment and outcome and nodes):
        print(json.dumps({"error": "missing treatment / outcome / nodes",
                          "kind": "input_validation_failed"}))
        sys.exit(0)

    # DoWhy's CausalModel requires a DataFrame for its constructor even when
    # we only need graph-level identification. Two rows of zeros are enough
    # to satisfy the constructor; we never .fit() or .estimate().
    df = pd.DataFrame({n: np.zeros(2) for n in nodes})

    # Build a DOT-format graph string (DoWhy accepts this).
    dot_lines = ["digraph {"]
    for n in nodes:
        dot_lines.append(f'  "{n}";')
    for u, v in edges:
        dot_lines.append(f'  "{u}" -> "{v}";')
    dot_lines.append("}")
    graph_dot = "\n".join(dot_lines)

    model = CausalModel(
        data=df, treatment=treatment, outcome=outcome, graph=graph_dot
    )
    identified = model.identify_effect(proceed_when_unidentifiable=True)

    # `identified` exposes get_backdoor_variables() in DoWhy >= 0.10.
    backdoor_set = []
    try:
        backdoor_set = list(identified.get_backdoor_variables() or [])
    except Exception:
        pass

    estimand_expr = ""
    try:
        # Different DoWhy versions print differently; both .estimands and
        # __str__ have been used historically. Try both.
        estimand_expr = identified.estimands.get("backdoor", {}).get("estimand", "")
    except Exception:
        try:
            estimand_expr = str(identified)
        except Exception:
            estimand_expr = ""

    identifiable = bool(backdoor_set) or "identifier_method" in str(identified)

    print(json.dumps({
        "identifiable": identifiable,
        "adjustment_set": backdoor_set,
        "estimand_expression": estimand_expr or f"backdoor: {backdoor_set}",
    }))
except Exception as e:
    print(json.dumps({"error": f"{type(e).__name__}: {e}", "kind": "dowhy_runtime_failed"}))
    sys.exit(0)
"#;

/// Parse the JSON output of the embedded Python driver into a typed result.
/// Returns `Err` on structural errors (driver couldn't produce JSON); maps
/// driver-reported errors to typed `anyhow::Error` with the `kind` preserved.
pub fn parse_pywhy_output(json_str: &str) -> anyhow::Result<PyWhyEstimate> {
    let v: serde_json::Value = serde_json::from_str(json_str)
        .map_err(|e| anyhow::anyhow!("driver_emitted_invalid_json: {}", e))?;
    if let Some(err) = v.get("error").and_then(|e| e.as_str()) {
        let kind = v.get("kind").and_then(|k| k.as_str()).unwrap_or("unknown");
        anyhow::bail!("{}: {}", kind, err);
    }
    let identifiable = v
        .get("identifiable")
        .and_then(|b| b.as_bool())
        .unwrap_or(false);
    let adjustment_set: Vec<String> = v
        .get("adjustment_set")
        .and_then(|a| a.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    let estimand_expression = v
        .get("estimand_expression")
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .to_string();
    Ok(PyWhyEstimate {
        identifiable,
        adjustment_set,
        estimand_expression,
        raw_output: json_str.to_string(),
    })
}

/// Resolve the Python binary. Precedence:
///   1. Explicit `python_override` parameter.
///   2. `PYTHON_BIN` env var.
///   3. `python3` on PATH.
fn resolve_python(python_override: Option<&str>) -> String {
    if let Some(b) = python_override.filter(|s| !s.is_empty()) {
        return b.to_string();
    }
    if let Ok(env) = std::env::var("PYTHON_BIN")
        && !env.is_empty()
    {
        return env;
    }
    "python3".to_string()
}

/// Run PyWhy/DoWhy on a caller-supplied causal DAG.
///
/// Returns a structured `PyWhyEstimate` on success. On failure (binary
/// missing, DoWhy not installed, identification raised) returns a typed
/// `anyhow::Error` whose message starts with the kind tag
/// (`python_unavailable` / `pywhy_unavailable` / `dowhy_runtime_failed`) so
/// the calling code can dispatch.
pub fn run_pywhy_backdoor(
    input: &PyWhyInput<'_>,
    python_override: Option<&str>,
) -> anyhow::Result<PyWhyEstimate> {
    let python = resolve_python(python_override);
    let mut child = Command::new(&python)
        .arg("-c")
        .arg(PYWHY_PYTHON_DRIVER)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            anyhow::anyhow!(
                "python_unavailable: failed to execute `{}`: {}. Install Python 3.9+ and set PYTHON_BIN, or pass `python_bin`.",
                python,
                e
            )
        })?;

    {
        let stdin = child.stdin.as_mut().ok_or_else(|| {
            anyhow::anyhow!("python_io_failed: could not open subprocess stdin")
        })?;
        let payload = serde_json::to_vec(input)?;
        stdin.write_all(&payload)?;
    }

    let output = child.wait_with_output().map_err(|e| {
        anyhow::anyhow!("python_io_failed: subprocess wait failed: {}", e)
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "python_exited_nonzero: exit={:?}, stderr={}",
            output.status.code(),
            stderr
        );
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    parse_pywhy_output(&stdout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_pywhy_output_handles_identifiable_case() {
        let json = r#"{"identifiable": true, "adjustment_set": ["Z1", "Z2"], "estimand_expression": "E[Y | T, Z1, Z2]"}"#;
        let parsed = parse_pywhy_output(json).unwrap();
        assert!(parsed.identifiable);
        assert_eq!(parsed.adjustment_set, vec!["Z1", "Z2"]);
        assert_eq!(parsed.estimand_expression, "E[Y | T, Z1, Z2]");
        assert_eq!(parsed.raw_output, json);
    }

    #[test]
    fn parse_pywhy_output_handles_unidentifiable_case() {
        let json = r#"{"identifiable": false, "adjustment_set": [], "estimand_expression": ""}"#;
        let parsed = parse_pywhy_output(json).unwrap();
        assert!(!parsed.identifiable);
        assert!(parsed.adjustment_set.is_empty());
    }

    #[test]
    fn parse_pywhy_output_maps_pywhy_unavailable_to_typed_error() {
        let json = r#"{"error": "No module named 'dowhy'", "kind": "pywhy_unavailable"}"#;
        let err = parse_pywhy_output(json).expect_err("should error");
        let s = format!("{}", err);
        assert!(s.starts_with("pywhy_unavailable"), "got: {}", s);
    }

    #[test]
    fn parse_pywhy_output_maps_dowhy_runtime_failed_to_typed_error() {
        let json = r#"{"error": "ValueError: bad graph", "kind": "dowhy_runtime_failed"}"#;
        let err = parse_pywhy_output(json).expect_err("should error");
        assert!(format!("{}", err).starts_with("dowhy_runtime_failed"));
    }

    #[test]
    fn parse_pywhy_output_rejects_invalid_json() {
        let err = parse_pywhy_output("not json at all").expect_err("should error");
        assert!(format!("{}", err).contains("driver_emitted_invalid_json"));
    }

    #[test]
    fn resolve_python_prefers_explicit_override() {
        assert_eq!(resolve_python(Some("/usr/local/bin/py3.13")), "/usr/local/bin/py3.13");
    }

    #[test]
    fn resolve_python_defaults_to_python3_when_no_inputs() {
        let prior = std::env::var("PYTHON_BIN").ok();
        // SAFETY: test-only env mutation under cargo test's single-process runtime.
        unsafe { std::env::remove_var("PYTHON_BIN"); }
        let resolved = resolve_python(None);
        assert_eq!(resolved, "python3");
        if let Some(p) = prior {
            unsafe { std::env::set_var("PYTHON_BIN", p); }
        }
    }

    #[test]
    fn run_pywhy_returns_python_unavailable_when_binary_missing() {
        let nodes = vec!["T".to_string(), "Y".to_string(), "Z".to_string()];
        let edges = vec![
            ("Z".to_string(), "T".to_string()),
            ("Z".to_string(), "Y".to_string()),
            ("T".to_string(), "Y".to_string()),
        ];
        let input = PyWhyInput {
            nodes: &nodes,
            edges: &edges,
            treatment: "T",
            outcome: "Y",
        };
        let err = run_pywhy_backdoor(
            &input,
            Some("/definitely/does/not/exist/python.xyz"),
        )
        .expect_err("should error");
        let s = format!("{}", err);
        assert!(s.starts_with("python_unavailable"), "got: {}", s);
    }

    #[test]
    fn embedded_driver_is_non_empty_and_imports_dowhy() {
        // Sanity check on the driver constant.
        assert!(PYWHY_PYTHON_DRIVER.contains("from dowhy import CausalModel"));
        assert!(PYWHY_PYTHON_DRIVER.contains("identify_effect"));
        assert!(PYWHY_PYTHON_DRIVER.contains("pywhy_unavailable"));
    }
}
