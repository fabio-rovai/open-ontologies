use crate::graph::GraphStore;
use oxigraph::io::{RdfFormat, RdfParser};
use oxigraph::sparql::{QueryResults, SparqlEvaluator};
use oxigraph::store::Store;
use std::collections::{HashMap, HashSet};
use std::io::Cursor;
use std::sync::Arc;

/// SHACL validator that checks data in a `GraphStore` against SHACL shapes.
///
/// Shapes are parsed from inline Turtle into a temporary Oxigraph store.
/// Constraints are translated into SPARQL queries run against the main graph.
/// Supports the core constraints `sh:minCount`, `sh:maxCount`, `sh:datatype`,
/// `sh:class`, `sh:pattern`, `sh:hasValue`, `sh:in`, `sh:nodeKind`, `sh:or`,
/// `sh:not`, the inclusive and exclusive range bounds, and SPARQL-based
/// constraints via `sh:sparql`.
///
/// All four target forms select focus nodes: `sh:targetClass` (including the
/// implicit class target), `sh:targetNode`, `sh:targetSubjectsOf` and
/// `sh:targetObjectsOf`.
///
/// Any constraint the validator cannot execute is recorded in
/// `skipped_constraints` and suppresses the conformance verdict: `conforms`
/// becomes null rather than true. That holds wherever the constraint sits: on a
/// property shape (`sh:minLength`, `sh:xone`, a `sh:not` nesting a form that is
/// not evaluated), or on the node shape itself (`sh:closed`, `sh:deactivated`).
/// A target that selects no nodes reaches the same null verdict by the other
/// route, `unmatched_shapes`. Reporting success for rules that were never run is
/// the one failure mode this validator must not have.
pub struct ShaclValidator;

