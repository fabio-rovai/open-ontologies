//! Apache Ossie ontology documents -> OWL 2 DL + SHACL (#ossie).
//!
//! [Apache Ossie](https://ossie.apache.org/) (incubating, formerly Open Semantic
//! Interchange) is the vendor-neutral semantic-model specification backed by
//! Snowflake, Salesforce, Databricks, dbt Labs and around fifty other data
//! platforms. Its `ontology/ontology.md` 0.2.0.dev0 module is a fact-based
//! conceptual model: `EntityType` / `ValueType` concepts, relationships carrying
//! ordered roles with multiplicities and verbalizations, preferred identifiers,
//! derivation rules and population constraints.
//!
//! The Ossie ontology spec references neither RDF, OWL, SKOS nor SHACL, so an
//! Ossie ontology is invisible to every reasoner and validator in the semantic
//! web stack. This module closes that gap: it compiles an Ossie ontology document
//! into OWL 2 DL plus SHACL shapes, which the rest of this crate can then reason
//! over (`reason`, `tableaux`), validate against (`shacl`), diff (`drift`) and
//! audit (`vocab_check`).
//!
//! # What OWL cannot take
//!
//! Four Ossie constructs have no OWL 2 DL expression. These are properties of the
//! logic, not gaps in this implementation, and each one is why SHACL is emitted
//! alongside the OWL rather than instead of it.
//!
//! 1. **`OneToOne` onto a `ValueType`** is `InverseFunctionalDataProperty`, which
//!    OWL 2 DL forbids because it costs decidability; it exists only in OWL Full.
//!    This is the *common* case rather than a corner: in fact-based modelling the
//!    preferred identifier of an entity type is by construction a relationship to
//!    a value type. Emitted as a SHACL SPARQL uniqueness constraint.
//! 2. **`ManyToOne` on a relationship of arity >= 3** is a functional dependency
//!    across a tuple of roles. OWL cardinality restrictions constrain one property
//!    at a time. Emitted as a SHACL SPARQL constraint.
//! 3. **`derived_by`** is a recursive rule language. OWL has no rule construct.
//!    Preserved as an annotation, unenforced.
//! 4. **`requires`** is an ungrammared SQL expression string. Only the scalar
//!    comparison fragment (`0 < SocialSecurityNr`) becomes an XSD facet; the rest
//!    is preserved verbatim rather than guessed at.
//!
//! Nothing is discarded. Every construct the target cannot enforce is also written
//! to an `ossie:` annotation property in a reserved namespace, all of them
//! declared `owl:AnnotationProperty` so the emitted graph stays inside OWL 2 DL.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt::Write as _;

/// Reserved namespace for the provenance annotations.
pub const OSSIE_NS: &str = "https://ossie.apache.org/ns/ontology#";
/// Default base for generated terms when the caller supplies none.
pub const DEFAULT_BASE: &str = "https://ossie.apache.org/ontology/";

// --------------------------------------------------------------------------- //
// Source document
// --------------------------------------------------------------------------- //

#[derive(Debug, Deserialize)]
pub struct OssieDocument {
    #[serde(default)]
    pub version: Option<String>,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub ai_context: Option<serde_json::Value>,
    #[serde(default)]
    pub requires: Vec<String>,
    pub ontology: Vec<Component>,
    #[serde(default)]
    pub ontology_mappings: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct Component {
    pub concept: String,
    #[serde(rename = "type")]
    pub concept_type: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub extends: Vec<String>,
    #[serde(default)]
    pub derived_by: Vec<String>,
    #[serde(default)]
    pub identify_by: Vec<String>,
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default)]
    pub relationships: Vec<Relationship>,
}

#[derive(Debug, Deserialize)]
pub struct Relationship {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub roles: Vec<Role>,
    #[serde(default)]
    pub multiplicity: Option<String>,
    #[serde(default)]
    pub derived_by: Vec<String>,
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default)]
    pub verbalizes: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Role {
    pub concept: String,
    #[serde(default)]
    pub name: Option<String>,
}

// --------------------------------------------------------------------------- //
// Result
// --------------------------------------------------------------------------- //

/// A construct that survived the conversion as an annotation but that the target
/// formalism cannot enforce.
#[derive(Debug, Clone, Serialize)]
pub struct Issue {
    pub kind: &'static str,
    pub element: String,
    pub detail: &'static str,
}

#[derive(Debug, Serialize)]
pub struct OssieConversion {
    /// OWL 2 DL terminology plus SHACL shapes, as Turtle.
    pub turtle: String,
    /// Base IRI the terms were minted under.
    pub base_iri: String,
    pub concepts: usize,
    pub entity_types: usize,
    pub value_types: usize,
    pub relationships: usize,
    /// SHACL SPARQL constraints carrying what OWL could not state.
    pub sparql_constraints: usize,
    pub issues: Vec<Issue>,
}

// --------------------------------------------------------------------------- //
// Built-ins
// --------------------------------------------------------------------------- //

/// Ossie built-in value types and their XSD counterparts.
///
/// `Float` maps to `xsd:double`, not `xsd:float`: the Ossie core spec calls
/// `Float` approximate with unspecified width, and `xsd:double` is the wider of
/// the two IEEE 754 binary types, so the mapping never narrows a value.
fn builtin_xsd(name: &str) -> Option<&'static str> {
    Some(match name {
        "Boolean" => "http://www.w3.org/2001/XMLSchema#boolean",
        "Date" => "http://www.w3.org/2001/XMLSchema#date",
        "DateTime" => "http://www.w3.org/2001/XMLSchema#dateTime",
        "Decimal" => "http://www.w3.org/2001/XMLSchema#decimal",
        "Float" => "http://www.w3.org/2001/XMLSchema#double",
        "Integer" => "http://www.w3.org/2001/XMLSchema#integer",
        "String" => "http://www.w3.org/2001/XMLSchema#string",
        _ => return None,
    })
}

