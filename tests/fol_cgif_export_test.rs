//! CGIF, the second of ISO/IEC 24707's three Common Logic dialects.
//!
//! This engine emitted CLIF only, which made its Common Logic support partial
//! in a way nothing said out loud. ISO/IEC 21838-1:2021 clause 4.3 asks for an
//! axiomatisation in a language conforming to ISO/IEC 24707, and its note names
//! CLIF, CGIF and XCL as the three that qualify. Emitting one of them and
//! calling it Common Logic support is the shape of overclaim this repository
//! exists to attack, in its own output. XCL is still absent and the report says
//! so in a field.
//!
//! # What conformance rests on here, stated first
//!
//! **There is no independent CGIF parser to check the output against.** No
//! package named `cgif` exists on PyPI; py-typedlogic, which reads the CLIF
//! output, has no CGIF front end; the Macleod toolchain reads CLIF. So the
//! conformance of the emitted text is **pinned by the checker in this file and
//! not by an independent implementation**, and every claim below is exactly as
//! good as that checker.
//!
//! That is a weaker footing than the CLIF side has, and the difference is
//! stated rather than blurred. `docs/first-order-export.md` records what two
//! external CLIF parsers do with the CLIF output, including that both return an
//! EMPTY theory from the wrapped comment shape. Nothing of that kind is
//! available here.
//!
//! What IS available is the normative text. ISO/IEC 24707:2018 is in ISO's
//! Publicly Available Standards list and was downloaded and read for this work
//! (sha256 e920b0c43a932e1ad0e2c76d3501f56e2b11ee5547265b14aeb69a5866eae5e3).
//! The checker below is transcribed from Annex B's EBNF, clause by clause, with
//! the clause cited at each rule. A transcription is a claim like any other;
//! what makes it worth something is that it is a SECOND reading of the grammar,
//! written against the emitter rather than with it, so a shape both agree on is
//! a shape two readings of Annex B agree on.
//!
//! Sowa's `jfsowa.com/cg/annexb.htm` is a PRE-PUBLICATION DRAFT of the same
//! annex and was not used: its `equiv` rewrite emits `~[` where the published
//! text emits `[`, which turns a conjunction of two implications into a
//! tautology, and its `nestedOrs` termination test drops the last disjunct.
//!
//! # The groups
//!
//!   1. A lexer and a parser for core CGIF, from Annex B.2.
//!   2. Syntax conformance: the emitted text parses, and it stays inside the
//!      compact first-order sub-dialect.
//!   3. Round trip: every sentence reads back to the `Form` it was built from,
//!      up to the two rewrites core CGIF forces.
//!   4. One translation, two Common Logic dialects: the CGIF and the CLIF file
//!      carry the same sentences, in the same order, under the same labels.
//!   5. What cannot be exported is named and counted, and what cannot be
//!      WRITTEN is refused rather than rewritten.

use open_ontologies::tptp::{
    cgif, clif, ClifComments, ClifDialect, Concept, FolProblem, Form, Ope, OwlAxiom, Syntax,
    Translation, P1, P2, Term,
};
use std::collections::{BTreeMap, BTreeSet, HashMap};

const TEXT_NAME: &str = "http://e/ontology";

fn c(s: &str) -> Concept {
    Concept::Atom(format!("http://e/{s}"))
}
fn iri(s: &str) -> String {
    format!("http://e/{s}")
}

/// A corpus that exercises every constructor of `Form` at least once. The same
/// one `tests/fol_translation_correspondence_test.rs` uses for CLIF, so the two
/// dialects are checked over the same axioms.
fn corpus() -> Vec<OwlAxiom> {
    vec![
        OwlAxiom::SubClass(c("A"), c("B")),
        OwlAxiom::SubClass(c("A"), Concept::Some_(iri("r"), Box::new(c("B")))),
        OwlAxiom::SubClass(c("A"), Concept::All_(iri("r"), Box::new(c("B")))),
        OwlAxiom::SubClass(c("A"), Concept::MinCard(2, iri("r"), Box::new(c("B")))),
        OwlAxiom::SubClass(c("A"), Concept::MaxCard(1, iri("r"), Box::new(Concept::Top))),
        OwlAxiom::SubClass(Concept::Bot, Concept::Compl(Box::new(c("B")))),
        OwlAxiom::SubClass(
            Concept::Inter(Box::new(c("A")), Box::new(c("B"))),
            Concept::Union(Box::new(c("A")), Box::new(c("B"))),
        ),
        OwlAxiom::SubClass(c("A"), Concept::OneOf(vec![iri("a"), iri("b")])),
        OwlAxiom::SubClass(c("A"), Concept::HasVal(iri("r"), iri("a"))),
        OwlAxiom::SubClass(c("A"), Concept::HasSelf(iri("r"))),
        OwlAxiom::SubClass(c("A"), Concept::DataSome(iri("p"), iri("D"))),
        OwlAxiom::SubClass(c("A"), Concept::DataAll(iri("p"), iri("D"))),
        OwlAxiom::EquivClass(c("A"), c("B")),
        OwlAxiom::DisjointWith(c("A"), c("B")),
        OwlAxiom::SubOProp(Ope::Named(iri("r")), Ope::Inv(iri("s"))),
        OwlAxiom::OPropDomain(Ope::Inv(iri("r")), c("A")),
        OwlAxiom::OPropRange(Ope::Named(iri("r")), c("A")),
        OwlAxiom::DPropDomain(iri("p"), c("A")),
        OwlAxiom::DPropRange(iri("p"), iri("D")),
        OwlAxiom::Transitive(iri("r")),
        OwlAxiom::Symmetric(iri("r")),
        OwlAxiom::Asymmetric(iri("r")),
        OwlAxiom::Reflexive(iri("r")),
        OwlAxiom::Irreflexive(iri("r")),
        OwlAxiom::Functional(iri("r")),
        OwlAxiom::InvFunctional(iri("r")),
        OwlAxiom::InverseOf(iri("r"), iri("s")),
        OwlAxiom::PropDisjoint(iri("r"), iri("s")),
        OwlAxiom::Chain(vec![iri("r"), iri("s")], iri("t")),
        OwlAxiom::ClassAssert(c("A"), iri("a")),
        OwlAxiom::OPropAssert(iri("r"), iri("a"), iri("b")),
        OwlAxiom::SameAs(iri("a"), iri("b")),
        OwlAxiom::DifferentFrom(iri("a"), iri("b")),
    ]
}

