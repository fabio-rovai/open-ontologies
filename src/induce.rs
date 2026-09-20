//! One sheet in, one ontology out, with the evidence for every line of it.
//!
//! A spreadsheet is a claim about a domain made by someone who did not write
//! it down: the columns are the properties, the rows are the individuals, the
//! header is the only schema. This module reads that claim back as an OWL
//! class with typed properties, a SHACL shape that the rows themselves
//! satisfy, and a mapping that loads the rows as instances, and it says next
//! to every induced statement WHICH rows made it say so.
//!
//! What it induces is a HYPOTHESIS about the sheet, not a truth about the
//! domain. `sh:maxCount 1` on a column means no row had two values, which is
//! exactly what a one-cell column guarantees and nothing more; `sh:in` on a
//! column means twelve or fewer distinct values were seen, not that a
//! thirteenth is wrong. The output says so, once at the top and once per
//! statement, so a reader can strike the lines the domain does not support.
//!
//! Three choices are deliberate and stated here rather than discovered later:
//!
//! 1. **Cardinality is a shape, not an axiom.** A one-value column could be
//!    rendered `owl:FunctionalProperty`. It is not, because under OWL RL a
//!    functional property with two values does not reject the second row, it
//!    IDENTIFIES the two objects (`prp-fp`). A shape rejects the next row; an
//!    axiom rewrites the world. The next row is what a sheet's author meant.
//! 2. **Observed ranges are evidence, not constraints.** The minimum and
//!    maximum of a numeric column are reported in the shape's description and
//!    never as `sh:minInclusive`, because a price of 0 in a sheet whose
//!    lowest price was 4 is not a violation of anything the author said.
//! 3. **Repeated columns are one property.** `topping1 .. topping7` is one
//!    multi-valued `topping` with `sh:maxCount 7`, which is what the author
//!    meant by numbering them. The group is reported, so a false merge
//!    (`address1`/`address2` as two lines of one address) is visible.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde::Serialize;

use crate::mapping::{FieldMapping, MappingConfig};

const XSD: &str = "http://www.w3.org/2001/XMLSchema#";

/// The datatypes this module can tell apart, in the order they are tried.
/// Every filled value of a column must parse for the column to earn the type;
/// one value that does not demotes the column to the next candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Dt {
    Boolean,
    Integer,
    Decimal,
    Double,
    Date,
    DateTime,
    AnyUri,
    String,
}

impl Dt {
    pub fn iri(self) -> String {
        let local = match self {
            Dt::Boolean => "boolean",
            Dt::Integer => "integer",
            Dt::Decimal => "decimal",
            Dt::Double => "double",
            Dt::Date => "date",
            Dt::DateTime => "dateTime",
            Dt::AnyUri => "anyURI",
            Dt::String => "string",
        };
        format!("{XSD}{local}")
    }

    /// The [`Dt`] an XSD datatype IRI names, for the ones this module knows.
    pub fn from_iri(iri: &str) -> Option<Dt> {
        Some(match iri.strip_prefix(XSD)? {
            "boolean" => Dt::Boolean,
            "integer" => Dt::Integer,
            "decimal" => Dt::Decimal,
            "double" | "float" => Dt::Double,
            "date" => Dt::Date,
            "dateTime" => Dt::DateTime,
            "anyURI" => Dt::AnyUri,
            "string" => Dt::String,
            _ => return None,
        })
    }