// --------------------------------------------------------------------------- //
// requires -> XSD facet
// --------------------------------------------------------------------------- //

/// One XSD facet derived from a recognised `requires` expression.
#[derive(Debug, Clone)]
pub struct Facet {
    pub subject: String,
    /// Facet local name: `minInclusive`, `maxInclusive`, `minExclusive`, `maxExclusive`.
    pub facet: &'static str,
    /// The bound, already rendered as a typed Turtle literal.
    pub literal: String,
}

fn is_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn numeric_literal(text: &str) -> Option<String> {
    let candidate = text.trim();
    if candidate.is_empty() {
        return None;
    }
    if candidate.parse::<i64>().is_ok() {
        return Some(format!(
            "\"{candidate}\"^^<http://www.w3.org/2001/XMLSchema#integer>"
        ));
    }
    if candidate.parse::<f64>().is_ok() && !candidate.contains(['e', 'E']) {
        return Some(format!(
            "\"{candidate}\"^^<http://www.w3.org/2001/XMLSchema#decimal>"
        ));
    }
    None
}

/// Parse one `requires` expression into a [`Facet`], or `None` if unrecognised.
///
/// Only a comparison between a bare identifier and a numeric literal is
/// recognised. Compound expressions, function calls and relationship navigation
/// (`Item.offers_in(Store)`) all return `None`; the caller preserves those
/// verbatim rather than translating them wrongly.
pub fn parse_requires(expression: &str) -> Option<Facet> {
    // Longest operators first so "<=" is not read as "<".
    const OPERATORS: &[&str] = &["<=", ">=", "<", ">"];
    for op in OPERATORS {
        // Reject anything carrying a second comparison or an (in)equality.
        if expression.contains("==") || expression.contains("!=") {
            return None;
        }
        let Some(index) = expression.find(op) else {
            continue;
        };
        let (lhs, rest) = expression.split_at(index);
        let rhs = &rest[op.len()..];
        if rhs.contains('<') || rhs.contains('>') {
            return None;
        }
        let (lhs, rhs) = (lhs.trim(), rhs.trim());

        let (subject, literal, subject_on_left) = if is_identifier(lhs) && !is_identifier(rhs) {
            (lhs, numeric_literal(rhs)?, true)
        } else if is_identifier(rhs) && !is_identifier(lhs) {
            (rhs, numeric_literal(lhs)?, false)
        } else {
            return None;
        };

        // `0 < X` means `X > 0`, so the facet flips when the subject is on the right.
        let facet = match (*op, subject_on_left) {
            ("<", true) | (">", false) => "maxExclusive",
            ("<=", true) | (">=", false) => "maxInclusive",
            (">", true) | ("<", false) => "minExclusive",
            (">=", true) | ("<=", false) => "minInclusive",
            _ => unreachable!("operator table is exhaustive"),
        };
        return Some(Facet {
            subject: subject.to_string(),
            facet,
            literal,
        });
    }
    None
}

// --------------------------------------------------------------------------- //
// Turtle helpers
// --------------------------------------------------------------------------- //

fn escape_literal(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 8);
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out
}

fn quote(text: &str) -> String {
    format!("\"{}\"", escape_literal(text))
}

/// Percent-encode an Ossie name for use as an IRI local part.
///
/// `.` is deliberately left alone so the `Concept.name` relationship identifier
/// survives intact, and the delimiters that would end an IRI are escaped.
fn iri_escape(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for byte in name.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => {
                let _ = write!(out, "%{byte:02X}");
            }
        }
    }
    out
}

// --------------------------------------------------------------------------- //
// Conversion
// --------------------------------------------------------------------------- //

struct Converter<'a> {
    base: String,
    doc: &'a OssieDocument,
    by_name: BTreeMap<&'a str, &'a Component>,
    facets: BTreeMap<String, Vec<Facet>>,
    issues: Vec<Issue>,
    sparql_constraints: usize,
    out: String,
}

/// Compile an Ossie ontology document into OWL 2 DL plus SHACL shapes.
///
/// `base_iri` defaults to `https://ossie.apache.org/ontology/{name}#`.
pub fn to_owl_shacl(
    doc: &OssieDocument,
    base_iri: Option<&str>,
    emit_shacl: bool,
) -> Result<OssieConversion, String> {
    let base = base_iri
        .map(str::to_string)
        .unwrap_or_else(|| format!("{DEFAULT_BASE}{}#", iri_escape(&doc.name)));

    let mut by_name = BTreeMap::new();
    for component in &doc.ontology {
        if by_name
            .insert(component.concept.as_str(), component)
            .is_some()
        {
            return Err(format!("concept {:?} declared twice", component.concept));
        }
    }

    let mut converter = Converter {
        base,
        doc,
        by_name,
        facets: BTreeMap::new(),
        issues: Vec::new(),
        sparql_constraints: 0,
        out: String::new(),
    };
    converter.run(emit_shacl)?;

    let entity_types = doc
        .ontology
        .iter()
        .filter(|c| c.concept_type != "ValueType")
        .count();
    let relationships = doc.ontology.iter().map(|c| c.relationships.len()).sum();

    Ok(OssieConversion {
        turtle: converter.out,
        base_iri: converter.base,
        concepts: doc.ontology.len(),
        entity_types,
        value_types: doc.ontology.len() - entity_types,
        relationships,
        sparql_constraints: converter.sparql_constraints,
        issues: converter.issues,
    })
}

impl<'a> Converter<'a> {
    fn run(&mut self, emit_shacl: bool) -> Result<(), String> {
        self.prologue();
        for component in &self.doc.ontology {
            self.concept(component)?;
        }
        for component in &self.doc.ontology {
            for relationship in &component.relationships {
                self.relationship(component, relationship)?;
            }
        }
        if emit_shacl {
            for component in &self.doc.ontology {
                if component.concept_type != "ValueType" {
                    self.node_shape(component)?;
                }
            }
        }
        Ok(())
    }

