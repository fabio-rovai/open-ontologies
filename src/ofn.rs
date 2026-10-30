//! OWL 2 Functional-Style Syntax (OFN) read into the RDF graph it denotes.
//!
//! # The gap this closes
//!
//! Every loader in this crate hands the bytes to an Oxigraph RDF parser, and
//! OFN is not RDF. It is the syntax the OWL 2 structural specification is
//! written in and the one the OWL API writes by default, so it is what the
//! reasoner-evaluation corpora ship: all 1,920 files of the ORE 2015 pool are
//! OFN under a `.owl` extension. `.owl` was routed to RDF/XML, so every one of
//! them failed with `Unexpected text event: 'Prefix(owl:='`, and a benchmark
//! that wanted to run this engine against the competition corpus could not
//! load a single file of it.
//!
//! # What this file produces, and on whose authority
//!
//! Triples, and exactly the triples the W3C "OWL 2 Web Ontology Language
//! Mapping to RDF Graphs (Second Edition)" says an ontology denotes: Table 1
//! for entities, expressions and axioms, Table 2 for annotations, and Section
//! 2.3 for axioms that carry annotations. Nothing here decides what an axiom
//! MEANS. The mapping is a function from the structural syntax to RDF and the
//! rest of this engine already reads RDF, so the reasoner, the SHACL validator
//! and every audit see an OFN file as precisely the graph the OWL API would
//! have written for it, up to the differences catalogued below.
//!
//! The tokeniser and parser are written by hand. The established Rust OWL
//! library is LGPL and this crate is MIT, and the grammar is small enough that
//! owning it costs less than the licence question.
//!
//! # Where the OWL API writes something else, and why this follows the spec
//!
//! Measured on a sample of the ORE pool against OWL API 4.5.29's RDF/XML
//! writer, and pinned on a fixture by `tests/ofn_reader_test.rs`. Each item is
//! a choice the mapping makes and the OWL API overrides, so none of them is a
//! defect in this reader, and each is kept because a reader that copied one
//! writer's habits would be describing that writer and not the ontology. All
//! seven leave the two graphs equivalent under the direct semantics.
//!
//! 1. A cardinality restriction written with an `owl:Thing` (or `rdfs:Literal`)
//!    filler is QUALIFIED in the source and Table 1 maps it to
//!    `owl:minQualifiedCardinality` with `owl:onClass owl:Thing`. The OWL API
//!    stores the unqualified and the `owl:Thing` forms as one object and writes
//!    both unqualified.
//! 2. `DifferentIndividuals` over more than two individuals is
//!    `owl:AllDifferent` with `owl:members` in Table 1. The OWL API writes the
//!    OWL 1 property `owl:distinctMembers`, which the reverse mapping still
//!    accepts.
//! 3. The OWL API adds an `rdf:type` triple for every entity in the signature
//!    that the file never declared, so that its own output parses back as OWL 2
//!    DL. Table 1 maps a `Declaration` to a type triple and maps nothing else
//!    to one, so an undeclared entity stays undeclared here. A file that wants
//!    the declaration can write it.
//! 4. The OWL API types the cells of some lists `rdf:List` (a `DataOneOf`
//!    literal list, a SWRL built-in's argument list). `T(SEQ ...)` in the
//!    mapping is `rdf:first` and `rdf:rest` and nothing else.
//! 5. For an annotated `DisjointUnion`, `HasKey` or property-chain axiom the
//!    OWL API writes the list twice, once as the main triple's object and once
//!    as the reification's `owl:annotatedTarget`. Section 2.3.1's own HasKey
//!    example names the same `_:y` in both places, and so does this reader.
//! 6. The OWL API writes some annotated axioms more than once: the reification
//!    of an annotated `EquivalentClasses` link three times, an annotated
//!    `NegativeObjectPropertyAssertion` node twice. One axiom is one
//!    translation here.
//! 7. The other way round: two axioms that differ only in their annotations
//!    are one axiom with two annotation sets to the OWL API, which writes its
//!    anonymous class expression once and points both reifications at it.
//!    The mapping translates each axiom by itself, so each gets its own blank
//!    node here.
//!
//! The OWL API also sorts the operands of set constructors (`ObjectUnionOf`,
//! `DisjointClasses` and the rest), reorders the atoms of a SWRL body, and
//! picks its own orientation for binary symmetric axioms (`InverseObjectProperties`,
//! `EquivalentDataProperties`). This reader keeps the order the source wrote,
//! which is the only order nobody had to choose.
//!
//! # What it refuses, and how
//!
//! A construct this reader does not translate is REFUSED BY NAME, LINE AND
//! COUNT and the whole document fails, never dropped. That is the house rule
//! `src/rulesyntax.rs` states for rule syntaxes and the reason is the same: an
//! ontology that silently lost the axioms the reader did not recognise loads,
//! reasons to a fixpoint and certifies green, about an ontology nobody wrote.
//! Parsing does not stop at the first refusal. It skips the refused axiom by
//! its parentheses and carries on, so one error names every construct that
//! would have been lost and how often, instead of making the caller fix them
//! one at a time.
//!
//! A malformed document is a different failure and is reported differently: a
//! syntax error names the line and what was expected there, and stops.
//!
//! # SWRL
//!
//! `DLSafeRule` is not part of the OWL 2 structural specification. It is the
//! OWL API's functional-syntax spelling of SWRL, and 22 files of the ORE 2015
//! pool carry one. It is translated to the `swrl:` RDF vocabulary of the SWRL
//! member submission, which is what the OWL API writes and what
//! `src/rulesyntax.rs` already reads, so a rule loaded from OFN reaches the
//! rule front end exactly as one loaded from RDF/XML does.

use std::collections::{HashMap, HashSet};
use std::fmt;

use oxigraph::model::vocab::{rdf, rdfs, xsd};
use oxigraph::model::{BlankNode, Literal, NamedNode, NamedOrBlankNode, Term, Triple};

macro_rules! owl {
    ($local:literal) => {
        NamedNode::new_unchecked(concat!("http://www.w3.org/2002/07/owl#", $local))
    };
}

macro_rules! swrl {
    ($local:literal) => {
        NamedNode::new_unchecked(concat!("http://www.w3.org/2003/11/swrl#", $local))
    };
}

const RDF_PLAIN_LITERAL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#PlainLiteral";

/// The prefixes every OFN document may use without declaring them. The
/// structural specification fixes these five, and the OWL API writes them out
/// anyway, so predefining them only matters for hand-written files.
pub(crate) const STANDARD_PREFIXES: &[(&str, &str)] = &[
    ("owl", "http://www.w3.org/2002/07/owl#"),
    ("rdf", "http://www.w3.org/1999/02/22-rdf-syntax-ns#"),
    ("rdfs", "http://www.w3.org/2000/01/rdf-schema#"),
    ("xsd", "http://www.w3.org/2001/XMLSchema#"),
    ("xml", "http://www.w3.org/XML/1998/namespace"),
];

/// Every keyword the grammar knows, in any position.
///
/// The list exists to keep two failures apart. A known keyword in the wrong
/// place (`DataUnionOf` where a class expression belongs) is a malformed
/// document and a syntax error. A keyword that is not here at all is a
/// construct this reader was never taught, which is a refusal: counted, named,
/// and the rest of the document still read so the report is complete.
const KNOWN_KEYWORDS: &[&str] = &[
    "Prefix",
    "Ontology",
    "Import",
    "Annotation",
    "Declaration",
    "Class",
    "Datatype",
    "ObjectProperty",
    "DataProperty",
    "AnnotationProperty",
    "NamedIndividual",
    "ObjectInverseOf",
    "ObjectPropertyChain",
    "DataIntersectionOf",
    "DataUnionOf",
    "DataComplementOf",
    "DataOneOf",
    "DatatypeRestriction",
    "ObjectIntersectionOf",
    "ObjectUnionOf",
    "ObjectComplementOf",
    "ObjectOneOf",
    "ObjectSomeValuesFrom",
    "ObjectAllValuesFrom",
    "ObjectHasValue",
    "ObjectHasSelf",
    "ObjectMinCardinality",
    "ObjectMaxCardinality",
    "ObjectExactCardinality",
    "DataSomeValuesFrom",
    "DataAllValuesFrom",
    "DataHasValue",
    "DataMinCardinality",
    "DataMaxCardinality",
    "DataExactCardinality",
    "SubClassOf",
    "EquivalentClasses",
    "DisjointClasses",
    "DisjointUnion",
    "SubObjectPropertyOf",
    "EquivalentObjectProperties",
    "DisjointObjectProperties",
    "InverseObjectProperties",
    "ObjectPropertyDomain",
    "ObjectPropertyRange",
    "FunctionalObjectProperty",
    "InverseFunctionalObjectProperty",
    "ReflexiveObjectProperty",
    "IrreflexiveObjectProperty",
    "SymmetricObjectProperty",
    "AsymmetricObjectProperty",
    "TransitiveObjectProperty",
    "SubDataPropertyOf",
    "EquivalentDataProperties",
    "DisjointDataProperties",
    "DataPropertyDomain",
    "DataPropertyRange",
    "FunctionalDataProperty",
    "DatatypeDefinition",
    "HasKey",
    "SameIndividual",
    "DifferentIndividuals",
    "ClassAssertion",
    "ObjectPropertyAssertion",
    "NegativeObjectPropertyAssertion",
    "DataPropertyAssertion",
    "NegativeDataPropertyAssertion",
    "AnnotationAssertion",
    "SubAnnotationPropertyOf",
    "AnnotationPropertyDomain",
    "AnnotationPropertyRange",
    "DLSafeRule",
    "Body",
    "Head",
    "ClassAtom",
    "DataRangeAtom",
    "ObjectPropertyAtom",
    "DataPropertyAtom",
    "BuiltInAtom",
    "SameIndividualAtom",
    "DifferentIndividualsAtom",
    "Variable",
];

