//! Human-readable rendering of JSON result values for CLI output.
//!
//! A result is rendered by the renderer registered for the NAME of the command
//! that produced it, and never by guessing from which keys the payload happens
//! to carry. Guessing printed wrong answers, not merely unformatted ones
//! (#350). `diff` reported any two files as identical, because its counts were
//! read as arrays. An undetermined `shacl` run printed "No violations found.",
//! because its empty `violations` array matched the branch written for
//! `enforce`. `validate` printed class counts it never computed, because its
//! `triples` key matched `stats`, and `monitor` printed itself as server status
//! because of its `status` key. A shape nobody had guessed at, like the
//! `defects` report, came out as raw JSON (#345).
//!
//! Registering a renderer for a new command is one arm in [`renderer_for`]. A
//! test in `main.rs` walks the clap command tree and fails for any subcommand
//! that has no arm there and no entry in [`PRINTS_ITS_OWN_TEXT`], so a command
//! cannot ship with `--human` quietly printing JSON.

use serde_json::Value;

/// The whole human-readable text for one command's result.
pub type Renderer = fn(&Value) -> String;

/// How the text for a result with no renderer begins. Public so a test can tell
/// a rendered result from one printed as labelled JSON.
pub const NO_RENDERING: &str = "No human rendering exists for";

/// Subcommands that never hand a JSON value to this module, and so need no
/// renderer. `init` prints its own progress lines and the three server modes
/// print nothing to stdout. `batch` is here because it has no result of its
/// own: under `--human` each result in it is rendered by the renderer of the
/// command its envelope names, exactly as the daemon path does.
pub const PRINTS_ITS_OWN_TEXT: &[&str] = &["init", "serve", "serve-http", "serve-unix", "batch"];

/// The renderer registered for `command`, the name the CLI and the batch
/// envelope give it. A nested subcommand is named by its path, so `daemon
/// status` and the top-level `status` are two commands with two shapes.
///
/// To add one, write a `fn(&Value) -> String` below and add its arm here.
pub fn renderer_for(command: &str) -> Option<Renderer> {
    // The batch runner accepts `vocab_check` as well as `vocab-check`, and its
    // envelope repeats the name as it was written, so both spellings must reach
    // the same renderer.
    let name = command.replace('_', "-");
    let render: Renderer = match name.as_str() {
        // Store and files
        "validate" => render_validate,
        "load" => render_load,
        "save" => render_save,
        "clear" | "push" | "monitor-clear" | "daemon stop" => render_message,
        "stats" => render_stats,
        "query" => render_query,
        "diff" => render_diff,
        "lint" => render_lint,
        "defects" => render_defects,
        "convert" => render_convert,
        "status" => render_status,
        "pull" => render_pull,
        "marketplace" => render_marketplace,
        "import-owl" => render_import_owl,
        // Versioning
        "version" => render_version,
        "history" => render_history,
        "rollback" => render_rollback,
        // Data pipeline
        "map" => render_map,
        "ingest" => render_ingest,
        "sql-ingest" => render_sql_ingest,
        "import-schema" => render_import_schema,
        "extend" => render_extend,
        // Verdicts
        "shacl" => render_shacl,
        "vocab-check" => render_vocab_check,
        "reason" => render_reason,
        "enforce" => render_enforce,
        "unpack" => render_unpack,
        "monitor" => render_monitor,
        // First-order export and the certificate commands
        "fol" => render_fol,
        "rules-import" => render_rules_import,
        "fol-model" => render_fol_model,
        "fol-prove" => render_fol_prove,
        "preserve" => render_preserve,
        "closure-diff" => render_closure_diff,
        "trace-label" => render_trace_label,
        // Lifecycle
        "plan" => render_plan,
        "compile-exceptions" => crate::exceptions::render_human,
        "apply" => render_apply,
        "lock" => render_lock,
        "drift" => render_drift,
        "lineage" => render_lineage,
        // Alignment and feedback
        "align" => render_align,
        "crosswalk-probe" | "contradiction-probe" => render_crosswalk_probe,
        "align-feedback" => render_align_feedback,
        "lint-feedback" | "enforce-feedback" => render_tool_feedback,
        // Clinical
        "crosswalk" => render_crosswalk,
        "enrich" => render_enrich,
        "validate-clinical" => render_validate_clinical,
        // Daemon management
        "daemon start" => render_daemon_start,
        "daemon status" => render_daemon_status,
        _ => return None,
    };
    Some(render)
}

/// Whether `command` is accounted for: it has a renderer, or it never hands
/// this module a value to render.
pub fn has_human_renderer(command: &str) -> bool {
    renderer_for(command).is_some() || PRINTS_ITS_OWN_TEXT.contains(&command)
}

/// Render a result as human-readable text, by the command that produced it.
///
/// `None` means the producer did not say what it was, which no path in this
/// crate does any longer: the one-shot CLI passes its subcommand and the batch
/// envelope names the command behind every result. Such a value is printed as
/// labelled JSON rather than matched against another command's shape, because
/// matching by shape is how a successful `daemon start` once printed "Daemon
/// dead" and how the wrong answers in #350 were printed.
pub fn render_human_for(command: Option<&str>, value: &Value) -> String {
    if let Some(msg) = value.get("error").and_then(Value::as_str) {
        return format!("Error: {}", msg);
    }
    match command.and_then(renderer_for) {
        Some(render) => render(value),
        None => unrendered(command, value),
    }
}

/// A result nobody has written a rendering for: its JSON, said to be JSON.
fn unrendered(command: Option<&str>, value: &Value) -> String {
    let json = serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string());
    match command {
        Some(c) => format!("{NO_RENDERING} `{c}` output; its JSON follows.\n{json}"),
        None => format!("{NO_RENDERING} this output; its JSON follows.\n{json}"),
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────

/// A JSON value as display text: a string without its quotes, `null` or an
/// empty list as "none", and a number or boolean as written.
fn text(v: &Value) -> String {
    match v {
        Value::Null => "none".to_string(),
        Value::String(s) => s.clone(),
        Value::Array(items) if items.is_empty() => "none".to_string(),
        Value::Array(items) => items.iter().map(text).collect::<Vec<_>>().join(", "),
        Value::Object(map) => map
            .iter()
            .map(|(k, x)| format!("{k}={}", text(x)))
            .collect::<Vec<_>>()
            .join(", "),
        other => other.to_string(),
    }
}

/// A triple held as a `[s, p, o]` array, or as one string, on one line.
fn triple_text(v: &Value) -> String {
    match v.as_array() {
        Some(terms) => terms.iter().map(text).collect::<Vec<_>>().join(" "),
        None => text(v),
    }
}

/// The array under `key`, or nothing when there is none.
fn items<'a>(v: &'a Value, key: &str) -> &'a [Value] {
    v.get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