impl ShaclValidator {
    /// Validate the data in `graph` against SHACL shapes (inline Turtle).
    /// Returns a JSON report: `{conforms, violation_count, violations[]}`.
    pub fn validate(graph: &Arc<GraphStore>, shapes_ttl: &str) -> anyhow::Result<String> {
        // 1. Parse shapes Turtle into a temporary store
        let shapes_store = Store::new()?;
        let reader = Cursor::new(shapes_ttl.as_bytes());
        let parser = RdfParser::from_format(RdfFormat::Turtle).for_reader(reader);
        for quad in parser {
            shapes_store.insert(&quad?)?;
        }

        // 2. Find all sh:NodeShape with sh:targetClass
        let shapes = query_solutions(
            &shapes_store,
            r#"
            PREFIX sh: <http://www.w3.org/ns/shacl#>
            PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
            PREFIX owl: <http://www.w3.org/2002/07/owl#>
            SELECT DISTINCT ?shape ?targetClass WHERE {
                { ?shape a sh:NodeShape ; sh:targetClass ?targetClass . }
                UNION
                { ?shape a sh:NodeShape, rdfs:Class . BIND(?shape AS ?targetClass) }
                UNION
                { ?shape a sh:NodeShape, owl:Class . BIND(?shape AS ?targetClass) }
            }
            "#,
        )?;

        // sh:targetClass selects SHACL instances, which the specification defines as
        // reachable by rdf:type followed by zero or more rdfs:subClassOf steps, not by a
        // direct rdf:type alone. Matching the direct type only made every shape targeting
        // a superclass silently select nothing: a shapes graph targeting an abstract
        // Assertion class over data typed with its concrete subclasses reported
        // `conforms: true` having evaluated no focus nodes at all.
        let mut violations: Vec<serde_json::Value> = Vec::new();
        let mut skipped: Vec<serde_json::Value> = Vec::new();

        // All four target forms, each reduced to a SPARQL pattern that binds one
        // focus variable. Reducing them to a pattern rather than to a class keeps
        // every constraint below written once: the constraint queries do not know
        // which form selected the node they are checking.
        //
        // `sh:targetClass` selects SHACL instances, which is rdf:type followed by
        // zero or more rdfs:subClassOf steps, not a direct rdf:type.
        //
        // The other three are explicit. `sh:targetNode` names its focus node
        // outright and selects it whether or not that node appears anywhere in
        // the data: checked against pyshacl, which reports a MinCount violation
        // on a targetNode absent from the data rather than an empty target. A
        // VALUES clause reproduces that, where a triple pattern would not.
        //
        // Until this existed the three explicit forms were recorded as skipped,
        // which was honest but empty: a shapes graph written to the specification
        // got no verdict at all.
        let mut targets: Vec<(String, &'static str, String)> = Vec::new();
        for row in &shapes {
            if let (Some(shape), Some(tc)) = (row.get("shape"), row.get("targetClass")) {
                targets.push((shape.clone(), "class", strip_angle_brackets(tc)));
            }
        }
        for (pred, kind) in [
            ("sh:targetNode", "node"),
            ("sh:targetSubjectsOf", "subjectsOf"),
            ("sh:targetObjectsOf", "objectsOf"),
        ] {
            let q = format!(
                r#"PREFIX sh: <http://www.w3.org/ns/shacl#>
                   SELECT DISTINCT ?shape ?t WHERE {{ ?shape {} ?t . }}"#,
                pred
            );
            for row in &query_solutions(&shapes_store, &q)? {
                if let (Some(shape), Some(t)) = (row.get("shape"), row.get("t")) {
                    targets.push((shape.clone(), kind, t.trim().to_string()));
                }
            }
        }
        let mut unmatched: Vec<serde_json::Value> = Vec::new();
        let mut focus_nodes_total: u64 = 0;

        // A constraint asserted on the node shape itself (`sh:closed`, a
        // node-level `sh:not`, `sh:nodeKind`, `sh:and`, `sh:or`, `sh:xone`,
        // `sh:in`, `sh:node`) never reaches the per-property complement in
        // the loop below, which starts one sh:property hop under the shape.
        // Before this complement existed, `sh:closed true` over data carrying
        // an undeclared predicate returned `conforms: true`.
        //
        // It covers every discovered shape in one query, for two reasons.
        // First, the shape is bound as a variable and matched to the
        // discovery row by its printed term, not spliced into the query text:
        // a shape written `[] a sh:NodeShape` prints as `_:label`, and a
        // blank-node label inside a SPARQL query is a non-distinguished
        // variable, not a name, so splicing it enumerated every predicate in
        // the shapes graph and routed the property shape's own sh:path to
        // skipped. Second, discovery yields one row per (shape, target class)
        // and a node constraint belongs to the shape, so running the
        // complement per row recorded `sh:closed` once per target class. It
        // is restricted to discovered shapes, like the property complement:
        // a shape with no target selects no focus nodes under SHACL, so its
        // constraints not running is what the specification asks for, not a
        // gap. (This clause used to also except shapes using a target form the
        // validator lacked. There are none left: all four core forms select.)
        //
        // The whitelist is the predicates the validator reads on the shape
        // node (the four target forms, all of which now select focus nodes,
        // plus sh:property and sh:sparql) and the annotation predicates,
        // which are never constraints. Any other sh: predicate lands in
        // skipped, even one a later change starts to evaluate, because a
        // whitelist that tracks what is implemented drifts the first time
        // someone adds a constraint and forgets the list. `sh:deactivated` is
        // deliberately absent: SHACL says a deactivated shape must not be
        // evaluated and this validator evaluates it anyway, so the predicate
        // is not implemented and must reach skipped like any other.
        //
        // Two predicates are exempt at a FALSE value, and only at a false
        // value, because at that value there is nothing left to implement.
        // `sh:closed false` is the SHACL default and imposes no closed-shape
        // restriction; `sh:deactivated false` says evaluate this shape, which
        // is exactly what this validator does. Both are honoured in full, so
        // recording them suppresses the verdict on a run in which nothing was
        // missed. That is a false undetermined, and it costs as much as the
        // false clean this complement exists to prevent: a null that fires on
        // a complete run teaches the reader to ignore null, which destroys
        // the signal the third answer carries. The value is read by value and
        // not by lexical form, so "0"^^xsd:boolean is false like any other
        // spelling. `sh:deactivated true` is unaffected and still reaches
        // skipped, for the reason above.
        //
        // The isLiteral/datatype pair in front of the equality is defensive
        // and, measured on this evaluator, changes no answer today: oxigraph
        // answers `"false"^^xsd:string = false` with false, so an unreadable
        // control stays skipped with or without it. It is kept because SPARQL
        // 1.1 does not require that. RDFterm-equal is a type error on two
        // literals that are neither the same term nor comparable, an error
        // inside FILTER drops the solution rather than failing loudly, and a
        // dropped solution here means a control this validator cannot read
        // never reaches skipped: the false clean, arriving through the one
        // code path written to prevent it, on an evaluator upgrade nobody
        // would think to test for. Testing isLiteral, then the datatype, then
        // the equality keeps every operand of the conjunction false rather
        // than errored, on any evaluator. Being unmutatable is the point of
        // it, so do not delete it for want of a reddening test.
        //
        // `sh:ignoredProperties` is knowingly not exempt and is not
        // whitelisted. It modifies `sh:closed` rather than constraining
        // anything by itself, so wherever it has an effect the shape also
        // carries `sh:closed true`, which is recorded here and suppresses the
        // verdict anyway. Carrying it without that is a shape that asks for
        // nothing, and reporting it costs a report nobody is reading.
        //
        // The complement is restricted to the sh: namespace. A shape that is
        // also an rdfs:Class or owl:Class (an implicit class target) carries
        // the class's own axioms on the same subject (rdfs:subClassOf,
        // rdfs:label, owl:equivalentClass), and an unrestricted complement
        // would report those as constraints and turn every such run
        // undetermined. A constraint is by definition a predicate in the sh:
        // namespace; a predicate from any other namespace on a shape node is
        // an annotation or an axiom, never a constraint.
        let discovered: HashSet<&str> = shapes
            .iter()
            .filter_map(|s| s.get("shape").map(String::as_str))
            .collect();
        let unknown_on_node = query_solutions(
            &shapes_store,
            r#"
            PREFIX sh: <http://www.w3.org/ns/shacl#>
            PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>
            SELECT DISTINCT ?shape ?pred WHERE {
                ?shape a sh:NodeShape ; ?pred ?o .
                FILTER(STRSTARTS(STR(?pred), "http://www.w3.org/ns/shacl#") && ?pred NOT IN (
                    sh:targetClass, sh:targetNode, sh:targetSubjectsOf,
                    sh:targetObjectsOf, sh:property, sh:sparql, sh:or,
                    sh:message, sh:severity, sh:name, sh:description,
                    sh:order, sh:group
                ) && !(?pred IN (sh:closed, sh:deactivated)
                    && isLiteral(?o) && datatype(?o) = xsd:boolean && ?o = false
                ))
            }
            "#,
        )?;
        for row in &unknown_on_node {
            let (Some(shape), Some(pred)) = (row.get("shape"), row.get("pred")) else {
                continue;
            };
            if !discovered.contains(shape.as_str()) {
                continue;
            }
            skipped.push(serde_json::json!({
                "shape": strip_angle_brackets(shape),
                "constraint": strip_angle_brackets(pred),
                "reason": "node-shape constraint not implemented; it was not evaluated",
            }));
        }

        for (shape_term, kind, target_value) in &targets {
            let kind = *kind;
            let focus_pattern = format!("{} .", target_pattern(kind, target_value, "focus"));

            // How many nodes does this shape actually apply to? A shape whose
            // target appears nowhere in the data evaluates every one of its
            // constraints against the empty set and contributes no violations,
            // which is indistinguishable in the report from a shape that checked
            // its nodes and found them sound.
            let focus_count = count_focus_nodes(graph, &focus_pattern)?;
            focus_nodes_total += focus_count;
            if focus_count == 0 {
                let mut entry = serde_json::json!({
                    "shape": strip_angle_brackets(shape_term),
                    "target_form": kind,
                    "target": strip_angle_brackets(target_value),
                });
                // `target_class` is the key callers already read for the common
                // form. Keep emitting it rather than renaming it under them.
                if kind == "class" {
                    entry["target_class"] = serde_json::json!(target_value);
                }
                unmatched.push(entry);
            }

            // 3. Find property constraints for this shape
            let shape_iri = shape_term.clone();

            let props = query_solutions(
                &shapes_store,
                &format!(
                    r#"
                    PREFIX sh: <http://www.w3.org/ns/shacl#>
                    SELECT ?prop ?path ?invPath ?minCount ?maxCount ?datatype ?class ?pattern ?hasValue ?nodeKind ?minInclusive ?maxInclusive ?minExclusive ?maxExclusive ?minLength ?maxLength ?lessThan ?lessThanOrEquals ?node ?message ?severity WHERE {{
                        {} sh:property ?prop .
                        ?prop sh:path ?path .
                        OPTIONAL {{ ?path sh:inversePath ?invPath }}
                        OPTIONAL {{ ?prop sh:class ?class }}
                        OPTIONAL {{ ?prop sh:minCount ?minCount }}
                        OPTIONAL {{ ?prop sh:maxCount ?maxCount }}
                        OPTIONAL {{ ?prop sh:datatype ?datatype }}
                        OPTIONAL {{ ?prop sh:pattern ?pattern }}
                        OPTIONAL {{ ?prop sh:hasValue ?hasValue }}
                        OPTIONAL {{ ?prop sh:nodeKind ?nodeKind }}
                        OPTIONAL {{ ?prop sh:minInclusive ?minInclusive }}
                        OPTIONAL {{ ?prop sh:maxInclusive ?maxInclusive }}
                        OPTIONAL {{ ?prop sh:minExclusive ?minExclusive }}
                        OPTIONAL {{ ?prop sh:maxExclusive ?maxExclusive }}
                        OPTIONAL {{ ?prop sh:minLength ?minLength }}
                        OPTIONAL {{ ?prop sh:maxLength ?maxLength }}
                        OPTIONAL {{ ?prop sh:lessThan ?lessThan }}
                        OPTIONAL {{ ?prop sh:lessThanOrEquals ?lessThanOrEquals }}
                        OPTIONAL {{ ?prop sh:node ?node }}
                        OPTIONAL {{ ?prop sh:message ?message }}
                        OPTIONAL {{ ?prop sh:severity ?severity }}
                    }}
                    "#,
                    shape_iri
                ),
            )?;

            // Any constraint predicate on a property shape that this implementation
            // does not evaluate must be reported, not ignored. Before this check,
            // `sh:not` was invisible: it was never collected, never evaluated, and
            // never recorded, so a shape whose only constraint was `sh:not` returned
            // `conforms: true` over data that violated it.
            let unknown = query_solutions(
                &shapes_store,
                &format!(
                    r#"
                    PREFIX sh: <http://www.w3.org/ns/shacl#>
                    SELECT DISTINCT ?pred WHERE {{
                        {} sh:property ?prop .
                        ?prop ?pred ?o .
                        FILTER(?pred NOT IN (
                            sh:path, sh:minCount, sh:maxCount, sh:datatype,
                            sh:class, sh:pattern, sh:hasValue, sh:message, sh:severity,
                            sh:minInclusive, sh:maxInclusive,
                            sh:minExclusive, sh:maxExclusive,
                            sh:or, sh:in, sh:nodeKind, sh:not,
                            sh:minLength, sh:maxLength,
                            sh:lessThan, sh:lessThanOrEquals,
                            sh:qualifiedValueShape, sh:qualifiedMinCount,
                            sh:qualifiedMaxCount,
                            sh:node,
                            sh:name, sh:description, sh:order, sh:group,
                            <http://www.w3.org/1999/02/22-rdf-syntax-ns#type>
                        ))
                    }}
                    "#,
                    shape_iri
                ),
            )?;
            for row in &unknown {
                if let Some(pred) = row.get("pred") {
                    skipped.push(serde_json::json!({
                        "shape": strip_angle_brackets(&shape_iri),
                        "constraint": strip_angle_brackets(pred),
                        "reason": "constraint not implemented; it was not evaluated",
                    }));
                }
            }

            // sh:or alternatives for this shape, collected once and keyed by the
            // property shape's own printed term.
            //
            // They cannot be looked up per property shape the obvious way: a
            // property shape is almost always a blank node, and a blank-node
            // label written into a SPARQL query is a fresh variable rather than
            // a reference to that node, so `?prop sh:or ...` with the label
            // substituted matches every property shape in the file instead of
            // one. Binding ?prop and matching on the printed term avoids naming
            // the blank node in query text while still telling two blocks apart.
            //
            // This was keyed by sh:path, which merged every property shape
            // sharing a path into one constraint. Two blocks became one
            // conjunction that no value could satisfy, so both reported nothing
            // and the run came back CLEAN. Same defect in sh:in and sh:not.
            let mut or_alternatives: HashMap<String, Vec<String>> = HashMap::new();
            let mut or_unsupported: HashSet<String> = HashSet::new();
            let or_rows = query_solutions(
                &shapes_store,
                &format!(
                    r#"
                    PREFIX sh: <http://www.w3.org/ns/shacl#>
                    PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
                    SELECT ?prop ?path ?datatype ?class ?hasValue ?other WHERE {{
                        {} sh:property ?prop .
                        ?prop sh:path ?path .
                        ?prop sh:or/rdf:rest*/rdf:first ?member .
                        OPTIONAL {{ ?member sh:datatype ?datatype }}
                        OPTIONAL {{ ?member sh:class ?class }}
                        OPTIONAL {{ ?member sh:hasValue ?hasValue }}
                        OPTIONAL {{
                            ?member ?other ?_v .
                            FILTER(?other NOT IN (sh:datatype, sh:class, sh:hasValue, rdf:type))
                        }}
                    }}
                    "#,
                    shape_iri
                ),
            )?;
            let mut or_paths: HashMap<String, String> = HashMap::new();
            for row in &or_rows {
                let p = match row.get("prop") {
                    Some(p) => p.clone(),
                    None => continue,
                };
                or_paths.insert(
                    p.clone(),
                    row.get("path").map(|x| strip_angle_brackets(x)).unwrap_or_default(),
                );
                if row.get("other").is_some() {
                    or_unsupported.insert(p);
                    continue;
                }
                let clause = if let Some(dt) = row.get("datatype") {
                    format!("DATATYPE(?val) = <{}>", strip_angle_brackets(dt))
                } else if let Some(c) = row.get("class") {
                    format!(
                        "EXISTS {{ ?val <http://www.w3.org/1999/02/22-rdf-syntax-ns#type>/<http://www.w3.org/2000/01/rdf-schema#subClassOf>* <{}> }}",
                        strip_angle_brackets(c)
                    )
                } else if let Some(hv) = row.get("hasValue") {
                    format!("?val = {}", hv.trim())
                } else {
                    or_unsupported.insert(p);
                    continue;
                };
                or_alternatives.entry(p).or_default().push(clause);
            }
            for p in &or_unsupported {
                or_alternatives.remove(p);
                skipped.push(serde_json::json!({
                    "shape": strip_angle_brackets(&shape_iri),
                    "constraint": "sh:or",
                    "path": or_paths.get(p).cloned().unwrap_or_default(),
                    "reason": "sh:or members use a constraint form that is not implemented; \
                               the disjunction was not evaluated",
                }));
            }

            // sh:not over a property shape, keyed by path for the same
            // blank-node reason as sh:or above.
            //
            // The nested shape is applied to each value node of the path, so a
            // violation is a value that CONFORMS to it. That inverts the sense
            // of every clause: where sh:or reports the values satisfying none of
            // its members, sh:not reports the values satisfying its one member.
            //
            // Only the leaf forms already evaluated in their positive sense are
            // attempted. Anything else goes to `skipped` and suppresses the
            // verdict, because a negation that never ran is precisely the false
            // clean this validator exists to prevent: in the Scottish land
            // register build a layer-2 shapes graph expressed its rule this way,
            // and 198 real violations were reported as a clean run.
            let mut not_clauses: HashMap<String, Vec<String>> = HashMap::new();
            let mut not_unsupported: HashSet<String> = HashSet::new();
            let not_rows = query_solutions(
                &shapes_store,
                &format!(
                    r#"
                    PREFIX sh: <http://www.w3.org/ns/shacl#>
                    PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
                    SELECT ?prop ?path ?datatype ?class ?hasValue ?pattern ?other WHERE {{
                        {} sh:property ?prop .
                        ?prop sh:path ?path .
                        ?prop sh:not ?inner .
                        OPTIONAL {{ ?inner sh:datatype ?datatype }}
                        OPTIONAL {{ ?inner sh:class ?class }}
                        OPTIONAL {{ ?inner sh:hasValue ?hasValue }}
                        OPTIONAL {{ ?inner sh:pattern ?pattern }}
                        OPTIONAL {{
                            ?inner ?other ?_v .
                            FILTER(?other NOT IN (
                                sh:datatype, sh:class, sh:hasValue, sh:pattern, rdf:type
                            ))
                        }}
                    }}
                    "#,
                    shape_iri
                ),
            )?;
            let mut not_paths: HashMap<String, String> = HashMap::new();
            for row in &not_rows {
                let p = match row.get("prop") {
                    Some(p) => p.clone(),
                    None => continue,
                };
                not_paths.insert(
                    p.clone(),
                    row.get("path").map(|x| strip_angle_brackets(x)).unwrap_or_default(),
                );
                if row.get("other").is_some() {
                    not_unsupported.insert(p);
                    continue;
                }
                let clause = if let Some(dt) = row.get("datatype") {
                    format!("DATATYPE(?val) = <{}>", strip_angle_brackets(dt))
                } else if let Some(c) = row.get("class") {
                    format!(
                        "EXISTS {{ ?val <http://www.w3.org/1999/02/22-rdf-syntax-ns#type>/<http://www.w3.org/2000/01/rdf-schema#subClassOf>* <{}> }}",
                        strip_angle_brackets(c)
                    )
                } else if let Some(hv) = row.get("hasValue") {
                    format!("?val = {}", hv.trim())
                } else if let Some(pattern_raw) = row.get("pattern") {
                    let escaped = strip_quotes(pattern_raw)
                        .replace('\\', "\\\\")
                        .replace('"', "\\\"");
                    format!("REGEX(STR(?val), \"{escaped}\")")
                } else {
                    // `sh:not` present with no readable nested constraint. Not
                    // evaluable, so not silently dropped.
                    not_unsupported.insert(p);
                    continue;
                };
                not_clauses.entry(p).or_default().push(clause);
            }
            for p in &not_unsupported {
                not_clauses.remove(p);
                skipped.push(serde_json::json!({
                    "shape": strip_angle_brackets(&shape_iri),
                    "constraint": "sh:not",
                    "path": not_paths.get(p).cloned().unwrap_or_default(),
                    "reason": "sh:not nests a constraint form that is not implemented; \
                               the negation was not evaluated",
                }));
            }

            // sh:or asserted on the node shape itself, over member SHAPES rather
            // than leaf constraints: the focus node must conform to at least one.
            // The per-property sh:or below handles the other form, a list of leaf
            // alternatives for the values of one path.
            //
            // The Italian register vertical expresses a rule this way to keep its
            // layer core-only: either the assertion is conformant, or it records
            // why not.
            //
            // The members are written inline, so they are blank nodes, and the
            // shape compiler cannot be pointed at them: a blank-node label inside
            // a SPARQL query is a fresh variable, not a reference to that node.
            // Their contents are therefore read through the parent in one query
            // and grouped by the member's printed term, the same way property
            // shapes are handled. That confines this form to one level: a member
            // that itself nests sh:node is not compiled, and says so.
            let node_or_rows = query_solutions(
                &shapes_store,
                &format!(
                    r#"
                    PREFIX sh: <http://www.w3.org/ns/shacl#>
                    PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
                    SELECT ?member ?path ?minCount ?maxCount ?class ?datatype ?nodeKind ?hasValue ?other WHERE {{
                        {} sh:or/rdf:rest*/rdf:first ?member .
                        ?member sh:property ?prop .
                        ?prop sh:path ?path .
                        OPTIONAL {{ ?prop sh:minCount ?minCount }}
                        OPTIONAL {{ ?prop sh:maxCount ?maxCount }}
                        OPTIONAL {{ ?prop sh:class ?class }}
                        OPTIONAL {{ ?prop sh:datatype ?datatype }}
                        OPTIONAL {{ ?prop sh:nodeKind ?nodeKind }}
                        OPTIONAL {{ ?prop sh:hasValue ?hasValue }}
                        OPTIONAL {{
                            ?prop ?other ?_v .
                            FILTER(?other NOT IN (
                                sh:path, sh:minCount, sh:maxCount, sh:class,
                                sh:datatype, sh:nodeKind, sh:hasValue,
                                sh:name, sh:description, sh:message, sh:severity,
                                sh:order, sh:group,
                                <http://www.w3.org/1999/02/22-rdf-syntax-ns#type>
                            ))
                        }}
                    }}
                    "#,
                    shape_term
                ),
            )?;
            if !node_or_rows.is_empty() {
                let mut per_member: HashMap<String, Vec<String>> = HashMap::new();
                let mut uncompilable = false;
                for (i, row) in node_or_rows.iter().enumerate() {
                    let (Some(member), Some(path_raw)) = (row.get("member"), row.get("path")) else {
                        uncompilable = true;
                        break;
                    };
                    if row.get("other").is_some() || !path_raw.trim().starts_with('<') {
                        uncompilable = true;
                        break;
                    }
                    let path = strip_angle_brackets(path_raw);
                    let v = format!("orv{i}");
                    let mut clauses: Vec<String> = Vec::new();
                    if let Some(min) = row.get("minCount") {
                        match strip_quotes(min).parse::<u64>().ok() {
                            Some(0) => {}
                            Some(1) => clauses.push(format!("EXISTS {{ ?focus <{path}> ?{v} }}")),
                            _ => {
                                uncompilable = true;
                                break;
                            }
                        }
                    }
                    if let Some(max) = row.get("maxCount") {
                        match strip_quotes(max).parse::<u64>().ok() {
                            Some(0) => clauses.push(format!("NOT EXISTS {{ ?focus <{path}> ?{v} }}")),
                            Some(1) => clauses.push(format!(
                                "NOT EXISTS {{ ?focus <{path}> ?{v}a . ?focus <{path}> ?{v}b . FILTER(?{v}a != ?{v}b) }}"
                            )),
                            _ => {
                                uncompilable = true;
                                break;
                            }
                        }
                    }
                    if let Some(hv) = row.get("hasValue") {
                        clauses.push(format!("EXISTS {{ ?focus <{path}> {} }}", hv.trim()));
                    }
                    if let Some(c) = row.get("class") {
                        clauses.push(format!(
                            "NOT EXISTS {{ ?focus <{path}> ?{v} . FILTER NOT EXISTS {{ ?{v} <http://www.w3.org/1999/02/22-rdf-syntax-ns#type>/<http://www.w3.org/2000/01/rdf-schema#subClassOf>* <{}> }} }}",
                            strip_angle_brackets(c)
                        ));
                    }
                    if let Some(dt) = row.get("datatype") {
                        let dt = strip_angle_brackets(dt);
                        if datatype_is_indistinguishable_in_store(&dt) {
                            uncompilable = true;
                            break;
                        }
                        clauses.push(format!(
                            "NOT EXISTS {{ ?focus <{path}> ?{v} . FILTER(DATATYPE(?{v}) != <{dt}>) }}"
                        ));
                    }
                    if let Some(nk) = row.get("nodeKind") {
                        match node_kind_test(&strip_angle_brackets(nk), &v) {
                            Some(test) => clauses.push(format!(
                                "NOT EXISTS {{ ?focus <{path}> ?{v} . FILTER(!({test})) }}"
                            )),
                            None => {
                                uncompilable = true;
                                break;
                            }
                        }
                    }
                    if clauses.is_empty() {
                        uncompilable = true;
                        break;
                    }
                    per_member.entry(member.clone()).or_default().extend(clauses);
                }
                if uncompilable || per_member.is_empty() {
                    skipped.push(serde_json::json!({
                        "shape": strip_angle_brackets(shape_term),
                        "constraint": "sh:or",
                        "reason": "a member shape uses a form that cannot be compiled; \
                                   the disjunction was not evaluated",
                    }));
                } else {
                    let mut members: Vec<String> = per_member
                        .values()
                        .map(|c| format!("({})", c.join(" && ")))
                        .collect();
                    members.sort();
                    let disjunction = members.join(" || ");
                    let node_message = query_solutions(
                        &shapes_store,
                        &format!(
                            r#"PREFIX sh: <http://www.w3.org/ns/shacl#>
                               SELECT ?m WHERE {{ {} sh:message ?m }}"#,
                            shape_term
                        ),
                    )?
                    .first()
                    .and_then(|r| r.get("m").map(|m| strip_quotes(m)))
                    .unwrap_or_default();
                    let query = format!(
                        r#"SELECT DISTINCT ?focus WHERE {{
                            {focus_pattern}
                            FILTER(!({disjunction}))
                        }}"#
                    );
                    for row in &graph_sparql_select(graph, &query)? {
                        if let Some(focus) = row.get("focus") {
                            let msg = if node_message.is_empty() {
                                "Node conforms to none of the sh:or member shapes".to_string()
                            } else {
                                node_message.clone()
                            };
                            violations.push(serde_json::json!({
                                "severity": "Violation",
                                "focus_node": strip_angle_brackets(focus),
                                "constraint": "or",
                                "message": msg,
                            }));
                        }
                    }
                }
            }

            // sh:qualifiedValueShape with sh:qualifiedMinCount / sh:qualifiedMaxCount:
            // how many of a path's value nodes conform to a nested shape.
            //
            // Collected as independent entries, deliberately NOT keyed by path the
            // way sh:or, sh:in and sh:not are. One shape may carry several qualified
            // constraints on the same path, each with its own nested shape, bounds
            // and message: the investment-fund vertical requires exactly one SEC
            // series identifier and at least one LEI, both on ifo:identifiedBy.
            // Keying by path merges the two into one rule and drops a bound.
            //
            // Binding ?prop keeps two blocks distinct even when they share a path
            // and a nested class, which DISTINCT over the other columns would fold
            // together.
            let qualified_rows = query_solutions(
                &shapes_store,
                &format!(
                    r#"
                    PREFIX sh: <http://www.w3.org/ns/shacl#>
                    PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
                    SELECT ?prop ?path ?class ?datatype ?hasValue ?other ?qmin ?qmax ?message ?severity WHERE {{
                        {} sh:property ?prop .
                        ?prop sh:path ?path ; sh:qualifiedValueShape ?q .
                        OPTIONAL {{ ?q sh:class ?class }}
                        OPTIONAL {{ ?q sh:datatype ?datatype }}
                        OPTIONAL {{ ?q sh:hasValue ?hasValue }}
                        OPTIONAL {{
                            ?q ?other ?_v .
                            FILTER(?other NOT IN (sh:class, sh:datatype, sh:hasValue, rdf:type))
                        }}
                        OPTIONAL {{ ?prop sh:qualifiedMinCount ?qmin }}
                        OPTIONAL {{ ?prop sh:qualifiedMaxCount ?qmax }}
                        OPTIONAL {{ ?prop sh:message ?message }}
                        OPTIONAL {{ ?prop sh:severity ?severity }}
                    }}
                    "#,
                    shape_iri
                ),
            )?;
            for row in &qualified_rows {
                let Some(path_raw) = row.get("path") else {
                    continue;
                };
                let q_path = strip_angle_brackets(path_raw);
                let q_message = row.get("message").map(|m| strip_quotes(m)).unwrap_or_default();
                let q_severity = row
                    .get("severity")
                    .map(|s| {
                        strip_angle_brackets(s)
                            .rsplit('#')
                            .next()
                            .unwrap_or("Violation")
                            .to_string()
                    })
                    .unwrap_or_else(|| "Violation".to_string());

                let clause = if row.get("other").is_some() {
                    None
                } else if let Some(c) = row.get("class") {
                    Some(format!(
                        "EXISTS {{ ?val <http://www.w3.org/1999/02/22-rdf-syntax-ns#type>/<http://www.w3.org/2000/01/rdf-schema#subClassOf>* <{}> }}",
                        strip_angle_brackets(c)
                    ))
                } else if let Some(dt) = row.get("datatype") {
                    Some(format!("DATATYPE(?val) = <{}>", strip_angle_brackets(dt)))
                } else {
                    row.get("hasValue").map(|hv| format!("?val = {}", hv.trim()))
                };
                let Some(clause) = clause else {
                    skipped.push(serde_json::json!({
                        "shape": strip_angle_brackets(&shape_iri),
                        "constraint": "sh:qualifiedValueShape",
                        "path": q_path,
                        "reason": "the nested shape uses a form that cannot be evaluated; \
                                   the qualified count was not taken",
                    }));
                    continue;
                };
                let q_min = row.get("qmin").and_then(|v| strip_quotes(v).parse::<i64>().ok());
                let q_max = row.get("qmax").and_then(|v| strip_quotes(v).parse::<i64>().ok());
                if q_min.is_none() && q_max.is_none() {
                    continue;
                }
                // OPTIONAL, so a focus node matching nothing still returns a row
                // with a count of zero. An inner join would hide exactly the nodes
                // a minimum count exists to catch.
                let query = format!(
                    r#"SELECT ?focus (COUNT(DISTINCT ?val) AS ?n) WHERE {{
                        {focus_pattern}
                        OPTIONAL {{ ?focus <{q_path}> ?val . FILTER({clause}) }}
                    }} GROUP BY ?focus"#
                );
                for result in &graph_sparql_select(graph, &query)? {
                    let (Some(focus), Some(n_raw)) = (result.get("focus"), result.get("n")) else {
                        continue;
                    };
                    let Ok(n) = strip_quotes(n_raw).parse::<i64>() else {
                        continue;
                    };
                    for (bound, breached, constraint, wording) in [
                        (q_min, q_min.is_some_and(|m| n < m), "qualifiedMinCount", "fewer than"),
                        (q_max, q_max.is_some_and(|m| n > m), "qualifiedMaxCount", "more than"),
                    ] {
                        if !breached {
                            continue;
                        }
                        let msg = if q_message.is_empty() {
                            format!(
                                "{n} value(s) conform to the qualified shape, {wording} the required {}",
                                bound.unwrap_or_default()
                            )
                        } else {
                            q_message.clone()
                        };
                        violations.push(serde_json::json!({
                            "severity": q_severity,
                            "focus_node": strip_angle_brackets(focus),
                            "path": q_path,
                            "constraint": constraint,
                            "message": msg,
                        }));
                    }
                }
            }

            // sh:in alternatives, collected per shape and keyed by path for the
            // same blank-node reason as sh:or above.
            let mut in_alternatives: HashMap<String, Vec<String>> = HashMap::new();
            let in_rows = query_solutions(
                &shapes_store,
                &format!(
                    r#"
                    PREFIX sh: <http://www.w3.org/ns/shacl#>
                    PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
                    SELECT ?prop ?path ?member WHERE {{
                        {} sh:property ?prop .
                        ?prop sh:path ?path .
                        ?prop sh:in/rdf:rest*/rdf:first ?member .
                    }}
                    "#,
                    shape_iri
                ),
            )?;
            for row in &in_rows {
                if let (Some(p), Some(m)) = (row.get("prop"), row.get("member")) {
                    in_alternatives
                        .entry(p.clone())
                        .or_default()
                        .push(m.trim().to_string());
                }
            }

            // 4. For each constraint, run SPARQL queries against the main graph
            for prop in &props {
                // Identifies THIS property shape among any that share its path.
                // sh:or, sh:in and sh:not are collected per shape and looked up
                // by this term; keying them by path merged sibling blocks into
                // one and returned a clean run over data that broke both.
                let prop_key = prop.get("prop").cloned();
                let raw_path = match prop.get("path") {
                    Some(p) => strip_angle_brackets(p),
                    None => continue,
                };

                // sh:path is either a direct IRI, or a blank node carrying a
                // property-path expression. sh:inversePath maps onto SPARQL's
                // `^` operator; any other blank-node path (sequence,
                // alternative, zero-or-more) is skipped and reported rather
                // than injected into a query it would break.
                let (path, path_expr) = match prop.get("invPath") {
                    Some(inv) => {
                        let inv = strip_angle_brackets(inv);
                        (format!("^{}", inv), format!("^<{}>", inv))
                    }
                    None if raw_path.starts_with("_:") => {
                        skipped.push(serde_json::json!({
                            "shape": strip_angle_brackets(&shape_iri),
                            "reason": "unsupported property path (only direct IRIs and sh:inversePath are executable)",
                        }));
                        continue;
                    }
                    None => (raw_path.clone(), format!("<{}>", raw_path)),
                };

                let message = prop
                    .get("message")
                    .map(|m| strip_quotes(m))
                    .unwrap_or_default();

                // sh:severity, defaulting to sh:Violation per the SHACL spec.
                let severity = prop
                    .get("severity")
                    .map(|s| {
                        let s = strip_angle_brackets(s);
                        s.rsplit('#').next().unwrap_or("Violation").to_string()
                    })
                    .unwrap_or_else(|| "Violation".to_string());

                // sh:minCount
                if let Some(min_count_str) = prop.get("minCount") {
                    let min_count = strip_quotes(min_count_str)
                        .parse::<u64>()
                        .unwrap_or(0);
                    if min_count > 0 {
                        let query = format!(
                            r#"SELECT ?focus (COUNT(DISTINCT ?val) AS ?cnt) WHERE {{
                                {focus_pattern}
                                OPTIONAL {{ ?focus {path_expr} ?val }}
                            }} GROUP BY ?focus HAVING (COUNT(DISTINCT ?val) < {min_count})"#
                        );
                        let results = graph_sparql_select(graph, &query)?;
                        for row in &results {
                            if let Some(focus) = row.get("focus") {
                                let msg = if message.is_empty() {
                                    format!(
                                        "Property <{}> has fewer than {} values",
                                        path, min_count
                                    )
                                } else {
                                    message.clone()
                                };
                                violations.push(serde_json::json!({
                                    "severity": severity,
                                    "focus_node": strip_angle_brackets(focus),
                                    "path": path,
                                    "constraint": "minCount",
                                    "message": msg,
                                }));
                            }
                        }
                    }
                }

                // sh:maxCount
                if let Some(max_count_str) = prop.get("maxCount") {
                    let max_count = strip_quotes(max_count_str)
                        .parse::<u64>()
                        .unwrap_or(u64::MAX);
                    let query = format!(
                        r#"SELECT ?focus (COUNT(DISTINCT ?val) AS ?cnt) WHERE {{
                            {focus_pattern}
                            ?focus {path_expr} ?val .
                        }} GROUP BY ?focus HAVING (COUNT(DISTINCT ?val) > {max_count})"#
                    );
                    let results = graph_sparql_select(graph, &query)?;
                    for row in &results {
                        if let Some(focus) = row.get("focus") {
                            let msg = if message.is_empty() {
                                format!(
                                    "Property <{}> has more than {} values",
                                    path, max_count
                                )
                            } else {
                                message.clone()
                            };
                            violations.push(serde_json::json!({
                                "severity": severity,
                                "focus_node": strip_angle_brackets(focus),
                                "path": path,
                                "constraint": "maxCount",
                                "message": msg,
                            }));
                        }
                    }
                }

                // sh:class. Every value node must be a SHACL instance of the class,
                // which the specification defines as reachable by rdf:type followed by
                // zero or more rdfs:subClassOf steps. A literal is never a SHACL
                // instance of anything, and the anti-join below excludes literals for
                // free because a literal cannot appear in subject position.
                if let Some(cls_str) = prop.get("class") {
                    let cls = strip_angle_brackets(cls_str);
                    let query = format!(
                        r#"SELECT ?focus ?val WHERE {{
                            {focus_pattern}
                            ?focus {path_expr} ?val .
                            FILTER NOT EXISTS {{
                                ?val <http://www.w3.org/1999/02/22-rdf-syntax-ns#type>/<http://www.w3.org/2000/01/rdf-schema#subClassOf>* <{cls}> .
                            }}
                        }}"#
                    );
                    let results = graph_sparql_select(graph, &query)?;
                    for row in &results {
                        if let Some(focus) = row.get("focus") {
                            let msg = if message.is_empty() {
                                format!("Value is not a SHACL instance of <{}>", cls)
                            } else {
                                message.clone()
                            };
                            violations.push(serde_json::json!({
                                "severity": severity,
                                "focus_node": strip_angle_brackets(focus),
                                "path": path,
                                "constraint": "class",
                                "message": msg,
                            }));
                        }
                    }
                }

                // sh:datatype
                if let Some(dt_str) = prop.get("datatype") {
                    let dt = strip_angle_brackets(dt_str);
                    // The store cannot tell these apart from the type they are
                    // derived from, so the constraint is not decidable here and
                    // must not be answered. Asking anyway produced a violation
                    // for every value that satisfied the shape: nine of them in
                    // jsonld-escaping-conformance, which is how this was found.
                    if datatype_is_indistinguishable_in_store(&dt) {
                        skipped.push(serde_json::json!({
                            "shape": strip_angle_brackets(&shape_iri),
                            "constraint": "sh:datatype",
                            "path": path,
                            "datatype": dt,
                            "reason": "the store does not preserve this datatype IRI, so a value \
                                       carrying it is indistinguishable from one carrying the type \
                                       it derives from; the constraint was not evaluated",
                        }));
                        continue;
                    }
                    let query = format!(
                        r#"SELECT ?focus ?val WHERE {{
                            {focus_pattern}
                            ?focus {path_expr} ?val .
                            FILTER(DATATYPE(?val) != <{dt}>)
                        }}"#
                    );
                    let results = graph_sparql_select(graph, &query)?;
                    for row in &results {
                        if let Some(focus) = row.get("focus") {
                            let msg = if message.is_empty() {
                                format!(
                                    "Value does not have datatype <{}>",
                                    dt
                                )
                            } else {
                                message.clone()
                            };
                            violations.push(serde_json::json!({
                                "severity": severity,
                                "focus_node": strip_angle_brackets(focus),
                                "path": path,
                                "constraint": "datatype",
                                "message": msg,
                            }));
                        }
                    }
                }

                // sh:pattern (regex over the string form of each value node,
                // per SHACL; `sh:flags` is not supported and simply absent
                // from real-world shapes we have seen so far).
                if let Some(pattern_raw) = prop.get("pattern") {
                    let pattern = strip_quotes(pattern_raw);
                    let escaped = pattern.replace('\\', "\\\\").replace('"', "\\\"");
                    let query = format!(
                        r#"SELECT ?focus ?val WHERE {{
                            {focus_pattern}
                            ?focus {path_expr} ?val .
                            FILTER(!REGEX(STR(?val), "{escaped}"))
                        }}"#
                    );
                    let results = graph_sparql_select(graph, &query)?;
                    for row in &results {
                        if let Some(focus) = row.get("focus") {
                            let msg = if message.is_empty() {
                                format!("Value does not match pattern {}", pattern)
                            } else {
                                message.clone()
                            };
                            violations.push(serde_json::json!({
                                "severity": severity,
                                "focus_node": strip_angle_brackets(focus),
                                "path": path,
                                "constraint": "pattern",
                                "message": msg,
                            }));
                        }
                    }
                }

                // sh:or over a property shape: each value on the path must
                // satisfy at least one member of the list. The alternatives were
                // collected for this shape above and are keyed by path.
                //
                // The general form nests arbitrary shapes and is not attempted.
                // What is evaluated is the form that occurs in practice, a list
                // of leaf alternatives each carrying exactly one of sh:datatype,
                // sh:class or sh:hasValue. The motivating case is a date recorded
                // at day, month or year precision: three sh:datatype members that
                // no single constraint can express. A list containing any other
                // form was sent to `skipped` above rather than evaluated, because
                // a disjunction evaluated over only the alternatives that happened
                // to be understood is not the disjunction that was written, and
                // would report a violation for a value the shape permits.
                if let Some(clauses) = prop_key.as_ref().and_then(|k| or_alternatives.get(k)) {
                    let disjunction = clauses.join(" || ");
                    let query = format!(
                        r#"SELECT ?focus ?val WHERE {{
                            {focus_pattern}
                            ?focus {path_expr} ?val .
                            FILTER(!({disjunction}))
                        }}"#
                    );
                    let results = graph_sparql_select(graph, &query)?;
                    for row in &results {
                        if let Some(focus) = row.get("focus") {
                            let msg = if message.is_empty() {
                                "Value satisfies none of the sh:or alternatives".to_string()
                            } else {
                                message.clone()
                            };
                            violations.push(serde_json::json!({
                                "severity": severity,
                                "focus_node": strip_angle_brackets(focus),
                                "path": path,
                                "constraint": "or",
                                "message": msg,
                            }));
                        }
                    }
                }

                // sh:minLength / sh:maxLength over the string form of each value
                // node. SHACL measures the lexical form of a literal and the
                // string of an IRI, and makes a blank node a violation of either
                // bound because it has no string form to measure. Both bounds are
                // inclusive.
                for (key, constraint, cmp, wording) in [
                    ("minLength", "minLength", "<", "shorter than"),
                    ("maxLength", "maxLength", ">", "longer than"),
                ] {
                    let Some(bound_raw) = prop.get(key) else {
                        continue;
                    };
                    let Ok(bound) = strip_quotes(bound_raw).parse::<u64>() else {
                        skipped.push(serde_json::json!({
                            "shape": strip_angle_brackets(&shape_iri),
                            "constraint": format!("sh:{constraint}"),
                            "path": path,
                            "reason": "bound is not a non-negative integer; it was not evaluated",
                        }));
                        continue;
                    };
                    let query = format!(
                        r#"SELECT ?focus ?val WHERE {{
                            {focus_pattern}
                            ?focus {path_expr} ?val .
                            FILTER(isBlank(?val) || STRLEN(STR(?val)) {cmp} {bound})
                        }}"#
                    );
                    for row in &graph_sparql_select(graph, &query)? {
                        if let Some(focus) = row.get("focus") {
                            let msg = if message.is_empty() {
                                format!("Value is {wording} {bound} characters")
                            } else {
                                message.clone()
                            };
                            violations.push(serde_json::json!({
                                "severity": severity,
                                "focus_node": strip_angle_brackets(focus),
                                "path": path,
                                "constraint": constraint,
                                "message": msg,
                            }));
                        }
                    }
                }

                // sh:lessThan / sh:lessThanOrEquals compare the values of this
                // path against the values of another property on the SAME focus
                // node, which is why they take a predicate and not a value. A
                // pair that cannot be ordered (comparing a string to an integer)
                // makes the SPARQL comparison an error rather than false, and an
                // error inside FILTER is dropped, so such a pair goes unreported
                // instead of counting as a violation. That is the one place these
                // two are weaker than pyshacl, and it is stated here rather than
                // discovered later.
                for (key, constraint, ok_cmp) in [
                    ("lessThan", "lessThan", "<"),
                    ("lessThanOrEquals", "lessThanOrEquals", "<="),
                ] {
                    let Some(other_raw) = prop.get(key) else {
                        continue;
                    };
                    let other = strip_angle_brackets(other_raw);
                    let query = format!(
                        r#"SELECT DISTINCT ?focus WHERE {{
                            {focus_pattern}
                            ?focus {path_expr} ?val .
                            ?focus <{other}> ?otherVal .
                            FILTER(!(?val {ok_cmp} ?otherVal))
                        }}"#
                    );
                    for row in &graph_sparql_select(graph, &query)? {
                        if let Some(focus) = row.get("focus") {
                            let msg = if message.is_empty() {
                                format!("Value is not {ok_cmp} the value of <{other}>")
                            } else {
                                message.clone()
                            };
                            violations.push(serde_json::json!({
                                "severity": severity,
                                "focus_node": strip_angle_brackets(focus),
                                "path": path,
                                "constraint": constraint,
                                "message": msg,
                            }));
                        }
                    }
                }

                // sh:node: every value on the path must conform to another
                // node shape. The referenced shape is compiled into a boolean
                // expression over the value node; if any part of it cannot be
                // compiled the whole constraint is recorded as unevaluated,
                // because a half-checked nested shape reads as a clean one.
                if let Some(node_ref) = prop.get("node") {
                    match compile_node_shape(&shapes_store, node_ref.trim(), "nodeval", 0) {
                        Some(expr) => {
                            let query = format!(
                                r#"SELECT DISTINCT ?focus WHERE {{
                                    {focus_pattern}
                                    ?focus {path_expr} ?nodeval .
                                    FILTER(!({expr}))
                                }}"#
                            );
                            for row in &graph_sparql_select(graph, &query)? {
                                if let Some(focus) = row.get("focus") {
                                    let msg = if message.is_empty() {
                                        format!(
                                            "Value does not conform to <{}>",
                                            strip_angle_brackets(node_ref)
                                        )
                                    } else {
                                        message.clone()
                                    };
                                    violations.push(serde_json::json!({
                                        "severity": severity,
                                        "focus_node": strip_angle_brackets(focus),
                                        "path": path,
                                        "constraint": "node",
                                        "message": msg,
                                    }));
                                }
                            }
                        }
                        None => skipped.push(serde_json::json!({
                            "shape": strip_angle_brackets(&shape_iri),
                            "constraint": "sh:node",
                            "path": path,
                            "node_shape": strip_angle_brackets(node_ref),
                            "reason": "the referenced shape uses a form that cannot be compiled, \
                                       or nests deeper than the bound; it was not evaluated",
                        })),
                    }
                }

                // sh:not: no value on the path may satisfy the nested shape.
                // The clauses were collected for this shape above and keyed by
                // path; a value matching one of them is the violation, which is
                // the sh:or filter with the negation removed rather than added.
                if let Some(clauses) = prop_key.as_ref().and_then(|k| not_clauses.get(k)) {
                    let conjunction = clauses.join(" && ");
                    let query = format!(
                        r#"SELECT ?focus ?val WHERE {{
                            {focus_pattern}
                            ?focus {path_expr} ?val .
                            FILTER({conjunction})
                        }}"#
                    );
                    let results = graph_sparql_select(graph, &query)?;
                    for row in &results {
                        if let Some(focus) = row.get("focus") {
                            let msg = if message.is_empty() {
                                "Value satisfies a shape forbidden by sh:not".to_string()
                            } else {
                                message.clone()
                            };
                            violations.push(serde_json::json!({
                                "severity": severity,
                                "focus_node": strip_angle_brackets(focus),
                                "path": path,
                                "constraint": "not",
                                "message": msg,
                            }));
                        }
                    }
                }

                // sh:in: the value must be one of an enumerated list of terms.
                // Members arrive from the shapes store in N-Triples form, which is
                // valid SPARQL as written, so no term needs rebuilding.
                if let Some(members) = prop_key.as_ref().and_then(|k| in_alternatives.get(k)) {
                    let list = members.join(", ");
                    let query = format!(
                        r#"SELECT ?focus ?val WHERE {{
                            {focus_pattern}
                            ?focus {path_expr} ?val .
                            FILTER(?val NOT IN ({list}))
                        }}"#
                    );
                    let results = graph_sparql_select(graph, &query)?;
                    for row in &results {
                        if let Some(focus) = row.get("focus") {
                            let msg = if message.is_empty() {
                                "Value is not one of the permitted sh:in terms".to_string()
                            } else {
                                message.clone()
                            };
                            violations.push(serde_json::json!({
                                "severity": severity,
                                "focus_node": strip_angle_brackets(focus),
                                "path": path,
                                "constraint": "in",
                                "message": msg,
                            }));
                        }
                    }
                }

                // sh:nodeKind. A node kind this implementation does not recognise
                // reaches skipped rather than passing: an unrecognised kind is not
                // a satisfied one.
                if let Some(nk_raw) = prop.get("nodeKind") {
                    let nk = strip_angle_brackets(nk_raw);
                    let kind = nk.rsplit('#').next().unwrap_or_default();
                    let test = match kind {
                        "IRI" => Some("isIRI(?val)".to_string()),
                        "Literal" => Some("isLiteral(?val)".to_string()),
                        "BlankNode" => Some("isBlank(?val)".to_string()),
                        "BlankNodeOrIRI" => Some("(isBlank(?val) || isIRI(?val))".to_string()),
                        "IRIOrLiteral" => Some("(isIRI(?val) || isLiteral(?val))".to_string()),
                        "BlankNodeOrLiteral" => {
                            Some("(isBlank(?val) || isLiteral(?val))".to_string())
                        }
                        _ => None,
                    };
                    match test {
                        Some(t) => {
                            let query = format!(
                                r#"SELECT ?focus ?val WHERE {{
                                    {focus_pattern}
                                    ?focus {path_expr} ?val .
                                    FILTER(!{t})
                                }}"#
                            );
                            let results = graph_sparql_select(graph, &query)?;
                            for row in &results {
                                if let Some(focus) = row.get("focus") {
                                    let msg = if message.is_empty() {
                                        format!("Value is not of node kind {}", kind)
                                    } else {
                                        message.clone()
                                    };
                                    violations.push(serde_json::json!({
                                        "severity": severity,
                                        "focus_node": strip_angle_brackets(focus),
                                        "path": path,
                                        "constraint": "nodeKind",
                                        "message": msg,
                                    }));
                                }
                            }
                        }
                        None => skipped.push(serde_json::json!({
                            "shape": strip_angle_brackets(&shape_iri),
                            "constraint": "sh:nodeKind",
                            "path": path,
                            "reason": "node kind not recognised; it was not evaluated",
                        })),
                    }
                }

                // sh:hasValue: every focus node must carry the exact term at
                // least once on the path. The term arrives from the shapes
                // store in N-Triples form (`<iri>` or `"lit"^^<dt>`), which is
                // valid SPARQL as-is.
                if let Some(has_value_term) = prop.get("hasValue") {
                    let term = has_value_term.trim();
                    let query = format!(
                        r#"SELECT ?focus WHERE {{
                            {focus_pattern}
                            FILTER NOT EXISTS {{ ?focus {path_expr} {term} }}
                        }}"#
                    );
                    let results = graph_sparql_select(graph, &query)?;
                    for row in &results {
                        if let Some(focus) = row.get("focus") {
                            let msg = if message.is_empty() {
                                format!("Required value {} is not present", term)
                            } else {
                                message.clone()
                            };
                            violations.push(serde_json::json!({
                                "severity": severity,
                                "focus_node": strip_angle_brackets(focus),
                                "path": path,
                                "constraint": "hasValue",
                                "message": msg,
                            }));
                        }
                    }
                }

                // sh:minInclusive, sh:maxInclusive, sh:minExclusive and
                // sh:maxExclusive. SHACL 4.6.1/4.6.2: a value node is a violation
                // wherever the SATISFYING comparison does not return true. A type
                // error (a string or date against a numeric bound) and NaN both
                // "do not return true", so they are violations, not passes. We flag
                // on the negated satisfying comparison wrapped in COALESCE(_, false):
                // an errored comparison collapses to false, whose negation flags it,
                // instead of an affirmative FILTER silently dropping the row. The
                // bound arrives from the shapes store in N-Triples form, valid SPARQL
                // as-is, exactly as for sh:hasValue above.
                for (key, satisfy_op, label) in [
                    ("minInclusive", ">=", "sh:minInclusive"),
                    ("maxInclusive", "<=", "sh:maxInclusive"),
                    ("minExclusive", ">", "sh:minExclusive"),
                    ("maxExclusive", "<", "sh:maxExclusive"),
                ] {
                    if let Some(bound_raw) = prop.get(key) {
                        let bound = bound_raw.trim();
                        let query = format!(
                            r#"SELECT ?focus ?val WHERE {{
                                {focus_pattern}
                                ?focus {path_expr} ?val .
                                FILTER(!COALESCE(?val {satisfy_op} {bound}, false))
                            }}"#
                        );
                        let results = graph_sparql_select(graph, &query)?;
                        for row in &results {
                            if let Some(focus) = row.get("focus") {
                                let msg = if message.is_empty() {
                                    format!("Value violates {} {}", label, bound)
                                } else {
                                    message.clone()
                                };
                                violations.push(serde_json::json!({
                                    "severity": severity,
                                    "focus_node": strip_angle_brackets(focus),
                                    "path": path,
                                    "constraint": key,
                                    "message": msg,
                                }));
                            }
                        }
                    }
                }
            }
        }

        // 5. SPARQL-based constraints (sh:sparql).
        //
        // These were previously not read at all, which meant a shapes file built
        // entirely on sh:sparql returned conforms:true having evaluated nothing.
        // A validator that reports success on rules it never ran is worse than
        // one that refuses, so every constraint here is either executed or
        // recorded in skipped_constraints, and skipping suppresses `conforms`.
        for (shape_term, kind, target_value) in &targets {
            let this_pattern = target_pattern(kind, target_value, "this");
            let shape_iri = shape_term.clone();

            let constraints = query_solutions(
                &shapes_store,
                &format!(
                    r#"
                    PREFIX sh: <http://www.w3.org/ns/shacl#>
                    SELECT ?select ?message ?severity WHERE {{
                        {} sh:sparql ?c .
                        ?c sh:select ?select .
                        OPTIONAL {{ ?c sh:message ?message }}
                        OPTIONAL {{ ?c sh:severity ?severity }}
                    }}
                    "#,
                    shape_iri
                ),
            )?;
            if constraints.is_empty() {
                continue;
            }

            // Focus nodes for this shape. Blank nodes are excluded because they
            // cannot be named in a VALUES clause; excluding them is recorded
            // rather than assumed harmless.
            let focus_rows = graph_sparql_select(
                graph,
                &format!("SELECT ?this WHERE {{ {this_pattern} }}"),
            )?;
            let focus_nodes: Vec<String> = focus_rows
                .iter()
                .filter_map(|r| r.get("this"))
                .filter(|t| t.starts_with('<'))
                .cloned()
                .collect();
            let blank_focus = focus_rows.len() - focus_nodes.len();
            if blank_focus > 0 {
                skipped.push(serde_json::json!({
                    "shape": strip_angle_brackets(&shape_iri),
                    "reason": format!(
                        "{} blank-node focus nodes excluded from sh:sparql evaluation (blank nodes cannot be bound in a VALUES clause)",
                        blank_focus
                    ),
                }));
            }
            if focus_nodes.is_empty() {
                continue;
            }

            let prefix_block = sparql_prefix_block(&shapes_store)?;

            for constraint in &constraints {
                let select_raw = match constraint.get("select") {
                    Some(s) => strip_quotes(s),
                    None => continue,
                };
                let message = constraint
                    .get("message")
                    .map(|m| strip_quotes(m))
                    .unwrap_or_default();
                let severity = constraint
                    .get("severity")
                    .map(|s| {
                        strip_angle_brackets(s)
                            .rsplit('#')
                            .next()
                            .unwrap_or("Violation")
                            .to_string()
                    })
                    .unwrap_or_else(|| "Violation".to_string());

                // SHACL pre-binds $this to the focus node. Rewrite it to the
                // ordinary variable ?this and bind it through a VALUES clause,
                // wrapping the author's SELECT as a subquery so that nothing is
                // spliced into the middle of their query text.
                let inner = select_raw.replace("$this", "?this");
                // The author's own PREFIX and BASE declarations have to be lifted
                // out of the subquery position and put where SPARQL allows them,
                // or the wrapper cannot parse at all. See split_sparql_prologue.
                let (author_prologue, inner_body) = split_sparql_prologue(&inner);
                let values = focus_nodes.join(" ");
                let wrapped = format!(
                    "{prefix_block}{author_prologue}SELECT ?this WHERE \
                     {{ VALUES ?this {{ {values} }} {{ {inner_body} }} }}"
                );

                match graph_sparql_select(graph, &wrapped) {
                    Ok(rows) => {
                        for row in &rows {
                            if let Some(focus) = row.get("this") {
                                let msg = if message.is_empty() {
                                    "SPARQL constraint violated".to_string()
                                } else {
                                    message.clone()
                                };
                                violations.push(serde_json::json!({
                                    "severity": severity,
                                    "focus_node": strip_angle_brackets(focus),
                                    "constraint": "sparql",
                                    "message": msg,
                                }));
                            }
                        }
                    }
                    Err(e) => {
                        // Most often an undeclared prefix inside the author's
                        // SELECT. Report it; never let it read as conformance.
                        skipped.push(serde_json::json!({
                            "shape": strip_angle_brackets(&shape_iri),
                            "constraint": "sparql",
                            "reason": format!("sh:sparql constraint could not be executed: {}", e),
                        }));
                    }
                }
            }
        }

        // A validator has three answers, not two. `true` means every constraint
        // ran and none failed, `false` means one failed, and null means the
        // question could not be answered. Every path that can select nothing
        // or execute nothing has to reach the third answer: a target that
        // selects nothing lands in `nothing_matched`, a target form that is
        // not implemented lands in `skipped`, and a constraint that cannot
        // execute lands in `skipped` whether it sits on a property shape or on
        // the node shape itself. A construct added later that is neither
        // evaluated nor routed to one of those is the false clean this module
        // exists to prevent; the reachability test in tests/shacl_test.rs is
        // where it should fail.
        //
        // A shapes graph that declared shapes we could not discover must not be
        // reported as a pass. `shapes.is_empty()` used to fall through to
        // `conforms: violations.is_empty()`, which is `true` for an empty run.
        let declared_any_shape = query_solutions(
            &shapes_store,
            r#"PREFIX sh: <http://www.w3.org/ns/shacl#>
               SELECT ?s WHERE { ?s a sh:NodeShape . }"#,
        )
        .map(|r| !r.is_empty())
        .unwrap_or(false);
        // Counted over targets, not over class-targeted shapes. While
        // `sh:targetClass` was the only form that selected anything, a shapes
        // graph declaring a NodeShape and yielding no class target had selected
        // nothing, and that is what this measured. It no longer follows: such a
        // graph may target by node, by subjects-of or by objects-of and select
        // plenty. Left on `shapes`, the guard suppressed the verdict of every
        // run whose targets were all explicit, however many nodes they checked.
        let nothing_matched = (!targets.is_empty() && focus_nodes_total == 0)
            || (targets.is_empty() && declared_any_shape);

        let mut report = serde_json::json!({
            "violation_count": violations.len(),
            "violations": violations,
            "focus_nodes": focus_nodes_total,
            "unmatched_shapes": unmatched,
            // A verdict that does not say what it selected over cannot be
            // replayed or compared against the next one, and this value is
            // about to stop being the only one: temporal scoping arrives as
            // an argument, and the moment two runs of the same shapes over
            // the same store can differ, an unlabelled report is a number
            // without units. Naming it now means the key does not appear for
            // the first time on the run where it matters.
            "scope": "all_graphs",
        });
        if nothing_matched && skipped.is_empty() {
            // Every shape targeted a class with no instances in the data, so
            // nothing was checked. Reporting `conforms: true` here would be the
            // same lie as reporting it for a constraint that never ran.
            report["conforms"] = serde_json::Value::Null;
            report["warning"] = serde_json::Value::String(format!(
                "no focus nodes matched: all {} target(s) selected nothing in the data, so conformance is undetermined. See unmatched_shapes.",
                targets.len()
            ));
        } else if skipped.is_empty() {
            report["conforms"] = serde_json::Value::Bool(violations.is_empty());
        } else {
            // Some constraints in this shapes graph were not evaluated, so no
            // conformance verdict can honestly be given. Null rather than true.
            report["conforms"] = serde_json::Value::Null;
            report["warning"] = serde_json::Value::String(format!(
                "{} constraint(s) were not evaluated, so conformance is undetermined. See skipped_constraints.",
                skipped.len()
            ));
            report["skipped_constraints"] = serde_json::Value::Array(skipped);
        }

        Ok(report.to_string())
    }

    /// Structural dry-run check on proposed SHACL shapes.
    ///
    /// Verifies that the shapes parse as Turtle and that every IRI they reference
    /// (`sh:targetClass`, `sh:path`, `sh:class`) actually exists in the loaded
    /// ontology, plus a lightweight XSD-prefix check on `sh:datatype`. Does NOT
    /// validate data against the shapes — that's `validate`. This is the primitive
    /// the orchestrating LLM needs to iterate on proposed SHACL before applying.
    ///
    /// Output is a JSON report with `ok` (true if no structural issues), `parses`,
    /// `shape_count`, and an `issues` array describing each missing reference.
    pub fn check_shapes(graph: &Arc<GraphStore>, shapes_ttl: &str) -> anyhow::Result<String> {
        // 1. Parse the proposed shapes into a temporary Oxigraph store.
        let shapes_store = Store::new()?;
        let reader = Cursor::new(shapes_ttl.as_bytes());
        let parser = RdfParser::from_format(RdfFormat::Turtle).for_reader(reader);
        for quad in parser {
            match quad {
                Ok(q) => shapes_store.insert(&q)?,
                Err(e) => {
                    return Ok(serde_json::json!({
                        "ok": false,
                        "parses": false,
                        "parse_error": format!("{}", e),
                        "issues": [],
                        "issue_count": 0,
                        "shape_count": 0,
                    })
                    .to_string());
                }
            };
        }

        // 2. Walk every NodeShape and collect its referenced IRIs (target_class +
        //    per-property path + optional class constraint + datatype).
        let shapes = query_solutions(
            &shapes_store,
            r#"
            PREFIX sh: <http://www.w3.org/ns/shacl#>
            SELECT ?shape ?targetClass WHERE {
                ?shape a sh:NodeShape ;
                       sh:targetClass ?targetClass .
            }
            "#,
        )?;

        let mut issues: Vec<serde_json::Value> = Vec::new();
        let mut shape_reports: Vec<serde_json::Value> = Vec::new();

        for shape in &shapes {
            let shape_iri = match shape.get("shape") {
                Some(s) => s.clone(),
                None => continue,
            };
            let target_class = match shape.get("targetClass") {
                Some(tc) => strip_angle_brackets(tc),
                None => continue,
            };

            let target_class_exists = class_exists(graph, &target_class)?;
            if !target_class_exists {
                issues.push(serde_json::json!({
                    "shape": strip_angle_brackets(&shape_iri),
                    "kind": "missing_target_class",
                    "value": target_class,
                    "message": format!(
                        "sh:targetClass <{}> is not declared as owl:Class or rdfs:Class in the loaded ontology",
                        target_class
                    ),
                }));
            }

            let props = query_solutions(
                &shapes_store,
                &format!(
                    r#"
                    PREFIX sh: <http://www.w3.org/ns/shacl#>
                    SELECT ?prop ?path ?class ?datatype WHERE {{
                        {} sh:property ?prop .
                        ?prop sh:path ?path .
                        OPTIONAL {{ ?prop sh:class ?class }}
                        OPTIONAL {{ ?prop sh:datatype ?datatype }}
                    }}
                    "#,
                    shape_iri
                ),
            )?;

            let mut prop_reports: Vec<serde_json::Value> = Vec::new();
            for prop in &props {
                let path = match prop.get("path") {
                    Some(p) => strip_angle_brackets(p),
                    None => continue,
                };
                let path_exists = property_exists(graph, &path)?;
                if !path_exists {
                    issues.push(serde_json::json!({
                        "shape": strip_angle_brackets(&shape_iri),
                        "kind": "missing_path",
                        "value": path.clone(),
                        "message": format!(
                            "sh:path <{}> is not declared as a property (owl:ObjectProperty, owl:DatatypeProperty, or rdf:Property) in the loaded ontology",
                            path
                        ),
                    }));
                }

                let class_constraint = prop.get("class").map(|c| strip_angle_brackets(c));
                let class_exists_value = match &class_constraint {
                    Some(iri) => {
                        let exists = class_exists(graph, iri)?;
                        if !exists {
                            issues.push(serde_json::json!({
                                "shape": strip_angle_brackets(&shape_iri),
                                "kind": "missing_class_constraint",
                                "value": iri.clone(),
                                "message": format!(
                                    "sh:class <{}> is not declared as owl:Class or rdfs:Class in the loaded ontology",
                                    iri
                                ),
                            }));
                        }
                        Some(exists)
                    }
                    None => None,
                };

                let datatype = prop.get("datatype").map(|d| strip_angle_brackets(d));
                let datatype_ok = datatype.as_deref().map(is_recognised_xsd_datatype);
                if let (Some(dt), Some(false)) = (datatype.as_deref(), datatype_ok) {
                    let dt_owned: String = dt.to_owned();
                    issues.push(serde_json::json!({
                        "shape": strip_angle_brackets(&shape_iri),
                        "kind": "unrecognised_datatype",
                        "value": dt_owned,
                        "message": format!(
                            "sh:datatype <{}> does not look like an XSD datatype IRI (expected something starting with http://www.w3.org/2001/XMLSchema#)",
                            dt
                        ),
                    }));
                }

                prop_reports.push(serde_json::json!({
                    "path": path,
                    "path_exists": path_exists,
                    "class_constraint": class_constraint,
                    "class_constraint_exists": class_exists_value,
                    "datatype": datatype,
                    "datatype_recognised": datatype_ok,
                }));
            }

            shape_reports.push(serde_json::json!({
                "shape_iri": strip_angle_brackets(&shape_iri),
                "target_class": target_class,
                "target_class_exists": target_class_exists,
                "property_constraints": prop_reports,
            }));
        }

        let ok = issues.is_empty();
        Ok(serde_json::json!({
            "ok": ok,
            "parses": true,
            "shape_count": shape_reports.len(),
            "issue_count": issues.len(),
            "issues": issues,
            "shapes": shape_reports,
        })
        .to_string())
    }
}