/// One construct the reader refused, with how often and where it first
/// appeared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    pub construct: String,
    pub first_line: usize,
    pub count: usize,
}

/// Why a document did not load.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OfnError {
    /// The document is not well-formed OFN. Parsing stopped at `line`.
    Syntax { line: usize, message: String },
    /// The document is well-formed and uses constructs this reader does not
    /// translate. Nothing was loaded, because loading the rest would be a
    /// smaller ontology reported as the whole one.
    Unsupported(Vec<Refusal>),
}

impl fmt::Display for OfnError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OfnError::Syntax { line, message } => {
                write!(f, "OWL functional syntax, line {line}: {message}")
            }
            OfnError::Unsupported(refusals) => {
                let total: usize = refusals.iter().map(|r| r.count).sum();
                write!(
                    f,
                    "OWL functional syntax: {total} axiom(s) use a construct this reader does \
                     not translate, so nothing was loaded rather than a smaller ontology: "
                )?;
                for (i, r) in refusals.iter().enumerate() {
                    if i > 0 {
                        write!(f, "; ")?;
                    }
                    write!(
                        f,
                        "`{}` x{} (first at line {})",
                        r.construct, r.count, r.first_line
                    )?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for OfnError {}

/// True when the first token of `content`, past whitespace and `#` comments,
/// is `Prefix(` or `Ontology(`.
///
/// No other serialisation this crate reads can open that way. Turtle's SPARQL
/// style `PREFIX ex: <...>` has a name and no parenthesis after the keyword,
/// and Manchester syntax writes `Prefix:` with a colon, so the parenthesis is
/// the discriminator and it is checked rather than assumed.
pub fn looks_like_ofn(content: &str) -> bool {
    let mut rest = content.trim_start_matches('\u{feff}');
    loop {
        rest = rest.trim_start();
        if let Some(after) = rest.strip_prefix('#') {
            rest = after.find('\n').map(|i| &after[i..]).unwrap_or("");
            continue;
        }
        break;
    }
    ["Prefix", "Ontology"].iter().any(|kw| {
        rest.strip_prefix(kw)
            .map(|after| after.trim_start().starts_with('('))
            .unwrap_or(false)
    })
}

/// Parse an OFN document into the triples the W3C mapping assigns it.
///
/// All or nothing: on any error no triple is returned, which is the contract
/// every loader in `src/graph.rs` keeps (issue #93).
pub fn parse(text: &str) -> Result<Vec<Triple>, OfnError> {
    let mut p = Parser::new(text);
    match p.document() {
        Ok(()) => {}
        Err(Fail::Syntax { line, message }) => return Err(OfnError::Syntax { line, message }),
        // `document` absorbs every refusal into the census and never returns
        // one, so reaching this arm is a defect in the parser, and it is
        // reported as a syntax error at that line rather than swallowed.
        Err(Fail::Refuse { construct, line }) => {
            return Err(OfnError::Syntax {
                line,
                message: format!("internal: refusal of `{construct}` escaped the axiom loop"),
            });
        }
    }
    if !p.refusals.is_empty() {
        let mut census: Vec<Refusal> = Vec::new();
        for (construct, line) in p.refusals {
            match census.iter_mut().find(|r| r.construct == construct) {
                Some(r) => r.count += 1,
                None => census.push(Refusal {
                    construct,
                    first_line: line,
                    count: 1,
                }),
            }
        }
        return Err(OfnError::Unsupported(census));
    }
    Ok(p.out)
}

/// The `Prefix(name:=<iri>)` declarations at the head of `text`, in the order
/// written, stopping at `Ontology(` or at the first token that is not part of
/// a well-formed declaration.
///
/// [`parse`] returns triples, and a graph has no prefixes, so the names a
/// document chose are lost by the time [`crate::ofn_writer`] sees it. Reading
/// them here, with the same lexer, is what lets an OFN-to-OFN conversion keep
/// `:A` as `:A` instead of spelling every IRI in full.
pub fn declared_prefixes(text: &str) -> Vec<(String, String)> {
    let mut lx = Lexer::new(text);
    let mut out = Vec::new();
    loop {
        let Ok((Tok::Word("Prefix"), _)) = lx.next() else {
            return out;
        };
        let (Ok((Tok::Open, _)), Ok((Tok::Word(name), _)), Ok((Tok::Eq, _)), Ok((Tok::Iri(iri), _)), Ok((Tok::Close, _))) =
            (lx.next(), lx.next(), lx.next(), lx.next(), lx.next())
        else {
            return out;
        };
        match name.strip_suffix(':') {
            Some(n) if !n.contains(':') => out.push((n.to_string(), iri.to_string())),
            _ => return out,
        }
    }
}

// --------------------------------------------------------------------------- //
// Tokens
// --------------------------------------------------------------------------- //

#[derive(Debug, Clone, PartialEq)]
enum Tok<'a> {
    Open,
    Close,
    Eq,
    /// A full IRI, without its angle brackets.
    Iri(&'a str),
    /// A keyword, an abbreviated IRI, a prefix name or a non-negative integer.
    /// Keywords never contain a colon and abbreviated IRIs always do, which is
    /// how the two are told apart.
    Word(&'a str),
    /// `_:label`, kept whole.
    Blank(&'a str),
    /// A quoted string with its escapes resolved.
    Str(String),
    /// `@tag`, without the `@`.
    Lang(&'a str),
    /// `^^`
    Hats,
    Eof,
}

impl fmt::Display for Tok<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Tok::Open => write!(f, "`(`"),
            Tok::Close => write!(f, "`)`"),
            Tok::Eq => write!(f, "`=`"),
            Tok::Iri(s) => write!(f, "`<{s}>`"),
            Tok::Word(s) => write!(f, "`{s}`"),
            Tok::Blank(s) => write!(f, "`{s}`"),
            Tok::Str(s) => {
                let short: String = s.chars().take(40).collect();
                write!(f, "string \"{short}\"")
            }
            Tok::Lang(s) => write!(f, "`@{s}`"),
            Tok::Hats => write!(f, "`^^`"),
            Tok::Eof => write!(f, "end of document"),
        }
    }
}

enum Fail {
    Syntax { line: usize, message: String },
    Refuse { construct: String, line: usize },
}

fn syntax<T>(line: usize, message: impl Into<String>) -> Result<T, Fail> {
    Err(Fail::Syntax {
        line,
        message: message.into(),
    })
}

/// A streaming tokeniser. It never holds more than one token of lookahead,
/// because the corpus this was written for has single files over 500 MB and a
/// token vector for one of them would cost several times the file.
struct Lexer<'a> {
    src: &'a str,
    pos: usize,
    line: usize,
    /// Parenthesis depth after the tokens consumed so far. A refused axiom is
    /// skipped by reading until the depth returns to where the axiom started,
    /// which is what lets parsing continue past it.
    depth: usize,
    peeked: Option<(Tok<'a>, usize)>,
}

impl<'a> Lexer<'a> {
    fn new(src: &'a str) -> Self {
        let src = src.strip_prefix('\u{feff}').unwrap_or(src);
        Self {
            src,
            pos: 0,
            line: 1,
            depth: 0,
            peeked: None,
        }
    }

    fn skip_trivia(&mut self) {
        let b = self.src.as_bytes();
        while self.pos < b.len() {
            match b[self.pos] {
                b'\n' => {
                    self.line += 1;
                    self.pos += 1;
                }
                b' ' | b'\t' | b'\r' | 0x0c => self.pos += 1,
                // A comment runs to the end of the line. It can only open
                // between tokens: inside an IRI or a string `#` is content,
                // and both are consumed whole by `lex` before this sees them.
                b'#' => {
                    while self.pos < b.len() && b[self.pos] != b'\n' {
                        self.pos += 1;
                    }
                }
                _ => break,
            }
        }
    }

    fn lex(&mut self) -> Result<(Tok<'a>, usize), Fail> {
        self.skip_trivia();
        let line = self.line;
        let b = self.src.as_bytes();
        if self.pos >= b.len() {
            return Ok((Tok::Eof, line));
        }
        let start = self.pos;
        let tok = match b[start] {
            b'(' => {
                self.pos += 1;
                Tok::Open
            }
            b')' => {
                self.pos += 1;
                Tok::Close
            }
            b'=' => {
                self.pos += 1;
                Tok::Eq
            }
            b'<' => {
                let mut i = start + 1;
                while i < b.len() && b[i] != b'>' {
                    // An IRI cannot hold whitespace, so a newline before the
                    // closing bracket is an unterminated IRI and reporting it
                    // here names the right line instead of one far below.
                    if matches!(b[i], b'\n' | b' ' | b'\t' | b'\r') {
                        return syntax(line, "unterminated IRI: `<` without a closing `>`");
                    }
                    i += 1;
                }
                if i >= b.len() {
                    return syntax(line, "unterminated IRI: `<` without a closing `>`");
                }
                self.pos = i + 1;
                Tok::Iri(&self.src[start + 1..i])
            }
            b'"' => {
                let mut out = String::new();
                let mut i = start + 1;
                let mut seg = i;
                loop {
                    if i >= b.len() {
                        return syntax(line, "unterminated string literal");
                    }
                    match b[i] {
                        b'"' => {
                            out.push_str(&self.src[seg..i]);
                            i += 1;
                            break;
                        }
                        b'\\' => {
                            out.push_str(&self.src[seg..i]);
                            match b.get(i + 1) {
                                Some(b'"') => out.push('"'),
                                Some(b'\\') => out.push('\\'),
                                // The grammar admits exactly these two escapes.
                                // Guessing what `\n` meant would change the
                                // literal's value without saying so.
                                _ => {
                                    return syntax(
                                        self.line,
                                        "a string literal may escape only `\\\"` and `\\\\`",
                                    );
                                }
                            }
                            i += 2;
                            seg = i;
                        }
                        b'\n' => {
                            self.line += 1;
                            i += 1;
                        }
                        _ => i += 1,
                    }
                }
                self.pos = i;
                Tok::Str(out)
            }
            b'@' => {
                let mut i = start + 1;
                while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'-') {
                    i += 1;
                }
                if i == start + 1 {
                    return syntax(line, "`@` without a language tag");
                }
                self.pos = i;
                Tok::Lang(&self.src[start + 1..i])
            }
            b'^' => {
                if b.get(start + 1) != Some(&b'^') {
                    return syntax(line, "a single `^`: a datatype is introduced by `^^`");
                }
                self.pos = start + 2;
                Tok::Hats
            }
            _ => {
                let mut i = start;
                while i < b.len()
                    && !matches!(
                        b[i],
                        b' ' | b'\t' | b'\r' | b'\n' | 0x0c | b'(' | b')' | b'<' | b'>' | b'"' | b'='
                    )
                {
                    i += 1;
                }
                if i == start {
                    let ch = self.src[start..].chars().next().unwrap_or('?');
                    return syntax(line, format!("unexpected character `{ch}`"));
                }
                self.pos = i;
                let word = &self.src[start..i];
                if word.starts_with("_:") {
                    Tok::Blank(word)
                } else {
                    Tok::Word(word)
                }
            }
        };
        Ok((tok, line))
    }

    fn peek(&mut self) -> Result<&Tok<'a>, Fail> {
        if self.peeked.is_none() {
            self.peeked = Some(self.lex()?);
        }
        Ok(&self.peeked.as_ref().expect("filled above").0)
    }

    fn next(&mut self) -> Result<(Tok<'a>, usize), Fail> {
        let (tok, line) = match self.peeked.take() {
            Some(t) => t,
            None => self.lex()?,
        };
        match tok {
            Tok::Open => self.depth += 1,
            Tok::Close => self.depth = self.depth.saturating_sub(1),
            _ => {}
        }
        Ok((tok, line))
    }
}