// ── 1. A lexer and parser for core CGIF, from Annex B.2 ─────────────────────

#[derive(Clone, Debug, PartialEq, Eq)]
enum Tok {
    Open,
    Close,
    LParen,
    RParen,
    Tilde,
    Star,
    Query,
    Colon,
    Bar,
    Hash,
    At,
    /// `/* … */`, with the delimiters stripped. B.2.4.
    Comment(String),
    /// A bare token: an `identifier`, a `numeral`, or a `seqmark`. Which one it
    /// is, is the checker's business and not the lexer's.
    Bare(String),
    /// `"…"`, a B.1.1 `enclosedname`, with the quotes stripped and escapes
    /// undone.
    Enclosed(String),
    /// `'…'`, a CLIF `quotedstring`. An INTERPRETED name, which is why the
    /// checker refuses one.
    Quoted(String),
}

/// Lex core CGIF. Everything the grammar can produce gets a token, including
/// the constructs the emitter must never produce, because a checker that could
/// not LEX an actor could not refuse one either.
fn lex(src: &str) -> Vec<Tok> {
    let mut out = Vec::new();
    let mut chars = src.chars().peekable();
    while let Some(&ch) = chars.peek() {
        match ch {
            c if c.is_whitespace() => {
                chars.next();
            }
            '[' => {
                chars.next();
                out.push(Tok::Open);
            }
            ']' => {
                chars.next();
                out.push(Tok::Close);
            }
            '(' => {
                chars.next();
                out.push(Tok::LParen);
            }
            ')' => {
                chars.next();
                out.push(Tok::RParen);
            }
            '~' => {
                chars.next();
                out.push(Tok::Tilde);
            }
            '*' => {
                chars.next();
                out.push(Tok::Star);
            }
            '?' => {
                chars.next();
                out.push(Tok::Query);
            }
            ':' => {
                chars.next();
                out.push(Tok::Colon);
            }
            '|' => {
                chars.next();
                out.push(Tok::Bar);
            }
            '#' => {
                chars.next();
                out.push(Tok::Hash);
            }
            '@' => {
                chars.next();
                out.push(Tok::At);
            }
            '/' => {
                chars.next();
                assert_eq!(chars.next(), Some('*'), "a bare / is not a CGIF token");
                let mut body = String::new();
                loop {
                    let c = chars.next().expect("unterminated /* comment");
                    if c == '*' && chars.peek() == Some(&'/') {
                        chars.next();
                        break;
                    }
                    body.push(c);
                }
                out.push(Tok::Comment(body));
            }
            '"' | '\'' => {
                let delim = ch;
                chars.next();
                let mut body = String::new();
                loop {
                    match chars.next().expect("unterminated name or string") {
                        '\\' => body.push(chars.next().expect("escape at end of input")),
                        c if c == delim => break,
                        c => body.push(c),
                    }
                }
                out.push(if delim == '"' { Tok::Enclosed(body) } else { Tok::Quoted(body) });
            }
            _ => {
                let mut name = String::new();
                while let Some(&n) = chars.peek() {
                    if n.is_whitespace()
                        || matches!(n, '[' | ']' | '(' | ')' | '~' | '*' | '?' | ':' | '|' | '"' | '\'' | '#' | '@' | '/')
                    {
                        break;
                    }
                    name.push(n);
                    chars.next();
                }
                assert!(!name.is_empty(), "lexer stuck at {ch:?}");
                out.push(Tok::Bare(name));
            }
        }
    }
    out
}

/// A CG name. B.1.1: `CGname = identifier | '"', (namesequence - identifier),
/// '"' | numeral | enclosedname | quotedstring`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Name {
    Bare(String),
    Enclosed(String),
    /// A `quotedstring`. Legal CGIF and an interpreted name, so the checker
    /// refuses one in this text.
    Quoted(String),
}

/// B.2.9: `reference = ["?"], CGname`. `bound` is the `?`.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Ref {
    bound: bool,
    name: Name,
}

/// The core CGIF node kinds, B.2.5 to B.2.8.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Node {
    Comment(String),
    /// B.2.7 `ordinaryRelation`. `bound_type_label` records a `#?` prefix, the
    /// construct that quantifies into a predicate position.
    Relation { bound_type_label: bool, name: Name, args: Vec<Ref>, seqmark: bool },
    /// B.2.1 `actor`, the CL function form. Parsed so it can be refused.
    Actor,
    /// B.2.8 `negation = "~", context`.
    Negation(Vec<Node>),
    /// B.2.5 `context`.
    Context(Vec<Node>),
    /// B.2.5 `existentialConcept`: `[*x]`. `seqmark` is `[*...x]`.
    Existential { name: Name, seqmark: bool },
    /// B.2.5 `coreferenceConcept`: `[: r …]`.
    Coreference(Vec<Ref>),
    /// Anything the core grammar does not have in a concept: a type label, a
    /// type expression, `@every`. Extended CGIF only, parsed so it can be
    /// refused rather than silently reinterpreted.
    ExtendedConcept(String),
}