/// The three-valued verdict as a word. `null` is "undetermined" and never
/// "true": an unanswered question rendered as a pass is the vacuous
/// conformance the undetermined verdict exists to prevent.
fn verdict(v: &Value) -> &'static str {
    match v.as_bool() {
        Some(true) => "true",
        Some(false) => "false",
        None => "undetermined",
    }
}

/// Append one indented line per item.
fn push_lines<T>(out: &mut String, items: &[T], line: impl Fn(&T) -> String) {
    for item in items {
        out.push_str("\n  ");
        out.push_str(&line(item));
    }
}

/// Append the payload's `warning` and every entry of its `warnings`.
fn push_warnings(out: &mut String, v: &Value) {
    let single = v.get("warning").and_then(Value::as_str);
    let listed = items(v, "warnings").iter().filter_map(Value::as_str);
    for w in single.into_iter().chain(listed) {
        out.push_str("\nWarning: ");
        out.push_str(w);
    }
}

/// A result nested inside another command's result, rendered by its own
/// renderer and indented under a heading.
fn push_nested(out: &mut String, heading: &str, v: &Value, render: Renderer) {
    let body = match v.get("error").and_then(Value::as_str) {
        Some(msg) => format!("Error: {msg}"),
        None => render(v),
    };
    out.push('\n');
    out.push_str(heading);
    for line in body.lines() {
        out.push_str("\n  ");
        out.push_str(line);
    }
}

// ─── Store and files ─────────────────────────────────────────────────────

/// `validate` parses and counts, and that is all, so the two counts the parse
/// produced are all it prints. It used to reach the `stats` rendering through
/// its `triples` key and print class, property and individual counts it never
/// computed, as zeros.
fn render_validate(v: &Value) -> String {
    format!(
        "Valid RDF: {} distinct triple(s) from {} statement(s).",
        text(&v["triples"]),
        text(&v["statements"])
    )
}

fn render_load(v: &Value) -> String {
    let mut out = format!(
        "Loaded {} triple(s) from {}.",
        text(&v["triples_loaded"]),
        text(&v["path"])
    );
    push_warnings(&mut out, v);
    out
}

fn render_save(v: &Value) -> String {
    format!("Saved to {} (format: {}).", text(&v["path"]), text(&v["format"]))
}

/// `convert` with `--output` reports where it wrote, and an OFN conversion
/// also reports what it could not carry across.
fn render_convert(v: &Value) -> String {
    let mut out = render_save(v);
    if let Some(n) = v.get("axioms_written") {
        out.push_str(&format!(
            "\n{} axiom(s) written; {} of {} triple(s) read were exported.",
            text(n),
            text(&v["triples_exported"]),
            text(&v["triples_read"])
        ));
        for (key, label) in [
            ("constructs_not_exported", "Not exported"),
            ("constructs_not_round_trippable", "Not round-trippable"),
        ] {
            let list = items(v, key);
            if !list.is_empty() {
                out.push_str(&format!("\n{label}: {} construct(s)", list.len()));
                push_lines(&mut out, list, text);
            }
        }
    }
    out
}

