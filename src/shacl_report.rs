// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

//! The W3C `sh:ValidationReport` graph for a SHACL run (#318).
//!
//! `onto_shacl` answered only in its own JSON. pyshacl, Jena and TopBraid all
//! read and write the report graph SHACL 3.6 defines, so a JSON report could
//! not be loaded into a store, queried, or compared with another engine's
//! report, and the W3C suite's full-compliance level (a report isomorphic to
//! the expected one) could not be measured at all. This module writes that
//! graph from the report [`crate::shacl::ShaclValidator`] already builds; it
//! evaluates nothing itself.
//!
//! Each result carries `sh:focusNode`, `sh:resultPath`, `sh:value`,
//! `sh:sourceShape`, `sh:sourceConstraintComponent`, `sh:resultSeverity` and
//! `sh:resultMessage`, wherever the JSON violation has the corresponding field.
//! `sh:value` is absent for the components SHACL defines without one
//! (`sh:minCount`, `sh:maxCount`, `sh:hasValue`, the qualified counts) and for
//! the two whose query does not project the value node (`sh:lessThan`,
//! `sh:lessThanOrEquals`, `sh:node`), which select `DISTINCT ?focus` and would
//! change how many results they emit if they projected it.

use oxigraph::io::{RdfFormat, RdfSerializer};
use oxigraph::model::{BlankNode, Literal, NamedNode, NamedOrBlankNode, Term, Triple};

const SH: &str = "http://www.w3.org/ns/shacl#";
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const XSD: &str = "http://www.w3.org/2001/XMLSchema#";
const RDF_FIRST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#first";
const RDF_REST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest";
const RDF_NIL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";

/// The namespace of the two non-standard terms an undetermined run carries.
pub const OO_SHACL: &str = "https://open-ontologies.org/shacl#";

/// Key, inside each JSON violation, of the shapes-graph terms only this writer
/// reads. See `RdfTerms` in `src/shacl.rs` for why they cannot be recovered
/// from the JSON fields; `ShaclReport::to_json_string` removes the key.
pub(crate) const RDF_TERMS_KEY: &str = "_rdf_terms";

/// How a SHACL report is written out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportFormat {
    /// The engine's own JSON, unchanged apart from `value` (#318).
    Json,
    /// A W3C `sh:ValidationReport` graph in Turtle.
    Turtle,
    /// The same graph in N-Triples.
    NTriples,
}

impl ReportFormat {
    /// Read `report_format` as `onto_shacl` and the `shacl` command take it.
    /// Absent means JSON, which is what every caller got before the option
    /// existed. Anything else is refused by name rather than defaulted, since
    /// a misspelt `"tutle"` answered in JSON is a caller parsing the wrong
    /// format without being told.
    pub fn parse(value: Option<&str>) -> Result<Self, String> {
        match value {
            None | Some("json") => Ok(Self::Json),
            Some("turtle") => Ok(Self::Turtle),
            Some("ntriples") => Ok(Self::NTriples),
            Some(other) => Err(format!(
                "report_format must be \"json\", \"turtle\" or \"ntriples\"; got {other:?}"
            )),
        }
    }
}

/// Write `report` (the validator's JSON, with its `_rdf_terms` still attached)
/// as a `sh:ValidationReport` graph.
///
/// # `conforms: null` is written `sh:conforms false` plus `oo:undetermined true`
///
/// The engine has three answers and the report vocabulary has two. The two
/// alternatives the ticket allowed were refusing the RDF form, or writing
/// `sh:conforms false` with a named marker. This writes the marker, for these
/// reasons:
///
/// * `sh:conforms true` is ruled out: it is the false clean the third answer
///   exists to prevent, and a consumer reading only `sh:conforms` would see a
///   pass for constraints that never ran.
/// * Leaving `sh:conforms` out, or giving it a non-boolean, is not a report.
///   SHACL 3.6.1 requires exactly one `sh:conforms` of type `xsd:boolean`, and
///   a strict reader rejects the graph, which is a refusal with a worse message.
/// * Refusing throws away results that WERE found. An undetermined run is not
///   an empty one: a skipped `sh:closed` on one shape suppresses the verdict
///   while another shape's `sh:datatype` violations stand, and those are what
///   someone repairing the data needs. It would also make the RDF form fail on
///   exactly the runs the CLI answers with exit code 2 and a full report.
/// * `false` fails closed. Every consumer that reads `sh:conforms` alone
///   (dashboards, pyshacl-shaped CI checks) treats the run as not passing,
///   which agrees with the non-zero exit code. The cost is stated rather than
///   hidden: SHACL defines `false` as "there are results", and an undetermined
///   run with no results breaks that reading. `oo:undetermined true` and
///   `oo:undeterminedReason` (the JSON `warning`) say so in the graph, so a
///   reader that knows to look can tell "failed" from "not decided".
pub fn to_rdf(report: &serde_json::Value, format: ReportFormat) -> anyhow::Result<String> {
    let rdf_format = match format {
        ReportFormat::Turtle => RdfFormat::Turtle,
        ReportFormat::NTriples => RdfFormat::NTriples,
        ReportFormat::Json => anyhow::bail!("to_rdf writes an RDF format; JSON is the report itself"),
    };
    let triples = report_triples(report)?;
    let mut serializer = RdfSerializer::from_format(rdf_format);
    if rdf_format == RdfFormat::Turtle {
        serializer = serializer
            .with_prefix("sh", SH)?
            .with_prefix("xsd", XSD)?
            .with_prefix("oo", OO_SHACL)?;
    }
    let mut writer = serializer.for_writer(Vec::new());
    for t in &triples {
        writer.serialize_triple(t)?;
    }
    Ok(String::from_utf8(writer.finish()?)?)
}