// --------------------------------------------------------------------------- //
// Parser and translation
// --------------------------------------------------------------------------- //

/// An annotation, parsed before the thing it annotates is known.
struct Ann {
    property: NamedNode,
    value: Term,
    annotations: Vec<Ann>,
}

/// One element of a `DataSomeValuesFrom` / `DataAllValuesFrom` argument list,
/// which cannot be classified until the list ends: the LAST element is the
/// data range and every one before it a data property.
enum DataArg {
    Iri(NamedNode),
    Range(NamedOrBlankNode, usize),
}

struct Parser<'a> {
    lx: Lexer<'a>,
    prefixes: HashMap<&'a str, String>,
    full_iris: HashMap<&'a str, NamedNode>,
    abbreviated_iris: HashMap<&'a str, NamedNode>,
    anonymous: HashMap<&'a str, BlankNode>,
    swrl_variables: HashSet<NamedNode>,
    out: Vec<Triple>,
    refusals: Vec<(String, usize)>,
}

fn is_iri_token(tok: &Tok<'_>) -> bool {
    match tok {
        Tok::Iri(_) => true,
        Tok::Word(w) => w.contains(':'),
        _ => false,
    }
}

impl<'a> Parser<'a> {
    fn new(src: &'a str) -> Self {
        let prefixes = STANDARD_PREFIXES
            .iter()
            .map(|(p, iri)| (*p, (*iri).to_string()))
            .collect();
        Self {
            lx: Lexer::new(src),
            prefixes,
            full_iris: HashMap::new(),
            abbreviated_iris: HashMap::new(),
            anonymous: HashMap::new(),
            swrl_variables: HashSet::new(),
            out: Vec::new(),
            refusals: Vec::new(),
        }
    }

    // ---- token helpers -------------------------------------------------- //

    fn expect_open(&mut self, context: &str) -> Result<(), Fail> {
        match self.lx.next()? {
            (Tok::Open, _) => Ok(()),
            (tok, line) => syntax(line, format!("expected `(` after `{context}`, found {tok}")),
        }
    }

    fn expect_close(&mut self, context: &str) -> Result<(), Fail> {
        match self.lx.next()? {
            (Tok::Close, _) => Ok(()),
            (tok, line) => syntax(
                line,
                format!("expected `)` closing `{context}`, found {tok}"),
            ),
        }
    }

    fn at_close(&mut self) -> Result<bool, Fail> {
        Ok(matches!(self.lx.peek()?, Tok::Close))
    }

    fn peek_keyword(&mut self, kw: &str) -> Result<bool, Fail> {
        Ok(matches!(self.lx.peek()?, Tok::Word(w) if *w == kw))
    }

    /// The failure for a keyword that cannot stand where it was found: a
    /// syntax error if the grammar knows it elsewhere, a refusal if not.
    fn misplaced(&self, kw: &str, line: usize, expected: &str) -> Fail {
        if KNOWN_KEYWORDS.contains(&kw) {
            Fail::Syntax {
                line,
                message: format!("`{kw}` cannot stand where {expected} is expected"),
            }
        } else {
            Fail::Refuse {
                construct: kw.to_string(),
                line,
            }
        }
    }

    // ---- terminals ------------------------------------------------------ //

