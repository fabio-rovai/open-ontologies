//! The OWL 2 functional-syntax reader (`src/ofn.rs`), held to the W3C mapping.
//!
//! Every family test writes an OFN fragment and the Turtle that "OWL 2 Mapping
//! to RDF Graphs (Second Edition)" Table 1, Table 2 or Section 2.3 says it
//! denotes, and asserts the two graphs are ISOMORPHIC: both are canonicalised
//! with RDFC 1.0 and compared triple for triple, so a reader that emitted one
//! triple too many, one too few, or the right triples around the wrong blank
//! node fails. The expected Turtle is written from the specification and never
//! from this reader's output, which is the only way such a test can catch the
//! reader being wrong.
//!
//! `owl_api_equivalence_on_the_fixture_is_exactly_the_documented_differences`
//! is the second, independent check. It compares the reader against what the
//! OWL API 4.5.29 writes for the same document, and requires the two to agree
//! once, and only once, the differences `src/ofn.rs` documents are applied.
//! An undocumented difference fails it.

use std::collections::BTreeSet;
use std::sync::Arc;

use open_ontologies::config::CacheConfig;
use open_ontologies::graph::GraphStore;
use open_ontologies::ofn::{self, OfnError};
use open_ontologies::ontology::OntologyService;
use open_ontologies::registry::{LoadOptions, OntologyRegistry};
use open_ontologies::state::StateDb;

const ONTOLOGY: &str = "http://e.org/o";

const TTL_PREFIXES: &str = "@prefix : <http://e.org/#> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
@prefix swrl: <http://www.w3.org/2003/11/swrl#> .
@prefix swrlb: <http://www.w3.org/2003/11/swrlb#> .
";

fn document(axioms: &str) -> String {
    format!(
        "Prefix(:=<http://e.org/#>)\nPrefix(swrlb:=<http://www.w3.org/2003/11/swrlb#>)\nOntology(<{ONTOLOGY}>\n{axioms}\n)\n"
    )
}

/// The graph as a sorted list of canonical N-Triples lines, blank nodes
/// relabelled by RDFC 1.0 so isomorphic graphs compare equal.
fn canonical(store: &GraphStore) -> Vec<String> {
    let mut lines: Vec<String> = store
        .canonicalize_blank_nodes()
        .unwrap()
        .all_triples()
        .unwrap()
        .into_iter()
        .map(|(s, p, o)| format!("{s} {p} {o} ."))
        .collect();
    lines.sort();
    lines
}

/// Load `doc` through the same file path every CLI and MCP load takes, under
/// a `.owl` name, which is the extension the reader most needs to survive.
fn load_ofn(doc: &str) -> GraphStore {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("fragment.owl");
    std::fs::write(&path, doc).unwrap();
    let store = GraphStore::new();
    store
        .load_file(path.to_str().unwrap())
        .unwrap_or_else(|e| panic!("OFN did not load: {e}\n{doc}"));
    store
}

fn turtle(ttl: &str) -> GraphStore {
    let store = GraphStore::new();
    store
        .load_turtle(&format!("{TTL_PREFIXES}{ttl}"), None)
        .unwrap_or_else(|e| panic!("expected Turtle does not parse: {e}\n{ttl}"));
    store
}

/// Assert the axioms map to exactly `expected` (plus the ontology header
/// triple every document has).
fn assert_maps_to(axioms: &str, expected: &str) {
    let got = canonical(&load_ofn(&document(axioms)));
    let want = canonical(&turtle(&format!(
        "<{ONTOLOGY}> a owl:Ontology .\n{expected}"
    )));
    if got != want {
        let g: BTreeSet<_> = got.iter().collect();
        let w: BTreeSet<_> = want.iter().collect();
        panic!(
            "OFN did not map to the specified graph.\nonly in reader output:\n  {}\nonly in expected:\n  {}\n",
            g.difference(&w).map(|s| s.as_str()).collect::<Vec<_>>().join("\n  "),
            w.difference(&g).map(|s| s.as_str()).collect::<Vec<_>>().join("\n  ")
        );
    }
}

// --------------------------------------------------------------------------- //
// Families, each against the specification
// --------------------------------------------------------------------------- //