    /// Whether `v` is in this datatype's lexical space, as far as this
    /// module's parser goes.
    pub fn accepts(self, v: &str) -> bool {
        let v = v.trim();
        match self {
            Dt::Boolean => matches!(v, "true" | "false" | "TRUE" | "FALSE" | "True" | "False"),
            Dt::Integer => {
                let d = v.strip_prefix(['+', '-']).unwrap_or(v);
                !d.is_empty() && d.chars().all(|c| c.is_ascii_digit())
            }
            Dt::Decimal => {
                let d = v.strip_prefix(['+', '-']).unwrap_or(v);
                let mut dots = 0;
                let mut digits = 0;
                for c in d.chars() {
                    match c {
                        '.' => dots += 1,
                        c if c.is_ascii_digit() => digits += 1,
                        _ => return false,
                    }
                }
                digits > 0 && dots <= 1
            }
            Dt::Double => {
                // A decimal, or a decimal with an exponent (`3e+05`), which a
                // sheet exported from a spreadsheet application will contain
                // for any large round number. INF and NaN as XSD writes them.
                if matches!(v, "INF" | "-INF" | "NaN") {
                    return true;
                }
                let (mant, exp) = match v.split_once(['e', 'E']) {
                    Some((m, e)) => (m, Some(e)),
                    None => (v, None),
                };
                Dt::Decimal.accepts(mant)
                    && exp.is_none_or(|e| {
                        let d = e.strip_prefix(['+', '-']).unwrap_or(e);
                        !d.is_empty() && d.chars().all(|c| c.is_ascii_digit())
                    })
            }
            Dt::Date => {
                let b = v.as_bytes();
                b.len() == 10
                    && b[4] == b'-'
                    && b[7] == b'-'
                    && b.iter().enumerate().all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
            }
            Dt::DateTime => {
                let (d, t) = match v.split_once('T') {
                    Some(x) => x,
                    None => return false,
                };
                let t = t.trim_end_matches('Z');
                let t = t.split(['+']).next().unwrap_or(t);
                let t = if t.len() > 8 && &t[8..9] == "-" { &t[..8] } else { t };
                Dt::Date.accepts(d)
                    && t.len() >= 8
                    && &t[2..3] == ":"
                    && &t[5..6] == ":"
                    && t[..8].chars().enumerate().all(|(i, c)| i == 2 || i == 5 || c.is_ascii_digit())
            }
            Dt::AnyUri => (v.starts_with("http://") || v.starts_with("https://")) && !v.contains(' '),
            Dt::String => true,
        }
    }

    /// The narrowest type every value of `values` parses as.
    fn infer<'a>(values: impl Iterator<Item = &'a str> + Clone) -> Dt {
        for dt in [Dt::Boolean, Dt::Integer, Dt::Decimal, Dt::Double, Dt::Date, Dt::DateTime, Dt::AnyUri] {
            if values.clone().all(|v| dt.accepts(v)) {
                return dt;
            }
        }
        Dt::String
    }
}

/// What was measured about one induced property, and what was decided.
#[derive(Debug, Clone, Serialize)]
pub struct Column {
    /// The property's local name.
    pub property: String,
    /// The sheet columns behind it: one, or a numbered group.
    pub columns: Vec<String>,
    pub rows: usize,
    /// Rows with at least one value in these columns.
    pub filled: usize,
    /// Distinct values across the group.
    pub distinct: usize,
    /// The most values one row had across the group.
    pub max_per_row: usize,
    pub datatype: Dt,
    /// `Some(class)`: the values are identifiers of rows of this sheet, so the
    /// property is an object property ranging over the induced class.
    pub refers_to: Option<String>,
    /// Observed enumeration, when the column is a small closed set of strings.
    pub values: Option<Vec<String>>,
    /// Numeric or length bounds actually seen. Evidence, never a constraint.
    pub observed: BTreeMap<&'static str, String>,
    /// One sentence a reader can check against the rows.
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Induced {
    pub class: String,
    pub class_iri: String,
    pub base_iri: String,
    pub id_column: String,
    /// `true` when no column identified the rows and a row number was used.
    pub id_synthesised: bool,
    pub rows: usize,
    pub columns: Vec<Column>,
    /// The OWL ontology, Turtle.
    pub ontology_ttl: String,
    /// The SHACL shapes graph, Turtle. The rows satisfy it by construction.
    pub shapes_ttl: String,
    /// The mapping that loads the rows as instances of the class.
    pub mapping: MappingConfig,
    /// What this output is and is not.
    pub means: &'static str,
}

const MEANS: &str = "an induced ontology is a HYPOTHESIS about the sheet, not a truth about the \
                     domain. Every class, property, datatype and shape here was read off the rows \
                     and carries the counts that produced it; the rows satisfy the shapes by \
                     construction, so a clean validation of this sheet against them says nothing. \
                     Cardinality is a shape and not an owl:FunctionalProperty on purpose (an axiom \
                     identifies, a shape rejects), observed numeric ranges are reported and never \
                     constrained, and a numbered column group is one multi-valued property. Strike \
                     what the domain does not support; the evidence column says what each line rests on";

fn local_name(raw: &str) -> String {
    let mut out = String::new();
    let mut upper_next = false;
    for c in raw.trim().chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            if upper_next && out.is_empty() {
                out.push(c);
            } else if upper_next {
                out.extend(c.to_uppercase());
            } else {
                out.push(c);
            }
            upper_next = false;
        } else {
            upper_next = !out.is_empty();
        }
    }
    if out.is_empty() {
        "column".to_string()
    } else if out.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        format!("c{out}")
    } else {
        out
    }
}