struct Parser {
    toks: Vec<Tok>,
    i: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.i)
    }
    fn next(&mut self) -> Tok {
        let t = self.toks.get(self.i).cloned().expect("unexpected end of CGIF");
        self.i += 1;
        t
    }
    fn eat(&mut self, t: Tok) {
        let got = self.next();
        assert_eq!(got, t, "expected {t:?}");
    }

    fn name(&mut self) -> Name {
        match self.next() {
            Tok::Bare(s) => Name::Bare(s),
            Tok::Enclosed(s) => Name::Enclosed(s),
            Tok::Quoted(s) => Name::Quoted(s),
            other => panic!("expected a CGname, found {other:?}"),
        }
    }

    /// B.2.9 `reference = ["?"], CGname`.
    fn reference(&mut self) -> Ref {
        if self.peek() == Some(&Tok::Query) {
            self.next();
            Ref { bound: true, name: self.name() }
        } else {
            Ref { bound: false, name: self.name() }
        }
    }

    /// B.2.6 `CG = {concept | conceptualRelation | negation | comment}`.
    fn cg(&mut self, terminator: Tok) -> Vec<Node> {
        let mut nodes = Vec::new();
        while self.peek() != Some(&terminator) {
            nodes.push(self.node());
        }
        nodes
    }

    fn node(&mut self) -> Node {
        match self.peek().cloned().expect("unexpected end of CGIF") {
            Tok::Comment(_) => match self.next() {
                Tok::Comment(s) => Node::Comment(s),
                _ => unreachable!(),
            },
            Tok::Tilde => {
                self.next();
                self.eat(Tok::Open);
                let body = self.cg(Tok::Close);
                self.eat(Tok::Close);
                Node::Negation(body)
            }
            Tok::Open => {
                self.next();
                self.concept_after_open()
            }
            Tok::LParen => {
                self.next();
                self.relation_after_lparen()
            }
            other => panic!("not the start of a CGIF node: {other:?}"),
        }
    }

    fn concept_after_open(&mut self) -> Node {
        match self.peek().cloned() {
            // `[*x]` or `[*...x]`. B.2.5 existentialConcept.
            Some(Tok::Star) => {
                self.next();
                let name = self.name();
                let seqmark = matches!(&name, Name::Bare(s) if s.starts_with("..."));
                self.eat(Tok::Close);
                Node::Existential { name, seqmark }
            }
            // `[: r …]`. B.2.5 coreferenceConcept, `{reference}-`.
            Some(Tok::Colon) => {
                self.next();
                let mut refs = Vec::new();
                while self.peek() != Some(&Tok::Close) {
                    refs.push(self.reference());
                }
                self.eat(Tok::Close);
                assert!(!refs.is_empty(), "a coreference concept takes at least one reference");
                Node::Coreference(refs)
            }
            // `[@every*x]` or `[@*x …]`. Extended CGIF only, B.3.6 and B.3.10.
            Some(Tok::At) => {
                let mut depth = 1;
                while depth > 0 {
                    match self.next() {
                        Tok::Open => depth += 1,
                        Tok::Close => depth -= 1,
                        _ => {}
                    }
                }
                Node::ExtendedConcept("@ (a universal quantifier or a type expression)".into())
            }
            // A bare name here is a TYPE LABEL, which core CGIF has no field
            // for: B.1.2, "extended syntax of CGIF, which adds type labels on
            // concepts".
            Some(Tok::Bare(_)) | Some(Tok::Enclosed(_)) | Some(Tok::Quoted(_)) => {
                let label = format!("{:?}", self.peek().expect("peeked"));
                let mut depth = 1;
                while depth > 0 {
                    match self.next() {
                        Tok::Open => depth += 1,
                        Tok::Close => depth -= 1,
                        _ => {}
                    }
                }
                Node::ExtendedConcept(format!("a type label on a concept: {label}"))
            }
            // `[ CG ]`. B.2.5 context, the arm every other case is not.
            _ => {
                let body = self.cg(Tok::Close);
                self.eat(Tok::Close);
                Node::Context(body)
            }
        }
    }

    fn relation_after_lparen(&mut self) -> Node {
        let bound_type_label = if self.peek() == Some(&Tok::Hash) {
            self.next();
            self.eat(Tok::Query);
            true
        } else {
            false
        };
        let name = self.name();
        let mut args = Vec::new();
        let mut seqmark = false;
        let mut actor = false;
        loop {
            match self.peek().cloned().expect("unterminated conceptual relation") {
                Tok::RParen => {
                    self.next();
                    break;
                }
                // B.2.1 actor: the bar splits the arc sequence.
                Tok::Bar => {
                    self.next();
                    actor = true;
                }
                _ => {
                    let r = self.reference();
                    if matches!(&r.name, Name::Bare(s) if s.starts_with("...")) {
                        seqmark = true;
                    }
                    args.push(r);
                }
            }
        }
        if actor {
            return Node::Actor;
        }
        Node::Relation { bound_type_label, name, args, seqmark }
    }
}

/// B.2.11 `text = "[", [comment], "Proposition", ":", [CGname], CG,
/// [endComment], "]"`. Returns the text's name and its CG.
fn parse_text(src: &str) -> (Name, Vec<Node>) {
    let mut p = Parser { toks: lex(src), i: 0 };
    p.eat(Tok::Open);
    assert_eq!(
        p.next(),
        Tok::Bare("Proposition".to_string()),
        "B.2.11 spells the text's type label `Proposition`"
    );
    p.eat(Tok::Colon);
    let name = p.name();
    let body = p.cg(Tok::Close);
    p.eat(Tok::Close);
    assert_eq!(p.i, p.toks.len(), "trailing tokens after the text");
    (name, body)
}

// ── 2. Syntax conformance ───────────────────────────────────────────────────

/// **Gate.** The emitted text is a B.2.11 `text` whose every node is a core
/// CGIF node, and it parses to the end.
///
/// A file nothing can read is the failure this whole layer exists to catch: the
/// CLIF side found that both parsers that exist return an EMPTY theory from the
/// shape ISO's own BFO files use. The check here is a second reading of the
/// grammar rather than an independent implementation, and the module header
/// says so.
#[test]
fn the_emitted_cgif_parses_as_a_core_cgif_text() {
    let problem = FolProblem::build(&corpus(), None).expect("translates");
    let text = problem.to_cgif(TEXT_NAME).expect("no comment closes itself early");
    let (name, body) = parse_text(&text);

    assert_eq!(
        name,
        Name::Enclosed(TEXT_NAME.to_string()),
        "B.1.1 admits only letters, digits and underscore in a bare identifier, so an IRI must \
         be an enclosed name. The CLIF writer emits a bare IRI for Macleod's sake and this one \
         cannot"
    );

    let n = problem.formulas().len();
    let comments = body.iter().filter(|x| matches!(x, Node::Comment(_))).count();
    let sentences: Vec<&Node> = body.iter().filter(|x| !matches!(x, Node::Comment(_))).collect();
    assert_eq!(comments, n + 1, "one label comment per formula, plus the header");
    assert_eq!(sentences.len(), n, "one sentence node per formula");
    for s in &sentences {
        assert!(
            matches!(s, Node::Context(_)),
            "every sentence is wrapped in a context of its own, so a formula whose root is a \
             conjunction cannot spread across the text's own graph: {s:?}"
        );
    }
}