/// Run a SPARQL SELECT against a temporary shapes `Store` and return results
/// as a vec of maps (variable name -> string value).
fn query_solutions(
    store: &Store,
    query: &str,
) -> anyhow::Result<Vec<HashMap<String, String>>> {
    match SparqlEvaluator::new()
        .parse_query(query)?
        .on_store(store)
        .execute()?
    {
        QueryResults::Solutions(solutions) => {
            let vars: Vec<String> = solutions
                .variables()
                .iter()
                .map(|v| v.as_str().to_string())
                .collect();
            let mut rows = Vec::new();
            for solution in solutions {
                let solution = solution?;
                let mut row = HashMap::new();
                for var in &vars {
                    if let Some(term) = solution.get(var.as_str()) {
                        row.insert(var.clone(), term.to_string());
                    }
                }
                rows.push(row);
            }
            Ok(rows)
        }
        _ => Ok(Vec::new()),
    }
}

/// Run a SPARQL SELECT against the main `GraphStore` and return results
/// as a vec of maps, using the existing `sparql_select` JSON output.
///
/// Every data-side query in this module runs over the union of every graph in
/// the store. It used to run over the store's default graph alone, which made
/// the verdict depend on the serialisation the data arrived in: an ontology
/// and its instances loaded from Turtle validated, and the identical triples
/// loaded from TriG selected no focus nodes at all and came back as
/// `nothing_matched` with a null verdict. That is the right answer to a
/// question nobody asked, and it is why the defect never arrived as a bug
/// report.
///
/// Reading every graph is the only default that cannot silently drop data.
/// The alternative, selecting instances from the graphs in temporal scope, is
/// the opposite direction: it can only remove focus nodes, so it can turn a
/// `conforms: false` into a `true` by dropping the data that failed. That
/// belongs behind an argument someone passes on purpose, never behind the
/// no-argument path. See the graph-scope rule on issue #108: a declaration is
/// read from every graph because a declaration is context-free, and instance
/// data is scoped because it is not.
fn graph_sparql_select(
    graph: &Arc<GraphStore>,
    query: &str,
) -> anyhow::Result<Vec<HashMap<String, String>>> {
    let json_str = graph.sparql_select_union(query)?;
    let parsed: serde_json::Value = serde_json::from_str(&json_str)?;
    let mut rows = Vec::new();
    if let Some(results) = parsed["results"].as_array() {
        for result in results {
            if let Some(obj) = result.as_object() {
                let mut row = HashMap::new();
                for (key, val) in obj {
                    if let Some(s) = val.as_str() {
                        row.insert(key.clone(), s.to_string());
                    }
                }
                rows.push(row);
            }
        }
    }
    Ok(rows)
}