fn class_name(stem: &str) -> String {
    let mut s = String::new();
    let mut up = true;
    for c in stem.chars() {
        if c.is_ascii_alphanumeric() {
            if up {
                s.extend(c.to_uppercase());
            } else {
                s.push(c);
            }
            up = false;
        } else {
            up = true;
        }
    }
    if s.is_empty() {
        "Row".to_string()
    } else if s.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        format!("Sheet{s}")
    } else {
        s
    }
}

/// `topping3` -> `("topping", 3)`, `address 2` -> `("address", 2)`; `abc` -> None.
fn numbered(h: &str) -> Option<(String, u32)> {
    let t = h.trim().trim_end_matches(|c: char| c.is_ascii_digit());
    let digits = &h.trim()[t.len()..];
    if digits.is_empty() || t.is_empty() {
        return None;
    }
    let stem = t.trim_end_matches([' ', '_', '-', '#']);
    if stem.is_empty() {
        return None;
    }
    Some((stem.to_string(), digits.parse().ok()?))
}

fn ttl_str(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '\\' => o.push_str("\\\\"),
            '"' => o.push_str("\\\""),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

/// Induce class, properties, shapes and mapping from `rows`.
///
/// `headers` fixes the column order (a `HashMap` row has none); `stem` names
/// the class (a file stem, usually); `base_iri` roots every minted IRI.
pub fn induce(rows: &[HashMap<String, String>], headers: &[String], stem: &str, base_iri: &str) -> Induced {
    let n = rows.len();
    let class = class_name(stem);
    let ont = format!("{base_iri}ont#");
    let class_iri = format!("{ont}{class}");

    // 1. The identifier: the first column if it is filled and unique in every
    //    row, else any column named like an identifier that is, else a row
    //    number, and the report says which.
    let filled_unique = |h: &String| -> bool {
        let mut seen = BTreeSet::new();
        rows.iter().all(|r| match r.get(h).map(|v| v.trim()) {
            Some(v) if !v.is_empty() => seen.insert(v.to_string()),
            _ => false,
        })
    };
    let id_like = |h: &str| {
        let l = h.to_ascii_lowercase();
        l == "id" || l.ends_with("_id") || l.ends_with("id") || l.contains("key") || l.contains("code")
    };
    let (id_column, id_synthesised) = match headers.first() {
        Some(h) if n > 0 && filled_unique(h) => (h.clone(), false),
        _ => match headers.iter().find(|h| id_like(h) && filled_unique(h)) {
            Some(h) => (h.clone(), false),
            None => ("__row".to_string(), true),
        },
    };
    let id_values: BTreeSet<String> = if id_synthesised {
        BTreeSet::new()
    } else {
        rows.iter().filter_map(|r| r.get(&id_column)).map(|v| v.trim().to_string()).collect()
    };

    // 2. Group numbered columns. A stem with two or more members is one
    //    property; a lone `address1` stays a column called address1.
    let mut groups: BTreeMap<String, Vec<(u32, String)>> = BTreeMap::new();
    for h in headers {
        if let Some((stem, k)) = numbered(h) {
            groups.entry(stem).or_default().push((k, h.clone()));
        }
    }
    let mut member_of: HashMap<String, String> = HashMap::new();
    for (stem, members) in &groups {
        if members.len() >= 2 {
            for (_, h) in members {
                member_of.insert(h.clone(), stem.clone());
            }
        }
    }
    let mut units: Vec<(String, Vec<String>)> = Vec::new();
    let mut seen_group: BTreeSet<String> = BTreeSet::new();
    for h in headers {
        if h == &id_column && !id_synthesised {
            continue;
        }
        match member_of.get(h) {
            Some(stem) => {
                if seen_group.insert(stem.clone()) {
                    let mut ms = groups[stem].clone();
                    ms.sort();
                    units.push((stem.clone(), ms.into_iter().map(|(_, h)| h).collect()));
                }
            }
            None => units.push((h.clone(), vec![h.clone()])),
        }
    }

    // 3. Profile each unit.
    let mut columns = Vec::new();
    let mut taken: BTreeSet<String> = BTreeSet::new();
    for (raw_name, cols) in &units {
        let mut property = local_name(raw_name);
        while !taken.insert(property.clone()) {
            property.push('_');
        }
        let per_row: Vec<Vec<&str>> = rows
            .iter()
            .map(|r| {
                cols.iter()
                    .filter_map(|c| r.get(c).map(|v| v.trim()))
                    .filter(|v| !v.is_empty())
                    .collect()
            })
            .collect();
        let filled = per_row.iter().filter(|v| !v.is_empty()).count();
        let max_per_row = per_row.iter().map(|v| v.len()).max().unwrap_or(0);
        let all: Vec<&str> = per_row.iter().flatten().copied().collect();
        let distinct: BTreeSet<&str> = all.iter().copied().collect();
        let datatype = if all.is_empty() { Dt::String } else { Dt::infer(all.iter().copied()) };

        // A column whose every value is an identifier of some row of this
        // sheet refers to the sheet's own class, the only referent a single
        // sheet can offer. Containment alone is not enough: a `quantity`
        // column holding 1..5 is contained in ids 1..100 and refers to
        // nothing, which the first version of this got wrong on a two-row
        // sheet. So the values must be non-numeric, or the column must be
        // named like a reference (`manager_id`, `parent`, `ref`, `key`).
        let named_like_ref = {
            let l = raw_name.to_ascii_lowercase();
            l.ends_with("_id") || l.ends_with("id") && l != "id" || l.contains("ref") || l.contains("parent")
                || l.contains("manager") || l.ends_with("key")
        };
        let refers_to = (!id_synthesised
            && !all.is_empty()
            && !matches!(datatype, Dt::Boolean | Dt::Date | Dt::DateTime)
            && (named_like_ref || !matches!(datatype, Dt::Integer | Dt::Decimal | Dt::Double))
            && all.iter().all(|v| id_values.contains(*v))
            && cols.iter().all(|c| c != &id_column))
        .then(|| class.clone());

        // A closed set needs more than a short column: at least two values
        // (one value is a constant, not an enumeration), at most twelve, and
        // each seen five times over on average. Thirty rows of a four-letter
        // code qualify; thirty rows of names do not.
        let values = (refers_to.is_none()
            && datatype == Dt::String
            && distinct.len() >= 2
            && distinct.len() <= 12
            && all.len() >= 5 * distinct.len())
        .then(|| distinct.iter().map(|v| v.to_string()).collect::<Vec<_>>());

        let mut observed = BTreeMap::new();
        match datatype {
            Dt::Integer | Dt::Decimal | Dt::Double => {
                let nums: Vec<f64> = all.iter().filter_map(|v| v.parse::<f64>().ok()).collect();
                if let (Some(lo), Some(hi)) = (
                    nums.iter().cloned().reduce(f64::min),
                    nums.iter().cloned().reduce(f64::max),
                ) {
                    observed.insert("min", format!("{lo}"));
                    observed.insert("max", format!("{hi}"));
                }
            }
            Dt::String | Dt::AnyUri => {
                if let Some(m) = all.iter().map(|v| v.chars().count()).max() {
                    observed.insert("max_length", m.to_string());
                }
            }
            _ => {}
        }

        let evidence = format!(
            "{filled} of {n} rows filled, {} distinct value(s), at most {max_per_row} per row across {} column(s); every value parses as xsd:{}{}{}",
            distinct.len(),
            cols.len(),
            datatype.iri().rsplit('#').next().unwrap_or(""),
            match &refers_to {
                Some(c) => format!("; every value is the {id_column} of a row{}, so it refers to a {c}", if named_like_ref { " and the column is named like a reference" } else { "" }),
                None => String::new(),
            },
            match &values {
                Some(v) => format!("; {} distinct values over {} occurrences, a closed set as far as this sheet goes", v.len(), all.len()),
                None => String::new(),
            },
        );
        columns.push(Column {
            property,
            columns: cols.clone(),
            rows: n,
            filled,
            distinct: distinct.len(),
            max_per_row,
            datatype,
            refers_to,
            values,
            observed,
            evidence,
        });
    }

    // 4. Render.
    let ctx = Ctx { ont: &ont, class: &class, class_iri: &class_iri, stem, n, id_synthesised, id_column: &id_column };
    let ontology_ttl = render_ontology(&ctx, &columns);
    let shapes_ttl = render_shapes(&ctx, &columns);
    let mut mappings = Vec::new();
    for c in &columns {
        for col in &c.columns {
            mappings.push(FieldMapping {
                field: col.clone(),
                predicate: format!("{ont}{}", c.property),
                datatype: c.refers_to.is_none().then(|| c.datatype.iri()),
                class: c.refers_to.as_ref().map(|_| class_iri.clone()),
                lookup: c.refers_to.is_some(),
            });
        }
    }
    let mapping = MappingConfig {
        base_iri: base_iri.to_string(),
        id_field: id_column.clone(),
        class: class_iri.clone(),
        mappings,
    };

    Induced {
        class,
        class_iri,
        base_iri: base_iri.to_string(),
        id_column,
        id_synthesised,
        rows: n,
        columns,
        ontology_ttl,
        shapes_ttl,
        mapping,
        means: MEANS,
    }
}

/// What both renderers need to know about the sheet as a whole.
struct Ctx<'a> {
    ont: &'a str,
    class: &'a str,
    class_iri: &'a str,
    stem: &'a str,
    n: usize,
    id_synthesised: bool,
    id_column: &'a str,
}