/// Walk every node of a parse, calling `f` on each, with the path depth.
fn walk(nodes: &[Node], f: &mut impl FnMut(&Node)) {
    for n in nodes {
        f(n);
        match n {
            Node::Negation(b) | Node::Context(b) => walk(b, f),
            _ => {}
        }
    }
}

/// **Gate.** The text stays inside the compact, first-order sub-dialect.
///
/// ISO/IEC 24707 clause 7.1.1 names it: "A compact sub-dialect is a dialect
/// that does not recognize sequence markers." Clause 6.5 says why that is the
/// one that matters: Common Logic with sequence markers "is not compact, and
/// therefore not first-order", and `OwlLean.adequacy` is about plain
/// first-order logic, so a text using one would sit outside the theorem.
///
/// Six properties, each tied to the production that would otherwise admit the
/// construct:
///
///   * no sequence marker, in B.2.5's `existentialConcept` or B.2.3's
///     `arcSequence`,
///   * no `#?` type label, which B.2.7's own comment says is how CGIF
///     "supports the CL ability to quantify over relations and functions",
///   * no actor, B.2.1, which is how a CL FUNCTION is written,
///   * no extended-CGIF concept: no type label, no type expression, no
///     `@every`,
///   * every relation name used at ONE arity throughout,
///   * no bound coreference label in a relation's type-label position.
#[test]
fn cgif_stays_in_the_compact_first_order_sub_dialect() {
    let problem = FolProblem::build(&corpus(), Some(&OwlAxiom::SubClass(c("A"), c("B"))))
        .expect("translates");
    let text = problem.to_cgif(TEXT_NAME).expect("writable");
    let (_, body) = parse_text(&text);
    // The check is on the PARSE and not on the raw text, because the header
    // names the constructs it excludes and a substring search for `...` finds
    // the header saying it emits none. B.2.4 lets a comment hold anything but
    // its own closing delimiter, so only the nodes can be searched.
    for n in &body {
        if let Node::Comment(_) = n {
            continue;
        }
        assert!(
            !format!("{n:?}").contains("..."),
            "a sequence marker in a sentence would take the text out of the compact sub-dialect"
        );
    }

    let mut arity: HashMap<Name, usize> = HashMap::new();
    let mut relations = 0usize;
    walk(&body, &mut |n| match n {
        Node::Actor => panic!("an actor is a CL function, and FSig has no function symbol"),
        Node::ExtendedConcept(what) => panic!("extended CGIF construct in a core text: {what}"),
        Node::Existential { seqmark, name } => {
            assert!(!seqmark, "a defining sequence label: {name:?}");
            assert!(
                matches!(name, Name::Bare(_)),
                "a defining label is an identifier, never a numeral or a quoted string, which \
                 CLIF's `bvar` could not bind: {name:?}"
            );
        }
        Node::Relation { bound_type_label, name, args, seqmark } => {
            assert!(!bound_type_label, "a `#?` type label quantifies into a predicate position");
            assert!(!seqmark, "a bound sequence label in an arc sequence");
            relations += 1;
            let seen = arity.entry(name.clone()).or_insert(args.len());
            assert_eq!(
                *seen,
                args.len(),
                "{name:?} is used at two arities, which is arity-free Common Logic and not plain \
                 first-order logic"
            );
        }
        _ => {}
    });
    assert!(relations > 0, "no relation was checked, so this gate proved nothing");
    assert_eq!(arity[&Name::Bare("thing".to_string())], 1);
    assert_eq!(arity[&Name::Bare("lit".to_string())], 1);
}

/// **Gate.** No numeral and no single-quoted string stands anywhere in the
/// text, and every predicate and constant is a name the grammar admits.
///
/// A.4.2 says of CLIF that "The subdialect of CLIF which does not use numerals
/// or quoted strings is exactly semantically conformant", and Annex B states no
/// analogue for CGIF, so this test holds the PROPERTY and the emitter does not
/// borrow the label. CGIF gets it without the exception CLIF needs: a CLIF
/// label rides on `cl:comment`, whose argument is a quoted string, while B.2.4
/// makes a CGIF comment a lexical construct that is not a name at all.
#[test]
fn cgif_uses_no_interpreted_name() {
    let problem = FolProblem::build(&corpus(), None).expect("translates");
    let text = problem.to_cgif(TEXT_NAME).expect("writable");
    let (_, body) = parse_text(&text);

    fn ok_name(n: &Name) {
        match n {
            Name::Quoted(s) => panic!(
                "a single-quoted string is an INTERPRETED name, whose denotation is fixed in \
                 every interpretation: {s}"
            ),
            Name::Bare(s) => {
                assert!(
                    !s.starts_with(|c: char| c.is_ascii_digit()),
                    "a bare decimal is a CGIF numeral, which is an interpreted name: {s}"
                );
                // B.1.1: `identifier = letter, {letter | digit | "_"}`. `letter`
                // is used there and never defined anywhere in ISO/IEC
                // 24707:2018, which is a defect in the standard; the emitter
                // produces ASCII letters only, so the ASCII reading is the one
                // checked and the choice is written down rather than assumed.
                let mut cs = s.chars();
                assert!(
                    cs.next().is_some_and(|c| c.is_ascii_alphabetic()),
                    "an identifier begins with a letter: {s}"
                );
                assert!(
                    cs.all(|c| c.is_ascii_alphanumeric() || c == '_'),
                    "an identifier is letters, digits and underscore, and an IRI is none of \
                     those, so it must be an enclosed name: {s}"
                );
            }
            Name::Enclosed(_) => {}
        }
    }

    let mut checked = 0usize;
    walk(&body, &mut |n| match n {
        Node::Relation { name, args, .. } => {
            ok_name(name);
            checked += 1;
            for a in args {
                ok_name(&a.name);
            }
        }
        Node::Coreference(refs) => {
            for r in refs {
                ok_name(&r.name);
            }
        }
        Node::Existential { name, .. } => ok_name(name),
        _ => {}
    });
    assert!(checked > 0);
}