    fn iri_from(&mut self, tok: Tok<'a>, line: usize, what: &str) -> Result<NamedNode, Fail> {
        match tok {
            Tok::Iri(s) => {
                if let Some(n) = self.full_iris.get(s) {
                    return Ok(n.clone());
                }
                let node = NamedNode::new(s).or_else(|e| {
                    syntax(line, format!("`<{s}>` is not a valid absolute IRI: {e}"))
                })?;
                self.full_iris.insert(s, node.clone());
                Ok(node)
            }
            Tok::Word(w) if w.contains(':') => {
                if let Some(n) = self.abbreviated_iris.get(w) {
                    return Ok(n.clone());
                }
                let (prefix, local) = w.split_once(':').expect("contains ':'");
                let Some(ns) = self.prefixes.get(prefix) else {
                    return syntax(line, format!("prefix `{prefix}:` in `{w}` was never declared"));
                };
                let full = format!("{ns}{local}");
                let node = NamedNode::new(full.as_str()).or_else(|e| {
                    syntax(
                        line,
                        format!("`{w}` expands to `{full}`, which is not a valid IRI: {e}"),
                    )
                })?;
                self.abbreviated_iris.insert(w, node.clone());
                Ok(node)
            }
            Tok::Word(w) if !w.is_empty() && !w.chars().all(|c| c.is_ascii_digit()) => {
                // A bare keyword followed by `(` where an IRI belongs.
                if matches!(self.lx.peek()?, Tok::Open) {
                    self.lx.next()?;
                    Err(self.misplaced(w, line, &format!("{what} IRI")))
                } else {
                    syntax(line, format!("expected {what} IRI, found `{w}`"))
                }
            }
            other => syntax(line, format!("expected {what} IRI, found {other}")),
        }
    }

    fn iri(&mut self, what: &str) -> Result<NamedNode, Fail> {
        let (tok, line) = self.lx.next()?;
        self.iri_from(tok, line, what)
    }

    fn anonymous(&mut self, label: &'a str) -> BlankNode {
        self.anonymous.entry(label).or_default().clone()
    }

    fn individual(&mut self) -> Result<NamedOrBlankNode, Fail> {
        let (tok, line) = self.lx.next()?;
        match tok {
            Tok::Blank(label) => Ok(self.anonymous(label).into()),
            other => Ok(self.iri_from(other, line, "individual")?.into()),
        }
    }

    fn literal(&mut self) -> Result<Literal, Fail> {
        let (tok, line) = self.lx.next()?;
        let Tok::Str(value) = tok else {
            return syntax(line, format!("expected a literal, found {tok}"));
        };
        self.literal_suffix(value, line)
    }

    fn literal_suffix(&mut self, value: String, line: usize) -> Result<Literal, Fail> {
        match self.lx.peek()? {
            Tok::Lang(_) => {
                let (Tok::Lang(tag), _) = self.lx.next()? else {
                    unreachable!("peeked a language tag")
                };
                Literal::new_language_tagged_literal(value, tag)
                    .or_else(|e| syntax(line, format!("`@{tag}` is not a language tag: {e}")))
            }
            Tok::Hats => {
                self.lx.next()?;
                let datatype = self.iri("datatype")?;
                if datatype.as_str() == RDF_PLAIN_LITERAL {
                    // Table 1's first two rows: an rdf:PlainLiteral is the
                    // plain RDF literal it spells, with the language tag after
                    // the LAST `@` and an empty tag meaning none.
                    let Some(at) = value.rfind('@') else {
                        return syntax(
                            line,
                            "an rdf:PlainLiteral lexical form must end in `@` or `@tag`",
                        );
                    };
                    let (text, tag) = (&value[..at], &value[at + 1..]);
                    if tag.is_empty() {
                        return Ok(Literal::new_simple_literal(text));
                    }
                    return Literal::new_language_tagged_literal(text, tag)
                        .or_else(|e| syntax(line, format!("`@{tag}` is not a language tag: {e}")));
                }
                Ok(Literal::new_typed_literal(value, datatype))
            }
            _ => Ok(Literal::new_simple_literal(value)),
        }
    }

    fn cardinality(&mut self) -> Result<Literal, Fail> {
        let (tok, line) = self.lx.next()?;
        match tok {
            Tok::Word(w) if !w.is_empty() && w.chars().all(|c| c.is_ascii_digit()) => {
                let n: u64 = w
                    .parse()
                    .or_else(|_| syntax(line, format!("cardinality `{w}` is out of range")))?;
                Ok(Literal::new_typed_literal(
                    n.to_string(),
                    xsd::NON_NEGATIVE_INTEGER.into_owned(),
                ))
            }
            other => syntax(
                line,
                format!("expected a non-negative integer cardinality, found {other}"),
            ),
        }
    }

    // ---- emission ------------------------------------------------------- //

    fn push(&mut self, s: impl Into<NamedOrBlankNode>, p: NamedNode, o: impl Into<Term>) {
        self.out.push(Triple::new(s, p, o));
    }

    /// `T(SEQ ...)`: an `rdf:List`, or `rdf:nil` when empty.
    fn list(&mut self, items: Vec<Term>) -> NamedOrBlankNode {
        let mut tail: NamedOrBlankNode = rdf::NIL.into_owned().into();
        for item in items.into_iter().rev() {
            let cell = BlankNode::default();
            self.push(cell.clone(), rdf::FIRST.into_owned(), item);
            self.push(cell.clone(), rdf::REST.into_owned(), tail);
            tail = cell.into();
        }
        tail
    }

    /// `TANN(ann, subject)` from Table 2.
    fn tann(&mut self, subject: NamedOrBlankNode, ann: &Ann) {
        self.push(subject.clone(), ann.property.clone(), ann.value.clone());
        if !ann.annotations.is_empty() {
            let x = BlankNode::default();
            self.push(x.clone(), rdf::TYPE.into_owned(), owl!("Annotation"));
            self.push(x.clone(), owl!("annotatedSource"), subject);
            self.push(x.clone(), owl!("annotatedProperty"), ann.property.clone());
            self.push(x.clone(), owl!("annotatedTarget"), ann.value.clone());
            for inner in &ann.annotations {
                self.tann(x.clone().into(), inner);
            }
        }
    }

    /// An axiom with a main triple (Section 2.3.1): the triple, and when the
    /// axiom is annotated, an `owl:Axiom` node reifying it that carries the
    /// annotations.
    fn main_triple(&mut self, s: NamedOrBlankNode, p: NamedNode, o: Term, anns: &[Ann]) {
        self.push(s.clone(), p.clone(), o.clone());
        if !anns.is_empty() {
            let x = BlankNode::default();
            self.push(x.clone(), rdf::TYPE.into_owned(), owl!("Axiom"));
            self.push(x.clone(), owl!("annotatedSource"), s);
            self.push(x.clone(), owl!("annotatedProperty"), p);
            self.push(x.clone(), owl!("annotatedTarget"), o);
            for a in anns {
                self.tann(x.clone().into(), a);
            }
        }
    }

    /// Section 2.3.2: an n-ary axiom becomes the chain of pairwise triples
    /// Table 1 gives, and every link of the chain is annotated separately.
    fn chain(&mut self, items: Vec<NamedOrBlankNode>, p: NamedNode, anns: &[Ann]) {
        for pair in items.windows(2) {
            self.main_triple(pair[0].clone(), p.clone(), pair[1].clone().into(), anns);
        }
    }

    /// Section 2.3.3: an axiom whose translation is rooted at a blank node
    /// carries its annotations on that node directly.
    fn annotate_node(&mut self, x: &BlankNode, anns: &[Ann]) {
        for a in anns {
            self.tann(x.clone().into(), a);
        }
    }

    /// The `n > 2` row of `DisjointClasses` and its kin: a typed blank node
    /// with `owl:members`.
    fn members_node(&mut self, class: NamedNode, items: Vec<NamedOrBlankNode>, anns: &[Ann]) {
        let x = BlankNode::default();
        self.push(x.clone(), rdf::TYPE.into_owned(), class);
        let list = self.list(items.into_iter().map(Into::into).collect());
        self.push(x.clone(), owl!("members"), list);
        self.annotate_node(&x, anns);
    }

    // ---- annotations ---------------------------------------------------- //

    fn annotation_value(&mut self) -> Result<Term, Fail> {
        let (tok, line) = self.lx.next()?;
        match tok {
            Tok::Str(v) => Ok(self.literal_suffix(v, line)?.into()),
            Tok::Blank(label) => Ok(self.anonymous(label).into()),
            other => Ok(self.iri_from(other, line, "annotation value")?.into()),
        }
    }

    /// The body of `Annotation( ... )`, the keyword already consumed.
    fn annotation_body(&mut self) -> Result<Ann, Fail> {
        self.expect_open("Annotation")?;
        let annotations = self.annotations()?;
        let property = self.iri("annotation property")?;
        let value = self.annotation_value()?;
        self.expect_close("Annotation")?;
        Ok(Ann {
            property,
            value,
            annotations,
        })
    }