/// Check whether `iri` is declared as `owl:Class` or `rdfs:Class` anywhere in
/// the store.
///
/// A declaration is a declaration wherever it lives. The bare pattern only
/// sees the default graph, so an ontology loaded from TriG or N-Quads, whose
/// declarations sit inside a `GRAPH` block, looked undeclared and every
/// shape that referenced it was reported as `missing_target_class` or
/// `missing_class_constraint`. The lookup therefore reads the union of the
/// default graph and every named graph, unconditionally: there is no scope
/// argument and no flag that narrows it back to the default graph.
fn class_exists(graph: &Arc<GraphStore>, iri: &str) -> anyhow::Result<bool> {
    let query = format!(
        r#"SELECT ?x WHERE {{
            {{ <{iri}> a ?type }}
            UNION
            {{ GRAPH ?g {{ <{iri}> a ?type }} }}
            FILTER(?type = <http://www.w3.org/2002/07/owl#Class>
                || ?type = <http://www.w3.org/2000/01/rdf-schema#Class>)
        }} LIMIT 1"#
    );
    let results = graph_sparql_select(graph, &query)?;
    Ok(!results.is_empty())
}

/// Check whether `iri` is declared as an `owl:ObjectProperty`,
/// `owl:DatatypeProperty`, or `rdf:Property` anywhere in the store.
///
/// Same rule as `class_exists`: the union of the default graph and every
/// named graph, unconditionally, so a property declared inside a `GRAPH`
/// block is not reported as `missing_path`.
fn property_exists(graph: &Arc<GraphStore>, iri: &str) -> anyhow::Result<bool> {
    let query = format!(
        r#"SELECT ?x WHERE {{
            {{ <{iri}> a ?type }}
            UNION
            {{ GRAPH ?g {{ <{iri}> a ?type }} }}
            FILTER(?type = <http://www.w3.org/2002/07/owl#ObjectProperty>
                || ?type = <http://www.w3.org/2002/07/owl#DatatypeProperty>
                || ?type = <http://www.w3.org/1999/02/22-rdf-syntax-ns#Property>)
        }} LIMIT 1"#
    );
    let results = graph_sparql_select(graph, &query)?;
    Ok(!results.is_empty())
}