/// **Gate.** No defining coreference label is ever shadowed, and no context
/// directly contains two of them.
///
/// B.2.10 says both that a context "shall not contain any concept other than x
/// with a defining label with the same CG name n" — where *contains* is
/// transitive — and, in the sentence after it, that a nested context may
/// redeclare one. The two cannot both hold. The emitter gives every binder a
/// context of its own and never redeclares a name inside its own scope, so the
/// text is legal under EITHER reading and nothing here depends on which is
/// right.
///
/// The third clause of B.2.10 is checked too: "No constant with CG name n shall
/// be in the scope associated with some concept with a defining label with CG
/// name n." That one holds structurally — a defining label is `Xn` and a
/// constant begins `i:` — and is checked rather than argued.
#[test]
fn no_defining_label_is_ever_shadowed() {
    let problem = FolProblem::build(&corpus(), None).expect("translates");
    let text = problem.to_cgif(TEXT_NAME).expect("writable");
    let (_, body) = parse_text(&text);

    fn check(nodes: &[Node], in_scope: &mut BTreeSet<Name>, path: &str) {
        let mut here: BTreeMap<Name, usize> = BTreeMap::new();
        for n in nodes {
            if let Node::Existential { name, .. } = n {
                *here.entry(name.clone()).or_default() += 1;
                assert!(
                    !in_scope.contains(name),
                    "the defining label {name:?} at {path} is inside the scope of another with \
                     the same name. B.2.10 is self-contradictory about whether that shadows or \
                     is illegal, and this emitter must not depend on the answer"
                );
            }
        }
        for (name, count) in &here {
            assert_eq!(
                *count, 1,
                "the context at {path} directly contains {count} defining labels named \
                 {name:?}; B.2.10 admits one"
            );
        }
        let added: Vec<Name> = here.keys().cloned().collect();
        for n in &added {
            in_scope.insert(n.clone());
        }
        for (i, n) in nodes.iter().enumerate() {
            // A constant whose name equals a defining label in scope is
            // forbidden outright by B.2.10's last clause.
            let names: Vec<&Ref> = match n {
                Node::Relation { args, .. } => args.iter().collect(),
                Node::Coreference(refs) => refs.iter().collect(),
                _ => Vec::new(),
            };
            for r in names {
                if !r.bound {
                    assert!(
                        !in_scope.contains(&r.name),
                        "the constant {:?} stands in the scope of a defining label of the same \
                         name, which B.2.10 forbids",
                        r.name
                    );
                }
            }
            if let Node::Negation(b) | Node::Context(b) = n {
                check(b, in_scope, &format!("{path}/{i}"));
            }
        }
        for n in &added {
            in_scope.remove(n);
        }
    }

    let mut scope = BTreeSet::new();
    check(&body, &mut scope, "text");

    // And the text's own graph directly contains no defining label at all, so
    // no sentence can bind a name another sentence reads.
    for n in &body {
        assert!(
            !matches!(n, Node::Existential { .. }),
            "a defining label directly in the text's CG would scope over every later sentence"
        );
    }
}

/// Every comment the emitter writes can be written: B.2.4 forbids `*/` inside
/// one and gives no escape.
#[test]
fn no_emitted_comment_can_close_itself_early() {
    let problem = FolProblem::build(&corpus(), Some(&OwlAxiom::SubClass(c("A"), c("B"))))
        .expect("translates");
    let text = problem.to_cgif(TEXT_NAME).expect("writable");
    let (_, body) = parse_text(&text);
    let mut seen = 0usize;
    walk(&body, &mut |n| {
        if let Node::Comment(s) = n {
            seen += 1;
            assert!(cgif::comment_is_writable(s), "a comment closes itself early: {s}");
        }
    });
    assert!(seen > 1, "the header and at least one label were checked");
}

// ── 3. The round trip ───────────────────────────────────────────────────────

/// Read a CG back into a `Form`.
///
/// B.2.6 gives the rule this inverts: "Let E be the subset of C of existential
/// concepts; and let X be the set of all concepts, conceptual relations, and
/// negations of g except for those in E. […] If E is non-empty, then cg2cl(g)
/// is a quantified sentence of type existential with the set of names
/// consisting of the CGname of the defining coreference label of every e in E
/// and with the body B." So an existential concept scopes over the whole graph
/// it sits in, and the rest of the graph is a conjunction.
fn read_cg(nodes: &[Node]) -> Form {
    let mut binders: Vec<u32> = Vec::new();
    let mut parts: Vec<Form> = Vec::new();
    for n in nodes {
        match n {
            Node::Comment(_) => {}
            Node::Existential { name, .. } => binders.push(var_index(name)),
            other => parts.push(read_node(other)),
        }
    }
    let body = conj(parts);
    binders.into_iter().rev().fold(body, |acc, n| Form::Ex(n, Box::new(acc)))
}

fn conj(mut parts: Vec<Form>) -> Form {
    match parts.len() {
        0 => Form::Tru,
        1 => parts.pop().expect("length checked"),
        _ => {
            let head = parts.remove(0);
            Form::And(Box::new(head), Box::new(conj(parts)))
        }
    }
}

fn var_index(n: &Name) -> u32 {
    let Name::Bare(s) = n else { panic!("a defining label is an identifier, found {n:?}") };
    s.strip_prefix('X')
        .and_then(|d| d.parse().ok())
        .unwrap_or_else(|| panic!("a variable is Xn, found {s}"))
}

fn read_ref(r: &Ref) -> Term {
    match (&r.name, r.bound) {
        (Name::Bare(s), true) => Term::Var(
            s.strip_prefix('X')
                .and_then(|d| d.parse().ok())
                .unwrap_or_else(|| panic!("a bound label is ?Xn, found {s}")),
        ),
        (Name::Enclosed(s), false) => Term::Const(
            s.strip_prefix("i:")
                .unwrap_or_else(|| panic!("a constant carries the i: prefix, found {s}"))
                .to_string(),
        ),
        other => panic!("not a term in this fragment: {other:?}"),
    }
}