    fn prologue(&mut self) {
        let base = self.base.clone();
        let out = &mut self.out;
        let _ = writeln!(out, "@prefix owl:   <http://www.w3.org/2002/07/owl#> .");
        let _ = writeln!(out, "@prefix rdf:   <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .");
        let _ = writeln!(out, "@prefix rdfs:  <http://www.w3.org/2000/01/rdf-schema#> .");
        let _ = writeln!(out, "@prefix xsd:   <http://www.w3.org/2001/XMLSchema#> .");
        let _ = writeln!(out, "@prefix sh:    <http://www.w3.org/ns/shacl#> .");
        let _ = writeln!(out, "@prefix ossie: <{OSSIE_NS}> .");
        let _ = writeln!(out, "@prefix :      <{base}> .");
        let _ = writeln!(out);

        // Declared so the provenance terms do not push the graph into OWL Full.
        const ANNOTATION_PROPERTIES: &[&str] = &[
            "ontologyName",
            "specVersion",
            "conceptName",
            "conceptType",
            "extends",
            "relationshipName",
            "declaringConcept",
            "arity",
            "roleIndex",
            "roleName",
            "roleConcept",
            "multiplicity",
            "verbalizes",
            "derivedBy",
            "requires",
            "aiContext",
            "linkClass",
            "unaryClass",
            "identifyBy",
            "ontologyMappings",
            "declarationIndex",
        ];
        for term in ANNOTATION_PROPERTIES {
            let _ = writeln!(out, "ossie:{term} a owl:AnnotationProperty .");
        }
        let _ = writeln!(out);

        let ontology_iri = base.trim_end_matches(['#', '/']).to_string();
        let _ = writeln!(out, "<{ontology_iri}> a owl:Ontology ;");
        let _ = writeln!(out, "    ossie:ontologyName {} ;", quote(&self.doc.name));
        let _ = writeln!(out, "    rdfs:label {} ;", quote(&self.doc.name));
        if let Some(version) = &self.doc.version {
            let _ = writeln!(out, "    ossie:specVersion {} ;", quote(version));
        }
        if let Some(description) = &self.doc.description {
            let _ = writeln!(out, "    rdfs:comment {} ;", quote(description));
        }
        if let Some(ai_context) = &self.doc.ai_context {
            let _ = writeln!(out, "    ossie:aiContext {} ;", quote(&ai_context.to_string()));
        }
        if let Some(mappings) = &self.doc.ontology_mappings {
            let _ = writeln!(
                out,
                "    ossie:ontologyMappings {} ;",
                quote(&mappings.to_string())
            );
        }
        for expression in &self.doc.requires {
            let _ = writeln!(out, "    ossie:requires {} ;", quote(expression));
        }
        let _ = writeln!(out, "    .");
        let _ = writeln!(out);

        if self.doc.ontology_mappings.is_some() {
            self.issue(
                "ONTOLOGY_MAPPINGS_NOT_CONVERTED",
                self.doc.name.clone(),
                "SQL-to-object mappings belong in R2RML; preserved verbatim",
            );
        }
    }

    // ----------------------------------------------------------- concepts

    fn concept(&mut self, component: &Component) -> Result<(), String> {
        if component.concept_type == "ValueType" {
            self.value_type(component)
        } else {
            self.entity_type(component);
            Ok(())
        }
    }

    fn entity_type(&mut self, component: &Component) {
        let iri = self.concept_iri(&component.concept);
        let index = self.declaration_index(&component.concept);
        let mut body = String::new();

        let _ = writeln!(body, "{iri} a owl:Class ;");
        let _ = writeln!(body, "    ossie:conceptName {} ;", quote(&component.concept));
        let _ = writeln!(body, "    ossie:conceptType \"EntityType\" ;");
        let _ = writeln!(body, "    ossie:declarationIndex {index} ;");
        let _ = writeln!(body, "    rdfs:label {} ;", quote(&component.concept));
        if let Some(description) = &component.description {
            let _ = writeln!(body, "    rdfs:comment {} ;", quote(description));
        }
        if component.extends.is_empty() {
            // "every entity type implicitly extends the built-in concept Any"
            let _ = writeln!(body, "    rdfs:subClassOf owl:Thing ;");
        } else {
            for supertype in &component.extends {
                let _ = writeln!(body, "    ossie:extends {} ;", quote(supertype));
                let target = if supertype == "Any" {
                    "owl:Thing".to_string()
                } else {
                    self.concept_iri(supertype)
                };
                let _ = writeln!(body, "    rdfs:subClassOf {target} ;");
            }
        }
        for expression in &component.derived_by {
            let _ = writeln!(body, "    ossie:derivedBy {} ;", quote(expression));
        }
        for expression in &component.requires {
            let _ = writeln!(body, "    ossie:requires {} ;", quote(expression));
        }
        for identifier in &component.identify_by {
            let _ = writeln!(body, "    ossie:identifyBy {} ;", quote(identifier));
        }

        // owl:hasKey needs binary identifying relationships; the spec says
        // identifying relationships are always binary.
        let key_ok = component.identify_by.iter().all(|identifier| {
            component
                .relationships
                .iter()
                .any(|r| &r.name == identifier && r.roles.len() == 1)
        });
        if !component.identify_by.is_empty() && key_ok {
            let key: Vec<String> = component
                .identify_by
                .iter()
                .map(|identifier| self.relationship_iri(&component.concept, identifier))
                .collect();
            let _ = writeln!(body, "    owl:hasKey ( {} ) ;", key.join(" "));
        }
        let _ = writeln!(body, "    .");
        let _ = writeln!(body);
        self.out.push_str(&body);

        for _ in &component.derived_by {
            self.issue(
                "DERIVATION_NOT_EXPRESSIBLE",
                component.concept.clone(),
                "OWL has no rule construct for deriving a class population",
            );
        }
        for _ in &component.requires {
            self.issue(
                "REQUIRES_NOT_EXPRESSIBLE",
                component.concept.clone(),
                "constraint over an entity population is not a datatype facet",
            );
        }
        if !component.identify_by.is_empty() && !key_ok {
            self.issue(
                "REQUIRES_NOT_EXPRESSIBLE",
                component.concept.clone(),
                "owl:hasKey needs a binary identifying relationship",
            );
        }
    }