    fn annotations(&mut self) -> Result<Vec<Ann>, Fail> {
        let mut anns = Vec::new();
        while self.peek_keyword("Annotation")? {
            self.lx.next()?;
            anns.push(self.annotation_body()?);
        }
        Ok(anns)
    }

    // ---- expressions ---------------------------------------------------- //

    fn class_expr(&mut self) -> Result<NamedOrBlankNode, Fail> {
        let (tok, line) = self.lx.next()?;
        match tok {
            Tok::Word(w) if !w.contains(':') && matches!(self.lx.peek()?, Tok::Open) => {
                self.class_constructor(w, line)
            }
            other => Ok(self.iri_from(other, line, "class")?.into()),
        }
    }

    fn class_exprs_until_close(&mut self) -> Result<Vec<NamedOrBlankNode>, Fail> {
        let mut items = Vec::new();
        while !self.at_close()? {
            items.push(self.class_expr()?);
        }
        Ok(items)
    }

    fn at_least<T>(items: &[T], n: usize, kw: &str, line: usize) -> Result<(), Fail> {
        if items.len() < n {
            return syntax(
                line,
                format!("`{kw}` needs at least {n} operand(s), found {}", items.len()),
            );
        }
        Ok(())
    }

    fn restriction(&mut self, property: NamedOrBlankNode) -> BlankNode {
        let x = BlankNode::default();
        self.push(x.clone(), rdf::TYPE.into_owned(), owl!("Restriction"));
        self.push(x.clone(), owl!("onProperty"), property);
        x
    }

    fn class_constructor(&mut self, kw: &'a str, line: usize) -> Result<NamedOrBlankNode, Fail> {
        self.expect_open(kw)?;
        let x = match kw {
            "ObjectIntersectionOf" | "ObjectUnionOf" => {
                let items = self.class_exprs_until_close()?;
                Self::at_least(&items, 2, kw, line)?;
                let x = BlankNode::default();
                self.push(x.clone(), rdf::TYPE.into_owned(), owl!("Class"));
                let list = self.list(items.into_iter().map(Into::into).collect());
                let p = if kw == "ObjectIntersectionOf" {
                    owl!("intersectionOf")
                } else {
                    owl!("unionOf")
                };
                self.push(x.clone(), p, list);
                x
            }
            "ObjectComplementOf" => {
                let ce = self.class_expr()?;
                let x = BlankNode::default();
                self.push(x.clone(), rdf::TYPE.into_owned(), owl!("Class"));
                self.push(x.clone(), owl!("complementOf"), ce);
                x
            }
            "ObjectOneOf" => {
                let mut items: Vec<Term> = Vec::new();
                while !self.at_close()? {
                    items.push(self.individual()?.into());
                }
                Self::at_least(&items, 1, kw, line)?;
                let x = BlankNode::default();
                self.push(x.clone(), rdf::TYPE.into_owned(), owl!("Class"));
                let list = self.list(items);
                self.push(x.clone(), owl!("oneOf"), list);
                x
            }
            "ObjectSomeValuesFrom" | "ObjectAllValuesFrom" => {
                let ope = self.object_property_expr()?;
                let ce = self.class_expr()?;
                let x = self.restriction(ope);
                let p = if kw == "ObjectSomeValuesFrom" {
                    owl!("someValuesFrom")
                } else {
                    owl!("allValuesFrom")
                };
                self.push(x.clone(), p, ce);
                x
            }
            "ObjectHasValue" => {
                let ope = self.object_property_expr()?;
                let a = self.individual()?;
                let x = self.restriction(ope);
                self.push(x.clone(), owl!("hasValue"), a);
                x
            }
            "ObjectHasSelf" => {
                let ope = self.object_property_expr()?;
                let x = self.restriction(ope);
                self.push(
                    x.clone(),
                    owl!("hasSelf"),
                    Literal::new_typed_literal("true", xsd::BOOLEAN.into_owned()),
                );
                x
            }
            "ObjectMinCardinality" | "ObjectMaxCardinality" | "ObjectExactCardinality" => {
                let n = self.cardinality()?;
                let ope = self.object_property_expr()?;
                let filler = if self.at_close()? {
                    None
                } else {
                    Some(self.class_expr()?)
                };
                let x = self.restriction(ope);
                self.cardinality_triples(&x, kw, n, filler, owl!("onClass"));
                x
            }
            "DataSomeValuesFrom" | "DataAllValuesFrom" => {
                let mut args = Vec::new();
                while !self.at_close()? {
                    let (tok, l) = self.lx.next()?;
                    match tok {
                        Tok::Word(w) if !w.contains(':') => {
                            args.push(DataArg::Range(self.data_range_constructor(w, l)?, l));
                        }
                        other => args.push(DataArg::Iri(self.iri_from(other, l, "data")?)),
                    }
                }
                Self::at_least(&args, 2, kw, line)?;
                let range = match args.pop().expect("at least two") {
                    DataArg::Iri(n) => NamedOrBlankNode::from(n),
                    DataArg::Range(r, _) => r,
                };
                let mut props: Vec<NamedNode> = Vec::new();
                for a in args {
                    match a {
                        DataArg::Iri(n) => props.push(n),
                        DataArg::Range(_, l) => {
                            return syntax(
                                l,
                                format!("`{kw}` takes data properties before its data range, not a second data range"),
                            );
                        }
                    }
                }
                let x = BlankNode::default();
                self.push(x.clone(), rdf::TYPE.into_owned(), owl!("Restriction"));
                if props.len() == 1 {
                    let p = props.pop().expect("one");
                    self.push(x.clone(), owl!("onProperty"), p);
                } else {
                    let list = self.list(props.into_iter().map(Into::into).collect());
                    self.push(x.clone(), owl!("onProperties"), list);
                }
                let p = if kw == "DataSomeValuesFrom" {
                    owl!("someValuesFrom")
                } else {
                    owl!("allValuesFrom")
                };
                self.push(x.clone(), p, range);
                x
            }
            "DataHasValue" => {
                let dp = self.iri("data property")?;
                let lt = self.literal()?;
                let x = self.restriction(dp.into());
                self.push(x.clone(), owl!("hasValue"), lt);
                x
            }
            "DataMinCardinality" | "DataMaxCardinality" | "DataExactCardinality" => {
                let n = self.cardinality()?;
                let dp = self.iri("data property")?;
                let filler = if self.at_close()? {
                    None
                } else {
                    Some(self.data_range()?)
                };
                let x = self.restriction(dp.into());
                self.cardinality_triples(&x, kw, n, filler, owl!("onDataRange"));
                x
            }
            other => return Err(self.misplaced(other, line, "a class expression")),
        };
        self.expect_close(kw)?;
        Ok(x.into())
    }

    /// Table 1's six cardinality rows per family: unqualified when the source
    /// gave no filler, qualified with `owl:onClass` / `owl:onDataRange` when
    /// it gave one, `owl:Thing` included (see the module notes, item 1).
    fn cardinality_triples(
        &mut self,
        x: &BlankNode,
        kw: &str,
        n: Literal,
        filler: Option<NamedOrBlankNode>,
        on: NamedNode,
    ) {
        let qualified = filler.is_some();
        let p = match (kw.trim_start_matches("Object").trim_start_matches("Data"), qualified) {
            ("MinCardinality", false) => owl!("minCardinality"),
            ("MinCardinality", true) => owl!("minQualifiedCardinality"),
            ("MaxCardinality", false) => owl!("maxCardinality"),
            ("MaxCardinality", true) => owl!("maxQualifiedCardinality"),
            (_, false) => owl!("cardinality"),
            (_, true) => owl!("qualifiedCardinality"),
        };
        self.push(x.clone(), p, n);
        if let Some(f) = filler {
            self.push(x.clone(), on, f);
        }
    }

    fn object_property_expr(&mut self) -> Result<NamedOrBlankNode, Fail> {
        let (tok, line) = self.lx.next()?;
        match tok {
            Tok::Word("ObjectInverseOf") => {
                let p = self.inverse_body()?;
                let x = BlankNode::default();
                self.push(x.clone(), owl!("inverseOf"), p);
                Ok(x.into())
            }
            Tok::Word(w) if !w.contains(':') && matches!(self.lx.peek()?, Tok::Open) => {
                self.lx.next()?;
                Err(self.misplaced(w, line, "an object property expression"))
            }
            other => Ok(self.iri_from(other, line, "object property")?.into()),
        }
    }