fn read_p1(n: &Name) -> P1 {
    match n {
        Name::Bare(s) if s == "thing" => P1::Thing,
        Name::Bare(s) if s == "lit" => P1::Lit,
        Name::Enclosed(s) => {
            if let Some(r) = s.strip_prefix("c:") {
                P1::Cls(r.to_string())
            } else if let Some(r) = s.strip_prefix("d:") {
                P1::Dt(r.to_string())
            } else {
                panic!("a unary predicate carries no kind prefix: {s}")
            }
        }
        other => panic!("not a unary predicate: {other:?}"),
    }
}

fn read_p2(n: &Name) -> P2 {
    let Name::Enclosed(s) = n else { panic!("not a binary predicate: {n:?}") };
    if let Some(r) = s.strip_prefix("op:") {
        P2::Op(r.to_string())
    } else if let Some(r) = s.strip_prefix("dp:") {
        P2::Dp(r.to_string())
    } else {
        panic!("a binary predicate carries no kind prefix: {s}")
    }
}

fn read_node(n: &Node) -> Form {
    match n {
        Node::Relation { name, args, .. } => match args.len() {
            1 => Form::App1(read_p1(name), read_ref(&args[0])),
            2 => Form::App2(read_p2(name), read_ref(&args[0]), read_ref(&args[1])),
            k => panic!("no first-order atom of arity {k}"),
        },
        // B.2.5: a coreference concept is a conjunction of equations. With two
        // references that conjunction is one equation. The standard lets the
        // first term be "any reference in R", so reading the first written as
        // the first term is one of the permitted readings and equality is
        // symmetric either way.
        Node::Coreference(refs) => {
            assert_eq!(refs.len(), 2, "the emitter writes exactly two references");
            Form::Eq(read_ref(&refs[0]), read_ref(&refs[1]))
        }
        Node::Context(body) => read_cg(body),
        Node::Negation(body) => {
            let real: Vec<&Node> = body.iter().filter(|x| !matches!(x, Node::Comment(_))).collect();
            // `~[ ]` is falsity. B.2.8: "The negation of the blank CG, written
            // `~[ ]`, is always false".
            if real.is_empty() {
                return Form::Fls;
            }
            // `~[ [*Xn] ~[ … ] ]` is a universal. B.3.7: the translation of a
            // CG containing universal concepts "shall be a nest of two
            // negations. The outer context shall contain the translations of
            // all the universal concepts, and the inner context shall contain
            // the translations of all other nodes". Nothing else can produce a
            // BARE existential concept directly inside a negation, because an
            // `Ex` is always emitted inside a context of its own.
            if real.len() == 2
                && let (Node::Existential { name, .. }, Node::Negation(inner)) =
                    (real[0], real[1])
            {
                return Form::All(var_index(name), Box::new(read_cg(inner)));
            }
            Form::Neg(Box::new(read_cg(body)))
        }
        Node::Comment(_)
        | Node::Actor
        | Node::ExtendedConcept(_)
        | Node::Existential { .. } => panic!("not a sentence node: {n:?}"),
    }
}

/// The two rewrites core CGIF forces, applied to a `Form` so both sides of a
/// round trip are compared in the same shape.
///
/// **Implication and disjunction have no operator in core CGIF.** B.3.5 defines
/// both by rewriting them away: `ifThen` becomes `"~[", CG(ante), "~[",
/// CG(conse), "]", "]"` and `eitherOr` becomes a negation holding one `~[…]`
/// per disjunct. Those are the standard's own definitions of the connectives,
/// not this project's encoding of them, so a round trip that recovered `Imp`
/// and `Or` would be recovering something the syntax does not carry.
///
/// **Conjunction is a SET of nodes, not a binary connective.** B.2.6: "A
/// conceptual graph consists of an unordered set of concepts, conceptual
/// relations, negations, and comments." So the associativity of an `And` tree
/// cannot survive either, and both sides are right-associated.
fn primitive(f: &Form) -> Form {
    let mut parts = Vec::new();
    spread(f, &mut parts);
    conj(parts)
}

/// The nodes a formula becomes in the CG it is written into.
///
/// The two jobs are one job, which is why they are one function. A conjunction
/// contributes its conjuncts as SEPARATE NODES, so nesting on either side
/// vanishes; and an implication's antecedent is written into the SAME graph as
/// the negation that carries its consequent, so the antecedent's conjuncts sit
/// beside that negation rather than under an `And` with it. Expanding the
/// implication first and flattening afterwards would produce a different tree
/// from the one the file can be read back as, which is how this function was
/// got wrong the first time.
fn spread(f: &Form, out: &mut Vec<Form>) {
    match f {
        Form::And(g, h) => {
            spread(g, out);
            spread(h, out);
        }
        // `~[ CG(g) ~[ CG(h) ]]`, B.3.5's own `ifThen` rewrite. `g`'s nodes and
        // the negation of `h` are siblings in one graph.
        Form::Imp(g, h) => {
            let mut inner = Vec::new();
            spread(g, &mut inner);
            inner.push(Form::Neg(Box::new(primitive(h))));
            out.push(Form::Neg(Box::new(conj(inner))));
        }
        // `~[ ~[CG(g)] ~[CG(h)] ]`, B.3.5's `eitherOr`. Each disjunct is under
        // a negation of its own, so neither spreads.
        Form::Or(g, h) => out.push(Form::Neg(Box::new(conj(vec![
            Form::Neg(Box::new(primitive(g))),
            Form::Neg(Box::new(primitive(h))),
        ])))),
        Form::Neg(g) => out.push(Form::Neg(Box::new(primitive(g)))),
        Form::All(n, g) => out.push(Form::All(*n, Box::new(primitive(g)))),
        Form::Ex(n, g) => out.push(Form::Ex(*n, Box::new(primitive(g)))),
        other => out.push(other.clone()),
    }
}

