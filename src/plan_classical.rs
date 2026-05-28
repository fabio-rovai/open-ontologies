//! Fast Downward subprocess wrapper (#50, Planner #45 follow-up).
//!
//! Per the LLM-Modulo convention (Kambhampati arXiv 2402.01817), the
//! classical solver is **client-side** — the server only emits PDDL
//! (`onto_plan_compile_pddl`) and validates returned plans
//! (`onto_plan_validate`). This module provides an optional subprocess
//! wrapper around Fast Downward as a *convenience*: a caller that does have
//! Fast Downward installed locally can ask the server to run it for them
//! and get back the raw sas_plan output.
//!
//! ## Bounded scope (v0.4 wrap)
//!
//! - Subprocess invocation via `std::process::Command`, no embedded planner.
//! - LAMA-first; expose the search-string as a parameter so callers can pick
//!   `"lama-first"` / `"lm-cut"` / `"hmax"` etc.
//! - Returns the raw sas_plan content plus a parsed operator list. The
//!   *identifier round-trip* (PDDL sanitised id → original IRI binding) is
//!   intentionally left to the orchestrator, who already has the registered
//!   schemas in hand.
//!
//! ## Honest behaviour when Fast Downward is missing
//!
//! If the configured binary is not found or fails to execute, the wrapper
//! returns a structured error with `kind = "binary_unavailable"`. It does
//! NOT fall back to a silent stub.

use serde::Serialize;
use std::path::Path;
use std::process::Command;

/// One operator instance in a parsed sas_plan. Operator name + positional
/// args, both still in PDDL-sanitised form (the orchestrator maps args back
/// to original IRIs using the `identifier_map` it kept from
/// `onto_plan_compile_pddl`).
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ParsedOperator {
    pub name: String,
    pub args: Vec<String>,
}

/// Result of running Fast Downward.
#[derive(Clone, Debug, Serialize)]
pub struct FastDownwardResult {
    /// Raw sas_plan content as Fast Downward wrote it.
    pub sas_plan: String,
    /// Parsed operator sequence (LLM-friendly).
    pub operators: Vec<ParsedOperator>,
    /// The `binary` the wrapper actually invoked (after env var resolution).
    pub binary_used: String,
    /// Optional `; cost = N (...)` footer from the sas_plan, if present.
    pub cost_footer: Option<String>,
}

/// Parse a sas_plan text into a list of operator instances. Blank lines and
/// `;`-prefixed comments are skipped; the trailing `; cost = N (...)` footer
/// is recognised separately so callers can show it.
pub fn parse_sas_plan(text: &str) -> (Vec<ParsedOperator>, Option<String>) {
    let mut operators: Vec<ParsedOperator> = Vec::new();
    let mut cost_footer: Option<String> = None;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix(';') {
            let r = rest.trim();
            if r.starts_with("cost") {
                cost_footer = Some(r.to_string());
            }
            continue;
        }
        // Strip the outer parens.
        let inner = trimmed
            .strip_prefix('(')
            .and_then(|s| s.strip_suffix(')'))
            .unwrap_or(trimmed);
        let mut parts = inner.split_whitespace();
        if let Some(name) = parts.next() {
            let args: Vec<String> = parts.map(|s| s.to_string()).collect();
            operators.push(ParsedOperator {
                name: name.to_string(),
                args,
            });
        }
    }
    (operators, cost_footer)
}

/// Resolve the Fast Downward binary path. Order of precedence:
/// 1. Explicit `binary_override` parameter.
/// 2. `FAST_DOWNWARD_BIN` env var.
/// 3. `fast-downward.py` on PATH (default).
fn resolve_binary(binary_override: Option<&str>) -> String {
    if let Some(b) = binary_override.filter(|s| !s.is_empty()) {
        return b.to_string();
    }
    if let Ok(env) = std::env::var("FAST_DOWNWARD_BIN")
        && !env.is_empty()
    {
        return env;
    }
    "fast-downward.py".to_string()
}