/// Quick prefix check for XSD datatypes (the SHACL spec allows others,
/// but the overwhelming majority of real-world `sh:datatype` constraints are XSD).
fn is_recognised_xsd_datatype(iri: &str) -> bool {
    iri.starts_with("http://www.w3.org/2001/XMLSchema#")
}

/// Trim angle brackets from IRI strings like `<http://example.org/foo>`.
/// Count the distinct nodes a `sh:targetClass` shape applies to.
///
/// Used to tell "checked and clean" apart from "checked nothing": a shape whose
/// target class has no instances passes every constraint vacuously.
/// How deep `sh:node` is followed before the compiler gives up.
///
/// A shapes graph may reference itself, directly or around a cycle, and a
/// compiler that follows it has no natural stopping point. Bounding it and
/// reporting the bound is the only honest option: an unbounded compile does not
/// return, and a silent cutoff would produce a filter that checks less than the
/// shape says.
const MAX_NESTED_SHAPE_DEPTH: usize = 5;

/// Compile a node shape into a SPARQL boolean expression that is true when the
/// term bound to `var` conforms to it.
///
/// Returns `None` if any constraint in the shape, or in a shape it references,
/// cannot be compiled. That is deliberately all-or-nothing. A partially compiled
/// nested shape would answer for the constraints it understood and stay silent on
/// the rest, which reads to the caller as conformance with the whole shape: the
/// false clean this validator must not produce. The caller records the whole
/// `sh:node` as unevaluated instead.
///
/// The supported forms are the ones the HealthDCAT-AP shapes use, which is what
/// the health-dataset-catalogue vertical validates against: `sh:minCount` and
/// `sh:maxCount` of 0 or 1, `sh:class`, `sh:datatype`, `sh:nodeKind`,
/// `sh:hasValue`, and `sh:node` for the recursion. A count other than 0 or 1
/// needs an aggregate, which SPARQL will not evaluate inside a FILTER, so it is
/// not compiled rather than approximated.
fn compile_node_shape(
    shapes_store: &Store,
    shape_term: &str,
    var: &str,
    depth: usize,
) -> Option<String> {
    if depth > MAX_NESTED_SHAPE_DEPTH {
        return None;
    }
    let mut clauses: Vec<String> = Vec::new();

    // Constraints asserted on the shape node itself apply to the value node.
    let node_level = query_solutions(
        shapes_store,
        &format!(
            r#"
            PREFIX sh: <http://www.w3.org/ns/shacl#>
            SELECT ?pred ?obj WHERE {{
                {shape_term} ?pred ?obj .
                FILTER(?pred NOT IN (
                    sh:property, sh:name, sh:description, sh:message, sh:severity,
                    sh:order, sh:group, sh:targetClass, sh:targetNode,
                    sh:targetSubjectsOf, sh:targetObjectsOf,
                    <http://www.w3.org/1999/02/22-rdf-syntax-ns#type>,
                    <http://www.w3.org/2000/01/rdf-schema#label>,
                    <http://www.w3.org/2000/01/rdf-schema#comment>
                ))
            }}
            "#
        ),
    )
    .ok()?;
    for row in &node_level {
        let (Some(pred), Some(obj)) = (row.get("pred"), row.get("obj")) else {
            return None;
        };
        let pred = strip_angle_brackets(pred);
        let local = pred.rsplit('#').next().unwrap_or_default();
        match local {
            "class" => clauses.push(format!(
                "EXISTS {{ ?{var} <http://www.w3.org/1999/02/22-rdf-syntax-ns#type>/<http://www.w3.org/2000/01/rdf-schema#subClassOf>* <{}> }}",
                strip_angle_brackets(obj)
            )),
            "datatype" => clauses.push(format!(
                "DATATYPE(?{var}) = <{}>",
                strip_angle_brackets(obj)
            )),
            "nodeKind" => clauses.push(node_kind_test(&strip_angle_brackets(obj), var)?),
            _ => return None,
        }
    }

    // Property shapes. `?prop` is bound rather than spliced, for the same reason
    // it is elsewhere: a property shape is almost always a blank node, and a
    // blank-node label inside a query is a fresh variable, not a reference.
    let props = query_solutions(
        shapes_store,
        &format!(
            r#"
            PREFIX sh: <http://www.w3.org/ns/shacl#>
            SELECT ?prop ?path ?minCount ?maxCount ?class ?datatype ?nodeKind ?hasValue ?node ?other WHERE {{
                {shape_term} sh:property ?prop .
                ?prop sh:path ?path .
                OPTIONAL {{ ?prop sh:minCount ?minCount }}
                OPTIONAL {{ ?prop sh:maxCount ?maxCount }}
                OPTIONAL {{ ?prop sh:class ?class }}
                OPTIONAL {{ ?prop sh:datatype ?datatype }}
                OPTIONAL {{ ?prop sh:nodeKind ?nodeKind }}
                OPTIONAL {{ ?prop sh:hasValue ?hasValue }}
                OPTIONAL {{ ?prop sh:node ?node }}
                OPTIONAL {{
                    ?prop ?other ?_v .
                    FILTER(?other NOT IN (
                        sh:path, sh:minCount, sh:maxCount, sh:class, sh:datatype,
                        sh:nodeKind, sh:hasValue, sh:node,
                        sh:name, sh:description, sh:message, sh:severity,
                        sh:order, sh:group,
                        <http://www.w3.org/1999/02/22-rdf-syntax-ns#type>
                    ))
                }}
            }}
            "#
        ),
    )
    .ok()?;
    for (i, row) in props.iter().enumerate() {
        if row.get("other").is_some() {
            return None;
        }
        let path = strip_angle_brackets(row.get("path")?);
        // A blank-node path is a property-path expression, which this compiler
        // does not read. Not compiled rather than guessed at.
        if !row.get("path")?.trim().starts_with('<') {
            return None;
        }
        let inner = format!("{var}_{depth}_{i}");

        if let Some(min) = row.get("minCount") {
            match strip_quotes(min).parse::<u64>().ok()? {
                0 => {}
                1 => clauses.push(format!("EXISTS {{ ?{var} <{path}> ?{inner} }}")),
                _ => return None,
            }
        }
        if let Some(max) = row.get("maxCount") {
            match strip_quotes(max).parse::<u64>().ok()? {
                0 => clauses.push(format!("NOT EXISTS {{ ?{var} <{path}> ?{inner} }}")),
                1 => clauses.push(format!(
                    "NOT EXISTS {{ ?{var} <{path}> ?{inner}a . ?{var} <{path}> ?{inner}b . FILTER(?{inner}a != ?{inner}b) }}"
                )),
                _ => return None,
            }
        }
        if let Some(hv) = row.get("hasValue") {
            clauses.push(format!("EXISTS {{ ?{var} <{path}> {} }}", hv.trim()));
        }
        // The value-level constraints hold for EVERY value on the path, so each
        // is written as the absence of a counterexample.
        if let Some(c) = row.get("class") {
            clauses.push(format!(
                "NOT EXISTS {{ ?{var} <{path}> ?{inner} . FILTER NOT EXISTS {{ ?{inner} <http://www.w3.org/1999/02/22-rdf-syntax-ns#type>/<http://www.w3.org/2000/01/rdf-schema#subClassOf>* <{}> }} }}",
                strip_angle_brackets(c)
            ));
        }
        if let Some(dt) = row.get("datatype") {
            let dt = strip_angle_brackets(dt);
            // The store cannot tell these apart from the type they derive from,
            // so a nested shape resting on one is not decidable either.
            if datatype_is_indistinguishable_in_store(&dt) {
                return None;
            }
            clauses.push(format!(
                "NOT EXISTS {{ ?{var} <{path}> ?{inner} . FILTER(DATATYPE(?{inner}) != <{dt}>) }}"
            ));
        }
        if let Some(nk) = row.get("nodeKind") {
            let test = node_kind_test(&strip_angle_brackets(nk), &inner)?;
            clauses.push(format!(
                "NOT EXISTS {{ ?{var} <{path}> ?{inner} . FILTER(!({test})) }}"
            ));
        }
        if let Some(node) = row.get("node") {
            let nested = compile_node_shape(shapes_store, node.trim(), &inner, depth + 1)?;
            clauses.push(format!(
                "NOT EXISTS {{ ?{var} <{path}> ?{inner} . FILTER(!({nested})) }}"
            ));
        }
    }

    if clauses.is_empty() {
        // Nothing to check. That is only meaningful if the shapes graph actually
        // declares this shape: an empty conjunction is `true`, so a reference to
        // a shape defined in some other file would otherwise be satisfied by
        // every value node, a false clean produced by absence rather than by a
        // wrong answer. The HealthDCAT-AP shapes reference three DCAT-AP shapes
        // that live elsewhere, so this is the ordinary case.
        let declared = query_solutions(
            shapes_store,
            &format!(
                r#"PREFIX sh: <http://www.w3.org/ns/shacl#>
                   ASK_SUBSTITUTE
                   SELECT ?p WHERE {{ {shape_term} a ?type . FILTER(?type IN (sh:NodeShape, sh:PropertyShape)) }}"#
            )
            .replace("ASK_SUBSTITUTE\n                   ", ""),
        )
        .ok()?;
        if declared.is_empty() {
            return None;
        }
        return Some("true".to_string());
    }
    Some(clauses.join(" && "))
}