/// **Gate.** Every sentence reads back to the formula it was built from.
///
/// Not a golden file. The reader above is written from Annex B's translation
/// clauses rather than from the emitter, so a drift in either is caught, which
/// is the same argument `clif_round_trips_to_the_same_forms` makes for CLIF.
#[test]
fn cgif_round_trips_to_the_same_forms() {
    let problem = FolProblem::build(&corpus(), None).expect("translates");
    let formulas = problem.formulas();
    assert!(formulas.len() >= 36, "the corpus should be substantial");
    for (name, _role, f) in &formulas {
        let mut p = Parser { toks: lex(&cgif::sentence(f)), i: 0 };
        let node = p.node();
        assert_eq!(p.i, p.toks.len(), "trailing tokens in the sentence for {name}");
        assert_eq!(
            &read_node(&node),
            &primitive(f),
            "CGIF round trip differs for {name}: {}",
            cgif::sentence(f)
        );
    }
}

/// Hand-pinned CGIF for the worked cases the CLIF test pins by hand too. The
/// round trip proves the writer and the reader agree; these pin what they agree
/// ON, against the productions rather than against a recorded run.
#[test]
fn cgif_worked_cases_are_pinned_by_hand() {
    // `A subClassOf exists r . B`. `allObj 0 (imp (cls A) (exObj 2 …))`.
    // `allObj n f` is `~[[*Xn] ~[ f ]]` and `imp g h` is `~[ g ~[ h ]]`.
    let f = Translation::axiom(&OwlAxiom::SubClass(
        c("A"),
        Concept::Some_(iri("r"), Box::new(c("B"))),
    ))
    .unwrap();
    assert_eq!(
        cgif::graph(&f),
        "~[[*X0] ~[~[(thing ?X0) ~[~[(\"c:http://e/A\" ?X0) \
         ~[[[*X2] (thing ?X2) (\"op:http://e/r\" ?X0 ?X2) (\"c:http://e/B\" ?X2)]]]]]]]"
    );

    let b = Translation::background();
    // `forall X0 . not (thing X0 and lit X0)`.
    assert_eq!(cgif::graph(&b[0]), "~[[*X0] ~[~[(thing ?X0) (lit ?X0)]]]");
    // `exists X0 . thing X0`. B.2.6: the existential concept scopes over its
    // whole graph, so the body needs no further bracket.
    assert_eq!(cgif::graph(&b[1]), "[[*X0] (thing ?X0)]");

    // B.2.5 and B.2.8: `[]` is truth, `~[]` is falsity. CGIF has no truth
    // constants any more than CLIF does.
    assert_eq!(cgif::graph(&Form::Tru), "[]");
    assert_eq!(cgif::graph(&Form::Fls), "~[]");

    // B.2.5: an equation is a coreference concept. CGIF has no `=`.
    let same = Translation::axiom(&OwlAxiom::SameAs(iri("a"), iri("b"))).unwrap();
    assert_eq!(cgif::graph(&same), "[: \"i:http://e/a\" \"i:http://e/b\"]");
}

// ── 4. One translation, two Common Logic dialects ───────────────────────────