#[test]
fn ontology_header_imports_annotations_and_every_declaration_kind() {
    let doc = "Prefix(:=<http://e.org/#>)
Ontology(<http://e.org/o> <http://e.org/o/1.0>
  Import(<http://e.org/imported>)
  Annotation(rdfs:comment \"about\")
  Declaration(Class(:C))
  Declaration(Datatype(:D))
  Declaration(ObjectProperty(:op))
  Declaration(DataProperty(:dp))
  Declaration(AnnotationProperty(:ap))
  Declaration(NamedIndividual(:i))
)";
    let got = canonical(&load_ofn(doc));
    let want = canonical(&turtle(
        "<http://e.org/o> a owl:Ontology ; owl:versionIRI <http://e.org/o/1.0> ;
           owl:imports <http://e.org/imported> ; rdfs:comment \"about\" .
         :C a owl:Class . :D a rdfs:Datatype . :op a owl:ObjectProperty .
         :dp a owl:DatatypeProperty . :ap a owl:AnnotationProperty . :i a owl:NamedIndividual .",
    ));
    assert_eq!(got, want);
}

#[test]
fn an_ontology_without_an_iri_is_a_blank_node() {
    let got = canonical(&load_ofn("Ontology(Declaration(Class(<http://e.org/#C>)))"));
    let want = canonical(&turtle("[] a owl:Ontology . :C a owl:Class ."));
    assert_eq!(got, want);
}

#[test]
fn object_class_expressions() {
    assert_maps_to(
        "SubClassOf(:A ObjectIntersectionOf(:B :C))
         SubClassOf(:A ObjectUnionOf(:B ObjectComplementOf(:C)))
         SubClassOf(:A ObjectOneOf(:i :j))
         SubClassOf(:A ObjectSomeValuesFrom(:r :B))
         SubClassOf(:A ObjectAllValuesFrom(ObjectInverseOf(:r) :B))
         SubClassOf(:A ObjectHasValue(:r :i))
         SubClassOf(:A ObjectHasSelf(:r))",
        ":A rdfs:subClassOf [ a owl:Class ; owl:intersectionOf ( :B :C ) ] ,
            [ a owl:Class ; owl:unionOf ( :B [ a owl:Class ; owl:complementOf :C ] ) ] ,
            [ a owl:Class ; owl:oneOf ( :i :j ) ] ,
            [ a owl:Restriction ; owl:onProperty :r ; owl:someValuesFrom :B ] ,
            [ a owl:Restriction ; owl:onProperty [ owl:inverseOf :r ] ; owl:allValuesFrom :B ] ,
            [ a owl:Restriction ; owl:onProperty :r ; owl:hasValue :i ] ,
            [ a owl:Restriction ; owl:onProperty :r ; owl:hasSelf true ] .",
    );
}

#[test]
fn object_cardinalities_qualified_unqualified_and_qualified_by_thing() {
    // The last row is the module notes' item 1: a filler of owl:Thing is a
    // qualified restriction because the source wrote one, and the reader does
    // not normalise it away as the OWL API does.
    assert_maps_to(
        "SubClassOf(:A ObjectMinCardinality(2 :r))
         SubClassOf(:A ObjectMaxCardinality(3 :r :B))
         SubClassOf(:A ObjectExactCardinality(1 :s :C))
         SubClassOf(:A ObjectExactCardinality(4 :s))
         SubClassOf(:A ObjectMaxCardinality(0 :s))
         SubClassOf(:A ObjectMinCardinality(1 :t owl:Thing))",
        ":A rdfs:subClassOf
            [ a owl:Restriction ; owl:onProperty :r ; owl:minCardinality \"2\"^^xsd:nonNegativeInteger ] ,
            [ a owl:Restriction ; owl:onProperty :r ; owl:maxQualifiedCardinality \"3\"^^xsd:nonNegativeInteger ; owl:onClass :B ] ,
            [ a owl:Restriction ; owl:onProperty :s ; owl:qualifiedCardinality \"1\"^^xsd:nonNegativeInteger ; owl:onClass :C ] ,
            [ a owl:Restriction ; owl:onProperty :s ; owl:cardinality \"4\"^^xsd:nonNegativeInteger ] ,
            [ a owl:Restriction ; owl:onProperty :s ; owl:maxCardinality \"0\"^^xsd:nonNegativeInteger ] ,
            [ a owl:Restriction ; owl:onProperty :t ; owl:minQualifiedCardinality \"1\"^^xsd:nonNegativeInteger ; owl:onClass owl:Thing ] .",
    );
}

#[test]
fn cardinalities_are_typed_non_negative_integer_before_any_store_sees_them() {
    // The parser's own output is the witness. Oxigraph 0.5.9's store wrote
    // every integer-derived literal back as xsd:integer, so a store round
    // trip could not tell nonNegativeInteger from integer; the pinned store
    // keeps it since #335, but this asks the parser, which is what is meant.
    let triples = ofn::parse(&document("SubClassOf(:A ObjectMinCardinality(007 :r))")).unwrap();
    let card = triples
        .iter()
        .find(|t| t.predicate.as_str().ends_with("#minCardinality"))
        .expect("a minCardinality triple");
    assert_eq!(
        card.object.to_string(),
        "\"7\"^^<http://www.w3.org/2001/XMLSchema#nonNegativeInteger>"
    );
}

#[test]
fn data_ranges_and_data_restrictions() {
    assert_maps_to(
        "SubClassOf(:A DataSomeValuesFrom(:age DatatypeRestriction(xsd:integer xsd:minInclusive \"0\"^^xsd:integer xsd:maxExclusive \"150\"^^xsd:integer)))
         SubClassOf(:A DataAllValuesFrom(:name DataUnionOf(xsd:string DataOneOf(\"x\" \"y\"@en))))
         SubClassOf(:A DataSomeValuesFrom(:age DataIntersectionOf(xsd:integer DataComplementOf(xsd:negativeInteger))))
         SubClassOf(:A DataSomeValuesFrom(:p1 :p2 xsd:integer))
         SubClassOf(:A DataAllValuesFrom(:p1 :p2 DataComplementOf(xsd:integer)))
         SubClassOf(:A DataHasValue(:name \"Bob\"))
         SubClassOf(:A DataMinCardinality(1 :name))
         SubClassOf(:A DataMaxCardinality(2 :name xsd:string))
         SubClassOf(:A DataExactCardinality(1 :age))
         SubClassOf(:A DataExactCardinality(3 :age xsd:int))
         DatatypeDefinition(:Small DatatypeRestriction(xsd:integer xsd:maxInclusive \"10\"^^xsd:integer))",
        ":A rdfs:subClassOf
            [ a owl:Restriction ; owl:onProperty :age ; owl:someValuesFrom
                [ a rdfs:Datatype ; owl:onDatatype xsd:integer ;
                  owl:withRestrictions ( [ xsd:minInclusive 0 ] [ xsd:maxExclusive 150 ] ) ] ] ,
            [ a owl:Restriction ; owl:onProperty :name ; owl:allValuesFrom
                [ a rdfs:Datatype ; owl:unionOf ( xsd:string [ a rdfs:Datatype ; owl:oneOf ( \"x\" \"y\"@en ) ] ) ] ] ,
            [ a owl:Restriction ; owl:onProperty :age ; owl:someValuesFrom
                [ a rdfs:Datatype ; owl:intersectionOf ( xsd:integer [ a rdfs:Datatype ; owl:datatypeComplementOf xsd:negativeInteger ] ) ] ] ,
            [ a owl:Restriction ; owl:onProperties ( :p1 :p2 ) ; owl:someValuesFrom xsd:integer ] ,
            [ a owl:Restriction ; owl:onProperties ( :p1 :p2 ) ; owl:allValuesFrom [ a rdfs:Datatype ; owl:datatypeComplementOf xsd:integer ] ] ,
            [ a owl:Restriction ; owl:onProperty :name ; owl:hasValue \"Bob\" ] ,
            [ a owl:Restriction ; owl:onProperty :name ; owl:minCardinality \"1\"^^xsd:nonNegativeInteger ] ,
            [ a owl:Restriction ; owl:onProperty :name ; owl:maxQualifiedCardinality \"2\"^^xsd:nonNegativeInteger ; owl:onDataRange xsd:string ] ,
            [ a owl:Restriction ; owl:onProperty :age ; owl:cardinality \"1\"^^xsd:nonNegativeInteger ] ,
            [ a owl:Restriction ; owl:onProperty :age ; owl:qualifiedCardinality \"3\"^^xsd:nonNegativeInteger ; owl:onDataRange xsd:int ] .
         :Small owl:equivalentClass [ a rdfs:Datatype ; owl:onDatatype xsd:integer ;
                  owl:withRestrictions ( [ xsd:maxInclusive 10 ] ) ] .",
    );
}

#[test]
fn class_axioms_including_the_n_ary_rows() {
    assert_maps_to(
        "EquivalentClasses(:A :B ObjectSomeValuesFrom(:r :C))
         DisjointClasses(:A :B)
         DisjointClasses(:A :B :C)
         DisjointUnion(:P :A :B)",
        ":A owl:equivalentClass :B .
         :B owl:equivalentClass [ a owl:Restriction ; owl:onProperty :r ; owl:someValuesFrom :C ] .
         :A owl:disjointWith :B .
         [] a owl:AllDisjointClasses ; owl:members ( :A :B :C ) .
         :P owl:disjointUnionOf ( :A :B ) .",
    );
}

#[test]
fn object_property_axioms_and_all_seven_characteristics() {
    assert_maps_to(
        "SubObjectPropertyOf(:s :r)
         SubObjectPropertyOf(ObjectInverseOf(:s) :t)
         SubObjectPropertyOf(ObjectPropertyChain(:p :p) :gp)
         EquivalentObjectProperties(:s :t :u)
         DisjointObjectProperties(:p :t)
         DisjointObjectProperties(:p :t ObjectInverseOf(:gp))
         InverseObjectProperties(:r ObjectInverseOf(:s))
         ObjectPropertyDomain(:r :A)
         ObjectPropertyRange(:r ObjectUnionOf(:A :B))
         FunctionalObjectProperty(:p)
         InverseFunctionalObjectProperty(ObjectInverseOf(:p))
         ReflexiveObjectProperty(:t)
         IrreflexiveObjectProperty(:p)
         SymmetricObjectProperty(:t)
         AsymmetricObjectProperty(:p)
         TransitiveObjectProperty(:gp)",
        ":s rdfs:subPropertyOf :r .
         [ owl:inverseOf :s ] rdfs:subPropertyOf :t .
         :gp owl:propertyChainAxiom ( :p :p ) .
         :s owl:equivalentProperty :t . :t owl:equivalentProperty :u .
         :p owl:propertyDisjointWith :t .
         [] a owl:AllDisjointProperties ; owl:members ( :p :t [ owl:inverseOf :gp ] ) .
         :r owl:inverseOf [ owl:inverseOf :s ] .
         :r rdfs:domain :A ; rdfs:range [ a owl:Class ; owl:unionOf ( :A :B ) ] .
         :p a owl:FunctionalProperty .
         [ owl:inverseOf :p ] a owl:InverseFunctionalProperty .
         :t a owl:ReflexiveProperty . :p a owl:IrreflexiveProperty .
         :t a owl:SymmetricProperty . :p a owl:AsymmetricProperty .
         :gp a owl:TransitiveProperty .",
    );
}

#[test]
fn data_property_axioms_and_has_key() {
    assert_maps_to(
        "SubDataPropertyOf(:nick :name)
         EquivalentDataProperties(:name :nick)
         DisjointDataProperties(:age :name)
         DisjointDataProperties(:age :name :nick)
         DataPropertyDomain(:age :Person)
         DataPropertyRange(:age xsd:nonNegativeInteger)
         FunctionalDataProperty(:age)
         HasKey(:Person (:parent ObjectInverseOf(:child)) (:name))
         HasKey(:Thing () ())",
        ":nick rdfs:subPropertyOf :name .
         :name owl:equivalentProperty :nick .
         :age owl:propertyDisjointWith :name .
         [] a owl:AllDisjointProperties ; owl:members ( :age :name :nick ) .
         :age rdfs:domain :Person ; rdfs:range xsd:nonNegativeInteger ; a owl:FunctionalProperty .
         :Person owl:hasKey ( :parent [ owl:inverseOf :child ] :name ) .
         :Thing owl:hasKey () .",
    );
}

#[test]
fn individual_axioms_anonymous_individuals_and_the_inverse_assertion_row() {
    assert_maps_to(
        "SameIndividual(:a :b :c)
         DifferentIndividuals(:a :b)
         DifferentIndividuals(:a :b :c)
         ClassAssertion(:C :a)
         ClassAssertion(ObjectSomeValuesFrom(:r :C) _:x)
         ObjectPropertyAssertion(:r _:x :a)
         ObjectPropertyAssertion(:r :a :b)
         ObjectPropertyAssertion(ObjectInverseOf(:r) :a :c)
         NegativeObjectPropertyAssertion(ObjectInverseOf(:r) :a :b)
         DataPropertyAssertion(:dp :a \"42\"^^xsd:integer)
         NegativeDataPropertyAssertion(:dp :b \"7\"^^xsd:integer)",
        ":a owl:sameAs :b . :b owl:sameAs :c .
         :a owl:differentFrom :b .
         [] a owl:AllDifferent ; owl:members ( :a :b :c ) .
         :a a :C .
         _:x a [ a owl:Restriction ; owl:onProperty :r ; owl:someValuesFrom :C ] ; :r :a .
         :a :r :b .
         :c :r :a .
         [] a owl:NegativePropertyAssertion ; owl:sourceIndividual :a ;
            owl:assertionProperty [ owl:inverseOf :r ] ; owl:targetIndividual :b .
         :a :dp 42 .
         [] a owl:NegativePropertyAssertion ; owl:sourceIndividual :b ;
            owl:assertionProperty :dp ; owl:targetValue 7 .",
    );
}

#[test]
fn annotation_axioms() {
    assert_maps_to(
        "AnnotationAssertion(rdfs:label :a \"A\"@en)
         AnnotationAssertion(:ap :a <http://elsewhere.org/x>)
         AnnotationAssertion(:ap _:x _:y)
         SubAnnotationPropertyOf(:ap2 :ap)
         AnnotationPropertyDomain(:ap :C)
         AnnotationPropertyRange(:ap xsd:string)",
        ":a rdfs:label \"A\"@en ; :ap <http://elsewhere.org/x> .
         [] :ap [] .
         :ap2 rdfs:subPropertyOf :ap .
         :ap rdfs:domain :C ; rdfs:range xsd:string .",
    );
}

#[test]
fn axiom_annotations_follow_section_2_3_for_each_of_its_three_cases() {
    // 2.3.1 main triple (with a nested annotation, Table 2's second row, and
    // with the side-triple rows DisjointUnion, HasKey and the property chain),
    // 2.3.2 the n-ary split with the annotation repeated per link, and 2.3.3
    // the axioms already rooted at a blank node.
    assert_maps_to(
        "SubClassOf(Annotation(Annotation(:ap \"nested\") rdfs:comment \"why\") :A :B)
         Declaration(Annotation(:ap \"d\") Class(:A))
         EquivalentClasses(Annotation(:ap \"three\") :A :B :C)
         DisjointClasses(Annotation(:ap \"n-ary\") :A :B :C)
         DisjointUnion(Annotation(:ap \"u\") :P :A :B)
         SubObjectPropertyOf(Annotation(:ap \"chain\") ObjectPropertyChain(:r :s) :t)
         HasKey(Annotation(:ap \"key\") :A () (:dp))
         NegativeObjectPropertyAssertion(Annotation(:ap \"neg\") :r :a :b)",
        ":A rdfs:subClassOf :B .
         _:ax1 a owl:Axiom ; owl:annotatedSource :A ; owl:annotatedProperty rdfs:subClassOf ;
            owl:annotatedTarget :B ; rdfs:comment \"why\" .
         [] a owl:Annotation ; owl:annotatedSource _:ax1 ; owl:annotatedProperty rdfs:comment ;
            owl:annotatedTarget \"why\" ; :ap \"nested\" .

         :A a owl:Class .
         [] a owl:Axiom ; owl:annotatedSource :A ; owl:annotatedProperty rdf:type ;
            owl:annotatedTarget owl:Class ; :ap \"d\" .

         :A owl:equivalentClass :B . :B owl:equivalentClass :C .
         [] a owl:Axiom ; owl:annotatedSource :A ; owl:annotatedProperty owl:equivalentClass ;
            owl:annotatedTarget :B ; :ap \"three\" .
         [] a owl:Axiom ; owl:annotatedSource :B ; owl:annotatedProperty owl:equivalentClass ;
            owl:annotatedTarget :C ; :ap \"three\" .

         [] a owl:AllDisjointClasses ; owl:members ( :A :B :C ) ; :ap \"n-ary\" .

         :P owl:disjointUnionOf _:u .
         _:u rdf:first :A ; rdf:rest ( :B ) .
         [] a owl:Axiom ; owl:annotatedSource :P ; owl:annotatedProperty owl:disjointUnionOf ;
            owl:annotatedTarget _:u ; :ap \"u\" .

         :t owl:propertyChainAxiom _:c .
         _:c rdf:first :r ; rdf:rest ( :s ) .
         [] a owl:Axiom ; owl:annotatedSource :t ; owl:annotatedProperty owl:propertyChainAxiom ;
            owl:annotatedTarget _:c ; :ap \"chain\" .

         :A owl:hasKey _:k .
         _:k rdf:first :dp ; rdf:rest rdf:nil .
         [] a owl:Axiom ; owl:annotatedSource :A ; owl:annotatedProperty owl:hasKey ;
            owl:annotatedTarget _:k ; :ap \"key\" .

         [] a owl:NegativePropertyAssertion ; owl:sourceIndividual :a ;
            owl:assertionProperty :r ; owl:targetIndividual :b ; :ap \"neg\" .",
    );
}

#[test]
fn literals_plain_tagged_typed_escaped_and_rdf_plain_literal() {
    assert_maps_to(
        "DataPropertyAssertion(:dp :a \"plain\")
         DataPropertyAssertion(:dp :a \"tagged\"@en-GB)
         DataPropertyAssertion(:dp :a \"2.5\"^^xsd:decimal)
         DataPropertyAssertion(:dp :a \"string\"^^xsd:string)
         DataPropertyAssertion(:dp :a \"say \\\"hi\\\" \\\\ bye\")
         DataPropertyAssertion(:dp :a \"multi
line\")
         DataPropertyAssertion(:dp :a \"x@y@\"^^rdf:PlainLiteral)
         DataPropertyAssertion(:dp :a \"bonjour@fr\"^^rdf:PlainLiteral)
         DataPropertyAssertion(:dp :a \"spaced\" ^^ <http://www.w3.org/2001/XMLSchema#token>)",
        ":a :dp \"plain\" , \"tagged\"@en-gb , 2.5 , \"string\" , \"say \\\"hi\\\" \\\\ bye\" ,
            \"multi\\nline\" , \"x@y\" , \"bonjour\"@fr , \"spaced\"^^xsd:token .",
    );
}

#[test]
fn lexical_freedom_comments_whitespace_full_iris_and_prefix_redefinition() {
    let doc = "# leading comment with ( parens ) and \"quotes\"
Prefix( ex: = <http://e.org/one#> )
Prefix(ex:=<http://e.org/#>)   # the later declaration wins
Ontology( <http://e.org/o>
   SubClassOf (   ex:A   # a comment between arguments (with parens)
                  <http://e.org/#B> )
   AnnotationAssertion(rdfs:comment ex:A \"a # inside a string is not a comment\")
   SubClassOf(<http://e.org/#C>ex:A)
)
# trailing comment";
    let got = canonical(&load_ofn(doc));
    let want = canonical(&turtle(
        "<http://e.org/o> a owl:Ontology .
         :A rdfs:subClassOf :B ; rdfs:comment \"a # inside a string is not a comment\" .
         :C rdfs:subClassOf :A .",
    ));
    assert_eq!(got, want);
}

#[test]
fn swrl_rules_map_to_the_vocabulary_the_rule_front_end_reads() {
    assert_maps_to(
        "DLSafeRule(Annotation(rdfs:comment \"r\")
            Body(ClassAtom(:A Variable(:x))
                 ObjectPropertyAtom(:r Variable(:x) Variable(:y))
                 DataPropertyAtom(:dp Variable(:x) Variable(:v))
                 BuiltInAtom(swrlb:greaterThan Variable(:v) \"1\"^^xsd:integer)
                 DataRangeAtom(xsd:integer Variable(:v))
                 SameIndividualAtom(Variable(:x) :a)
                 DifferentIndividualsAtom(Variable(:x) Variable(:y)))
            Head(ClassAtom(:B Variable(:x))))",
        ":x a swrl:Variable . :y a swrl:Variable . :v a swrl:Variable .
         [] a swrl:Imp ; rdfs:comment \"r\" ;
            swrl:body
              [ a swrl:AtomList ; rdf:first [ a swrl:ClassAtom ; swrl:classPredicate :A ; swrl:argument1 :x ] ; rdf:rest
              [ a swrl:AtomList ; rdf:first [ a swrl:IndividualPropertyAtom ; swrl:propertyPredicate :r ; swrl:argument1 :x ; swrl:argument2 :y ] ; rdf:rest
              [ a swrl:AtomList ; rdf:first [ a swrl:DatavaluedPropertyAtom ; swrl:propertyPredicate :dp ; swrl:argument1 :x ; swrl:argument2 :v ] ; rdf:rest
              [ a swrl:AtomList ; rdf:first [ a swrl:BuiltinAtom ; swrl:builtin swrlb:greaterThan ; swrl:arguments ( :v 1 ) ] ; rdf:rest
              [ a swrl:AtomList ; rdf:first [ a swrl:DataRangeAtom ; swrl:dataRange xsd:integer ; swrl:argument1 :v ] ; rdf:rest
              [ a swrl:AtomList ; rdf:first [ a swrl:SameIndividualAtom ; swrl:argument1 :x ; swrl:argument2 :a ] ; rdf:rest
              [ a swrl:AtomList ; rdf:first [ a swrl:DifferentIndividualsAtom ; swrl:argument1 :x ; swrl:argument2 :y ] ; rdf:rest rdf:nil ] ] ] ] ] ] ] ;
            swrl:head
              [ a swrl:AtomList ; rdf:first [ a swrl:ClassAtom ; swrl:classPredicate :B ; swrl:argument1 :x ] ; rdf:rest rdf:nil ] .",
    );
}

#[test]
fn a_swrl_rule_loaded_from_ofn_reaches_the_rule_front_end() {
    let store = load_ofn(&document(
        "DLSafeRule(Body(ClassAtom(:A Variable(:x)) ObjectPropertyAtom(:r Variable(:x) Variable(:y)))
                    Head(ClassAtom(:B Variable(:y))))",
    ));
    let imp = open_ontologies::rulesyntax::swrl_from_triples(&store.all_triples().unwrap()).unwrap();
    assert_eq!(imp.source_rules, 1);
    assert_eq!(imp.rules.len(), 1, "{:?}", imp.refused);
    assert!(imp.refused.is_empty());
}

// --------------------------------------------------------------------------- //
// Refusals and syntax errors: never a silent drop
// --------------------------------------------------------------------------- //

#[test]
fn an_unknown_construct_is_refused_by_name_line_and_count_and_nothing_loads() {
    let doc = document(
        "SubClassOf(:A :B)
FancyAxiom(:a (nested (parens)) :b)
SubClassOf(:A FancyClassExpression(:x))
FancyAxiom(:c)
SubClassOf(:B :C)",
    );
    match ofn::parse(&doc) {
        Err(OfnError::Unsupported(census)) => {
            assert_eq!(census.len(), 2, "{census:?}");
            assert_eq!(census[0].construct, "FancyAxiom");
            assert_eq!(census[0].count, 2);
            assert_eq!(census[0].first_line, 5);
            assert_eq!(census[1].construct, "FancyClassExpression");
            assert_eq!(census[1].count, 1);
            assert_eq!(census[1].first_line, 6);
        }
        other => panic!("expected a refusal, got {other:?}"),
    }

    // Through the load path: an error naming the construct, and an untouched store.
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("refused.owl");
    std::fs::write(&path, &doc).unwrap();
    let store = GraphStore::new();
    store
        .load_turtle("<http://e.org/#x> <http://e.org/#y> <http://e.org/#z> .", None)
        .unwrap();
    let err = store.load_file(path.to_str().unwrap()).unwrap_err().to_string();
    assert!(err.contains("FancyAxiom") && err.contains("line 5"), "{err}");
    assert!(err.contains("FancyClassExpression") && err.contains("line 6"), "{err}");
    assert_eq!(store.triple_count(), 1, "a refused document must load nothing");
}

#[test]
fn syntax_errors_name_the_line_and_what_was_wrong() {
    let cases: &[(&str, usize, &str)] = &[
        ("SubClassOf(:A undeclared:B)", 4, "prefix `undeclared:`"),
        ("AnnotationAssertion(rdfs:label :a \"bad \\n escape\")", 4, "may escape only"),
        ("EquivalentClasses(:A)", 4, "at least 2"),
        ("SubClassOf(:A DataOneOf(\"x\"))", 4, "`DataOneOf` cannot stand"),
        ("SubClassOf(:A <relative>)", 4, "not a valid absolute IRI"),
        ("SubClassOf(:A :B :C)", 4, "expected `)` closing `SubClassOf`"),
        ("ObjectPropertyAssertion(ObjectInverseOf(ObjectInverseOf(:r)) :a :b)", 4, "cannot stand where object property IRI"),
        ("SubClassOf(:A ObjectMinCardinality(-1 :r))", 4, "cardinality"),
        ("DataPropertyAssertion(:dp :a \"x\"@)", 4, "language tag"),
    ];
    for (axioms, line, needle) in cases {
        match ofn::parse(&document(axioms)) {
            Err(OfnError::Syntax { line: l, message }) => {
                assert_eq!(l, *line, "{axioms}: {message}");
                assert!(message.contains(needle), "{axioms}: {message}");
            }
            other => panic!("{axioms}: expected a syntax error, got {other:?}"),
        }
    }
    match ofn::parse("Prefix(:=<http://e.org/#>)\nOntology()\nSubClassOf(:A :B)") {
        Err(OfnError::Syntax { line: 3, message }) => assert!(message.contains("after the closing")),
        other => panic!("trailing text: {other:?}"),
    }
    match ofn::parse("Prefix(:=<http://e.org/#>)\n") {
        Err(OfnError::Syntax { message, .. }) => assert!(message.contains("no `Ontology(`")),
        other => panic!("no ontology: {other:?}"),
    }
}

// --------------------------------------------------------------------------- //
// Wiring: every path that reads a file reaches the reader
// --------------------------------------------------------------------------- //

const SMALL: &str = "Prefix(:=<http://e.org/#>)
Ontology(<http://e.org/o>
Declaration(Class(:A))
SubClassOf(:A ObjectSomeValuesFrom(:r :B))
SubClassOf(:A ObjectSomeValuesFrom(:r :B))
)";

#[test]
fn every_file_path_and_every_extension_reaches_the_reader() {
    let tmp = tempfile::tempdir().unwrap();
    // 1 ontology + 1 declaration + 2 x (subClassOf + 3 restriction triples).
    // The repeated axiom makes 10 statements; its restriction is a fresh blank
    // node each time, so the graph keeps all 10 triples.
    for name in ["a.owl", "a.ofn", "a.fss", "a.OFN", "no_extension"] {
        let path = tmp.path().join(name);
        std::fs::write(&path, SMALL).unwrap();
        let p = path.to_str().unwrap();

        let store = GraphStore::new();
        assert_eq!(store.load_file(p).unwrap(), 10, "{name}: load_file");

        let counts = GraphStore::validate_file(p).unwrap();
        assert_eq!((counts.statements, counts.triples), (10, 10), "{name}: validate_file");

        let report: serde_json::Value =
            serde_json::from_str(&OntologyService::validate_file(p).unwrap()).unwrap();
        assert_eq!(report["valid"], true, "{name}: onto_validate (path)");

        let ttl = GraphStore::read_as_turtle(p).unwrap();
        let back = GraphStore::new();
        back.load_turtle(&ttl, None).unwrap();
        assert_eq!(back.triple_count(), 10, "{name}: read_as_turtle");
    }
    // Inline content with no file name: `validate -` and onto_validate inline.
    let counts = GraphStore::validate_text(SMALL).unwrap();
    assert_eq!(counts.statements, 10);
    let report: serde_json::Value =
        serde_json::from_str(&OntologyService::validate_string(SMALL).unwrap()).unwrap();
    assert_eq!(report["valid"], true);
    // And the inline Turtle path is unchanged.
    assert_eq!(
        GraphStore::validate_text("<http://e/a> <http://e/b> <http://e/c> .").unwrap().triples,
        1
    );
}

#[test]
fn a_malformed_ofn_file_says_why_instead_of_blaming_rdf_xml() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("broken.owl");
    std::fs::write(&path, "Prefix(:=<http://e.org/#>)\nOntology(\nSubClassOf(:A\n").unwrap();
    let err = GraphStore::validate_file(path.to_str().unwrap()).unwrap_err().to_string();
    assert!(err.starts_with("OWL functional syntax"), "{err}");
    assert!(!err.contains("Unexpected text event"), "{err}");
}

#[test]
fn the_mcp_load_path_through_the_registry_and_its_cache_reads_ofn() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("ontology.owl");
    std::fs::write(&source, SMALL).unwrap();
    let db = StateDb::open(&tmp.path().join("state.db")).unwrap();
    let graph = Arc::new(GraphStore::new());
    let cfg = CacheConfig {
        enabled: true,
        dir: tmp.path().join("cache").to_string_lossy().into_owned(),
        idle_ttl_secs: 3600,
        evictor_interval_secs: 3600,
        auto_refresh: false,
        hash_prefix_bytes: 64 * 1024,
    };
    let registry = OntologyRegistry::new(graph.clone(), db, cfg).unwrap();
    let first = registry
        .load_file(source.to_str().unwrap(), LoadOptions::default())
        .unwrap();
    assert_eq!(first.triple_count, 10);
    assert_eq!(first.origin, "source");
    // The second load comes from the N-Quads compile cache, which must give
    // back the same graph the reader produced.
    let before = canonical(&graph);
    let second = registry
        .load_file(source.to_str().unwrap(), LoadOptions::default())
        .unwrap();
    assert_eq!(second.origin, "cache");
    assert_eq!(canonical(&graph), before);
}

// --------------------------------------------------------------------------- //
// Against the OWL API
// --------------------------------------------------------------------------- //

/// The graph as a SET of triples in which every blank node is replaced by a
/// Merkle hash of what it reaches along outgoing edges, which is the shape of
/// every blank node the mapping produces (an expression, a list cell, a
/// reification). Two structurally identical blank-node subgraphs collapse to
/// one, so this compares the graphs' lean quotients, and that is what the OWL
/// API comparison needs: the OWL API writes some annotated axioms more than
/// once (item 6 below), which RDFC would rightly call a different graph and
/// which says nothing more than the axiom written once.
type LeanGraph = BTreeSet<(String, String, String)>;

fn lean_triples(store: &GraphStore) -> LeanGraph {
    use std::collections::{HashMap, HashSet};
    use std::hash::{Hash, Hasher};
    let triples = store.all_triples().unwrap();
    let mut out: HashMap<&str, Vec<(&str, &str)>> = HashMap::new();
    for (s, p, o) in &triples {
        if s.starts_with("_:") {
            out.entry(s.as_str()).or_default().push((p.as_str(), o.as_str()));
        }
    }
    fn hash_of<'a>(
        term: &'a str,
        out: &HashMap<&'a str, Vec<(&'a str, &'a str)>>,
        memo: &mut HashMap<&'a str, String>,
        active: &mut HashSet<&'a str>,
    ) -> String {
        if !term.starts_with("_:") {
            return term.to_string();
        }
        if let Some(h) = memo.get(term) {
            return h.clone();
        }
        if !active.insert(term) {
            return "_:cycle".to_string();
        }
        let mut parts: Vec<String> = out
            .get(term)
            .map(|v| v.iter().map(|(p, o)| format!("{p} {}", hash_of(o, out, memo, active))).collect())
            .unwrap_or_default();
        parts.sort();
        active.remove(term);
        let mut h = std::collections::hash_map::DefaultHasher::new();
        parts.hash(&mut h);
        let label = format!("_:h{:016x}", h.finish());
        memo.insert(term, label.clone());
        label
    }
    let mut memo = HashMap::new();
    let mut active = HashSet::new();
    triples
        .iter()
        .map(|(s, p, o)| {
            (
                hash_of(s, &out, &mut memo, &mut active),
                p.clone(),
                hash_of(o, &out, &mut memo, &mut active),
            )
        })
        .collect()
}

/// `tests/fixtures/ofn/all_constructs.ofn` is every construct family in one
/// document; `all_constructs.owlapi.nt` is OWL API 4.5.29's RDF/XML for it,
/// converted to N-Triples by rdflib (not by Oxigraph's store, which would
/// rewrite every integer-derived literal to xsd:integer on the way through).
/// The OWL API sorts the operands of set constructors and the fixture is
/// written in that order, so list order is not one of the differences.
///
/// What remains is exactly what the OWL API writes that the mapping does not,
/// numbered as in `src/ofn.rs`. Items 1 to 4 are each undone by one SPARQL
/// update; items 5 and 6 (and 7, which this fixture does not exercise) are
/// absorbed by comparing lean quotients
/// ([`lean_triples`]). If the reader drifts, or the OWL API differs in a way
/// nobody wrote down, the sets differ and the failure prints both sides.
///
/// `skip` leaves one rewrite out, which is how the test proves each of them
/// is load-bearing.
fn owl_api_comparison(skip: Option<usize>) -> (LeanGraph, LeanGraph) {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ofn");
    let mine = GraphStore::new();
    mine.load_file(dir.join("all_constructs.ofn").to_str().unwrap())
        .unwrap();
    let reference = GraphStore::new();
    reference
        .load_file(dir.join("all_constructs.owlapi.nt").to_str().unwrap())
        .unwrap();
    const P: &str = "PREFIX owl: <http://www.w3.org/2002/07/owl#>
PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
PREFIX ex: <http://example.org/ofn#>
";
    let rewrites: [(&GraphStore, String); 4] = [
        // Item 1: the OWL API writes an owl:Thing-qualified cardinality unqualified.
        (
            &mine,
            format!(
                "{P} DELETE {{ ?x owl:minQualifiedCardinality ?n ; owl:onClass owl:Thing }}
                     INSERT {{ ?x owl:minCardinality ?n }}
                     WHERE  {{ ?x owl:minQualifiedCardinality ?n ; owl:onClass owl:Thing }}"
            ),
        ),
        // Item 2: owl:distinctMembers where the mapping says owl:members.
        (
            &reference,
            format!(
                "{P} DELETE {{ ?x owl:distinctMembers ?l }} INSERT {{ ?x owl:members ?l }}
                     WHERE {{ ?x owl:distinctMembers ?l }}"
            ),
        ),
        // Item 3: a type triple for the one entity the fixture never declares.
        (&reference, format!("{P} DELETE DATA {{ ex:Undeclared a owl:Class }}")),
        // Item 4: rdf:type rdf:List on the cells of a DataOneOf literal list.
        (&reference, format!("{P} DELETE WHERE {{ ?c a rdf:List }}")),
    ];
    for (i, (store, update)) in rewrites.iter().enumerate() {
        if skip != Some(i) {
            store.sparql_update(update).unwrap();
        }
    }
    // Items 5 and 6 need no rewrite under the lean comparison: the copy of a
    // list the OWL API writes as a reified axiom's annotatedTarget hashes to
    // the list it copies, and a reified or blank-node-rooted axiom written
    // twice hashes to itself.
    (lean_triples(&mine), lean_triples(&reference))
}

#[test]
fn owl_api_equivalence_on_the_fixture_is_exactly_the_documented_differences() {
    let (got, want) = owl_api_comparison(None);
    assert!(
        got == want,
        "undocumented difference from the OWL API.\nonly in reader:\n  {}\nonly in OWL API:\n  {}",
        got.difference(&want).map(|t| format!("{t:?}")).collect::<Vec<_>>().join("\n  "),
        want.difference(&got).map(|t| format!("{t:?}")).collect::<Vec<_>>().join("\n  ")
    );
    // Each documented difference is real: leaving its rewrite out breaks the
    // equality. Without this the list in src/ofn.rs could rot into a list of
    // differences that no longer exist, and the equality above would pass
    // just the same.
    for item in 0..4 {
        let (got, want) = owl_api_comparison(Some(item));
        assert_ne!(got, want, "documented difference {} no longer occurs", item + 1);
    }
}