fn sh(local: &str) -> NamedNode {
    NamedNode::new_unchecked(format!("{SH}{local}"))
}

/// A term from the JSON report. The validator strips the angle brackets off
/// IRIs and leaves blank nodes (`_:id`) and literals (`"lex"^^<dt>`) in
/// N-Triples form, and an absolute IRI can start with neither `"` nor `_:`
/// (its scheme starts with a letter), so the three are told apart without
/// guessing.
fn term(s: &str) -> anyhow::Result<Term> {
    let s = s.trim();
    if s.starts_with('"') || s.starts_with("_:") || s.starts_with('<') {
        s.parse::<Term>()
            .map_err(|e| anyhow::anyhow!("report term {s:?} does not read back as RDF: {e}"))
    } else {
        Ok(NamedNode::new(s)
            .map_err(|e| anyhow::anyhow!("report IRI {s:?} is not an IRI: {e}"))?
            .into())
    }
}

/// The SHACL path graph for the tree the validator carried in
/// `_rdf_terms.path_form`.
///
/// The JSON `path` is the display form the conformance comparator and messages
/// read, and it deliberately has no angle brackets; that makes it impossible
/// to parse back into SHACL's structure without guessing where an IRI ends and
/// an operator begins. The tree is therefore passed through separately and
/// written here: a sequence is its `rdf:List` head, an alternative is a blank
/// node carrying `sh:alternativePath` over such a list, and every other form
/// is a blank node carrying that one predicate to its operand.
fn path_from_json(value: &serde_json::Value, out: &mut Vec<Triple>) -> anyhow::Result<Term> {
    if let Some(iri) = value["predicate"].as_str() {
        return Ok(NamedNode::new(iri)?.into());
    }
    if let Some(inner) = value.get("inverse") {
        let node: NamedOrBlankNode = BlankNode::default().into();
        let operand = path_from_json(inner, out)?;
        out.push(Triple::new(node.clone(), sh("inversePath"), operand));
        return Ok(node.into());
    }
    for (key, predicate) in [
        ("zeroOrMore", "zeroOrMorePath"),
        ("oneOrMore", "oneOrMorePath"),
        ("zeroOrOne", "zeroOrOnePath"),
    ] {
        if let Some(inner) = value.get(key) {
            let node: NamedOrBlankNode = BlankNode::default().into();
            let operand = path_from_json(inner, out)?;
            out.push(Triple::new(node.clone(), sh(predicate), operand));
            return Ok(node.into());
        }
    }
    if let Some(parts) = value["sequence"].as_array() {
        return path_list(parts, out);
    }
    if let Some(parts) = value["alternative"].as_array() {
        let node: NamedOrBlankNode = BlankNode::default().into();
        let list = path_list(parts, out)?;
        out.push(Triple::new(node.clone(), sh("alternativePath"), list));
        return Ok(node.into());
    }
    anyhow::bail!("validator path form {value} names none of SHACL's seven path forms")
}

/// One `rdf:List` of path terms, returned as its head. The caller has already
/// refused the empty list, so a head always exists.
fn path_list(parts: &[serde_json::Value], out: &mut Vec<Triple>) -> anyhow::Result<Term> {
    let mut head: Option<NamedOrBlankNode> = None;
    let mut previous: Option<NamedOrBlankNode> = None;
    for part in parts {
        let cell: NamedOrBlankNode = BlankNode::default().into();
        let item = path_from_json(part, out)?;
        out.push(Triple::new(cell.clone(), NamedNode::new_unchecked(RDF_FIRST), item));
        match previous.take() {
            Some(prev) => out.push(Triple::new(prev, NamedNode::new_unchecked(RDF_REST), cell.clone())),
            None => head = Some(cell.clone()),
        }
        previous = Some(cell);
    }
    match (head, previous) {
        (Some(head), Some(last)) => {
            out.push(Triple::new(
                last,
                NamedNode::new_unchecked(RDF_REST),
                NamedNode::new_unchecked(RDF_NIL),
            ));
            Ok(head.into())
        }
        _ => anyhow::bail!("an empty SHACL path list has no path form"),
    }
}