    /// `ObjectInverseOf( P )` with the keyword consumed. The grammar admits a
    /// named property only, so `ObjectInverseOf(ObjectInverseOf(P))` is a
    /// syntax error here as it is in the specification.
    fn inverse_body(&mut self) -> Result<NamedNode, Fail> {
        self.expect_open("ObjectInverseOf")?;
        let p = self.iri("object property")?;
        self.expect_close("ObjectInverseOf")?;
        Ok(p)
    }

    fn data_range(&mut self) -> Result<NamedOrBlankNode, Fail> {
        let (tok, line) = self.lx.next()?;
        match tok {
            Tok::Word(w) if !w.contains(':') && matches!(self.lx.peek()?, Tok::Open) => {
                self.data_range_constructor(w, line)
            }
            other => Ok(self.iri_from(other, line, "datatype")?.into()),
        }
    }

    fn data_range_constructor(&mut self, kw: &'a str, line: usize) -> Result<NamedOrBlankNode, Fail> {
        self.expect_open(kw)?;
        let x = BlankNode::default();
        match kw {
            "DataIntersectionOf" | "DataUnionOf" => {
                let mut items: Vec<Term> = Vec::new();
                while !self.at_close()? {
                    items.push(self.data_range()?.into());
                }
                Self::at_least(&items, 2, kw, line)?;
                self.push(x.clone(), rdf::TYPE.into_owned(), rdfs::DATATYPE.into_owned());
                let list = self.list(items);
                let p = if kw == "DataIntersectionOf" {
                    owl!("intersectionOf")
                } else {
                    owl!("unionOf")
                };
                self.push(x.clone(), p, list);
            }
            "DataComplementOf" => {
                let dr = self.data_range()?;
                self.push(x.clone(), rdf::TYPE.into_owned(), rdfs::DATATYPE.into_owned());
                self.push(x.clone(), owl!("datatypeComplementOf"), dr);
            }
            "DataOneOf" => {
                let mut items: Vec<Term> = Vec::new();
                while !self.at_close()? {
                    items.push(self.literal()?.into());
                }
                Self::at_least(&items, 1, kw, line)?;
                self.push(x.clone(), rdf::TYPE.into_owned(), rdfs::DATATYPE.into_owned());
                let list = self.list(items);
                self.push(x.clone(), owl!("oneOf"), list);
            }
            "DatatypeRestriction" => {
                let dt = self.iri("datatype")?;
                let mut facets: Vec<Term> = Vec::new();
                while !self.at_close()? {
                    let facet = self.iri("constraining facet")?;
                    let lt = self.literal()?;
                    let y = BlankNode::default();
                    self.push(y.clone(), facet, lt);
                    facets.push(y.into());
                }
                Self::at_least(&facets, 1, kw, line)?;
                self.push(x.clone(), rdf::TYPE.into_owned(), rdfs::DATATYPE.into_owned());
                self.push(x.clone(), owl!("onDatatype"), dt);
                let list = self.list(facets);
                self.push(x.clone(), owl!("withRestrictions"), list);
            }
            other => return Err(self.misplaced(other, line, "a data range")),
        }
        self.expect_close(kw)?;
        Ok(x.into())
    }

    fn iris_until_close(&mut self, what: &str) -> Result<Vec<NamedOrBlankNode>, Fail> {
        let mut items = Vec::new();
        while !self.at_close()? {
            items.push(self.iri(what)?.into());
        }
        Ok(items)
    }

    fn object_property_exprs_until_close(&mut self) -> Result<Vec<NamedOrBlankNode>, Fail> {
        let mut items = Vec::new();
        while !self.at_close()? {
            items.push(self.object_property_expr()?);
        }
        Ok(items)
    }

    // ---- axioms --------------------------------------------------------- //