    fn value_type(&mut self, component: &Component) -> Result<(), String> {
        let iri = self.concept_iri(&component.concept);
        let index = self.declaration_index(&component.concept);
        let xsd_base = self.xsd_base(&component.concept, &mut Vec::new())?;

        let mut own = Vec::new();
        let mut unrecognised = 0usize;
        for expression in &component.requires {
            match parse_requires(expression) {
                Some(facet) if facet.subject == component.concept => own.push(facet),
                _ => unrecognised += 1,
            }
        }

        let mut body = String::new();
        let _ = writeln!(body, "{iri} a rdfs:Datatype ;");
        let _ = writeln!(body, "    ossie:conceptName {} ;", quote(&component.concept));
        let _ = writeln!(body, "    ossie:conceptType \"ValueType\" ;");
        let _ = writeln!(body, "    ossie:declarationIndex {index} ;");
        let _ = writeln!(body, "    rdfs:label {} ;", quote(&component.concept));
        if let Some(description) = &component.description {
            let _ = writeln!(body, "    rdfs:comment {} ;", quote(description));
        }
        for supertype in &component.extends {
            let _ = writeln!(body, "    ossie:extends {} ;", quote(supertype));
        }
        for expression in &component.derived_by {
            let _ = writeln!(body, "    ossie:derivedBy {} ;", quote(expression));
        }
        for expression in &component.requires {
            let _ = writeln!(body, "    ossie:requires {} ;", quote(expression));
        }
        if own.is_empty() {
            let _ = writeln!(body, "    owl:equivalentClass <{xsd_base}> ;");
        } else {
            let restrictions: Vec<String> = own
                .iter()
                .map(|facet| format!("[ xsd:{} {} ]", facet.facet, facet.literal))
                .collect();
            let _ = writeln!(
                body,
                "    owl:equivalentClass [ a rdfs:Datatype ; owl:onDatatype <{xsd_base}> ;\n        owl:withRestrictions ( {} ) ] ;",
                restrictions.join(" ")
            );
        }
        let _ = writeln!(body, "    .");
        let _ = writeln!(body);
        self.out.push_str(&body);

        self.facets.insert(component.concept.clone(), own);
        for _ in &component.derived_by {
            self.issue(
                "DERIVATION_NOT_EXPRESSIBLE",
                component.concept.clone(),
                "OWL has no rule construct for deriving a datatype population",
            );
        }
        for _ in 0..unrecognised {
            self.issue(
                "REQUIRES_NOT_EXPRESSIBLE",
                component.concept.clone(),
                "outside the recognised scalar-comparison fragment",
            );
        }
        Ok(())
    }

    // ------------------------------------------------------ relationships

    fn relationship(
        &mut self,
        component: &Component,
        relationship: &Relationship,
    ) -> Result<(), String> {
        match relationship.roles.len() + 1 {
            1 => self.unary(component, relationship),
            2 => self.binary(component, relationship)?,
            _ => self.nary(component, relationship)?,
        }
        for _ in &relationship.derived_by {
            self.issue(
                "DERIVATION_NOT_EXPRESSIBLE",
                format!("{}.{}", component.concept, relationship.name),
                "OWL has no rule construct for deriving a relationship population",
            );
        }
        for expression in &relationship.requires {
            if parse_requires(expression).is_none() {
                self.issue(
                    "REQUIRES_NOT_EXPRESSIBLE",
                    format!("{}.{}", component.concept, relationship.name),
                    "outside the recognised scalar-comparison fragment",
                );
            }
        }
        Ok(())
    }

    /// Annotations shared by every relationship shape, regardless of arity.
    fn relationship_common(&self, component: &Component, relationship: &Relationship) -> String {
        let index = component
            .relationships
            .iter()
            .position(|r| r.name == relationship.name)
            .unwrap_or(0);
        let mut body = String::new();
        let _ = writeln!(body, "    ossie:relationshipName {} ;", quote(&relationship.name));
        let _ = writeln!(body, "    ossie:declaringConcept {} ;", quote(&component.concept));
        let _ = writeln!(body, "    ossie:arity {} ;", relationship.roles.len() + 1);
        let _ = writeln!(body, "    ossie:declarationIndex {index} ;");
        let _ = writeln!(
            body,
            "    rdfs:label {} ;",
            quote(&format!("{}.{}", component.concept, relationship.name))
        );
        if let Some(description) = &relationship.description {
            let _ = writeln!(body, "    rdfs:comment {} ;", quote(description));
        }
        for pattern in &relationship.verbalizes {
            let _ = writeln!(body, "    ossie:verbalizes {} ;", quote(pattern));
        }
        if let Some(multiplicity) = &relationship.multiplicity {
            let _ = writeln!(body, "    ossie:multiplicity {} ;", quote(multiplicity));
        }
        for expression in &relationship.derived_by {
            let _ = writeln!(body, "    ossie:derivedBy {} ;", quote(expression));
        }
        for expression in &relationship.requires {
            let _ = writeln!(body, "    ossie:requires {} ;", quote(expression));
        }
        body
    }