fn render_ontology(ctx: &Ctx<'_>, columns: &[Column]) -> String {
    let Ctx { ont, class, class_iri, stem, n, id_synthesised, id_column } = *ctx;
    let mut t = String::new();
    t.push_str(&format!("@prefix : <{ont}> .\n"));
    t.push_str("@prefix owl: <http://www.w3.org/2002/07/owl#> .\n");
    t.push_str("@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n");
    t.push_str("@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n");
    t.push_str("@prefix dcterms: <http://purl.org/dc/terms/> .\n\n");
    t.push_str(&format!(
        "<{}> a owl:Ontology ;\n    rdfs:comment {} .\n\n",
        ont.trim_end_matches('#'),
        ttl_str(&format!(
            "Induced from the sheet {stem:?}: {n} rows, {} properties. {}",
            columns.len(),
            MEANS
        ))
    ));
    t.push_str(&format!(
        ":{class} a owl:Class ;\n    rdfs:label {} ;\n    rdfs:comment {} .\n\n",
        ttl_str(class),
        ttl_str(&format!(
            "One instance per row of {stem:?}, {n} in the sheet. Identified by {}.",
            if id_synthesised {
                "row number, because no column was filled and unique in every row".to_string()
            } else {
                format!("the column {id_column:?}, filled and unique in every row")
            }
        ))
    ));
    let _ = class_iri;
    for c in columns {
        let (kind, range) = match &c.refers_to {
            Some(cls) => ("owl:ObjectProperty", format!(":{cls}")),
            None => ("owl:DatatypeProperty", format!("xsd:{}", c.datatype.iri().rsplit('#').next().unwrap_or("string"))),
        };
        t.push_str(&format!(
            ":{p} a {kind} ;\n    rdfs:label {} ;\n    rdfs:domain :{class} ;\n    rdfs:range {range} ;\n    rdfs:comment {} .\n\n",
            ttl_str(&c.columns.join(", ")),
            ttl_str(&c.evidence),
            p = c.property,
        ));
    }
    t
}