    fn axiom(&mut self, kw: &'a str, line: usize) -> Result<(), Fail> {
        self.expect_open(kw)?;
        let anns = self.annotations()?;
        let ty = rdf::TYPE.into_owned();
        match kw {
            "Declaration" => {
                let (tok, l) = self.lx.next()?;
                let class = match tok {
                    Tok::Word("Class") => owl!("Class"),
                    Tok::Word("Datatype") => rdfs::DATATYPE.into_owned(),
                    Tok::Word("ObjectProperty") => owl!("ObjectProperty"),
                    Tok::Word("DataProperty") => owl!("DatatypeProperty"),
                    Tok::Word("AnnotationProperty") => owl!("AnnotationProperty"),
                    Tok::Word("NamedIndividual") => owl!("NamedIndividual"),
                    Tok::Word(w) if !w.contains(':') => {
                        self.expect_open(w)?;
                        return Err(self.misplaced(w, l, "an entity kind"));
                    }
                    other => return syntax(l, format!("expected an entity kind, found {other}")),
                };
                self.expect_open("entity")?;
                let iri = self.iri("entity")?;
                self.expect_close("entity")?;
                self.main_triple(iri.into(), ty, class.into(), &anns);
            }
            "SubClassOf" => {
                let sub = self.class_expr()?;
                let sup = self.class_expr()?;
                self.main_triple(sub, rdfs::SUB_CLASS_OF.into_owned(), sup.into(), &anns);
            }
            "EquivalentClasses" => {
                let items = self.class_exprs_until_close()?;
                Self::at_least(&items, 2, kw, line)?;
                self.chain(items, owl!("equivalentClass"), &anns);
            }
            "DisjointClasses" => {
                let items = self.class_exprs_until_close()?;
                Self::at_least(&items, 2, kw, line)?;
                if items.len() == 2 {
                    self.chain(items, owl!("disjointWith"), &anns);
                } else {
                    self.members_node(owl!("AllDisjointClasses"), items, &anns);
                }
            }
            "DisjointUnion" => {
                let c = self.iri("class")?;
                let items = self.class_exprs_until_close()?;
                Self::at_least(&items, 2, kw, line)?;
                let list = self.list(items.into_iter().map(Into::into).collect());
                self.main_triple(c.into(), owl!("disjointUnionOf"), list.into(), &anns);
            }
            "SubObjectPropertyOf" => {
                if self.peek_keyword("ObjectPropertyChain")? {
                    let (_, l) = self.lx.next()?;
                    self.expect_open("ObjectPropertyChain")?;
                    let chain = self.object_property_exprs_until_close()?;
                    Self::at_least(&chain, 2, "ObjectPropertyChain", l)?;
                    self.expect_close("ObjectPropertyChain")?;
                    let sup = self.object_property_expr()?;
                    let list = self.list(chain.into_iter().map(Into::into).collect());
                    self.main_triple(sup, owl!("propertyChainAxiom"), list.into(), &anns);
                } else {
                    let sub = self.object_property_expr()?;
                    let sup = self.object_property_expr()?;
                    self.main_triple(sub, rdfs::SUB_PROPERTY_OF.into_owned(), sup.into(), &anns);
                }
            }
            "EquivalentObjectProperties" => {
                let items = self.object_property_exprs_until_close()?;
                Self::at_least(&items, 2, kw, line)?;
                self.chain(items, owl!("equivalentProperty"), &anns);
            }
            "DisjointObjectProperties" => {
                let items = self.object_property_exprs_until_close()?;
                Self::at_least(&items, 2, kw, line)?;
                if items.len() == 2 {
                    self.chain(items, owl!("propertyDisjointWith"), &anns);
                } else {
                    self.members_node(owl!("AllDisjointProperties"), items, &anns);
                }
            }
            "ObjectPropertyDomain" | "ObjectPropertyRange" => {
                let ope = self.object_property_expr()?;
                let ce = self.class_expr()?;
                let p = if kw == "ObjectPropertyDomain" {
                    rdfs::DOMAIN.into_owned()
                } else {
                    rdfs::RANGE.into_owned()
                };
                self.main_triple(ope, p, ce.into(), &anns);
            }
            "InverseObjectProperties" => {
                let a = self.object_property_expr()?;
                let b = self.object_property_expr()?;
                self.main_triple(a, owl!("inverseOf"), b.into(), &anns);
            }
            "FunctionalObjectProperty"
            | "InverseFunctionalObjectProperty"
            | "ReflexiveObjectProperty"
            | "IrreflexiveObjectProperty"
            | "SymmetricObjectProperty"
            | "AsymmetricObjectProperty"
            | "TransitiveObjectProperty" => {
                let ope = self.object_property_expr()?;
                let class = match kw {
                    "FunctionalObjectProperty" => owl!("FunctionalProperty"),
                    "InverseFunctionalObjectProperty" => owl!("InverseFunctionalProperty"),
                    "ReflexiveObjectProperty" => owl!("ReflexiveProperty"),
                    "IrreflexiveObjectProperty" => owl!("IrreflexiveProperty"),
                    "SymmetricObjectProperty" => owl!("SymmetricProperty"),
                    "AsymmetricObjectProperty" => owl!("AsymmetricProperty"),
                    _ => owl!("TransitiveProperty"),
                };
                self.main_triple(ope, ty, class.into(), &anns);
            }
            "SubDataPropertyOf" => {
                let a = self.iri("data property")?;
                let b = self.iri("data property")?;
                self.main_triple(a.into(), rdfs::SUB_PROPERTY_OF.into_owned(), b.into(), &anns);
            }
            "EquivalentDataProperties" => {
                let items = self.iris_until_close("data property")?;
                Self::at_least(&items, 2, kw, line)?;
                self.chain(items, owl!("equivalentProperty"), &anns);
            }
            "DisjointDataProperties" => {
                let items = self.iris_until_close("data property")?;
                Self::at_least(&items, 2, kw, line)?;
                if items.len() == 2 {
                    self.chain(items, owl!("propertyDisjointWith"), &anns);
                } else {
                    self.members_node(owl!("AllDisjointProperties"), items, &anns);
                }
            }
            "DataPropertyDomain" => {
                let dp = self.iri("data property")?;
                let ce = self.class_expr()?;
                self.main_triple(dp.into(), rdfs::DOMAIN.into_owned(), ce.into(), &anns);
            }
            "DataPropertyRange" => {
                let dp = self.iri("data property")?;
                let dr = self.data_range()?;
                self.main_triple(dp.into(), rdfs::RANGE.into_owned(), dr.into(), &anns);
            }
            "FunctionalDataProperty" => {
                let dp = self.iri("data property")?;
                self.main_triple(dp.into(), ty, owl!("FunctionalProperty").into(), &anns);
            }
            "DatatypeDefinition" => {
                let dt = self.iri("datatype")?;
                let dr = self.data_range()?;
                self.main_triple(dt.into(), owl!("equivalentClass"), dr.into(), &anns);
            }
            "HasKey" => {
                let ce = self.class_expr()?;
                self.expect_open("HasKey object properties")?;
                let mut keys = self.object_property_exprs_until_close()?;
                self.expect_close("HasKey object properties")?;
                self.expect_open("HasKey data properties")?;
                keys.extend(self.iris_until_close("data property")?);
                self.expect_close("HasKey data properties")?;
                let list = self.list(keys.into_iter().map(Into::into).collect());
                self.main_triple(ce, owl!("hasKey"), list.into(), &anns);
            }
            "SameIndividual" => {
                let mut items = Vec::new();
                while !self.at_close()? {
                    items.push(self.individual()?);
                }
                Self::at_least(&items, 2, kw, line)?;
                self.chain(items, owl!("sameAs"), &anns);
            }
            "DifferentIndividuals" => {
                let mut items = Vec::new();
                while !self.at_close()? {
                    items.push(self.individual()?);
                }
                Self::at_least(&items, 2, kw, line)?;
                if items.len() == 2 {
                    self.chain(items, owl!("differentFrom"), &anns);
                } else {
                    self.members_node(owl!("AllDifferent"), items, &anns);
                }
            }
            "ClassAssertion" => {
                let ce = self.class_expr()?;
                let a = self.individual()?;
                self.main_triple(a, ty, ce.into(), &anns);
            }
            "ObjectPropertyAssertion" => {
                // Table 1 gives the inverse its own row: the assertion of
                // `ObjectInverseOf(P)` from a1 to a2 IS the triple a2 P a1,
                // because RDF has no predicate a blank node could stand in.
                let inverse = self.peek_keyword("ObjectInverseOf")?;
                if inverse {
                    self.lx.next()?;
                }
                let p = if inverse {
                    self.inverse_body()?
                } else {
                    self.iri("object property")?
                };
                let a1 = self.individual()?;
                let a2 = self.individual()?;
                let (s, o) = if inverse { (a2, a1) } else { (a1, a2) };
                self.main_triple(s, p, o.into(), &anns);
            }
            "NegativeObjectPropertyAssertion" => {
                let ope = self.object_property_expr()?;
                let a1 = self.individual()?;
                let a2 = self.individual()?;
                let x = BlankNode::default();
                self.push(x.clone(), ty, owl!("NegativePropertyAssertion"));
                self.push(x.clone(), owl!("sourceIndividual"), a1);
                self.push(x.clone(), owl!("assertionProperty"), ope);
                self.push(x.clone(), owl!("targetIndividual"), a2);
                self.annotate_node(&x, &anns);
            }
            "DataPropertyAssertion" => {
                let dp = self.iri("data property")?;
                let a = self.individual()?;
                let lt = self.literal()?;
                self.main_triple(a, dp, lt.into(), &anns);
            }
            "NegativeDataPropertyAssertion" => {
                let dp = self.iri("data property")?;
                let a = self.individual()?;
                let lt = self.literal()?;
                let x = BlankNode::default();
                self.push(x.clone(), ty, owl!("NegativePropertyAssertion"));
                self.push(x.clone(), owl!("sourceIndividual"), a);
                self.push(x.clone(), owl!("assertionProperty"), dp);
                self.push(x.clone(), owl!("targetValue"), lt);
                self.annotate_node(&x, &anns);
            }
            "AnnotationAssertion" => {
                let ap = self.iri("annotation property")?;
                let subject = self.individual()?;
                let value = self.annotation_value()?;
                self.main_triple(subject, ap, value, &anns);
            }
            "SubAnnotationPropertyOf" => {
                let a = self.iri("annotation property")?;
                let b = self.iri("annotation property")?;
                self.main_triple(a.into(), rdfs::SUB_PROPERTY_OF.into_owned(), b.into(), &anns);
            }
            "AnnotationPropertyDomain" | "AnnotationPropertyRange" => {
                let ap = self.iri("annotation property")?;
                let u = self.iri("domain or range")?;
                let p = if kw == "AnnotationPropertyDomain" {
                    rdfs::DOMAIN.into_owned()
                } else {
                    rdfs::RANGE.into_owned()
                };
                self.main_triple(ap.into(), p, u.into(), &anns);
            }
            "DLSafeRule" => self.swrl_rule(&anns)?,
            other => return Err(self.misplaced(other, line, "an axiom")),
        }
        self.expect_close(kw)
    }

    // ---- SWRL ----------------------------------------------------------- //

    /// `DLSafeRule( annotations Body(atoms) Head(atoms) )` as a `swrl:Imp`.
    /// The annotations sit on the rule node itself, as the OWL API writes
    /// them, because a rule has no main triple to reify.
    fn swrl_rule(&mut self, anns: &[Ann]) -> Result<(), Fail> {
        let rule = BlankNode::default();
        self.push(rule.clone(), rdf::TYPE.into_owned(), swrl!("Imp"));
        self.annotate_node(&rule, anns);
        for (part, p) in [("Body", swrl!("body")), ("Head", swrl!("head"))] {
            let (tok, l) = self.lx.next()?;
            if tok != Tok::Word(part) {
                return syntax(l, format!("expected `{part}(` in DLSafeRule, found {tok}"));
            }
            self.expect_open(part)?;
            let mut atoms: Vec<Term> = Vec::new();
            while !self.at_close()? {
                atoms.push(self.swrl_atom()?.into());
            }
            self.expect_close(part)?;
            let list = self.swrl_atom_list(atoms);
            self.push(rule.clone(), p, list);
        }
        Ok(())
    }

    /// An `rdf:List` of atoms whose cells are also typed `swrl:AtomList`, the
    /// class the SWRL submission defines for exactly these lists and the type
    /// the OWL API writes on every cell.
    fn swrl_atom_list(&mut self, atoms: Vec<Term>) -> NamedOrBlankNode {
        let mut tail: NamedOrBlankNode = rdf::NIL.into_owned().into();
        for item in atoms.into_iter().rev() {
            let cell = BlankNode::default();
            self.push(cell.clone(), rdf::TYPE.into_owned(), swrl!("AtomList"));
            self.push(cell.clone(), rdf::FIRST.into_owned(), item);
            self.push(cell.clone(), rdf::REST.into_owned(), tail);
            tail = cell.into();
        }
        tail
    }