    fn unary(&mut self, component: &Component, relationship: &Relationship) {
        let iri = self.relationship_iri(&component.concept, &relationship.name);
        let concept_iri = self.concept_iri(&component.concept);
        let common = self.relationship_common(component, relationship);

        let _ = writeln!(self.out, "{iri} a owl:Class ;");
        let _ = writeln!(self.out, "    ossie:unaryClass true ;");
        let _ = writeln!(self.out, "    rdfs:subClassOf {concept_iri} ;");
        self.out.push_str(&common);
        let _ = writeln!(self.out, "    .");
        let _ = writeln!(self.out);

        self.issue(
            "UNARY_RELATIONSHIP_AS_CLASS",
            format!("{}.{}", component.concept, relationship.name),
            "a unary fact type becomes a subclass; the relationship is no longer a property",
        );
    }

    fn binary(&mut self, component: &Component, relationship: &Relationship) -> Result<(), String> {
        let role = &relationship.roles[0];
        let is_value = self.is_value_type(&role.concept)?;
        let iri = self.relationship_iri(&component.concept, &relationship.name);
        let concept_iri = self.concept_iri(&component.concept);
        let range = self.range_iri(&role.concept);
        let common = self.relationship_common(component, relationship);
        let multiplicity = relationship.multiplicity.as_deref();

        let mut types = vec![if is_value {
            "owl:DatatypeProperty"
        } else {
            "owl:ObjectProperty"
        }];
        if matches!(multiplicity, Some("ManyToOne") | Some("OneToOne")) {
            types.push("owl:FunctionalProperty");
        }
        if multiplicity == Some("OneToOne") && !is_value {
            types.push("owl:InverseFunctionalProperty");
        }

        let _ = writeln!(self.out, "{iri} a {} ;", types.join(" , "));
        let _ = writeln!(self.out, "    rdfs:domain {concept_iri} ;");
        let _ = writeln!(self.out, "    rdfs:range {range} ;");
        let _ = writeln!(self.out, "    ossie:roleIndex 1 ;");
        let _ = writeln!(self.out, "    ossie:roleConcept {} ;", quote(&role.concept));
        if let Some(role_name) = &role.name {
            let _ = writeln!(self.out, "    ossie:roleName {} ;", quote(role_name));
        }
        self.out.push_str(&common);
        let _ = writeln!(self.out, "    .");
        let _ = writeln!(self.out);

        if multiplicity == Some("OneToOne") && is_value {
            // OWL 2 DL has no InverseFunctionalDataProperty: it costs decidability
            // and exists only in OWL Full. SHACL carries the constraint instead.
            self.issue(
                "INVERSE_FUNCTIONAL_DATA_PROPERTY",
                format!("{}.{}", component.concept, relationship.name),
                "OneToOne onto a ValueType is not expressible in OWL 2 DL; emitted as sh:sparql",
            );
        }
        Ok(())
    }

    fn nary(&mut self, component: &Component, relationship: &Relationship) -> Result<(), String> {
        let iri = self.relationship_iri(&component.concept, &relationship.name);
        let common = self.relationship_common(component, relationship);

        let _ = writeln!(self.out, "{iri} a owl:Class ;");
        let _ = writeln!(self.out, "    ossie:linkClass true ;");
        self.out.push_str(&common);
        let _ = writeln!(self.out, "    .");
        let _ = writeln!(self.out);

        for (index, (player, role_name)) in self.roles(component, relationship).iter().enumerate() {
            let is_value = self.is_value_type(player)?;
            let role_iri = self.role_property_iri(&component.concept, &relationship.name, index);
            let range = self.range_iri(player);
            let kind = if is_value {
                "owl:DatatypeProperty"
            } else {
                "owl:ObjectProperty"
            };
            let _ = writeln!(self.out, "{role_iri} a {kind} , owl:FunctionalProperty ;");
            let _ = writeln!(self.out, "    rdfs:domain {iri} ;");
            let _ = writeln!(self.out, "    rdfs:range {range} ;");
            let _ = writeln!(self.out, "    ossie:roleIndex {index} ;");
            let _ = writeln!(self.out, "    ossie:roleConcept {} ;", quote(player));
            if let Some(name) = role_name {
                let _ = writeln!(self.out, "    ossie:roleName {} ;", quote(name));
            }
            let _ = writeln!(self.out, "    .");
            let _ = writeln!(self.out);
        }

        self.issue(
            "NARY_RELATIONSHIP_REIFIED",
            format!("{}.{}", component.concept, relationship.name),
            "reified into a link class (W3C n-ary relations pattern 1)",
        );
        if relationship.multiplicity.as_deref() == Some("ManyToOne") {
            self.issue(
                "NARY_MULTIPLICITY_SHACL_ONLY",
                format!("{}.{}", component.concept, relationship.name),
                "a tuple functional dependency is beyond OWL cardinality restrictions",
            );
        }
        Ok(())
    }

    // ------------------------------------------------------------- SHACL

    fn node_shape(&mut self, component: &Component) -> Result<(), String> {
        let shape = format!("{}Shape", self.concept_iri(&component.concept).trim_end_matches('>'));
        let shape = format!("{shape}>");
        let concept_iri = self.concept_iri(&component.concept);

        let mut properties = Vec::new();
        for relationship in &component.relationships {
            if relationship.roles.len() != 1 {
                continue;
            }
            let role = &relationship.roles[0];
            let path = self.relationship_iri(&component.concept, &relationship.name);
            let mut shape_body = format!("        sh:path {path} ;\n        sh:name {} ;\n", quote(&relationship.name));
            if self.is_value_type(&role.concept)? {
                // sh:datatype compares the literal's own datatype IRI and does no
                // datatype entailment, so a user-declared value type must resolve
                // to its XSD base or every real graph would be rejected. The value
                // type's facets ride along on the same property shape.
                let xsd_base = self.xsd_base(&role.concept, &mut Vec::new())?;
                let _ = writeln!(shape_body, "        sh:datatype <{xsd_base}> ;");
                for facet in self.facets.get(&role.concept).into_iter().flatten() {
                    let _ = writeln!(shape_body, "        sh:{} {} ;", facet.facet, facet.literal);
                }
            } else {
                let _ = writeln!(shape_body, "        sh:class {} ;", self.range_iri(&role.concept));
            }
            if matches!(
                relationship.multiplicity.as_deref(),
                Some("ManyToOne") | Some("OneToOne")
            ) {
                let _ = writeln!(shape_body, "        sh:maxCount 1 ;");
            }
            properties.push(format!("    sh:property [\n{shape_body}    ] ;"));
        }

        let _ = writeln!(self.out, "{shape} a sh:NodeShape ;");
        let _ = writeln!(self.out, "    sh:targetClass {concept_iri} ;");
        for property in properties {
            let _ = writeln!(self.out, "{property}");
        }
        let _ = writeln!(self.out, "    .");
        let _ = writeln!(self.out);

        for relationship in &component.relationships {
            if relationship.roles.len() == 1
                && relationship.multiplicity.as_deref() == Some("OneToOne")
                && self.is_value_type(&relationship.roles[0].concept)?
            {
                self.uniqueness_constraint(component, relationship);
            }
            if relationship.roles.len() >= 2 {
                self.link_class_shape(component, relationship)?;
            }
        }
        Ok(())
    }