/// Invoke Fast Downward on a domain + problem and return the parsed plan.
///
/// `search` is the Fast Downward search-engine string (e.g.
/// `"astar(lmcut())"`, `"lama-first"`). Default `"lama-first"`.
pub fn run_fast_downward(
    domain_pddl: &str,
    problem_pddl: &str,
    binary_override: Option<&str>,
    search: Option<&str>,
) -> anyhow::Result<FastDownwardResult> {
    let binary = resolve_binary(binary_override);
    let search_str = search.unwrap_or("lama-first").to_string();

    // Write PDDL to a working directory under the OS temp dir.
    let tmp_root = std::env::temp_dir().join(format!(
        "oo_fd_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&tmp_root)?;
    let domain_path = tmp_root.join("domain.pddl");
    let problem_path = tmp_root.join("problem.pddl");
    std::fs::write(&domain_path, domain_pddl)?;
    std::fs::write(&problem_path, problem_pddl)?;

    // Fast Downward CLI: `fast-downward.py [--plan-file FILE] DOMAIN PROBLEM
    // --search SEARCH`. We force the plan file so we can reliably read it.
    let plan_path = tmp_root.join("sas_plan");
    let output = Command::new(&binary)
        .arg("--plan-file")
        .arg(&plan_path)
        .arg(&domain_path)
        .arg(&problem_path)
        .arg("--search")
        .arg(&search_str)
        .output();
    let output = match output {
        Ok(o) => o,
        Err(e) => {
            anyhow::bail!(
                "binary_unavailable: failed to execute `{}`: {}. Install Fast Downward and set FAST_DOWNWARD_BIN, or pass `fast_downward_bin`.",
                binary,
                e
            );
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "fast_downward_failed: exit={:?}, stderr={}",
            output.status.code(),
            stderr
        );
    }

    // Fast Downward emits multiple plans for satisficing searches as
    // `sas_plan.1`, `sas_plan.2`, …; we prefer the highest-numbered one
    // ("best so far"), falling back to the unsuffixed path for unit plans.
    let sas_plan_text = read_best_sas_plan(&tmp_root, &plan_path)?;
    let (operators, cost_footer) = parse_sas_plan(&sas_plan_text);

    // Best-effort cleanup; ignore failures.
    let _ = std::fs::remove_dir_all(&tmp_root);

    Ok(FastDownwardResult {
        sas_plan: sas_plan_text,
        operators,
        binary_used: binary,
        cost_footer,
    })
}

/// Read the best (highest-numbered) sas_plan variant under `tmp_root`.
/// Falls back to the unsuffixed `plan_path` if no numbered variants exist.
fn read_best_sas_plan(tmp_root: &Path, plan_path: &Path) -> anyhow::Result<String> {
    let mut best: Option<(u32, std::path::PathBuf)> = None;
    if let Ok(rd) = std::fs::read_dir(tmp_root) {
        for entry in rd.flatten() {
            let name = entry.file_name();
            let name_s = name.to_string_lossy();
            if let Some(suffix) = name_s.strip_prefix("sas_plan.")
                && let Ok(n) = suffix.parse::<u32>()
            {
                let path = entry.path();
                match &best {
                    Some((cur, _)) if *cur >= n => {}
                    _ => best = Some((n, path)),
                }
            }
        }
    }
    let chosen = match best {
        Some((_, p)) => p,
        None => plan_path.to_path_buf(),
    };
    if !chosen.exists() {
        anyhow::bail!(
            "no_plan_emitted: Fast Downward exited 0 but produced no sas_plan file under {:?}",
            tmp_root
        );
    }
    Ok(std::fs::read_to_string(&chosen)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sas_plan_handles_three_operator_plan_with_cost_footer() {
        let txt = "(add_class new_iri)\n\
                   (add_subclass_edge cat new_iri)\n\
                   (add_subclass_edge dog new_iri)\n\
                   ; cost = 3 (unit cost)\n";
        let (ops, cost) = parse_sas_plan(txt);
        assert_eq!(ops.len(), 3);
        assert_eq!(ops[0].name, "add_class");
        assert_eq!(ops[0].args, vec!["new_iri"]);
        assert_eq!(ops[1].name, "add_subclass_edge");
        assert_eq!(ops[1].args, vec!["cat", "new_iri"]);
        assert_eq!(cost.as_deref(), Some("cost = 3 (unit cost)"));
    }

    #[test]
    fn parse_sas_plan_skips_blank_lines_and_comments() {
        let txt = "\n\
                   ; this is a comment line that is not a cost footer\n\
                   (op1 a b)\n\
                   \n\
                   (op2 c)\n\
                   ; trailing free-form\n";
        let (ops, cost) = parse_sas_plan(txt);
        assert_eq!(ops.len(), 2);
        assert_eq!(ops[0].name, "op1");
        assert_eq!(ops[1].name, "op2");
        // No `cost = N` footer in the input → None.
        assert!(cost.is_none());
    }

    #[test]
    fn parse_sas_plan_returns_empty_for_empty_input() {
        let (ops, cost) = parse_sas_plan("");
        assert!(ops.is_empty());
        assert!(cost.is_none());
    }

    #[test]
    fn parse_sas_plan_handles_zero_arg_operators() {
        let (ops, _) = parse_sas_plan("(reset)\n(commit)\n");
        assert_eq!(ops.len(), 2);
        assert!(ops[0].args.is_empty());
        assert_eq!(ops[1].name, "commit");
    }

    #[test]
    fn resolve_binary_prefers_explicit_override() {
        let resolved = resolve_binary(Some("/usr/local/bin/fd"));
        assert_eq!(resolved, "/usr/local/bin/fd");
    }

    #[test]
    fn resolve_binary_defaults_to_fast_downward_py_when_no_inputs() {
        // Take a snapshot of FAST_DOWNWARD_BIN, clear, restore at end.
        // (rustc test runners share env; this is best-effort.)
        let prior = std::env::var("FAST_DOWNWARD_BIN").ok();
        // SAFETY: env mutation is unsafe in 2024 edition; this is test-only.
        unsafe {
            std::env::remove_var("FAST_DOWNWARD_BIN");
        }
        let resolved = resolve_binary(None);
        assert_eq!(resolved, "fast-downward.py");
        if let Some(p) = prior {
            unsafe {
                std::env::set_var("FAST_DOWNWARD_BIN", p);
            }
        }
    }

    #[test]
    fn run_fast_downward_returns_binary_unavailable_when_binary_missing() {
        // Point at a clearly nonexistent binary; expect the structured
        // binary_unavailable error.
        let err = run_fast_downward(
            "(define (domain d))",
            "(define (problem p) (:domain d))",
            Some("/definitely/does/not/exist/fast-downward.bin.xyz"),
            None,
        )
        .expect_err("should error");
        let s = format!("{}", err);
        assert!(
            s.contains("binary_unavailable"),
            "expected binary_unavailable error, got: {}",
            s
        );
    }
}