fn render_message(v: &Value) -> String {
    match v.get("message").and_then(Value::as_str) {
        Some(m) => m.to_string(),
        None => "OK".to_string(),
    }
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

/// One command, three answers: a SELECT table, an ASK boolean, or the triples
/// a CONSTRUCT or DESCRIBE built. Which one came back is a property of the
/// query that was asked, so reading it off the payload is reading the answer
/// and not guessing at the command.
fn render_query(v: &Value) -> String {
    if let Some(rows) = sparql_rows(v) {
        let vars: Vec<&str> = items(v, "variables")
            .iter()
            .filter_map(Value::as_str)
            .collect();
        return render_sparql_table(&vars, rows);
    }
    if let Some(answer) = v.get("result").and_then(Value::as_bool) {
        return format!("ASK: {answer}");
    }
    // One N-Triples line per triple, so the text is itself a document that can
    // be read back. The `stats` rendering this used to reach printed
    // "Triples: 0" over a CONSTRUCT that had built triples.
    if let Some(triples) = v.get("triples").and_then(Value::as_array) {
        if triples.is_empty() {
            return "No triples.".into();
        }
        return triples
            .iter()
            .map(|t| {
                format!(
                    "{} {} {} .",
                    text(&t["subject"]),
                    text(&t["predicate"]),
                    text(&t["object"])
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
    }
    unrendered(Some("query"), v)
}

/// `added` and `removed` are COUNTS and the triples are listed beside them.
/// Reading the counts as arrays, as the shape-guessing renderer did, found no
/// array and printed `+0 -0` for any two files whatever they held.
fn render_diff(v: &Value) -> String {
    let mut out = format!(
        "Diff: +{} triple(s), -{} triple(s).",
        text(&v["added"]),
        text(&v["removed"])
    );
    if v.get("exact").and_then(Value::as_bool) == Some(false) {
        out.push_str("\nApproximate: ");
        out.push_str(&text(&v["approximate_reason"]));
    }
    for (key, sign) in [("added_triples", '+'), ("removed_triples", '-')] {
        let mut lines: Vec<&str> = items(v, key).iter().filter_map(Value::as_str).collect();
        lines.sort_unstable();
        push_lines(&mut out, &lines, |t| format!("{sign} {t}"));
    }
    out
}

fn render_lint(v: &Value) -> String {
    let issues = items(v, "issues");
    let mut out = if issues.is_empty() {
        "No issues found.".to_string()
    } else {
        format!("{} issue(s):", issues.len())
    };
    push_lines(&mut out, issues, |i| {
        format!(
            "[{}] {}",
            i["severity"].as_str().unwrap_or("warning"),
            text(&i["message"])
        )
    });
    let suppressed = v.get("suppressed_count").and_then(Value::as_u64).unwrap_or(0);
    if suppressed > 0 {
        out.push_str(&format!("\n{suppressed} more suppressed by recorded feedback."));
    }
    out
}

/// One line for one defect: severity, kind, the class or property it is
/// about, the rest of the row in parentheses, and why it matters. Shared with
/// `plan`, which lists the defects a change introduces in the same form.
fn defect_line(d: &Value, kind_width: usize) -> String {
    // The row names its subject `class` or `property`; every other binding is
    // detail (the ancestor, the disjoint pair, the inverse).
    const NOT_DETAIL: [&str; 5] = ["kind", "severity", "explanation", "class", "property"];
    let subject = d
        .get("class")
        .or_else(|| d.get("property"))
        .map(text)
        .unwrap_or_default();
    let detail: Vec<String> = d
        .as_object()
        .into_iter()
        .flatten()
        .filter(|(k, _)| !NOT_DETAIL.contains(&k.as_str()))
        .map(|(k, x)| format!("{k} {}", text(x)))
        .collect();
    let mut line = format!(
        "{:<7} {:<kind_width$} {}",
        text(&d["severity"]),
        text(&d["kind"]),
        subject
    );
    if !detail.is_empty() {
        line.push_str(&format!("  ({})", detail.join(", ")));
    }
    line.push_str("  ");
    line.push_str(&text(&d["explanation"]));
    line
}

fn kind_width(defects: &[Value]) -> usize {
    defects
        .iter()
        .filter_map(|d| d["kind"].as_str())
        .map(str::len)
        .max()
        .unwrap_or(0)
}

/// The count line first, then one line per defect (#345). A clean report says
/// how many kinds were looked for, because "no defects" over no checks and
/// over eight are different statements.
fn render_defects(v: &Value) -> String {
    let checked = items(v, "kinds_checked").len();
    let total = v.get("defect_count").and_then(Value::as_u64).unwrap_or(0);
    if total == 0 {
        return format!("No defects ({checked} kinds checked).");
    }
    let sev = &v["severity_counts"];
    let mut out = format!(
        "{total} defect(s) ({} error, {} warning, {} info) in {} class(es) and {} property(ies); {checked} kinds checked.",
        text(&sev["error"]),
        text(&sev["warning"]),
        text(&sev["info"]),
        text(&v["affected_classes"]),
        text(&v["affected_properties"]),
    );
    let defects = items(v, "defects");
    let width = kind_width(defects);
    push_lines(&mut out, defects, |d| defect_line(d, width));
    if let Some(cut) = v["truncated"].as_object().filter(|m| !m.is_empty()) {
        let kinds: Vec<String> = cut
            .iter()
            .map(|(k, n)| format!("{k} ({} in all)", text(n)))
            .collect();
        out.push_str(&format!(
            "\nListed at most {} per kind, so these were cut short: {}. Pass --limit 0 to list every row.",
            text(&v["limit_per_kind"]),
            kinds.join(", ")
        ));
    }
    out
}

fn render_status(v: &Value) -> String {
    format!(
        "Status: {} (v{}, {} triples in store)",
        text(&v["status"]),
        text(&v["version"]),
        text(&v["triples_loaded"])
    )
}

fn render_pull(v: &Value) -> String {
    format!(
        "Pulled {} triple(s) from {}.",
        text(&v["triples_loaded"]),
        text(&v["source"])
    )
}

/// `marketplace list` and `marketplace install` answer under one command name,
/// so the action's own answer is told apart here.
fn render_marketplace(v: &Value) -> String {
    if let Some(list) = v.get("ontologies").and_then(Value::as_array) {
        let mut out = render_marketplace_list(list);
        if let Some(e) = v.get("community_registry_error").and_then(Value::as_str) {
            out.push_str("\nCommunity registry unavailable: ");
            out.push_str(e);
        }
        return out;
    }
    if v.get("installed").is_some() {
        let stats = &v["stats"];
        return format!(
            "Installed \"{}\": {} triple(s) loaded ({} classes, {} properties).",
            text(&v["name"]),
            text(&v["triples_loaded"]),
            text(&stats["classes"]),
            text(&stats["properties"])
        );
    }
    unrendered(Some("marketplace"), v)
}

fn render_import_owl(v: &Value) -> String {
    let mut out = format!(
        "Imported {} document(s) through owl:imports.",
        text(&v["imported"])
    );
    push_lines(&mut out, items(v, "urls"), text);
    out
}

// ─── Versioning ──────────────────────────────────────────────────────────

fn render_version(v: &Value) -> String {
    format!(
        "Saved version \"{}\" ({} triple(s)).",
        text(&v["label"]),
        text(&v["triple_count"])
    )
}

fn render_history(v: &Value) -> String {
    let versions = items(v, "versions");
    if versions.is_empty() {
        return "No saved versions.".into();
    }
    let mut out = String::from("Saved versions:");
    push_lines(&mut out, versions, |ver| {
        let mut line = text(&ver["label"]);
        if let Some(ts) = ver.get("created_at").and_then(Value::as_str) {
            line.push_str(&format!("  ({ts})"));
        }
        line
    });
    out
}

fn render_rollback(v: &Value) -> String {
    format!(
        "Rolled back to \"{}\": {} triple(s) restored.",
        text(&v["label"]),
        text(&v["triples_restored"])
    )
}

// ─── Data pipeline ───────────────────────────────────────────────────────

fn render_map(v: &Value) -> String {
    if let Some(saved) = v.get("saved") {
        return format!("Mapping saved to {}.", text(saved));
    }
    let m = &v["suggested_mapping"];
    let mut out = format!("Data fields: {}", text(&v["data_fields"]));
    out.push_str(&format!(
        "\nOntology in the store: {} class(es), {} property(ies).",
        items(v, "ontology_classes").len(),
        items(v, "ontology_properties").len()
    ));
    out.push_str(&format!(
        "\nSuggested mapping: one {} per row, identified by `{}`, under {}",
        text(&m["class"]),
        text(&m["id_field"]),
        text(&m["base_iri"])
    ));
    push_lines(&mut out, items(m, "mappings"), |f| {
        format!(
            "{} -> {} ({})",
            text(&f["field"]),
            text(&f["predicate"]),
            text(&f["datatype"])
        )
    });
    out
}

fn render_ingest(v: &Value) -> String {
    let mut out = match v.get("rows") {
        Some(rows) => format!(
            "Ingested {} row(s): {} triple(s) loaded.",
            text(rows),
            text(&v["triples_loaded"])
        ),
        None => format!("{} triple(s) loaded.", text(&v["triples_loaded"])),
    };
    push_warnings(&mut out, v);
    out
}

fn render_sql_ingest(v: &Value) -> String {
    let mut out = format!(
        "Ingested {} row(s) through {}: {} triple(s) loaded.",
        text(&v["rows_processed"]),
        text(&v["driver"]),
        text(&v["triples_loaded"])
    );
    if let Some(n) = v.get("mapping_fields") {
        out.push_str(&format!("\n{} field(s) mapped.", text(n)));
    }
    push_warnings(&mut out, v);
    out
}

fn render_import_schema(v: &Value) -> String {
    format!(
        "Imported {} table(s) through {} as {} class(es): {} triple(s) under {}.",
        text(&v["tables"]),
        text(&v["driver"]),
        text(&v["classes"]),
        text(&v["triples"]),
        text(&v["base_iri"])
    )
}

/// `extend` runs ingest, then SHACL and reasoning when asked, and each nested
/// result is rendered by that command's own renderer. Printing only the load
/// line, as before, dropped an undetermined SHACL verdict on the floor.
fn render_extend(v: &Value) -> String {
    let mut out = format!(
        "Ingested {} row(s): {} triple(s) loaded.",
        text(&v["rows"]),
        text(&v["triples_loaded"])
    );
    if v["shacl"].is_object() {
        push_nested(&mut out, "SHACL:", &v["shacl"], render_shacl);
    }
    if v["reason"].is_object() {
        push_nested(&mut out, "Reasoning:", &v["reason"], render_reason);
    }
    out
}

// ─── Verdicts ────────────────────────────────────────────────────────────

/// The verdict first, in the three words the exit code also distinguishes,
/// then the warning that says why a verdict is undetermined, then the
/// violations. An undetermined run used to print "No violations found.",
/// which reads as a pass over a run that checked nothing.
fn render_shacl(v: &Value) -> String {
    let mut out = format!("conforms: {}", verdict(&v["conforms"]));
    if v.get("verified").and_then(Value::as_bool) == Some(true) {
        out.push_str(&format!(" (verified: {})", text(&v["theorem"])));
    }
    push_warnings(&mut out, v);
    if let Some(why) = v.get("undetermined").and_then(Value::as_str) {
        out.push_str("\nUndetermined: ");
        out.push_str(why);
    }
    if let Some(note) = v.get("note").and_then(Value::as_str) {
        out.push_str("\nNote: ");
        out.push_str(note);
    }
    if let Some(n) = v.get("focus_nodes") {
        out.push_str(&format!("\nFocus nodes: {}", text(n)));
    }
    let skipped = items(v, "skipped_constraints");
    if !skipped.is_empty() {
        out.push_str(&format!("\nNot evaluated: {} constraint(s)", skipped.len()));
        push_lines(&mut out, skipped, text);
    }
    // The verified evaluator carries its findings inside its own report.
    let violations = match v.get("violations") {
        Some(_) => items(v, "violations"),
        None => items(&v["report"], "violations"),
    };
    out.push_str(&format!("\nViolations: {}", violations.len()));
    push_lines(&mut out, violations, |x| {
        let mut line = format!(
            "[{}] {}: {}",
            text(&x["severity"]),
            text(&x["focus_node"]),
            text(&x["message"])
        );
        if let Some(c) = x.get("constraint") {
            line.push_str(&format!(" ({})", text(c)));
        }
        line
    });
    out
}

fn render_vocab_check(v: &Value) -> String {
    let mut out = format!("conforms: {}", verdict(&v["conforms"]));
    push_warnings(&mut out, v);
    let terms = items(v, "hallucinated_terms");
    if !terms.is_empty() {
        out.push_str(&format!(
            "\n{} term(s) the ontology does not declare:",
            terms.len()
        ));
        push_lines(&mut out, terms, text);
    }
    out.push_str(&format!(
        "\nChecked {} predicate(s) and {} type(s) against {} ontology term(s); namespaces policed: {}.",
        text(&v["predicates_checked"]),
        text(&v["types_checked"]),
        text(&v["ontology_terms"]),
        text(&v["checked_namespaces"])
    ));
    out
}

/// The DL profile answers a consistency question and the rule profiles do
/// not, so the verdict line appears only when the run has one.
fn render_reason(v: &Value) -> String {
    let mut lines: Vec<String> = Vec::new();
    if let Some(c) = v.get("consistent") {
        let mut line = format!("consistent: {}", verdict(c));
        if v.get("complete").and_then(Value::as_bool) == Some(false) {
            line.push_str(&format!(
                " (incomplete: budget exhausted in {})",
                text(&v["budget_exhausted_in"])
            ));
        }
        lines.push(line);
    }
    let mut summary = format!(
        "Reasoning ({}): {} triple(s) inferred, {} -> {} triples",
        text(&v["profile_used"]),
        text(&v["inferred_count"]),
        text(&v["initial_triples"]),
        text(&v["final_triples"])
    );
    if let Some(n) = v.get("iterations") {
        summary.push_str(&format!(", {} iteration(s)", text(n)));
    }
    summary.push('.');
    lines.push(summary);
    if v.get("fixpoint_reached").and_then(Value::as_bool) == Some(false) {
        lines.push("Fixpoint NOT reached: the closure is partial.".into());
    }
    if v.get("dry_run").and_then(Value::as_bool) == Some(true) {
        lines.push("Dry run: nothing was written to the store.".into());
    }
    if let Some(n) = v.get("inferred_subsumptions") {
        lines.push(format!("{} subsumption(s) inferred.", text(n)));
    }
    for (key, label) in [
        ("unsatisfiable_classes", "Unsatisfiable classes"),
        ("undetermined_classes", "Undetermined classes"),
    ] {
        let list = items(v, key);
        if !list.is_empty() {
            let mut block = format!("{label}: {}", list.len());
            push_lines(&mut block, list, text);
            lines.push(block);
        }
    }
    if let Some(n) = v.get("skipped_unserialisable") {
        lines.push(format!(
            "{} conclusion(s) skipped as unserialisable.",
            text(n)
        ));
    }
    let mut out = lines.join("\n");
    push_warnings(&mut out, v);
    let sample = items(v, "sample_inferences");
    if !sample.is_empty() {
        out.push_str("\nSample inferences:");
        push_lines(&mut out, sample, text);
    }
    out
}

fn render_enforce(v: &Value) -> String {
    let violations = items(v, "violations");
    let mut out = format!(
        "Rule pack {}: {} of {} rule(s) passed (compliance {}).",
        text(&v["rule_pack"]),
        text(&v["passed_rules"]),
        text(&v["total_rules"]),
        text(&v["compliance"])
    );
    if violations.is_empty() {
        out.push_str("\nNo violations found.");
    } else {
        out.push_str(&format!("\n{} violation(s):", violations.len()));
        push_lines(&mut out, violations, |x| {
            format!("[{}] {}", text(&x["rule"]), text(&x["message"]))
        });
    }
    let suppressed = v.get("suppressed_count").and_then(Value::as_u64).unwrap_or(0);
    if suppressed > 0 {
        out.push_str(&format!("\n{suppressed} more suppressed by recorded feedback."));
    }
    out
}

/// The certificate's verdict first, since it is what the exit code reads,
/// then whether anything was loaded.
fn render_unpack(v: &Value) -> String {
    let cert = &v["certificate"];
    let mut out = String::new();
    if cert.is_object() {
        out.push_str(&format!("Certificate: {}", text(&cert["verdict"])));
        if let Some(means) = cert.get("means").and_then(Value::as_str) {
            out.push_str(&format!(" ({means})"));
        }
        out.push('\n');
    }
    if v.get("loaded").and_then(Value::as_bool) == Some(true) {
        out.push_str(&format!(
            "Unpacked and loaded {} triple(s).",
            text(&v["triples_loaded"])
        ));
    } else if let Some(why) = v.get("why_not_loaded").and_then(Value::as_str) {
        out.push_str(&format!("Not loaded: {why}"));
    } else {
        out.push_str("Verified; nothing was loaded.");
    }
    out
}

/// A monitor sweep, not a server status. Its `status` key sent it to the
/// server-status rendering, which printed "v?" and a triple count the sweep
/// never reported.
fn render_monitor(v: &Value) -> String {
    let alerts = items(v, "alerts");
    let passed = items(v, "passed");
    let mut out = format!(
        "Monitor: {}. {} alert(s), {} watcher(s) passed.",
        text(&v["status"]),
        alerts.len(),
        passed.len()
    );
    if alerts.is_empty() && passed.is_empty() {
        // An `ok` from a sweep that ran no watcher is not evidence of health.
        out.push_str(" No watcher ran.");
    }
    push_lines(&mut out, alerts, |a| {
        format!(
            "[{}] {}: {} (value {}, threshold {}, action {})",
            text(&a["severity"]),
            text(&a["watcher"]),
            text(&a["detail"]),
            text(&a["value"]),
            text(&a["threshold"]),
            text(&a["action"])
        )
    });
    if !passed.is_empty() {
        out.push_str(&format!("\nPassed: {}", text(&v["passed"])));
    }
    out
}

// ─── First-order export and the certificate commands ─────────────────────

fn render_fol(v: &Value) -> String {
    let mut out = format!(
        "Exported {} axiom(s) as {} to {}.",
        text(&v["axioms_exported"]),
        text(&v["syntax"]),
        text(&v["ontology_file"])
    );
    out.push_str(&format!(
        "\n{} background axiom(s), {} individual typing axiom(s) over {} individual(s); {} goal problem(s), {} goal(s) not asked.",
        text(&v["background_axioms"]),
        text(&v["individual_typing_axioms"]),
        text(&v["individuals"]),
        text(&v["goal_problems"]),
        text(&v["goals_not_asked"])
    ));
    let dropped = items(v, "constructs_not_exported");
    if v.get("exports_a_weaker_axiom_set").and_then(Value::as_bool) == Some(true) {
        out.push_str(&format!(
            "\nWEAKER axiom set: {} construct(s) not exported:",
            dropped.len()
        ));
        push_lines(&mut out, dropped, text);
    }
    out
}

fn render_rules_import(v: &Value) -> String {
    let mut out = format!(
        "Imported {} rule(s) from {} {} rule(s); {} refused.",
        text(&v["rules_emitted"]),
        text(&v["source_rules"]),
        text(&v["source_syntax"]),
        text(&v["rules_refused"])
    );
    if v.get("certifies_a_weaker_rule_set").and_then(Value::as_bool) == Some(true) {
        out.push_str("\nThe table is a WEAKER rule set than the source states.");
    }
    if let Some(eligible) = v.get("verdict_this_table_is_eligible_for") {
        out.push_str(&format!(
            "\nVerdict a certificate over this table can earn: {}.",
            text(eligible)
        ));
    }
    push_lines(&mut out, items(v, "refused"), text);
    out
}

/// The stop-the-line count first when there is one: it means a verified
/// checker and the thing it checked disagree, and it fails the process.
fn push_stop_the_line(out: &mut String, v: &Value) {
    let stops = v.get("stop_the_line").and_then(Value::as_u64).unwrap_or(0);
    if stops > 0 {
        out.push_str(&format!(
            "STOP THE LINE: {stops} disagreement(s). {}\n",
            text(&v["stop_the_line_means"])
        ));
    }
}

fn render_fol_model(v: &Value) -> String {
    let mut out = String::new();
    push_stop_the_line(&mut out, v);
    let onto = &v["ontology"];
    out.push_str(&format!(
        "Ontology: {}: {}",
        text(&onto["verdict"]),
        text(&onto["verdict_means"])
    ));
    out.push_str(&format!(
        "\nVerdicts ({}, domain up to {}): {}",
        text(&v["solver"]),
        text(&v["max_domain"]),
        text(&v["verdict_counts"])
    ));
    let goals = items(v, "goals");
    if !goals.is_empty() {
        out.push_str(&format!("\nGoals: {}", goals.len()));
        push_lines(&mut out, goals, |g| {
            format!("{}  {}", text(&g["outcome"]["verdict"]), triple_text(&g["triple"]))
        });
    }
    out
}

fn render_fol_prove(v: &Value) -> String {
    let mut out = String::new();
    push_stop_the_line(&mut out, v);
    out.push_str(&format!("Certified: {}.", text(&v["certified"])));
    let onto = &v["ontology"];
    out.push_str(&format!(
        "\nOntology: {}: {}",
        text(&onto["verdict"]),
        text(&onto["verdict_means"])
    ));
    out.push_str(&format!(
        "\nVerdicts ({}): {}",
        text(&v["prover"]),
        text(&v["verdict_counts"])
    ));
    let goals = items(v, "goals");
    if !goals.is_empty() {
        out.push_str(&format!("\nGoals: {}", goals.len()));
        push_lines(&mut out, goals, |g| {
            format!("{}  {}", text(&g["report"]["verdict"]), triple_text(&g["triple"]))
        });
    }
    out
}

/// The report's own headline, which already puts the verdict counts first,
/// then one line per goal.
fn render_preserve(v: &Value) -> String {
    let mut out = text(&v["headline"]);
    if let Some(d) = v.get("disagreement").filter(|d| !d.is_null()) {
        out.push_str(&format!("\nDisagreement: {}", text(d)));
    }
    push_lines(&mut out, items(v, "per_goal"), |g| {
        format!("{}  {}", text(&g["verdict"]), text(&g["as_written"]))
    });
    out
}

fn render_closure_diff(v: &Value) -> String {
    let mut out = text(&v["headline"]);
    push_lines(&mut out, items(v, "entailments_lost"), |l| {
        format!("lost ({})  {}", text(&l["warrant_word"]), triple_text(&l["triple"]))
    });
    out
}

fn render_trace_label(v: &Value) -> String {
    let steps = items(v, "steps");
    let mut out = format!(
        "{} step(s) labelled under {}: {}.",
        steps.len(),
        text(&v["profile"]),
        text(&v["by_label"])
    );
    out.push_str(&format!("\nLocal checks: {}.", text(&v["by_local_verdict"])));
    push_lines(&mut out, steps, |s| {
        format!(
            "#{} {}  {}  {}",
            text(&s["index"]),
            text(&s["label"]),
            text(&s["rule_claimed"]),
            triple_text(&s["conclusion"])
        )
    });
    out
}

// ─── Lifecycle ───────────────────────────────────────────────────────────

/// The risk score and its reasons first, since a pipeline gates on them, then
/// the size of the change, then the logic and conservativity checks, then the
/// terms the change adds and removes.
fn render_plan(v: &Value) -> String {
    let mut out = format!("Plan {}: risk {}.", text(&v["plan_id"]), text(&v["risk_score"]));
    push_lines(&mut out, items(v, "risk_reasons"), text);
    let n = |key: &str| items(v, key).len();
    out.push_str(&format!(
        "\nClasses +{} -{}, properties +{} -{}, individuals +{} -{}.",
        n("added_classes"),
        n("removed_classes"),
        n("added_properties"),
        n("removed_properties"),
        n("added_individuals"),
        n("removed_individuals")
    ));
    let delta = &v["triple_delta"];
    out.push_str(&format!(
        "\nTriples +{} -{} ({} strategy); blast radius {} triple(s).",
        text(&delta["insertions"]),
        text(&delta["deletions"]),
        text(&delta["strategy"]),
        text(&v["blast_radius"]["triples_affected"])
    ));
    let locked = items(v, "locked_violations");
    if !locked.is_empty() {
        out.push_str(&format!("\nLocked terms touched: {}", locked.len()));
        push_lines(&mut out, locked, |l| {
            format!("{} ({})", text(&l["iri"]), text(&l["reason"]))
        });
    }
    let logic = &v["logic"];
    if logic.is_object() {
        if logic["checked_by"] == "not_checked" {
            out.push_str(&format!("\nLogic: not checked. {}", text(&logic["reason"])));
        } else {
            out.push_str(&format!(
                "\nLogic: ABox consistent: {}; {} new defect(s).",
                verdict(&logic["abox_consistent"]),
                text(&logic["new_defects"])
            ));
            let found = items(logic, "new_defect_findings");
            let width = kind_width(found);
            push_lines(&mut out, found, |d| defect_line(d, width));
        }
    }
    let cons = &v["conservativity"];
    if cons.is_object() {
        if cons["ran"].as_bool() == Some(false) {
            out.push_str(&format!("\nConservativity: not run. {}", text(&cons["skipped"])));
        } else {
            out.push_str(&format!(
                "\nConservativity: {}; {} new consequence(s) over names the base already used, {} in all.",
                text(&cons["conservativity_verdict"]),
                text(&cons["new_consequences_over_old_signature_total"]),
                text(&cons["new_consequences_total"])
            ));
        }
    }
    for (key, label) in [
        ("added_classes", "Added classes"),
        ("removed_classes", "Removed classes"),
        ("added_properties", "Added properties"),
        ("removed_properties", "Removed properties"),
        ("added_individuals", "Added individuals"),
        ("removed_individuals", "Removed individuals"),
    ] {
        let list = items(v, key);
        if !list.is_empty() {
            out.push_str(&format!("\n{label}:"));
            push_lines(&mut out, list, text);
        }
    }
    out
}

fn render_apply(v: &Value) -> String {
    if v.get("blocked").and_then(Value::as_bool) == Some(true) {
        return format!("Blocked: {}", text(&v["message"]));
    }
    let mut out = format!(
        "Applied plan {} ({} mode, {} strategy): +{} -{} triple(s); the store now holds {}.",
        text(&v["plan_id"]),
        text(&v["mode"]),
        text(&v["strategy"]),
        text(&v["triples_inserted"]),
        text(&v["triples_deleted"]),
        text(&v["triples_loaded"])
    );
    if let Some(n) = v.get("bridges_created") {
        out.push_str(&format!("\n{} bridge(s) created.", text(n)));
        let unbridged = items(v, "unbridged_removals");
        if !unbridged.is_empty() {
            out.push_str(&format!("\nRemoved with no bridge: {}", unbridged.len()));
            push_lines(&mut out, unbridged, text);
        }
    }
    out
}

fn render_lock(v: &Value) -> String {
    let locked = items(v, "locked");
    let mut out = format!(
        "Locked {} IRI(s) (reason: {}).",
        locked.len(),
        text(&v["reason"])
    );
    push_lines(&mut out, locked, text);
    out
}

/// Drift compares the vocabularies two versions declare, so its `added` and
/// `removed` list TERMS, not triples. The diff rendering it used to reach
/// called them triples and dropped the velocity and the likely renames.
fn render_drift(v: &Value) -> String {
    let added = items(v, "added");
    let removed = items(v, "removed");
    let renames = items(v, "likely_renames");
    let mut out = format!(
        "Drift velocity: {} ({} vocabulary term(s) before, {} after).",
        text(&v["drift_velocity"]),
        text(&v["v1_count"]),
        text(&v["v2_count"])
    );
    out.push_str(&format!(
        "\n+{} term(s) added, -{} removed, {} likely rename(s).",
        added.len(),
        removed.len(),
        renames.len()
    ));
    if v.get("exact").and_then(Value::as_bool) == Some(false) {
        out.push_str("\nApproximate: ");
        out.push_str(&text(&v["approximate_reason"]));
    }
    push_lines(&mut out, renames, |r| {
        format!(
            "~ {} -> {} (confidence {})",
            text(&r["from"]),
            text(&r["to"]),
            text(&r["confidence"])
        )
    });
    push_lines(&mut out, added, |t| format!("+ {}", text(t)));
    push_lines(&mut out, removed, |t| format!("- {}", text(t)));
    out
}

fn render_lineage(v: &Value) -> String {
    let session = text(&v["session_id"]);
    let events: Vec<&str> = v["events"]
        .as_str()
        .unwrap_or("")
        .lines()
        .filter(|l| !l.trim().is_empty())
        .collect();
    if events.is_empty() {
        return format!("No lineage events for session {session}.");
    }
    let mut out = format!("{} lineage event(s) for session {session}:", events.len());
    push_lines(&mut out, &events, |e| e.to_string());
    out
}

// ─── Alignment and feedback ──────────────────────────────────────────────

fn render_align(v: &Value) -> String {
    let candidates = items(v, "candidates");
    let mut out = format!(
        "{} candidate(s): {} applied, {} borderline (scoring {}, threshold {}, {}).",
        text(&v["total_candidates"]),
        text(&v["applied_count"]),
        text(&v["borderline_count"]),
        text(&v["scoring"]),
        text(&v["high_threshold"]),
        text(&v["cardinality"])
    );
    push_lines(&mut out, candidates, |c| {
        let state = if c["applied"].as_bool() == Some(true) {
            "applied"
        } else if c["requires_review"].as_bool() == Some(true) {
            "needs review"
        } else {
            "not applied"
        };
        format!(
            "{}  {} -> {}  {}  [{state}]",
            text(&c["confidence"]),
            text(&c["source_iri"]),
            text(&c["target_iri"]),
            text(&c["relation"])
        )
    });
    if let Some(s) = v.get("summary_for_review").and_then(Value::as_str).filter(|s| !s.is_empty()) {
        out.push('\n');
        out.push_str(s);
    }
    out
}

fn render_crosswalk_probe(v: &Value) -> String {
    let mut out = format!(
        "contradicted: {} ({})",
        verdict(&v["contradicted"]),
        text(&v["verdict"])
    );
    out.push_str(&format!(
        "\n{} claim(s) carried through the mapping; {} clash(es) added by it, {} in the target alone.",
        text(&v["carried"]),
        text(&v["clashes_added_by_the_mapping"]),
        text(&v["target_alone"]["clash_count"])
    ));
    push_lines(&mut out, items(v, "clashes"), text);
    out
}

fn render_align_feedback(v: &Value) -> String {
    let what = if v["accepted"].as_bool() == Some(true) {
        "acceptance"
    } else {
        "rejection"
    };
    format!(
        "Recorded {what} of {} -> {}. {} feedback record(s); weight learning is {}.",
        text(&v["source_iri"]),
        text(&v["target_iri"]),
        text(&v["feedback_count"]),
        text(&v["weights_learning"])
    )
}

fn render_tool_feedback(v: &Value) -> String {
    let what = if v["accepted"].as_bool() == Some(true) {
        "acceptance"
    } else {
        "dismissal"
    };
    format!(
        "Recorded {what} of {} rule {} for {}.",
        text(&v["tool"]),
        text(&v["rule_id"]),
        text(&v["entity"])
    )
}

// ─── Clinical ────────────────────────────────────────────────────────────

fn render_crosswalk(v: &Value) -> String {
    let mappings = items(v, "mappings");
    let mut out = format!(
        "{} in {}: {} mapping(s).",
        text(&v["code"]),
        text(&v["system"]),
        mappings.len()
    );
    push_lines(&mut out, mappings, |m| {
        format!(
            "{} {}:{}  {}",
            text(&m["relation"]),
            text(&m["target_system"]),
            text(&m["target_code"]),
            text(&m["target_label"])
        )
    });
    out
}

fn render_enrich(v: &Value) -> String {
    if v["ok"].as_bool() == Some(false) {
        return render_message(v);
    }
    format!(
        "Enriched {} with {}: {} triple(s) added.",
        text(&v["enriched"]),
        text(&v["mapped_to"]),
        text(&v["triples_added"])
    )
}

fn render_validate_clinical(v: &Value) -> String {
    let unmatched = items(v, "unmatched");
    let mut out = format!(
        "{} of {} class(es) matched a clinical term; {} unmatched.",
        items(v, "validated").len(),
        text(&v["total_classes"]),
        unmatched.len()
    );
    push_lines(&mut out, unmatched, |u| {
        format!("{} ({})", text(&u["class"]), text(&u["label"]))
    });
    out
}

// ─── Daemon management ───────────────────────────────────────────────────

fn render_daemon_start(v: &Value) -> String {
    format!("Daemon started: PID {} at {}.", text(&v["pid"]), text(&v["url"]))
}

/// Liveness is printed only from `alive`, which only `daemon status`
/// computes. A successful `daemon start` once printed "Daemon dead" because
/// its `{ok, pid, url}` was matched to this shape.
fn render_daemon_status(v: &Value) -> String {
    match v.get("alive").and_then(Value::as_bool) {
        Some(true) => format!(
            "Daemon running: PID {} at {}.",
            text(&v["pid"]),
            text(&v["url"])
        ),
        Some(false) if v.get("pid").is_some() => format!(
            "Daemon dead: PID {} at {} is not running.",
            text(&v["pid"]),
            text(&v["url"])
        ),
        _ => format!("No daemon running ({}).", text(&v["message"])),
    }
}

/// Accept the native store's flat rows as well as SPARQL binding objects.
/// Native SELECT results always carry variables; an unrelated results array
/// must still fall back to its existing rendering.
fn sparql_rows(value: &Value) -> Option<&Vec<Value>> {
    let results = value.get("results")?;
    if let Some(bindings) = results.get("bindings").and_then(Value::as_array) {
        return Some(bindings);
    }
    value.get("variables")?.as_array()?;
    results.as_array()
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
                            v.as_str()
                                .or_else(|| v.get("value").and_then(Value::as_str))
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn render(command: &str, payload: Value) -> String {
        render_human_for(Some(command), &payload)
    }

    #[test]
    fn a_started_daemon_is_not_reported_as_dead() {
        // The regression this exists for: `daemon start` answers {ok, pid, url},
        // which matched the daemon-status branch, a branch that defaulted
        // liveness to false, and printed "Daemon dead" for a daemon that had
        // just come up.
        let rendered = render(
            "daemon start",
            json!({"ok": true, "pid": 4242, "url": "http://127.0.0.1:8080"}),
        );
        assert!(!rendered.contains("dead"), "got: {rendered}");
        assert!(rendered.contains("4242"), "got: {rendered}");
    }

    #[test]
    fn daemon_status_still_reports_liveness_both_ways() {
        let alive = json!({"pid": 1, "url": "http://127.0.0.1:8080", "alive": true});
        assert!(render("daemon status", alive).contains("running"));
        let dead = json!({"pid": 1, "url": "http://127.0.0.1:8080", "alive": false});
        assert!(render("daemon status", dead).contains("dead"));
        let none = json!({"alive": false, "message": "no daemon.json found"});
        assert!(render("daemon status", none).contains("No daemon running"));
    }

    #[test]
    fn a_known_command_dispatches_without_sniffing_keys() {
        // `stats` renders as stats because the command was `stats`, not
        // because the payload happened to carry a `triples` key.
        let payload = json!({"triples": 12});
        assert_eq!(render("stats", payload.clone()), render_stats(&payload));
        // And `validate`, which also carries `triples`, does not.
        let validated = render("validate", json!({"ok": true, "triples": 4, "statements": 4}));
        assert!(!validated.contains("Classes"), "got: {validated}");
    }

    #[test]
    fn an_error_is_an_error_whatever_produced_it() {
        let payload = json!({"error": "no such file"});
        assert_eq!(render("load", payload.clone()), "Error: no such file");
        assert_eq!(render_human_for(None, &payload), "Error: no such file");
    }

    #[test]
    fn an_unknown_command_is_printed_as_labelled_json_and_never_by_shape() {
        // This payload is exactly `load`'s. Under a name with no renderer it
        // must not be rendered as a load, which is what guessing would do.
        let payload = json!({"ok": true, "triples_loaded": 3, "path": "/tmp/x.ttl"});
        for command in [None, Some("no-such-command")] {
            let rendered = render_human_for(command, &payload);
            assert!(rendered.starts_with(NO_RENDERING), "got: {rendered}");
            assert!(!rendered.contains("Loaded"), "got: {rendered}");
        }
        assert!(render("load", payload).contains("Loaded 3 triple(s)"));
    }

    #[test]
    fn both_spellings_of_a_batch_command_find_one_renderer() {
        assert!(renderer_for("vocab_check").is_some());
        assert!(renderer_for("vocab-check").is_some());
        assert!(has_human_renderer("batch"));
        assert!(!has_human_renderer("no-such-command"));
    }

    #[test]
    fn diff_counts_are_counts() {
        let rendered = render(
            "diff",
            json!({"added": 1, "removed": 0, "exact": true,
                   "added_triples": ["<urn:s> <urn:p> <urn:o>"], "removed_triples": []}),
        );
        assert!(rendered.starts_with("Diff: +1 triple(s), -0 triple(s)."), "got: {rendered}");
        assert!(rendered.contains("+ <urn:s> <urn:p> <urn:o>"), "got: {rendered}");
    }

    #[test]
    fn an_undetermined_shacl_verdict_is_said_first_and_never_reads_as_a_pass() {
        let rendered = render(
            "shacl",
            json!({"conforms": null, "focus_nodes": 0, "violations": [],
                   "warning": "no focus nodes matched, so conformance is undetermined."}),
        );
        assert!(rendered.starts_with("conforms: undetermined"), "got: {rendered}");
        assert!(rendered.contains("no focus nodes matched"), "got: {rendered}");
        assert!(!rendered.contains("No violations found"), "got: {rendered}");
    }

    #[test]
    fn a_verified_shacl_report_names_its_theorem() {
        let rendered = render(
            "shacl",
            json!({"verified": true, "conforms": false, "theorem": "Shacl.validate_spec",
                   "report": {"violations": [{"focus_node": "urn:a", "message": "bad"}]}}),
        );
        assert!(rendered.starts_with("conforms: false (verified: Shacl.validate_spec)"));
        assert!(rendered.contains("Violations: 1"), "got: {rendered}");
        assert!(rendered.contains("urn:a: bad"), "got: {rendered}");
    }

    #[test]
    fn a_monitor_sweep_is_not_a_server_status() {
        let rendered = render("monitor", json!({"status": "ok", "alerts": [], "passed": []}));
        assert!(!rendered.contains("v?"), "got: {rendered}");
        assert!(rendered.starts_with("Monitor: ok."), "got: {rendered}");
        assert!(rendered.contains("No watcher ran."), "got: {rendered}");
    }

    #[test]
    fn query_renders_ask_and_construct_answers() {
        assert_eq!(render("query", json!({"result": true})), "ASK: true");
        let built = json!({"triples": [{"subject": "<urn:s>", "predicate": "<urn:p>", "object": "\"x\""}]});
        assert_eq!(render("query", built), "<urn:s> <urn:p> \"x\" .");
        assert_eq!(render("query", json!({"triples": []})), "No triples.");
    }

    #[test]
    fn an_unpacked_pack_leads_with_its_certificate() {
        let rendered = render(
            "unpack",
            json!({"ok": true, "verified": true, "loaded": true, "triples_loaded": 7,
                   "certificate": {"present": false, "verdict": "no_certificate_in_pack",
                                   "means": "nothing to re-derive"}}),
        );
        assert!(rendered.starts_with("Certificate: no_certificate_in_pack"), "got: {rendered}");
        assert!(rendered.contains("loaded 7 triple(s)"), "got: {rendered}");
    }

    #[test]
    fn extend_renders_its_nested_results_with_their_own_renderers() {
        let rendered = render(
            "extend",
            json!({"ok": true, "rows": 2, "triples_loaded": 6,
                   "shacl": {"conforms": null, "violations": []},
                   "reason": {"profile_used": "rdfs", "inferred_count": 0,
                              "initial_triples": 6, "final_triples": 6}}),
        );
        assert!(rendered.contains("SHACL:\n  conforms: undetermined"), "got: {rendered}");
        assert!(rendered.contains("Reasoning:\n  Reasoning (rdfs)"), "got: {rendered}");
    }
}