    /// SHACL replacement for the `InverseFunctionalDataProperty` OWL 2 DL forbids.
    fn uniqueness_constraint(&mut self, component: &Component, relationship: &Relationship) {
        let path = self.relationship_iri(&component.concept, &relationship.name);
        let path = path.trim_start_matches('<').trim_end_matches('>').to_string();
        let shape = format!(
            "<{}{}.{}Unique>",
            self.base,
            iri_escape(&component.concept),
            iri_escape(&relationship.name)
        );
        let concept_iri = self.concept_iri(&component.concept);
        let select = format!(
            "SELECT $this ?value WHERE {{ $this <{path}> ?value . ?other <{path}> ?value . FILTER (?other != $this) }}"
        );
        let message = format!(
            "{}.{} is OneToOne: each value identifies at most one {}",
            component.concept, relationship.name, component.concept
        );

        let _ = writeln!(self.out, "{shape} a sh:NodeShape ;");
        let _ = writeln!(self.out, "    sh:targetClass {concept_iri} ;");
        let _ = writeln!(self.out, "    sh:sparql [ a sh:SPARQLConstraint ;");
        let _ = writeln!(self.out, "        sh:message {} ;", quote(&message));
        let _ = writeln!(self.out, "        sh:select {} ] ;", quote(&select));
        let _ = writeln!(self.out, "    .");
        let _ = writeln!(self.out);
        self.sparql_constraints += 1;
    }

    fn link_class_shape(
        &mut self,
        component: &Component,
        relationship: &Relationship,
    ) -> Result<(), String> {
        let link_class = self.relationship_iri(&component.concept, &relationship.name);
        let shape = format!(
            "<{}{}.{}Shape>",
            self.base,
            iri_escape(&component.concept),
            iri_escape(&relationship.name)
        );

        let mut relationship_facets: BTreeMap<String, Vec<&Facet>> = BTreeMap::new();
        let parsed: Vec<Facet> = relationship
            .requires
            .iter()
            .filter_map(|e| parse_requires(e))
            .collect();
        for facet in &parsed {
            relationship_facets
                .entry(facet.subject.clone())
                .or_default()
                .push(facet);
        }

        let roles = self.roles(component, relationship);
        let mut properties = Vec::new();
        let mut role_iris = Vec::new();
        for (index, (player, role_name)) in roles.iter().enumerate() {
            let role_iri = self.role_property_iri(&component.concept, &relationship.name, index);
            role_iris.push(role_iri.trim_start_matches('<').trim_end_matches('>').to_string());
            let label = role_name.clone().unwrap_or_else(|| player.clone());
            let mut body = format!("        sh:path {role_iri} ;\n        sh:name {} ;\n", quote(&label));
            // Links do not contain nulls and each role is filled exactly once.
            let _ = writeln!(body, "        sh:minCount 1 ;");
            let _ = writeln!(body, "        sh:maxCount 1 ;");
            if self.is_value_type(player)? {
                let xsd_base = self.xsd_base(player, &mut Vec::new())?;
                let _ = writeln!(body, "        sh:datatype <{xsd_base}> ;");
                for facet in self.facets.get(player).into_iter().flatten() {
                    let _ = writeln!(body, "        sh:{} {} ;", facet.facet, facet.literal);
                }
            } else {
                let _ = writeln!(body, "        sh:class {} ;", self.range_iri(player));
            }
            for facet in relationship_facets.get(&label).into_iter().flatten() {
                let _ = writeln!(body, "        sh:{} {} ;", facet.facet, facet.literal);
            }
            properties.push(format!("    sh:property [\n{body}    ] ;"));
        }

        let _ = writeln!(self.out, "{shape} a sh:NodeShape ;");
        let _ = writeln!(self.out, "    sh:targetClass {link_class} ;");
        for property in properties {
            let _ = writeln!(self.out, "{property}");
        }

        if relationship.multiplicity.as_deref() == Some("ManyToOne") && role_iris.len() >= 2 {
            // The last role is functionally determined by the tuple of the others.
            // OWL cardinality restrictions constrain one property at a time and
            // cannot quantify over a tuple, so this is SHACL-SPARQL only.
            let (dependent, determinants) = role_iris.split_last().expect("checked len >= 2");
            let this_patterns: Vec<String> = determinants
                .iter()
                .enumerate()
                .map(|(i, p)| format!("$this <{p}> ?r{i} ."))
                .collect();
            let other_patterns: Vec<String> = determinants
                .iter()
                .enumerate()
                .map(|(i, p)| format!("?other <{p}> ?r{i} ."))
                .collect();
            let select = format!(
                "SELECT $this WHERE {{ {} $this <{dependent}> ?last . {} ?other <{dependent}> ?otherLast . FILTER (?other != $this && ?otherLast != ?last) }}",
                this_patterns.join(" "),
                other_patterns.join(" ")
            );
            let message = format!(
                "{}.{} is ManyToOne: the last role is functionally determined by the tuple of the preceding roles",
                component.concept, relationship.name
            );
            let _ = writeln!(self.out, "    sh:sparql [ a sh:SPARQLConstraint ;");
            let _ = writeln!(self.out, "        sh:message {} ;", quote(&message));
            let _ = writeln!(self.out, "        sh:select {} ] ;", quote(&select));
            self.sparql_constraints += 1;
        }
        let _ = writeln!(self.out, "    .");
        let _ = writeln!(self.out);
        Ok(())
    }