    fn swrl_variable(&mut self) -> Result<NamedNode, Fail> {
        self.expect_open("Variable")?;
        let v = self.iri("variable")?;
        self.expect_close("Variable")?;
        if self.swrl_variables.insert(v.clone()) {
            self.push(v.clone(), rdf::TYPE.into_owned(), swrl!("Variable"));
        }
        Ok(v)
    }

    fn swrl_i_arg(&mut self) -> Result<NamedOrBlankNode, Fail> {
        if self.peek_keyword("Variable")? {
            self.lx.next()?;
            return Ok(self.swrl_variable()?.into());
        }
        self.individual()
    }

    fn swrl_d_arg(&mut self) -> Result<Term, Fail> {
        if self.peek_keyword("Variable")? {
            self.lx.next()?;
            return Ok(self.swrl_variable()?.into());
        }
        Ok(self.literal()?.into())
    }

    fn swrl_atom(&mut self) -> Result<BlankNode, Fail> {
        let (tok, line) = self.lx.next()?;
        let Tok::Word(kw) = tok else {
            return syntax(line, format!("expected a SWRL atom, found {tok}"));
        };
        self.expect_open(kw)?;
        let x = BlankNode::default();
        let ty = rdf::TYPE.into_owned();
        match kw {
            "ClassAtom" => {
                let ce = self.class_expr()?;
                let a = self.swrl_i_arg()?;
                self.push(x.clone(), ty, swrl!("ClassAtom"));
                self.push(x.clone(), swrl!("classPredicate"), ce);
                self.push(x.clone(), swrl!("argument1"), a);
            }
            "DataRangeAtom" => {
                let dr = self.data_range()?;
                let a = self.swrl_d_arg()?;
                self.push(x.clone(), ty, swrl!("DataRangeAtom"));
                self.push(x.clone(), swrl!("dataRange"), dr);
                self.push(x.clone(), swrl!("argument1"), a);
            }
            "ObjectPropertyAtom" => {
                let ope = self.object_property_expr()?;
                let a = self.swrl_i_arg()?;
                let b = self.swrl_i_arg()?;
                self.push(x.clone(), ty, swrl!("IndividualPropertyAtom"));
                self.push(x.clone(), swrl!("propertyPredicate"), ope);
                self.push(x.clone(), swrl!("argument1"), a);
                self.push(x.clone(), swrl!("argument2"), b);
            }
            "DataPropertyAtom" => {
                let dp = self.iri("data property")?;
                let a = self.swrl_i_arg()?;
                let b = self.swrl_d_arg()?;
                self.push(x.clone(), ty, swrl!("DatavaluedPropertyAtom"));
                self.push(x.clone(), swrl!("propertyPredicate"), dp);
                self.push(x.clone(), swrl!("argument1"), a);
                self.push(x.clone(), swrl!("argument2"), b);
            }
            "BuiltInAtom" => {
                let builtin = self.iri("built-in")?;
                let mut args: Vec<Term> = Vec::new();
                while !self.at_close()? {
                    args.push(self.swrl_d_arg()?);
                }
                self.push(x.clone(), ty, swrl!("BuiltinAtom"));
                self.push(x.clone(), swrl!("builtin"), builtin);
                let list = self.list(args);
                self.push(x.clone(), swrl!("arguments"), list);
            }
            "SameIndividualAtom" | "DifferentIndividualsAtom" => {
                let a = self.swrl_i_arg()?;
                let b = self.swrl_i_arg()?;
                let class = if kw == "SameIndividualAtom" {
                    swrl!("SameIndividualAtom")
                } else {
                    swrl!("DifferentIndividualsAtom")
                };
                self.push(x.clone(), ty, class);
                self.push(x.clone(), swrl!("argument1"), a);
                self.push(x.clone(), swrl!("argument2"), b);
            }
            other => return Err(self.misplaced(other, line, "a SWRL atom")),
        }
        self.expect_close(kw)?;
        Ok(x)
    }

    // ---- document ------------------------------------------------------- //

    fn document(&mut self) -> Result<(), Fail> {
        loop {
            let (tok, line) = self.lx.next()?;
            match tok {
                Tok::Word("Prefix") => self.prefix()?,
                Tok::Word("Ontology") => {
                    self.ontology()?;
                    break;
                }
                Tok::Eof => return syntax(line, "no `Ontology(` found"),
                other => {
                    return syntax(line, format!("expected `Prefix(` or `Ontology(`, found {other}"));
                }
            }
        }
        match self.lx.next()? {
            (Tok::Eof, _) => Ok(()),
            (tok, line) => syntax(
                line,
                format!("text after the closing `)` of `Ontology(`: {tok}"),
            ),
        }
    }

    fn prefix(&mut self) -> Result<(), Fail> {
        self.expect_open("Prefix")?;
        let (tok, line) = self.lx.next()?;
        let name = match tok {
            Tok::Word(w) if w.ends_with(':') && w.matches(':').count() == 1 => &w[..w.len() - 1],
            other => return syntax(line, format!("expected a prefix name such as `owl:`, found {other}")),
        };
        match self.lx.next()? {
            (Tok::Eq, _) => {}
            (tok, line) => return syntax(line, format!("expected `=` in Prefix, found {tok}")),
        }
        let (tok, line) = self.lx.next()?;
        let Tok::Iri(iri) = tok else {
            return syntax(line, format!("expected `<IRI>` in Prefix, found {tok}"));
        };
        self.prefixes.insert(name, iri.to_string());
        self.expect_close("Prefix")
    }

    fn ontology(&mut self) -> Result<(), Fail> {
        self.expect_open("Ontology")?;
        let subject: NamedOrBlankNode = if is_iri_token(self.lx.peek()?) {
            let iri = self.iri("ontology")?;
            if is_iri_token(self.lx.peek()?) {
                let version = self.iri("version")?;
                self.push(iri.clone(), owl!("versionIRI"), version);
            }
            iri.into()
        } else {
            BlankNode::default().into()
        };
        self.push(subject.clone(), rdf::TYPE.into_owned(), owl!("Ontology"));

        loop {
            let (tok, line) = self.lx.next()?;
            match tok {
                Tok::Close => return Ok(()),
                Tok::Word("Import") => {
                    self.expect_open("Import")?;
                    let iri = self.iri("imported ontology")?;
                    self.expect_close("Import")?;
                    self.push(subject.clone(), owl!("imports"), iri);
                }
                Tok::Word("Annotation") => {
                    let ann = self.annotation_body()?;
                    self.tann(subject.clone(), &ann);
                }
                Tok::Word(kw) if !kw.contains(':') => {
                    // The depth OUTSIDE this axiom. Whatever goes wrong inside
                    // it, a refusal is recovered from by reading until the
                    // parentheses close back to here.
                    let depth = self.lx.depth;
                    match self.axiom(kw, line) {
                        Ok(()) => {}
                        Err(Fail::Refuse { construct, line }) => {
                            self.refusals.push((construct, line));
                            while self.lx.depth > depth {
                                if let (Tok::Eof, l) = self.lx.next()? {
                                    return syntax(l, "document ended inside an axiom");
                                }
                            }
                        }
                        Err(e) => return Err(e),
                    }
                }
                Tok::Eof => return syntax(line, "document ended before `Ontology(` was closed"),
                other => return syntax(line, format!("expected an axiom, found {other}")),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sniffing_tells_ofn_from_turtle_and_manchester() {
        assert!(looks_like_ofn("Prefix(:=<http://e/>)\nOntology()"));
        assert!(looks_like_ofn("# c\n\n  Ontology (<http://e/>)"));
        assert!(!looks_like_ofn("PREFIX ex: <http://e/>\n"));
        assert!(!looks_like_ofn("Prefix: ex: <http://e/>\n"));
        assert!(!looks_like_ofn("@prefix ex: <http://e/> ."));
    }

    #[test]
    fn line_numbers_survive_comments_and_multiline_strings() {
        let doc = "Prefix(:=<http://e/>)\n# comment ( with parens\nOntology(\nAnnotationAssertion(rdfs:label :a \"two\nlines\")\nSubClassOf(:a)\n)";
        match parse(doc) {
            Err(OfnError::Syntax { line, .. }) => assert_eq!(line, 6),
            other => panic!("expected a syntax error, got {other:?}"),
        }
    }
}