/// **Gate.** The CGIF file and the CLIF file carry the same sentences, in the
/// same order, under the same labels.
///
/// This is the claim the whole design rests on: five serialisers, one
/// translation, never five translations. Each formula is checked three ways —
/// the CLIF file contains its CLIF rendering, the CGIF file contains its CGIF
/// rendering, and the CGIF rendering reads back to the formula — so a writer
/// that dropped, added, reordered or altered one sentence breaks it.
#[test]
fn cgif_and_clif_carry_the_same_sentences_from_the_same_translation() {
    let problem = FolProblem::build(&corpus(), Some(&OwlAxiom::SubClass(c("A"), c("B"))))
        .expect("translates");
    let clif_text = problem.to_clif(ClifDialect::Iso, ClifComments::Standalone, TEXT_NAME);
    let cgif_text = problem.to_cgif(TEXT_NAME).expect("writable");

    let formulas = problem.formulas();
    for (label, role, f) in &formulas {
        assert!(
            clif_text.contains(&clif::form(f)),
            "the CLIF file does not carry {label}"
        );
        assert!(
            cgif_text.contains(&cgif::sentence(f)),
            "the CGIF file does not carry {label}"
        );
        assert!(
            clif_text.contains(&format!("(cl:comment '{label} ({role})')")),
            "the CLIF label for {label} is missing"
        );
        assert!(
            cgif_text.contains(&format!("/*{label} ({role})*/")),
            "the CGIF label for {label} is missing"
        );
    }

    // The label SEQUENCES, read out of the two files independently, are equal.
    // A count alone would pass on a reordering.
    let clif_labels: Vec<String> = clif_text
        .match_indices("(cl:comment '")
        .map(|(i, m)| {
            let rest = &clif_text[i + m.len()..];
            rest[..rest.find('\'').expect("a closed comment string")].to_string()
        })
        .collect();
    let (_, cgif_body) = parse_text(&cgif_text);
    let cgif_labels: Vec<String> = cgif_body
        .iter()
        .filter_map(|n| match n {
            Node::Comment(s) => Some(s.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(clif_labels.len(), formulas.len() + 1, "one per formula plus the header");
    assert_eq!(cgif_labels.len(), formulas.len() + 1);
    // Both files put the header first; after it the labels must agree exactly.
    assert_eq!(
        clif_labels[1..],
        cgif_labels[1..],
        "the two Common Logic dialects disagree about which sentences the file has, or about \
         their order"
    );

    // And the CGIF sentences, read back, are the formulas themselves.
    let sentences: Vec<&Node> =
        cgif_body.iter().filter(|n| !matches!(n, Node::Comment(_))).collect();
    assert_eq!(sentences.len(), formulas.len());
    for (node, (label, _, f)) in sentences.iter().zip(formulas.iter()) {
        assert_eq!(&read_node(node), &primitive(f), "sentence {label} differs");
    }
}

/// The emitted CGIF states its own restriction, in the file. A limit stated
/// only in a README gets read without one.
#[test]
fn the_cgif_header_states_the_restriction() {
    let problem = FolProblem::build(&corpus(), None).expect("translates");
    let text = problem.to_cgif(TEXT_NAME).expect("writable");
    for wanted in [
        "CORE CGIF, not extended CGIF",
        "THE COMPACT SUB-DIALECT",
        "clause 6.5",
        "PINNED BY TESTS AND IS NOT ITSELF PROVED",
        "[Proposition:",
    ] {
        assert!(text.contains(wanted), "the header does not say {wanted:?}");
    }
    assert!(
        !text.contains("ORACLE OPINION"),
        "a file with no conjecture says nothing about a prover's verdict"
    );
    let with_goal = FolProblem::build(&[], Some(&OwlAxiom::SubClass(c("A"), c("B"))))
        .unwrap()
        .to_cgif(TEXT_NAME)
        .expect("writable");
    assert!(with_goal.contains("ORACLE OPINION, not a certificate"));
}

/// `cgif` is a syntax the parser knows, it takes no dialect or encoding flag,
/// and the refusal message for an unknown syntax names it.
#[test]
fn cgif_is_a_syntax_and_takes_no_flags() {
    let s = Syntax::parse("cgif", None, None, None).expect("cgif is a syntax");
    assert_eq!(s.name(), "cgif");
    assert_eq!(s.extension(), "cgif");
    assert_eq!(s.dialect(), None, "CGIF has one spelling of its operators");
    assert_eq!(s.comments(), None, "B.2.4 gives comments a lexical syntax of their own");
    assert_eq!(s.encoding(), None);
    assert_eq!(Syntax::parse("cg", None, None, None).expect("an alias").name(), "cgif");

    let e = Syntax::parse("sexpr", None, None, None).expect_err("sexpr is not a syntax");
    assert!(
        ["tptp", "clif", "cgif", "smtlib", "ladr"].iter().all(|n| e.to_string().contains(n)),
        "the message must name every alternative, cgif included: {e}"
    );
}

// ── 5. Nothing is dropped, and nothing is rewritten ─────────────────────────

/// **Gate.** A construct outside the fragment is NAMED AND COUNTED in a CGIF
/// run, exactly as in a TPTP or CLIF one.
///
/// The restriction discipline is a property of the translation and not of a
/// serialiser, so adding a fourth output must not create a fourth place where
/// the report could be silent. `owl:hasKey` is outside `OwlLean/Syntax.lean`;
/// the CGIF report must say so, with the count and the reason, and must set
/// `exports_a_weaker_axiom_set`.
#[test]
fn a_cgif_run_names_and_counts_what_it_cannot_export() {
    let ttl = r#"
@prefix : <http://e/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
:A a owl:Class ; rdfs:subClassOf :B .
:B a owl:Class .
:k a owl:ObjectProperty .
:A owl:hasKey ( :k ) .
"#;
    let graph = std::sync::Arc::new(open_ontologies::graph::GraphStore::new());
    graph.load_turtle(ttl, None).expect("turtle parses");
    let dir = tempfile::tempdir().expect("tempdir");
    let report = open_ontologies::tptp::export(&graph, dir.path(), Syntax::Cgif, None, 0)
        .expect("export");
    let v: serde_json::Value = serde_json::from_str(&report).expect("json");

    assert_eq!(v["syntax"], "cgif");
    assert_eq!(v["cgif_dialect"], "core");
    assert!(v["cgif_sub_dialect"].as_str().unwrap_or("").contains("compact"));
    assert_eq!(
        v["common_logic_dialects_emitted"],
        serde_json::json!(["clif", "cgif"]),
        "two of ISO/IEC 24707's three dialects"
    );
    assert!(
        v["common_logic_dialects_not_emitted"][0]
            .as_str()
            .unwrap_or("")
            .contains("xcl"),
        "the third is named as absent rather than left to be assumed present"
    );

    assert_eq!(
        v["exports_a_weaker_axiom_set"], true,
        "owl:hasKey is outside the fragment, so the export is weaker than the ontology"
    );
    let named: Vec<String> = v["constructs_not_exported"]
        .as_array()
        .expect("a list")
        .iter()
        .map(|d| d["construct"].as_str().unwrap_or("").to_string())
        .collect();
    assert!(named.iter().any(|n| n.contains("hasKey")), "got {named:?}");
    let entry = &v["constructs_not_exported"][0];
    assert!(entry["occurrences"].as_u64().unwrap_or(0) >= 1, "counted, not only named");
    assert!(!entry["why"].as_str().unwrap_or("").is_empty(), "with the reason");

    // The file itself lands, parses, and carries the subsumption that IS in the
    // fragment. A report that named the drop while writing nothing would be
    // worse than either.
    let text = std::fs::read_to_string(dir.path().join("ontology.cgif")).expect("the file");
    let (_, body) = parse_text(&text);
    assert!(body.iter().any(|n| matches!(n, Node::Context(_))), "sentences landed");
    assert!(text.contains("owl_1_subClassOf"));
}

/// **Gate.** A comment that would close itself early is REFUSED, with the
/// reason, and no file is written.
///
/// B.2.4 forbids `*/` inside a comment and defines no escape, so there are
/// three possible behaviours and two of them are wrong: rewriting the text
/// emits a different comment, and dropping it loses the label that says which
/// axiom a sentence is. This is the same rule `UnwritableSymbol` follows for
/// SMT-LIB and LADR.
///
/// The refusal is reachable rather than theoretical: the CGIF header describes
/// the comment syntax, and an earlier draft of it named the delimiters
/// literally and tripped this on every run.
#[test]
fn a_comment_that_could_close_itself_early_is_refused() {
    assert!(cgif::comment("background_1 (axiom)").is_ok());
    let e = cgif::comment("a comment that ends the comment */ and then some")
        .expect_err("B.2.4 forbids it");
    assert!(e.text.contains("*/"));
    assert!(
        e.why.contains("B.2.4") && e.why.contains("no escape"),
        "the refusal must name the clause and say why there is no third option: {}",
        e.why
    );
    assert!(
        e.to_string().contains("cannot be written in CGIF"),
        "got {e}"
    );
    assert!(!cgif::comment_is_writable("*/"));
    assert!(cgif::comment_is_writable("/* nested opener is fine"));
}