    // --------------------------------------------------------- utilities

    fn roles(&self, component: &Component, relationship: &Relationship) -> Vec<(String, Option<String>)> {
        let mut roles = vec![(component.concept.clone(), None)];
        for role in &relationship.roles {
            roles.push((role.concept.clone(), role.name.clone()));
        }
        roles
    }

    fn declaration_index(&self, concept: &str) -> usize {
        self.doc
            .ontology
            .iter()
            .position(|c| c.concept == concept)
            .unwrap_or(0)
    }

    fn concept_iri(&self, name: &str) -> String {
        format!("<{}{}>", self.base, iri_escape(name))
    }

    fn relationship_iri(&self, concept: &str, relationship: &str) -> String {
        format!(
            "<{}{}.{}>",
            self.base,
            iri_escape(concept),
            iri_escape(relationship)
        )
    }

    fn role_property_iri(&self, concept: &str, relationship: &str, index: usize) -> String {
        format!(
            "<{}{}.{}-role{index}>",
            self.base,
            iri_escape(concept),
            iri_escape(relationship)
        )
    }

    fn range_iri(&self, name: &str) -> String {
        if let Some(xsd) = builtin_xsd(name) {
            return format!("<{xsd}>");
        }
        if name == "Any" {
            return "owl:Thing".to_string();
        }
        self.concept_iri(name)
    }

    fn is_value_type(&self, name: &str) -> Result<bool, String> {
        if builtin_xsd(name).is_some() {
            return Ok(true);
        }
        if name == "Any" {
            return Ok(false);
        }
        match self.by_name.get(name) {
            Some(component) => Ok(component.concept_type == "ValueType"),
            None => Err(format!("role references undeclared concept {name:?}")),
        }
    }

    /// Walk `extends` until a built-in value type is reached.
    fn xsd_base(&self, name: &str, seen: &mut Vec<String>) -> Result<&'static str, String> {
        if let Some(xsd) = builtin_xsd(name) {
            return Ok(xsd);
        }
        if seen.iter().any(|s| s == name) {
            return Err(format!("cyclic extends chain through value type {name:?}"));
        }
        seen.push(name.to_string());
        let component = self
            .by_name
            .get(name)
            .ok_or_else(|| format!("value type {name:?} is not declared"))?;
        for supertype in &component.extends {
            if let Ok(xsd) = self.xsd_base(supertype, seen) {
                return Ok(xsd);
            }
        }
        // "Any value type concept must either directly or indirectly extend one of
        // the built-in value types", so an unrooted chain is a document error.
        Err(format!(
            "value type {name:?} does not extend a built-in value type"
        ))
    }

    fn issue(&mut self, kind: &'static str, element: String, detail: &'static str) {
        self.issues.push(Issue {
            kind,
            element,
            detail,
        });
    }
}