fn render_shapes(ctx: &Ctx<'_>, columns: &[Column]) -> String {
    let Ctx { ont, class, class_iri, n, .. } = *ctx;
    let mut t = String::new();
    t.push_str(&format!("@prefix : <{ont}> .\n"));
    t.push_str("@prefix sh: <http://www.w3.org/ns/shacl#> .\n");
    t.push_str("@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n\n");
    t.push_str(&format!(
        ":{class}Shape a sh:NodeShape ;\n    sh:targetClass <{class_iri}> ;\n    sh:description {} ;\n",
        ttl_str(&format!(
            "Induced from {n} rows. The rows satisfy this shape by construction; it constrains the NEXT row. {MEANS}"
        ))
    ));
    let mut props = Vec::new();
    for c in columns {
        let mut p = format!("    sh:property [\n        sh:path :{} ;\n", c.property);
        match &c.refers_to {
            Some(_) => p.push_str(&format!("        sh:class <{class_iri}> ;\n        sh:nodeKind sh:IRI ;\n")),
            None => p.push_str(&format!(
                "        sh:datatype xsd:{} ;\n",
                c.datatype.iri().rsplit('#').next().unwrap_or("string")
            )),
        }
        p.push_str(&format!("        sh:maxCount {} ;\n", c.max_per_row.max(1)));
        let mut why = vec![format!(
            "maxCount {} because no row had more values across {} column(s)",
            c.max_per_row.max(1),
            c.columns.len()
        )];
        if c.filled == n && n > 0 {
            p.push_str("        sh:minCount 1 ;\n");
            why.push(format!("minCount 1 because every one of {n} rows was filled"));
        } else {
            why.push(format!("no minCount because {} of {n} rows were empty", n - c.filled));
        }
        if let Some(vals) = &c.values {
            let list: Vec<String> = vals.iter().map(|v| ttl_str(v)).collect();
            p.push_str(&format!("        sh:in ( {} ) ;\n", list.join(" ")));
            why.push(format!(
                "in: {} distinct values over {} filled rows; a value outside this set is new, not wrong",
                vals.len(),
                c.filled
            ));
        }
        for (k, v) in &c.observed {
            why.push(format!("observed {k} {v}, reported and not constrained"));
        }
        p.push_str(&format!("        sh:description {} ;\n    ]", ttl_str(&why.join("; "))));
        props.push(p);
    }
    t.push_str(&props.join(" ;\n"));
    t.push_str(" .\n");
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(csv: &str) -> (Vec<HashMap<String, String>>, Vec<String>) {
        let mut lines = csv.trim().lines();
        let headers: Vec<String> = lines.next().unwrap().split(',').map(|s| s.to_string()).collect();
        let rows = lines
            .map(|l| {
                headers
                    .iter()
                    .cloned()
                    .zip(l.split(',').map(|s| s.to_string()))
                    .collect::<HashMap<_, _>>()
            })
            .collect();
        (rows, headers)
    }

    #[test]
    fn datatypes_are_the_narrowest_every_value_parses_as() {
        assert_eq!(Dt::infer(["1", "2", "-3"].into_iter()), Dt::Integer);
        assert_eq!(Dt::infer(["1", "2.5"].into_iter()), Dt::Decimal);
        assert_eq!(Dt::infer(["1", "3e+05"].into_iter()), Dt::Double);
        assert_eq!(Dt::infer(["1", "x"].into_iter()), Dt::String);
        assert_eq!(Dt::infer(["2014-07-11"].into_iter()), Dt::Date);
        assert_eq!(Dt::infer(["2014-07-11T10:00:00Z"].into_iter()), Dt::DateTime);
        assert_eq!(Dt::infer(["true", "false"].into_iter()), Dt::Boolean);
        assert_eq!(Dt::infer(["https://a.b/c"].into_iter()), Dt::AnyUri);
    }

    #[test]
    fn numbered_columns_become_one_property() {
        let (r, h) = rows("name,topping1,topping2,topping3\nA,x,y,\nB,x,,\nC,z,y,w\n");
        let i = induce(&r, &h, "pizza-menu", "http://ex.org/");
        assert_eq!(i.class, "PizzaMenu");
        assert_eq!(i.id_column, "name");
        assert!(!i.id_synthesised);
        assert_eq!(i.columns.len(), 1, "{:?}", i.columns);
        let t = &i.columns[0];
        assert_eq!(t.property, "topping");
        assert_eq!(t.columns, vec!["topping1", "topping2", "topping3"]);
        assert_eq!(t.max_per_row, 3);
        assert_eq!(t.filled, 3);
        assert!(i.shapes_ttl.contains("sh:maxCount 3"), "{}", i.shapes_ttl);
        assert!(i.shapes_ttl.contains("sh:minCount 1"));
        assert!(i.ontology_ttl.contains(":topping a owl:DatatypeProperty"));
        assert!(!i.ontology_ttl.contains("a owl:FunctionalProperty"), "cardinality is a shape, not an axiom");
    }

    #[test]
    fn a_self_reference_is_an_object_property_and_a_small_set_is_an_enumeration() {
        let (r, h) = rows(
            "emp_id,name,manager_id,grade,salary\n1,Ann,,A,100\n2,Bob,1,B,80\n3,Cy,1,B,80.5\n4,Di,2,A,90\n5,Ed,2,B,70\n6,Fi,3,A,60\n7,Gus,3,A,55\n8,Hal,4,B,50\n9,Ida,4,A,45\n10,Jo,5,B,40\n",
        );
        let i = induce(&r, &h, "staff", "http://ex.org/");
        let by = |p: &str| i.columns.iter().find(|c| c.property == p).unwrap_or_else(|| panic!("{p}: {:?}", i.columns));
        assert_eq!(by("manager_id").refers_to.as_deref(), Some("Staff"));
        assert!(i.ontology_ttl.contains(":manager_id a owl:ObjectProperty"));
        assert!(i.shapes_ttl.contains("sh:class <http://ex.org/ont#Staff>"));
        assert_eq!(by("grade").values.as_ref().map(|v| v.len()), Some(2));
        assert!(i.shapes_ttl.contains("sh:in ( \"A\" \"B\" )"));
        assert_eq!(by("salary").datatype, Dt::Decimal);
        assert_eq!(by("salary").observed.get("min").map(String::as_str), Some("40"));
        assert!(!i.shapes_ttl.contains("sh:minInclusive"), "observed ranges are evidence, not constraints");
        assert!(by("manager_id").filled < 10 && !i.shapes_ttl.contains("sh:path :manager_id ;\n        sh:class <http://ex.org/ont#Staff> ;\n        sh:nodeKind sh:IRI ;\n        sh:maxCount 1 ;\n        sh:minCount 1"));
        // name is not an enumeration: ten distinct over ten values.
        assert!(by("name").values.is_none());
    }

    /// The false positive the first version had: small integers contained in
    /// the id range are a quantity, not a reference.
    #[test]
    fn an_integer_column_inside_the_id_range_is_not_a_reference() {
        let (r, h) = rows("id,label,n\n1,one,1\n2,two,2\n3,three,1\n");
        let i = induce(&r, &h, "nums", "http://ex.org/");
        let n = i.columns.iter().find(|c| c.property == "n").unwrap();
        assert!(n.refers_to.is_none(), "{:?}", n);
        assert_eq!(n.datatype, Dt::Integer);
        // ...but the same values under a reference-like name are one.
        let (r, h) = rows("id,label,parent_id\n1,one,1\n2,two,2\n3,three,1\n");
        let i = induce(&r, &h, "nums", "http://ex.org/");
        assert_eq!(i.columns.iter().find(|c| c.property == "parent_id").unwrap().refers_to.as_deref(), Some("Nums"));
    }

    #[test]
    fn no_identifier_means_a_row_number_and_says_so() {
        let (r, h) = rows("colour,size\nred,1\nred,2\n");
        let i = induce(&r, &h, "swatches", "http://ex.org/");
        assert!(i.id_synthesised);
        assert_eq!(i.id_column, "__row");
        assert!(i.ontology_ttl.contains("row number, because no column was filled and unique"));
        assert_eq!(i.columns.len(), 2);
    }

    #[test]
    fn the_turtle_parses_and_the_mapping_loads_rows_as_instances() {
        let (r, h) = rows("id,label,n\n1,one,1\n2,two,2\n");
        let i = induce(&r, &h, "nums", "http://ex.org/");
        let g = crate::graph::GraphStore::new();
        g.load_turtle(&i.ontology_ttl, None).expect("ontology turtle parses");
        g.load_turtle(&i.shapes_ttl, None).expect("shapes turtle parses");
        let nt = i.mapping.rows_to_ntriples(&r);
        assert!(nt.contains("<http://ex.org/1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://ex.org/ont#Nums>"), "{nt}");
        assert!(nt.contains("\"1\"^^<http://www.w3.org/2001/XMLSchema#integer>"), "{nt}");
    }
}