/// The SPARQL test for one `sh:nodeKind` value, or None for a value this
/// compiler does not know, so an unfamiliar node kind is never treated as passing.
fn node_kind_test(node_kind: &str, var: &str) -> Option<String> {
    Some(match node_kind.rsplit('#').next().unwrap_or_default() {
        "IRI" => format!("isIRI(?{var})"),
        "Literal" => format!("isLiteral(?{var})"),
        "BlankNode" => format!("isBlank(?{var})"),
        "BlankNodeOrIRI" => format!("(isBlank(?{var}) || isIRI(?{var}))"),
        "BlankNodeOrLiteral" => format!("(isBlank(?{var}) || isLiteral(?{var}))"),
        "IRIOrLiteral" => format!("(isIRI(?{var}) || isLiteral(?{var}))"),
        _ => return None,
    })
}

/// Datatype IRIs the storage layer does not preserve.
///
/// oxigraph 0.5 encodes a literal by value, and in `numeric_encoder.rs` twelve
/// XSD integer-derived datatype IRIs all route to `parse_integer_str`, which
/// yields `EncodedTerm::IntegerLiteral` — one variant, carrying no datatype IRI.
/// Reading back can only reconstruct `xsd:integer`. `xsd:dateTimeStamp` collapses
/// into `xsd:dateTime` the same way. The Turtle parser is correct; the loss is at
/// storage. That it is a defect rather than a deliberate simplification is settled
/// by `xsd:yearMonthDuration` and `xsd:dayTimeDuration`, equally derived, which
/// have their own encodings and survive intact.
///
/// A `sh:datatype` constraint naming one of these cannot be decided against the
/// store: a conforming value and a widened one are the same term by the time the
/// query runs. Answering anyway reports a violation for every value that in fact
/// satisfies the shape.
fn datatype_is_indistinguishable_in_store(datatype: &str) -> bool {
    const XSD: &str = "http://www.w3.org/2001/XMLSchema#";
    let Some(local) = datatype.strip_prefix(XSD) else {
        return false;
    };
    matches!(
        local,
        "byte"
            | "short"
            | "int"
            | "long"
            | "unsignedByte"
            | "unsignedShort"
            | "unsignedInt"
            | "unsignedLong"
            | "positiveInteger"
            | "negativeInteger"
            | "nonPositiveInteger"
            | "nonNegativeInteger"
            | "dateTimeStamp"
    )
}