/// Parse an Ossie ontology document from YAML (or JSON, which is a YAML subset).
pub fn parse_document(source: &str) -> Result<OssieDocument, String> {
    serde_yaml::from_str(source).map_err(|e| format!("cannot parse Ossie ontology document: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = r#"
version: 0.2.0.dev0
name: Tiny
ontology:
  - concept: SocialSecurityNr
    type: ValueType
    extends: [Integer]
    requires: ["0 < SocialSecurityNr", "SocialSecurityNr <= 999999999"]
  - concept: Salary
    type: ValueType
    extends: [Decimal]
  - concept: NrDays
    type: ValueType
    extends: [Integer]
  - concept: Person
    type: EntityType
    identify_by: [nr]
    relationships:
      - name: nr
        roles: [{concept: SocialSecurityNr}]
        multiplicity: OneToOne
        verbalizes: ["{Person} is identified by {SocialSecurityNr}"]
      - name: earns
        roles: [{concept: Salary}]
        multiplicity: ManyToOne
        verbalizes: ["{Person} earns {Salary}"]
      - name: files_married_joint
        verbalizes: ["{Person} files married filing joint"]
  - concept: Store
    type: EntityType
    relationships:
      - name: ships_to_in_days
        roles:
          - concept: Store
            name: destination
          - concept: NrDays
        multiplicity: ManyToOne
        verbalizes: ["{Store} ships to {Store:destination} in {NrDays}"]
"#;

    fn convert() -> OssieConversion {
        let doc = parse_document(DOC).expect("parses");
        to_owl_shacl(&doc, Some("https://example.org/o#"), true).expect("converts")
    }

    #[test]
    fn entity_and_value_types_land_on_the_right_owl_construct() {
        let ttl = convert().turtle;
        assert!(ttl.contains("<https://example.org/o#Person> a owl:Class ;"));
        assert!(ttl.contains("<https://example.org/o#Salary> a rdfs:Datatype ;"));
    }

    #[test]
    fn value_type_facets_become_a_datatype_restriction() {
        let ttl = convert().turtle;
        assert!(ttl.contains("owl:onDatatype <http://www.w3.org/2001/XMLSchema#integer>"));
        assert!(ttl.contains("xsd:minExclusive \"0\"^^<http://www.w3.org/2001/XMLSchema#integer>"));
        assert!(
            ttl.contains("xsd:maxInclusive \"999999999\"^^<http://www.w3.org/2001/XMLSchema#integer>")
        );
    }

    #[test]
    fn value_type_without_facets_aliases_its_xsd_base() {
        let ttl = convert().turtle;
        assert!(ttl
            .contains("<https://example.org/o#Salary> a rdfs:Datatype ;"));
        assert!(ttl.contains("owl:equivalentClass <http://www.w3.org/2001/XMLSchema#decimal>"));
    }

    #[test]
    fn many_to_one_is_functional() {
        let ttl = convert().turtle;
        assert!(ttl.contains("<https://example.org/o#Person.earns> a owl:DatatypeProperty , owl:FunctionalProperty ;"));
    }

    #[test]
    fn one_to_one_onto_a_value_type_is_never_inverse_functional() {
        let result = convert();
        // OWL 2 DL forbids InverseFunctionalDataProperty.
        assert!(!result.turtle.contains("owl:InverseFunctionalProperty"));
        assert!(result
            .issues
            .iter()
            .any(|i| i.kind == "INVERSE_FUNCTIONAL_DATA_PROPERTY" && i.element == "Person.nr"));
        assert!(result.turtle.contains("sh:SPARQLConstraint"));
    }

    #[test]
    fn identify_by_becomes_has_key() {
        let ttl = convert().turtle;
        assert!(ttl.contains("owl:hasKey ( <https://example.org/o#Person.nr> )"));
    }

    #[test]
    fn unary_relationship_becomes_a_subclass() {
        let result = convert();
        assert!(result
            .turtle
            .contains("<https://example.org/o#Person.files_married_joint> a owl:Class ;"));
        assert!(result.turtle.contains("ossie:unaryClass true"));
        assert!(result.issues.iter().any(|i| i.kind == "UNARY_RELATIONSHIP_AS_CLASS"));
    }

    #[test]
    fn ternary_relationship_is_reified_with_ordered_roles() {
        let result = convert();
        assert!(result.turtle.contains("ossie:linkClass true"));
        assert!(result
            .turtle
            .contains("<https://example.org/o#Store.ships_to_in_days-role0>"));
        assert!(result
            .turtle
            .contains("<https://example.org/o#Store.ships_to_in_days-role2>"));
        assert!(result.issues.iter().any(|i| i.kind == "NARY_RELATIONSHIP_REIFIED"));
        assert!(result
            .issues
            .iter()
            .any(|i| i.kind == "NARY_MULTIPLICITY_SHACL_ONLY"));
    }

    #[test]
    fn nary_multiplicity_emits_a_tuple_dependency() {
        let ttl = convert().turtle;
        assert!(ttl.contains("?otherLast != ?last"));
    }

    #[test]
    fn role_names_survive() {
        let ttl = convert().turtle;
        assert!(ttl.contains("ossie:roleName \"destination\""));
    }

    #[test]
    fn counts_are_reported() {
        let result = convert();
        assert_eq!(result.concepts, 5);
        assert_eq!(result.entity_types, 2);
        assert_eq!(result.value_types, 3);
        assert_eq!(result.relationships, 4);
        assert_eq!(result.sparql_constraints, 2);
    }

    #[test]
    fn requires_parser_accepts_the_recognised_fragment() {
        let facet = parse_requires("0 < X").expect("recognised");
        assert_eq!(facet.subject, "X");
        assert_eq!(facet.facet, "minExclusive");

        let facet = parse_requires("X <= 10").expect("recognised");
        assert_eq!(facet.facet, "maxInclusive");

        let facet = parse_requires("Salary >= 0.0").expect("recognised");
        assert_eq!(facet.facet, "minInclusive");
        assert!(facet.literal.contains("decimal"));
    }

    #[test]
    fn requires_parser_declines_everything_else() {
        assert!(parse_requires("Item.offers_in(Store)").is_none());
        assert!(parse_requires("A < B").is_none());
        assert!(parse_requires("TaxRate == 10.0").is_none());
        assert!(parse_requires("EXISTS ( Person.earns )").is_none());
    }

    #[test]
    fn undeclared_role_concept_is_rejected() {
        let doc = parse_document(
            "version: 0.2.0.dev0\nname: B\nontology:\n  - concept: A\n    type: EntityType\n    relationships:\n      - name: r\n        roles: [{concept: Missing}]\n        verbalizes: [\"x\"]\n",
        )
        .expect("parses");
        let err = to_owl_shacl(&doc, None, true).expect_err("must fail");
        assert!(err.contains("undeclared concept"));
    }

    #[test]
    fn unrooted_value_type_is_rejected() {
        let doc = parse_document(
            "version: 0.2.0.dev0\nname: B\nontology:\n  - concept: V\n    type: ValueType\n",
        )
        .expect("parses");
        let err = to_owl_shacl(&doc, None, true).expect_err("must fail");
        assert!(err.contains("does not extend a built-in value type"));
    }

    #[test]
    fn emitted_turtle_parses_into_the_store() {
        // The whole point is that the rest of this crate can consume the output,
        // so the emitted Turtle must survive oxigraph's parser verbatim.
        let result = convert();
        let store = crate::graph::GraphStore::new();
        let triples = store
            .load_turtle(&result.turtle, None)
            .expect("emitted Turtle must parse");
        assert!(
            triples > 50,
            "expected a substantial graph, got {triples} triples"
        );
    }

    #[test]
    fn literals_are_escaped() {
        let doc = parse_document(
            "version: 0.2.0.dev0\nname: B\nontology:\n  - concept: A\n    type: EntityType\n    description: 'has \"quotes\" and \\ backslash'\n",
        )
        .expect("parses");
        let ttl = to_owl_shacl(&doc, Some("https://example.org/o#"), true)
            .expect("converts")
            .turtle;
        assert!(ttl.contains("\\\"quotes\\\""));
        assert!(ttl.contains("\\\\ backslash"));
    }
}