fn report_triples(report: &serde_json::Value) -> anyhow::Result<Vec<Triple>> {
    let mut out = Vec::new();
    let node: NamedOrBlankNode = BlankNode::default().into();
    out.push(Triple::new(node.clone(), NamedNode::new_unchecked(RDF_TYPE), sh("ValidationReport")));

    let conforms = report["conforms"].as_bool();
    // `None` is written as `false`: see `to_rdf` for the decision and why.
    out.push(Triple::new(node.clone(), sh("conforms"), Literal::from(conforms == Some(true))));
    if conforms.is_none() {
        out.push(Triple::new(
            node.clone(),
            NamedNode::new_unchecked(format!("{OO_SHACL}undetermined")),
            Literal::from(true),
        ));
        if let Some(reason) = report["warning"].as_str() {
            out.push(Triple::new(
                node.clone(),
                NamedNode::new_unchecked(format!("{OO_SHACL}undeterminedReason")),
                Literal::new_simple_literal(reason),
            ));
        }
    }

    for v in report["violations"].as_array().map(Vec::as_slice).unwrap_or_default() {
        let r: NamedOrBlankNode = BlankNode::default().into();
        out.push(Triple::new(node.clone(), sh("result"), r.clone()));
        out.push(Triple::new(r.clone(), NamedNode::new_unchecked(RDF_TYPE), sh("ValidationResult")));
        if let Some(f) = v["focus_node"].as_str() {
            out.push(Triple::new(r.clone(), sh("focusNode"), term(f)?));
        }
        let path_form = &v[RDF_TERMS_KEY]["path_form"];
        if path_form.is_object() {
            let path = path_from_json(path_form, &mut out)?;
            out.push(Triple::new(r.clone(), sh("resultPath"), path));
        } else if let Some(p) = v["result_path"].as_str() {
            // `^p` is how a report without the tree prints `sh:inversePath p`,
            // and SHACL 2.3.1 writes it as a blank node carrying that one
            // triple. A compound display form has no such fallback and is
            // refused by `term` rather than written as a pretend IRI.
            match p.strip_prefix('^') {
                Some(inv) => {
                    let path: NamedOrBlankNode = BlankNode::default().into();
                    out.push(Triple::new(r.clone(), sh("resultPath"), path.clone()));
                    out.push(Triple::new(path, sh("inversePath"), term(inv)?));
                }
                None => out.push(Triple::new(r.clone(), sh("resultPath"), term(p)?)),
            }
        }
        if let Some(val) = v["value"].as_str() {
            out.push(Triple::new(r.clone(), sh("value"), term(val)?));
        }
        if let Some(s) = v["source_shape"].as_str() {
            out.push(Triple::new(r.clone(), sh("sourceShape"), term(s)?));
        }
        if let Some(c) = v["source_constraint_component"].as_str() {
            out.push(Triple::new(r.clone(), sh("sourceConstraintComponent"), term(c)?));
        }
        let terms = &v[RDF_TERMS_KEY];
        let severity = match terms["severity"].as_str() {
            Some(s) => term(s)?,
            None => sh("Violation").into(),
        };
        out.push(Triple::new(r.clone(), sh("resultSeverity"), severity));
        let message = match (terms["message"].as_str(), v["message"].as_str()) {
            (Some(written), _) => Some(term(written)?),
            (None, Some(default)) => Some(Literal::new_simple_literal(default).into()),
            (None, None) => None,
        };
        if let Some(m) = message {
            out.push(Triple::new(r, sh("resultMessage"), m));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unknown_format_is_refused_by_name() {
        let e = ReportFormat::parse(Some("tutle")).unwrap_err();
        assert!(e.contains("tutle") && e.contains("turtle"), "{e}");
        assert_eq!(ReportFormat::parse(None), Ok(ReportFormat::Json));
        assert_eq!(ReportFormat::parse(Some("ntriples")), Ok(ReportFormat::NTriples));
    }

    #[test]
    fn an_inverse_path_is_written_as_sh_inverse_path() {
        let report = serde_json::json!({
            "conforms": false,
            "violations": [{
                "focus_node": "http://ex/a",
                "result_path": "^http://ex/p",
                "source_shape": "http://ex/S",
                "source_constraint_component": "http://www.w3.org/ns/shacl#MinCountConstraintComponent",
                "message": "m",
                "_rdf_terms": {"severity": null, "message": null},
            }],
        });
        let nt = to_rdf(&report, ReportFormat::NTriples).unwrap();
        assert!(nt.contains("<http://www.w3.org/ns/shacl#inversePath> <http://ex/p>"), "{nt}");
    }
}