/// Split a SPARQL query into its prologue (PREFIX and BASE declarations, with any
/// leading comments) and the body that follows.
///
/// SPARQL permits PREFIX and BASE only in the prologue, at the very start of a
/// query. SHACL pre-binds `$this`, and this validator binds it by wrapping the
/// author's SELECT as a subquery under a VALUES clause, which puts any prologue
/// the author wrote into a position where it cannot parse and takes the whole
/// query down with it. Hoisting it to the front of the wrapper is the fix.
///
/// Declaring prefixes inside `sh:select` is the portable way to write a SPARQL
/// constraint and is what pyshacl accepts. All seven `sh:sparql` constraints in
/// the banking vertical were being reported as unrunnable for this reason alone,
/// found by the differential run against pyshacl rather than by any unit test.
fn split_sparql_prologue(query: &str) -> (String, &str) {
    let mut prologue = String::new();
    let mut rest = query;
    loop {
        let trimmed = rest.trim_start();
        if let Some(line) = trimmed.strip_prefix('#') {
            // A comment carries no meaning to the parser but may carry a lot to
            // a reader, so it is moved rather than dropped.
            let end = line.find('\n').map(|i| i + 2).unwrap_or(trimmed.len());
            prologue.push_str(&trimmed[..end]);
            rest = &trimmed[end..];
            continue;
        }
        let lower = trimmed.to_ascii_lowercase();
        let keyword_len = if lower.starts_with("prefix") {
            6
        } else if lower.starts_with("base") {
            4
        } else {
            return (prologue, trimmed);
        };
        // Require a separator after the keyword so an identifier merely starting
        // with those letters is not mistaken for a declaration.
        match trimmed[keyword_len..].chars().next() {
            Some(c) if c.is_whitespace() || c == '<' => {}
            _ => return (prologue, trimmed),
        }
        // Every declaration ends at the closing '>' of its IRI.
        match trimmed.find('>') {
            Some(i) => {
                prologue.push_str(&trimmed[..=i]);
                prologue.push('\n');
                rest = &trimmed[i + 1..];
            }
            None => return (prologue, trimmed),
        }
    }
}

/// The SPARQL pattern selecting the focus nodes of one target, bound to `var`.
///
/// Note for anyone counting over the result: the class selector binds a focus
/// node ONCE PER PATH to the target class, so a node typed two ways under one
/// class is bound twice. Every count taken over this pattern must therefore
/// count DISTINCT value nodes. Counting rows instead inflated maxCount into 258
/// false violations on the investment-fund vertical, which loads a FIBO
/// alignment supplying the extra subclass paths, and would equally have hidden
/// a minCount breach.
///
/// One function, so the two passes that need focus nodes — property constraints
/// and `sh:sparql` constraints — cannot drift apart on what a target means. That
/// drift is not hypothetical: the two passes each had their own copy of the class
/// selector, and a fix to one would silently have left the other behind.
///
/// `sh:targetClass` selects SHACL instances: rdf:type followed by zero or more
/// rdfs:subClassOf steps. The other three forms are explicit, and `sh:targetNode`
/// uses VALUES rather than a triple pattern because it must select its node even
/// when that node appears in no triple.
fn target_pattern(kind: &str, value: &str, var: &str) -> String {
    match kind {
        "class" => format!(
            "?{var} <http://www.w3.org/1999/02/22-rdf-syntax-ns#type>/<http://www.w3.org/2000/01/rdf-schema#subClassOf>* <{value}>"
        ),
        "node" => format!("VALUES ?{var} {{ {value} }}"),
        "subjectsOf" => format!("?{var} {value} ?_target_object"),
        _ => format!("?_target_subject {value} ?{var}"),
    }
}

/// Count the focus nodes a target selects, given the SPARQL pattern that binds
/// `?focus`. Taking the pattern rather than a class is what lets all four target
/// forms share one counter, and keeps `focus_nodes` in the report meaning the
/// same thing whichever form selected them.
fn count_focus_nodes(graph: &Arc<GraphStore>, focus_pattern: &str) -> anyhow::Result<u64> {
    let query = format!(
        r#"SELECT (COUNT(DISTINCT ?focus) AS ?cnt) WHERE {{ {focus_pattern} }}"#
    );
    let rows = graph_sparql_select(graph, &query)?;
    Ok(rows
        .first()
        .and_then(|row| row.get("cnt"))
        .map(|c| strip_quotes(c))
        .and_then(|c| c.parse::<u64>().ok())
        .unwrap_or(0))
}

fn strip_angle_brackets(s: &str) -> String {
    let s = s.trim();
    if s.starts_with('<') && s.ends_with('>') {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

/// Trim quotes and handle typed literals like `"1"^^<http://...>`.
fn strip_quotes(s: &str) -> String {
    let s = s.trim();
    // Handle typed literals: "value"^^<datatype>
    let s = if let Some(idx) = s.find("^^") {
        &s[..idx]
    } else {
        s
    };
    // Handle language-tagged literals: "value"@en
    let s = if let Some(idx) = s.find("\"@") {
        &s[..idx + 1]
    } else {
        s
    };
    let s = s.trim_matches('"');
    unescape_literal(s)
}

/// Undo the N-Triples escaping that Oxigraph applies when rendering a literal
/// through `Term::to_string()`.
///
/// This matters well beyond cosmetics. A multi-line `sh:select` string arrives
/// here carrying the two characters backslash and n where the author wrote a
/// newline, and a SPARQL parser rejects that outright. Before this was fixed,
/// every multi-line SPARQL constraint failed to parse.
fn unescape_literal(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('b') => out.push('\u{8}'),
            Some('f') => out.push('\u{c}'),
            Some('"') => out.push('"'),
            Some('\'') => out.push('\''),
            Some('\\') => out.push('\\'),
            Some('u') => {
                let hex: String = chars.by_ref().take(4).collect();
                match u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32) {
                    Some(decoded) => out.push(decoded),
                    None => {
                        out.push_str("\\u");
                        out.push_str(&hex);
                    }
                }
            }
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

/// Build a SPARQL PREFIX block from any `sh:declare` blocks in the shapes graph.
///
/// SHACL lets a `sh:sparql` constraint reference prefixed names and point at its
/// prefix declarations with `sh:prefixes`. Rather than resolve that pointer
/// strictly, every declaration present in the shapes graph is collected, which is
/// permissive but never wrong: an unused PREFIX line changes no result, whereas a
/// missing one turns an executable constraint into an unevaluated one.
fn sparql_prefix_block(shapes_store: &Store) -> anyhow::Result<String> {
    let rows = query_solutions(
        shapes_store,
        r#"
        PREFIX sh: <http://www.w3.org/ns/shacl#>
        SELECT ?prefix ?namespace WHERE {
            ?decl sh:prefix ?prefix ; sh:namespace ?namespace .
        }
        "#,
    )?;
    let mut block = String::new();
    for row in &rows {
        if let (Some(prefix), Some(namespace)) = (row.get("prefix"), row.get("namespace")) {
            block.push_str(&format!(
                "PREFIX {}: <{}>\n",
                strip_quotes(prefix),
                strip_angle_brackets(&strip_quotes(namespace))
            ));
        }
    }
    Ok(block)
}
