//! Reading a finite structure back out of a solver, and writing it in the form
//! the verified checker reads.
//!
//! # Where this sits
//!
//! `src/tptp.rs` writes the question, in SMT-LIB 2 for Z3 and in LADR for
//! Mace4. `lean/Fol/` checks the answer, and `Fol.satisfiable_of_check` is why
//! that check is worth anything. This module is the join: it turns whatever
//! the solver printed into a [`FiniteModel`], and a [`FiniteModel`] into the
//! `model.tsv` the checker parses.
//!
//! **One structure, two front ends.** The two printed formats have nothing in
//! common — Z3 prints SMT-LIB `define-fun` bodies that have to be EVALUATED,
//! Mace4 prints dense integer tables that have to be INDEXED — and the whole
//! reason for the shape of this module is that a third finder should cost only
//! a third `parse_model`. Everything after the front end is shared.
//!
//! # Nothing here is trusted, and that is not a slogan
//!
//! A bug in this module cannot produce a false certificate. The checker
//! re-evaluates the ORIGINAL formulas against whatever structure comes out, so
//! a misread table yields a structure that fails `Fol.check` and the pipeline
//! reports a stop-the-line disagreement. The failure mode of a parser bug here
//! is a false alarm, never a false clean. That asymmetry is worth stating
//! because it is the reason the ingestion can be a plain Rust parser at all
//! while the checker has to be Lean.
//!
//! What a bug here CAN do is break attribution: silently substituting a
//! different structure for the solver's would leave a run reporting that "the
//! solver's model was checked" when it checked something else. So every
//! failure is loud. There is no defaulting, no `unwrap_or(false)`, no skipped
//! row: a symbol the solver did not interpret is an error naming the symbol,
//! a construct the evaluator does not know is an error naming the construct,
//! and a table of the wrong length is an error giving both lengths.
//!
//! # The reduct
//!
//! Both solvers interpret symbols the translation never emitted. Z3 invents
//! auxiliary functions; Mace4 clausifies and Skolemises, so its models carry
//! `f1(_)` and constants it chose itself. Those are DROPPED, which is taking
//! the reduct of the solver's structure, and it needs no lemma because the
//! checker re-evaluates formulas that cannot mention a symbol the translation
//! never emitted. The dropped names are reported rather than passed over, so
//! the reduct is visible.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use crate::tptp::ladr::{SymKind, SymbolTable};
use crate::tptp::Vocabulary;

/// A finite first-order structure over the carrier `0 .. domain-1`.
///
/// The carrier is the integers because that is what Mace4 prints and what
/// `Fin N` is, so a Mace4 `interpretation(N, …)` block ingests with no
/// renaming at all.
#[derive(Clone, Debug)]
pub struct FiniteModel {
    pub domain: usize,
    /// `unary[s][i]` is true iff `s` holds of element `i`. Length `domain`.
    pub unary: BTreeMap<String, Vec<bool>>,
    /// `binary[s][i][j]` is true iff `s` relates `i` to `j`. `domain` rows of
    /// `domain`.
    pub binary: BTreeMap<String, Vec<Vec<bool>>>,
    pub consts: BTreeMap<String, usize>,
    /// Which solver said this. UNTRUSTED: echoed into the report and into
    /// `model.tsv`'s `source` line, and nothing checks it.
    pub source: &'static str,
    /// Symbols the solver interpreted that the translation never emitted, so
    /// this structure is a reduct of the solver's. Reported, not hidden.
    pub dropped: Vec<String>,
}

/// A model that could not be read. Every arm names what was being read and
/// what was wrong with it; none of them is recoverable by defaulting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IngestError {
    /// The solver's output held no model at all where one was expected.
    NoModel { solver: &'static str, saw: String },
    /// The S-expression or the LADR block did not parse.
    Syntax { solver: &'static str, at: String, why: String },
    /// A symbol the PROBLEM uses has no interpretation in the model. This is
    /// the attribution failure `Fol.covers` would catch one step later; it is
    /// caught here so the message can name the solver.
    Uninterpreted { solver: &'static str, symbol: String, arity: &'static str },
    /// The model evaluator met a construct it does not know. Named rather than
    /// approximated, because an approximated model is a different model.
    Unsupported { solver: &'static str, construct: String, context: String },
    /// A carrier index the solver printed is outside the carrier it declared.
    OutOfRange { solver: &'static str, symbol: String, index: usize, domain: usize },
    /// A table whose length is not what the declared carrier requires.
    BadWidth { solver: &'static str, symbol: String, want: usize, got: usize },
    /// A carrier of zero elements. A first-order structure cannot have one,
    /// and `Fol.FinModel` is defined only at `n+1`.
    EmptyDomain { solver: &'static str },
}

impl std::fmt::Display for IngestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IngestError::NoModel { solver, saw } => write!(
                f,
                "{solver} printed no model where one was expected; it said: {saw}"
            ),
            IngestError::Syntax { solver, at, why } => {
                write!(f, "could not parse {solver}'s model near {at:?}: {why}")
            }
            IngestError::Uninterpreted { solver, symbol, arity } => write!(
                f,
                "{solver} returned no interpretation for the {arity} symbol {symbol:?}, which \
                 the problem uses. A missing row would DEFAULT in the checker rather than fail, \
                 and a defaulted structure is not the one the solver described, so this is an \
                 error here"
            ),
            IngestError::Unsupported { solver, construct, context } => write!(
                f,
                "{solver}'s model uses {construct:?}, which this evaluator does not know, in \
                 {context}. Approximating it would certify a DIFFERENT structure from the one \
                 the solver built"
            ),
            IngestError::OutOfRange { solver, symbol, index, domain } => write!(
                f,
                "{solver} interpreted {symbol:?} at carrier element {index}, outside the \
                 declared carrier of {domain}"
            ),
            IngestError::BadWidth { solver, symbol, want, got } => write!(
                f,
                "{solver}'s table for {symbol:?} has {got} entries where the declared carrier \
                 requires {want}"
            ),
            IngestError::EmptyDomain { solver } => write!(
                f,
                "{solver} reported a carrier of zero elements; a first-order structure cannot \
                 have one"
            ),
        }
    }
}

impl IngestError {
    /// Re-attribute the error to the solver whose output was actually being
    /// read.
    ///
    /// Every arm carries the solver's name because the message has to say
    /// whose output could not be read, and the SMT-LIB reader below is shared
    /// between Z3 and cvc5 because both print standard `(get-model)` output.
    /// Sharing the reader without this would print "could not parse z3's
    /// model" over cvc5's bytes, which is a mis-attribution of exactly the kind
    /// `Fol.covers` is labelled `attribution` to avoid. Naming the wrong tool
    /// in a diagnostic is cheap to do and expensive to debug.
    pub fn with_solver(self, solver: &'static str) -> IngestError {
        match self {
            IngestError::NoModel { saw, .. } => IngestError::NoModel { solver, saw },
            IngestError::Syntax { at, why, .. } => IngestError::Syntax { solver, at, why },
            IngestError::Uninterpreted { symbol, arity, .. } => {
                IngestError::Uninterpreted { solver, symbol, arity }
            }
            IngestError::Unsupported { construct, context, .. } => {
                IngestError::Unsupported { solver, construct, context }
            }
            IngestError::OutOfRange { symbol, index, domain, .. } => {
                IngestError::OutOfRange { solver, symbol, index, domain }
            }
            IngestError::BadWidth { symbol, want, got, .. } => {
                IngestError::BadWidth { solver, symbol, want, got }
            }
            IngestError::EmptyDomain { .. } => IngestError::EmptyDomain { solver },
        }
    }
}

impl std::error::Error for IngestError {}

impl FiniteModel {
    /// `model.tsv`, the file `oo-folmodel` reads.
    ///
    /// `digest` binds it to a `problem.tsv`; the Lean recomputes that digest
    /// from the problem and refuses on a mismatch, so a model pasted next to
    /// the wrong problem is exit 1 with `problem_digest_mismatch` rather than
    /// a silently meaningless run.
    pub fn to_model_tsv(&self, digest: &str, search: &[u32]) -> String {
        let mut s = String::new();
        let _ = writeln!(s, "domain\t{}", self.domain);
        let _ = writeln!(s, "problem\t{digest}");
        let _ = writeln!(s, "source\t{}", self.source);
        for k in self.unary.keys() {
            let _ = writeln!(s, "decl1\t{k}");
        }
        for k in self.binary.keys() {
            let _ = writeln!(s, "decl2\t{k}");
        }
        for k in self.consts.keys() {
            let _ = writeln!(s, "declc\t{k}");
        }
        for (k, row) in &self.unary {
            let _ = writeln!(s, "p1\t{k}\t{}", bits(row));
        }
        for (k, rows) in &self.binary {
            for (i, row) in rows.iter().enumerate() {
                let _ = writeln!(s, "p2\t{k}\t{i}\t{}", bits(row));
            }
        }
        for (k, i) in &self.consts {
            let _ = writeln!(s, "const\t{k}\t{i}");
        }
        if !search.is_empty() {
            let strs: Vec<String> = search.iter().map(|k| k.to_string()).collect();
            let _ = writeln!(s, "cardinality_search\t{}", strs.join(","));
        }
        s
    }

    /// Check the shape before writing it out: every table the declared carrier
    /// size, every constant inside it.
    ///
    /// The Lean parser checks all of this again and exits 2 on any of it. Doing
    /// it here as well is not redundancy for its own sake: an exit 2 is
    /// "unreadable", which is not a verdict in either direction, so a writer
    /// bug that produced one would show up as a mysterious unreadable file
    /// rather than as a named defect in this module.
    pub fn self_check(&self, solver: &'static str) -> Result<(), IngestError> {
        if self.domain == 0 {
            return Err(IngestError::EmptyDomain { solver });
        }
        for (k, row) in &self.unary {
            if row.len() != self.domain {
                return Err(IngestError::BadWidth {
                    solver,
                    symbol: k.clone(),
                    want: self.domain,
                    got: row.len(),
                });
            }
        }
        for (k, rows) in &self.binary {
            if rows.len() != self.domain {
                return Err(IngestError::BadWidth {
                    solver,
                    symbol: k.clone(),
                    want: self.domain,
                    got: rows.len(),
                });
            }
            for row in rows {
                if row.len() != self.domain {
                    return Err(IngestError::BadWidth {
                        solver,
                        symbol: k.clone(),
                        want: self.domain,
                        got: row.len(),
                    });
                }
            }
        }
        for (k, i) in &self.consts {
            if *i >= self.domain {
                return Err(IngestError::OutOfRange {
                    solver,
                    symbol: k.clone(),
                    index: *i,
                    domain: self.domain,
                });
            }
        }
        Ok(())
    }

    /// Every symbol the problem uses is interpreted here. The checker's
    /// `Fol.covers` gate asks the same question one step later; asking it here
    /// too is what lets the error name the solver that omitted the symbol.
    pub fn covers(&self, v: &Vocabulary, solver: &'static str) -> Result<(), IngestError> {
        for s in &v.unary {
            if !self.unary.contains_key(s) {
                return Err(IngestError::Uninterpreted {
                    solver,
                    symbol: s.clone(),
                    arity: "unary",
                });
            }
        }
        for s in &v.binary {
            if !self.binary.contains_key(s) {
                return Err(IngestError::Uninterpreted {
                    solver,
                    symbol: s.clone(),
                    arity: "binary",
                });
            }
        }
        for s in &v.consts {
            if !self.consts.contains_key(s) {
                return Err(IngestError::Uninterpreted {
                    solver,
                    symbol: s.clone(),
                    arity: "constant",
                });
            }
        }
        Ok(())
    }
}

fn bits(row: &[bool]) -> String {
    row.iter().map(|b| if *b { '1' } else { '0' }).collect()
}

// ── Front end 1: Z3, an SMT-LIB model ───────────────────────────────────────

/// Reading `(get-model)` back from an SMT solver over the finite encoding.
///
/// Z3 prints a list of `define-fun`s whose bodies are TERMS, not tables, so
/// the reading is an evaluation: the body of `p` is applied at every carrier
/// element and the resulting bit vector is the row. Measured shapes on
/// Z3 4.16.0 over an enumeration sort, all four of which appear in practice:
///
/// ```text
/// (define-fun |c:Person| ((x!0 U)) Bool true)
/// (define-fun |c:Company| ((x!0 U)) Bool (ite (= x!0 e1) true false))
/// (define-fun p ((x!0 U)) Bool (and (not (= x!0 e1)) (not (= x!0 e3))))
/// (define-fun r ((x!0 U) (x!1 U)) Bool
///   (or (and (= x!0 e1) (= x!1 e0)) (and (not (= x!0 e1)) (= x!1 e1))))
/// ```
///
/// The evaluator therefore has to know `ite`, `=`, `and`, `or`, `not` and the
/// two truth constants, and it also knows `=>`, `xor`, `distinct` and `let`
/// because they are cheap and Z3 emits them on larger models. A head it does
/// NOT know is [`IngestError::Unsupported`] naming the head, never an
/// approximation: a structure guessed at is a different structure, and
/// certifying it would say something true about the wrong object.
pub mod z3 {
    use super::*;

    const SOLVER: &str = "z3";

    /// A minimal S-expression.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum Sexp {
        Atom(String),
        List(Vec<Sexp>),
    }

    impl Sexp {
        fn atom(&self) -> Option<&str> {
            match self {
                Sexp::Atom(a) => Some(a),
                Sexp::List(_) => None,
            }
        }
        fn list(&self) -> Option<&[Sexp]> {
            match self {
                Sexp::List(l) => Some(l),
                Sexp::Atom(_) => None,
            }
        }
    }

    /// Parse every top-level S-expression in `text`.
    ///
    /// Handles `|quoted symbols|`, `"strings"` and `;` line comments, because
    /// Z3's model output contains all three: the symbols this repository emits
    /// are `|…|` and Z3 annotates the unbounded encoding's universe with `;;`
    /// comment lines.
    pub fn parse_sexps(text: &str) -> Result<Vec<Sexp>, IngestError> {
        let b: Vec<char> = text.chars().collect();
        let mut i = 0usize;
        let mut out = Vec::new();
        loop {
            skip_ws(&b, &mut i);
            if i >= b.len() {
                return Ok(out);
            }
            out.push(parse_one(&b, &mut i)?);
        }
    }

    fn skip_ws(b: &[char], i: &mut usize) {
        loop {
            while *i < b.len() && b[*i].is_whitespace() {
                *i += 1;
            }
            if *i < b.len() && b[*i] == ';' {
                while *i < b.len() && b[*i] != '\n' {
                    *i += 1;
                }
                continue;
            }
            return;
        }
    }

    fn parse_one(b: &[char], i: &mut usize) -> Result<Sexp, IngestError> {
        skip_ws(b, i);
        if *i >= b.len() {
            return Err(IngestError::Syntax {
                solver: SOLVER,
                at: "end of output".into(),
                why: "the expression ended in the middle of a form".into(),
            });
        }
        if b[*i] == '(' {
            *i += 1;
            let mut items = Vec::new();
            loop {
                skip_ws(b, i);
                if *i >= b.len() {
                    return Err(IngestError::Syntax {
                        solver: SOLVER,
                        at: "end of output".into(),
                        why: "unclosed (".into(),
                    });
                }
                if b[*i] == ')' {
                    *i += 1;
                    return Ok(Sexp::List(items));
                }
                items.push(parse_one(b, i)?);
            }
        }
        if b[*i] == ')' {
            return Err(IngestError::Syntax {
                solver: SOLVER,
                at: ")".into(),
                why: "a closing bracket with nothing open".into(),
            });
        }
        // A `|quoted symbol|` runs to the matching bar and admits everything
        // except `|` and `\`, so there is no escape to honour inside it.
        if b[*i] == '|' {
            *i += 1;
            let start = *i;
            while *i < b.len() && b[*i] != '|' {
                *i += 1;
            }
            if *i >= b.len() {
                return Err(IngestError::Syntax {
                    solver: SOLVER,
                    at: b[start..].iter().take(40).collect::<String>(),
                    why: "unterminated |quoted symbol|".into(),
                });
            }
            let s: String = b[start..*i].iter().collect();
            *i += 1;
            return Ok(Sexp::Atom(s));
        }
        if b[*i] == '"' {
            *i += 1;
            let start = *i;
            while *i < b.len() && b[*i] != '"' {
                *i += 1;
            }
            let s: String = b[start..*i.min(&mut b.len().clone())].iter().collect();
            if *i < b.len() {
                *i += 1;
            }
            return Ok(Sexp::Atom(s));
        }
        let start = *i;
        while *i < b.len() && !b[*i].is_whitespace() && b[*i] != '(' && b[*i] != ')' {
            *i += 1;
        }
        Ok(Sexp::Atom(b[start..*i].iter().collect()))
    }

    /// A value in a model body: a truth value or a carrier element.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Val {
        B(bool),
        E(usize),
    }

    struct Defn<'a> {
        args: Vec<String>,
        /// The declared return sort, `Bool` or `U`. Kept because it is the
        /// cheapest guard against reading a `define-fun` off by one: the first
        /// version of this parser took `l[3]` for the body, which is the SORT,
        /// and every ingestion failed with `z3's model uses "Bool"`. The
        /// checker would have caught the resulting structure, but the message
        /// is better here.
        sort: String,
        body: &'a Sexp,
    }

    /// `e17` → 17. The enumeration constructors the finite encoding declares.
    fn element(a: &str, domain: usize) -> Option<usize> {
        let n: usize = a.strip_prefix('e')?.parse().ok()?;
        if n < domain { Some(n) } else { None }
    }

    #[allow(clippy::too_many_arguments)]
    fn eval(
        e: &Sexp,
        env: &BTreeMap<String, Val>,
        defs: &BTreeMap<String, Defn<'_>>,
        domain: usize,
        depth: usize,
        context: &str,
    ) -> Result<Val, IngestError> {
        if depth == 0 {
            return Err(IngestError::Unsupported {
                solver: SOLVER,
                construct: "a definition chain deeper than 64".into(),
                context: context.to_string(),
            });
        }
        match e {
            Sexp::Atom(a) => {
                if a == "true" {
                    return Ok(Val::B(true));
                }
                if a == "false" {
                    return Ok(Val::B(false));
                }
                if let Some(v) = env.get(a) {
                    return Ok(*v);
                }
                if let Some(n) = element(a, domain) {
                    return Ok(Val::E(n));
                }
                if let Some(d) = defs.get(a)
                    && d.args.is_empty()
                {
                    return eval(d.body, &BTreeMap::new(), defs, domain, depth - 1, context);
                }
                Err(IngestError::Unsupported {
                    solver: SOLVER,
                    construct: a.clone(),
                    context: context.to_string(),
                })
            }
            Sexp::List(items) => {
                let (head, args) = items.split_first().ok_or(IngestError::Syntax {
                    solver: SOLVER,
                    at: "()".into(),
                    why: "an empty application".into(),
                })?;
                let h = head.atom().ok_or_else(|| IngestError::Unsupported {
                    solver: SOLVER,
                    construct: "an application whose head is itself a list".into(),
                    context: context.to_string(),
                })?;
                let ev = |x: &Sexp| eval(x, env, defs, domain, depth - 1, context);
                match h {
                    "not" => match ev(&args[0])? {
                        Val::B(b) => Ok(Val::B(!b)),
                        Val::E(_) => Err(unsupported("not applied to a carrier element", context)),
                    },
                    "and" | "or" => {
                        let mut acc = h == "and";
                        for a in args {
                            match ev(a)? {
                                Val::B(b) => {
                                    if h == "and" {
                                        acc &= b
                                    } else {
                                        acc |= b
                                    }
                                }
                                Val::E(_) => {
                                    return Err(unsupported(
                                        "and/or applied to a carrier element",
                                        context,
                                    ));
                                }
                            }
                        }
                        Ok(Val::B(acc))
                    }
                    "xor" => {
                        let mut acc = false;
                        for a in args {
                            match ev(a)? {
                                Val::B(b) => acc ^= b,
                                Val::E(_) => {
                                    return Err(unsupported(
                                        "xor applied to a carrier element",
                                        context,
                                    ));
                                }
                            }
                        }
                        Ok(Val::B(acc))
                    }
                    "=>" => {
                        // Right-associative in SMT-LIB, and `(=> a)` is `a`.
                        let mut vals = Vec::new();
                        for a in args {
                            match ev(a)? {
                                Val::B(b) => vals.push(b),
                                Val::E(_) => {
                                    return Err(unsupported(
                                        "=> applied to a carrier element",
                                        context,
                                    ));
                                }
                            }
                        }
                        let mut acc = *vals.last().unwrap_or(&true);
                        for v in vals.iter().rev().skip(1) {
                            acc = !v || acc;
                        }
                        Ok(Val::B(acc))
                    }
                    "=" => {
                        let first = ev(&args[0])?;
                        let mut all = true;
                        for a in &args[1..] {
                            all &= ev(a)? == first;
                        }
                        Ok(Val::B(all))
                    }
                    "distinct" => {
                        let mut vals = Vec::new();
                        for a in args {
                            vals.push(ev(a)?);
                        }
                        let mut ok = true;
                        for i in 0..vals.len() {
                            for j in (i + 1)..vals.len() {
                                ok &= vals[i] != vals[j];
                            }
                        }
                        Ok(Val::B(ok))
                    }
                    "ite" => match ev(&args[0])? {
                        Val::B(true) => ev(&args[1]),
                        Val::B(false) => ev(&args[2]),
                        Val::E(_) => Err(unsupported("ite with a non-boolean test", context)),
                    },
                    "let" => {
                        let binds = args[0].list().ok_or_else(|| {
                            unsupported("a let whose bindings are not a list", context)
                        })?;
                        let mut inner = env.clone();
                        for b in binds {
                            let pair = b.list().ok_or_else(|| {
                                unsupported("a let binding that is not a pair", context)
                            })?;
                            let name = pair
                                .first()
                                .and_then(Sexp::atom)
                                .ok_or_else(|| unsupported("a nameless let binding", context))?;
                            // Bindings are simultaneous in SMT-LIB, so each is
                            // evaluated in the OUTER environment.
                            let v = eval(&pair[1], env, defs, domain, depth - 1, context)?;
                            inner.insert(name.to_string(), v);
                        }
                        eval(&args[1], &inner, defs, domain, depth - 1, context)
                    }
                    other => {
                        // An auxiliary function Z3 defined earlier in the same
                        // model. Anything else is refused by name.
                        if let Some(d) = defs.get(other)
                            && d.args.len() == args.len()
                        {
                            let mut inner = BTreeMap::new();
                            for (n, a) in d.args.iter().zip(args) {
                                inner.insert(n.clone(), ev(a)?);
                            }
                            return eval(d.body, &inner, defs, domain, depth - 1, context);
                        }
                        Err(IngestError::Unsupported {
                            solver: SOLVER,
                            construct: other.to_string(),
                            context: context.to_string(),
                        })
                    }
                }
            }
        }
    }

    fn unsupported(what: &str, context: &str) -> IngestError {
        IngestError::Unsupported {
            solver: SOLVER,
            construct: what.to_string(),
            context: context.to_string(),
        }
    }

    /// The declared shape of a `define-fun` against the shape the problem
    /// says the symbol has. A mismatch is a defect in this parser or a change
    /// in Z3's output, and either way it is named rather than worked around.
    fn check_sort(
        d: &Defn<'_>,
        want_sort: &str,
        want_args: usize,
        symbol: &str,
    ) -> Result<(), IngestError> {
        if d.sort != want_sort || d.args.len() != want_args {
            return Err(IngestError::Unsupported {
                solver: SOLVER,
                construct: format!(
                    "a define-fun of {} argument(s) returning {:?}",
                    d.args.len(),
                    d.sort
                ),
                context: format!(
                    "the symbol {symbol:?}, which the problem uses with {want_args} \
                     argument(s) and result sort {want_sort}"
                ),
            });
        }
        Ok(())
    }

    /// Turn Z3's `(get-model)` output into a structure over `0 .. domain-1`.
    ///
    /// `vocab` is the problem's own vocabulary. Only those symbols are read;
    /// everything else Z3 defined is the reduct and is reported in
    /// [`FiniteModel::dropped`].
    pub fn parse_model(
        text: &str,
        vocab: &Vocabulary,
        domain: usize,
    ) -> Result<FiniteModel, IngestError> {
        if domain == 0 {
            return Err(IngestError::EmptyDomain { solver: SOLVER });
        }
        let forms = parse_sexps(text)?;
        // Z3 prints the model as one top-level list of define-funs. Older
        // spellings wrap it in `(model …)`. Take whichever is there.
        let items: Vec<&Sexp> = forms
            .iter()
            .flat_map(|f| match f {
                Sexp::List(l) => {
                    if l.first().and_then(Sexp::atom) == Some("model") {
                        l[1..].iter().collect::<Vec<_>>()
                    } else {
                        l.iter().collect::<Vec<_>>()
                    }
                }
                Sexp::Atom(_) => Vec::new(),
            })
            .collect();

        let mut defs: BTreeMap<String, Defn<'_>> = BTreeMap::new();
        let mut order: Vec<String> = Vec::new();
        for it in &items {
            let l = match it.list() {
                Some(l) => l,
                None => continue,
            };
            // `(define-fun NAME ((arg sort)…) RETURN-SORT BODY)`: five
            // elements, and the body is the LAST of them.
            if l.first().and_then(Sexp::atom) != Some("define-fun") || l.len() < 5 {
                continue;
            }
            let name = match l[1].atom() {
                Some(n) => n.to_string(),
                None => continue,
            };
            let mut args = Vec::new();
            if let Some(params) = l[2].list() {
                for p in params {
                    let pair = p.list().ok_or_else(|| IngestError::Syntax {
                        solver: SOLVER,
                        at: name.clone(),
                        why: "a parameter that is not a (name sort) pair".into(),
                    })?;
                    args.push(
                        pair.first()
                            .and_then(Sexp::atom)
                            .ok_or_else(|| IngestError::Syntax {
                                solver: SOLVER,
                                at: name.clone(),
                                why: "a nameless parameter".into(),
                            })?
                            .to_string(),
                    );
                }
            }
            let sort = l[3].atom().unwrap_or("").to_string();
            order.push(name.clone());
            defs.insert(name, Defn { args, sort, body: &l[4] });
        }
        if defs.is_empty() {
            return Err(IngestError::NoModel {
                solver: SOLVER,
                saw: text.lines().take(3).collect::<Vec<_>>().join(" / "),
            });
        }

        let mut m = FiniteModel {
            domain,
            unary: BTreeMap::new(),
            binary: BTreeMap::new(),
            consts: BTreeMap::new(),
            source: "z3",
            dropped: Vec::new(),
        };
        let wanted: BTreeSet<&String> =
            vocab.unary.iter().chain(&vocab.binary).chain(&vocab.consts).collect();
        for name in &order {
            if !wanted.contains(name) {
                m.dropped.push(name.clone());
            }
        }

        for s in &vocab.unary {
            let d = defs.get(s).ok_or_else(|| IngestError::Uninterpreted {
                solver: SOLVER,
                symbol: s.clone(),
                arity: "unary",
            })?;
            check_sort(d, "Bool", 1, s)?;
            let mut row = Vec::with_capacity(domain);
            for i in 0..domain {
                let mut env = BTreeMap::new();
                if let Some(a) = d.args.first() {
                    env.insert(a.clone(), Val::E(i));
                }
                match eval(d.body, &env, &defs, domain, 64, s)? {
                    Val::B(b) => row.push(b),
                    Val::E(_) => {
                        return Err(unsupported("a unary predicate returning a carrier element", s));
                    }
                }
            }
            m.unary.insert(s.clone(), row);
        }
        for s in &vocab.binary {
            let d = defs.get(s).ok_or_else(|| IngestError::Uninterpreted {
                solver: SOLVER,
                symbol: s.clone(),
                arity: "binary",
            })?;
            check_sort(d, "Bool", 2, s)?;
            let mut rows = Vec::with_capacity(domain);
            for i in 0..domain {
                let mut row = Vec::with_capacity(domain);
                for j in 0..domain {
                    let mut env = BTreeMap::new();
                    if d.args.len() >= 2 {
                        env.insert(d.args[0].clone(), Val::E(i));
                        env.insert(d.args[1].clone(), Val::E(j));
                    }
                    match eval(d.body, &env, &defs, domain, 64, s)? {
                        Val::B(b) => row.push(b),
                        Val::E(_) => {
                            return Err(unsupported(
                                "a binary predicate returning a carrier element",
                                s,
                            ));
                        }
                    }
                }
                rows.push(row);
            }
            m.binary.insert(s.clone(), rows);
        }
        for s in &vocab.consts {
            let d = defs.get(s).ok_or_else(|| IngestError::Uninterpreted {
                solver: SOLVER,
                symbol: s.clone(),
                arity: "constant",
            })?;
            check_sort(d, "U", 0, s)?;
            match eval(d.body, &BTreeMap::new(), &defs, domain, 64, s)? {
                Val::E(n) => {
                    m.consts.insert(s.clone(), n);
                }
                Val::B(_) => {
                    return Err(unsupported("a constant of sort U evaluating to a boolean", s));
                }
            }
        }
        m.self_check(SOLVER)?;
        Ok(m)
    }
}

// ── Front end 1b: cvc5, the same SMT-LIB 2 surface syntax ───────────────────

/// Reading cvc5's `(get-model)` output, which is the SAME format Z3 prints and
/// is therefore read by the SAME parser above.
///
/// This module is four lines of delegation and a page of measured facts,
/// because the interesting differences between the two solvers are not in the
/// syntax and a reader who assumed they were would get every one of them wrong.
///
/// # Why the parser is shared rather than copied
///
/// `(define-fun NAME ((arg sort)…) RETURN-SORT BODY)` is SMT-LIB 2.6 and both
/// solvers emit it. Measured on cvc5 1.3.4 and Z3 4.16.0 over the engine's own
/// `finite(2)` export, the only differences in the bytes are cosmetic: cvc5
/// names its parameters `_arg_1` or `$x1` where Z3 uses `x!0`, and cvc5 puts
/// the body on the same line. The parser reads parameter names out of the file
/// rather than assuming any of them, so neither spelling matters. A second
/// parser here would be a second thing to keep correct for no gain.
///
/// # THE TRAP, and it is not a small one
///
/// **cvc5 prints a model block after `unknown`, and that block is not a
/// model.** Measured on cvc5 1.3.4, default options, over a problem asserting
/// `(forall ((X0 U)) (thing X0))` on a two-element datatype carrier: cvc5
/// answered `unknown` and then printed `(define-fun thing ((_arg_1 U)) Bool
/// false)`, a structure that falsifies an asserted axiom. Z3 in the same
/// position prints `(error "model is not available")`.
///
/// So the status line is load-bearing in a way it is not for Z3, and
/// [`crate::fol_solve::solve`] only ingests on `sat` for exactly this reason.
/// If it ever ingested on `unknown` the verified checker would reject the
/// structure, the run would report `satisfiable_oracle` with a
/// `model_not_confirmed` disagreement, and the engine would be blaming cvc5 for
/// a structure cvc5 never claimed was a model. That is a false stop-the-line,
/// which costs as much credibility as a missed one.
///
/// # What cvc5 answers, and the flag that changes it
///
/// With default options cvc5 answers `unknown` on the engine's quantified
/// problems, including ones whose carrier is a one-element datatype, because
/// its default quantifier strategy is E-matching and E-matching is incomplete.
/// `--finite-model-find` makes it answer. On the FINITE encoding that flag
/// adds no assumption at all, since `(declare-datatypes ((U 0)) ((e0) … ))`
/// already fixes a finite carrier of known size, so "look for a finite model"
/// is the question the file asks. On the UNBOUNDED encoding it would be a
/// different question, and reporting its answer under the unbounded file's name
/// would be the `no_model_up_to_size_k` mistake of decision 0006 item 4 wearing
/// a solver flag instead of a cardinality constraint.
///
/// # What is NOT read here
///
/// cvc5's unbounded model, exactly as Z3's unbounded model is not read. It
/// prints its carrier elements as `(as @U_0 U)` and annotates the block with
/// `; cardinality of U is 1`, so the values are qualified identifiers rather
/// than the enumeration constructors `e0 … e(k-1)` this evaluator understands.
/// Such a model reaches [`IngestError::Unsupported`], which names the construct
/// rather than approximating it, and the run reports `satisfiable_oracle` with
/// nothing certified. Decision 0006 already lists ingesting the unbounded
/// encoding's models as open work; cvc5 does not close it and does not widen it.
pub mod cvc5 {
    use super::{FiniteModel, IngestError, Vocabulary};

    pub const SOLVER: &str = "cvc5";

    /// Turn cvc5's `(get-model)` output into a structure over `0 .. domain-1`.
    ///
    /// The provenance is rewritten on both the success and the failure path.
    /// `FiniteModel::source` is echoed into `model.tsv`'s `source` line, which
    /// decision 0006 item 6 records as untrusted and reported; untrusted is not
    /// a licence to write the name of a solver that did not produce the file.
    pub fn parse_model(
        text: &str,
        vocab: &Vocabulary,
        domain: usize,
    ) -> Result<FiniteModel, IngestError> {
        match super::z3::parse_model(text, vocab, domain) {
            Ok(m) => Ok(FiniteModel { source: SOLVER, ..m }),
            Err(e) => Err(e.with_solver(SOLVER)),
        }
    }
}

// ── Front end 2: Mace4, a LADR interpretation ───────────────────────────────

/// Reading Mace4's `interpretation(…)` block.
///
/// Nothing is evaluated here: LADR prints DENSE INTEGER TABLES and the reading
/// is an indexing. The measured shape, LADR 2009-11A:
///
/// ```text
/// interpretation( 2, [number=1, seconds=0], [
///         function(c0, [ 0 ]),
///         function(f1(_), [ 1, 0 ]),
///         relation(p0(_), [ 1, 0 ]),
///         relation(r0(_,_), [ 0, 1,
///                             0, 0 ])
/// ]).
/// ```
///
/// Arity is read off the `(_)` in the name. A binary table is ROW MAJOR: entry
/// `i*n + j` is `r(i, j)`, verified against a run whose only true pair was
/// `r0(0,1)` and whose table was `[0, 1, 0, 0]`.
///
/// `f1` is a Skolem function and is dropped. Mace4 also invents Skolem
/// CONSTANTS, and it names them `c1, c2, …`, which is the same space this
/// repository's mangler uses. Measured: given input already mentioning `c0`
/// and `c1`, Mace4 named its Skolem constant `c2`, so it picks a name not
/// already in the problem and no collision occurs. The demangling does not
/// rely on that anyway: a name the [`SymbolTable`] never issued is dropped
/// whatever it is called, and a symbol the table DID issue that Mace4 did not
/// interpret is [`IngestError::Uninterpreted`]. That second half is the
/// mechanical guard against LADR's variable-letter trap, in which a mangled
/// constant becomes a variable and simply disappears from the model.
pub mod mace4 {
    use super::*;

    const SOLVER: &str = "mace4";

    struct Entry {
        name: String,
        arity: usize,
        table: Vec<i64>,
        is_relation: bool,
    }

    /// The first `interpretation(…)` block, or `None` if there is none.
    fn first_block(text: &str) -> Option<&str> {
        let start = text.find("interpretation(")?;
        let rest = &text[start..];
        let bytes = rest.as_bytes();
        let mut depth = 0usize;
        for (i, b) in bytes.iter().enumerate() {
            match b {
                b'(' => depth += 1,
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(&rest[..=i]);
                    }
                }
                _ => {}
            }
        }
        None
    }

    fn parse_entries(block: &str) -> Result<(usize, Vec<Entry>), IngestError> {
        // interpretation( N , [stats] , [ entries ] )
        let inner = block
            .strip_prefix("interpretation(")
            .and_then(|s| s.strip_suffix(')'))
            .ok_or_else(|| IngestError::Syntax {
                solver: SOLVER,
                at: block.chars().take(40).collect::<String>(),
                why: "not an interpretation( … ) form".into(),
            })?;
        let comma = inner.find(',').ok_or_else(|| IngestError::Syntax {
            solver: SOLVER,
            at: inner.chars().take(40).collect::<String>(),
            why: "no comma after the carrier size".into(),
        })?;
        let domain: usize =
            inner[..comma].trim().parse().map_err(|_| IngestError::Syntax {
                solver: SOLVER,
                at: inner[..comma].trim().to_string(),
                why: "the carrier size is not a number".into(),
            })?;

        let mut entries = Vec::new();
        let chars: Vec<char> = inner.chars().collect();
        let mut i = comma;
        while i < chars.len() {
            let rest: String = chars[i..].iter().collect();
            let (kind, is_relation) = match (rest.find("function("), rest.find("relation(")) {
                (None, None) => break,
                (Some(f), None) => (f, false),
                (None, Some(r)) => (r, true),
                (Some(f), Some(r)) => {
                    if f < r {
                        (f, false)
                    } else {
                        (r, true)
                    }
                }
            };
            let head_len = if is_relation { "relation(".len() } else { "function(".len() };
            let entry_start = i + rest[..kind].chars().count() + head_len;
            // name runs to the comma at depth 0
            let mut j = entry_start;
            let mut depth = 0usize;
            while j < chars.len() {
                match chars[j] {
                    '(' => depth += 1,
                    ')' => {
                        if depth == 0 {
                            break;
                        }
                        depth -= 1;
                    }
                    ',' if depth == 0 => break,
                    _ => {}
                }
                j += 1;
            }
            let raw_name: String = chars[entry_start..j].iter().collect();
            let raw_name = raw_name.trim().to_string();
            let (name, arity) = match raw_name.find('(') {
                None => (raw_name.clone(), 0usize),
                Some(p) => {
                    let base = raw_name[..p].to_string();
                    let args = &raw_name[p + 1..raw_name.len().saturating_sub(1)];
                    (base, args.split(',').filter(|s| !s.trim().is_empty()).count())
                }
            };
            // table runs from the next '[' to its matching ']'
            let open = chars[j..].iter().position(|c| *c == '[').ok_or_else(|| {
                IngestError::Syntax {
                    solver: SOLVER,
                    at: name.clone(),
                    why: "no [ … ] table after the name".into(),
                }
            })? + j;
            let close = chars[open..].iter().position(|c| *c == ']').ok_or_else(|| {
                IngestError::Syntax {
                    solver: SOLVER,
                    at: name.clone(),
                    why: "an unterminated [ … ] table".into(),
                }
            })? + open;
            let body: String = chars[open + 1..close].iter().collect();
            let mut table = Vec::new();
            for tok in body.split(',') {
                let t = tok.trim();
                if t.is_empty() {
                    continue;
                }
                table.push(t.parse::<i64>().map_err(|_| IngestError::Syntax {
                    solver: SOLVER,
                    at: format!("{name} table entry {t:?}"),
                    why: "a table entry that is not an integer".into(),
                })?);
            }
            entries.push(Entry { name, arity, table, is_relation });
            i = close + 1;
        }
        Ok((domain, entries))
    }

    /// Turn Mace4's output into a structure, demangling through `tab`.
    pub fn parse_model(text: &str, tab: &SymbolTable) -> Result<FiniteModel, IngestError> {
        let block = first_block(text).ok_or_else(|| IngestError::NoModel {
            solver: SOLVER,
            saw: text
                .lines()
                .filter(|l| l.contains("exit") || l.contains("Exiting"))
                .take(2)
                .collect::<Vec<_>>()
                .join(" / "),
        })?;
        let (domain, entries) = parse_entries(block)?;
        if domain == 0 {
            return Err(IngestError::EmptyDomain { solver: SOLVER });
        }
        let mut m = FiniteModel {
            domain,
            unary: BTreeMap::new(),
            binary: BTreeMap::new(),
            consts: BTreeMap::new(),
            source: "mace4",
            dropped: Vec::new(),
        };
        for e in entries {
            let (image, kind) = match tab.demangle_kind(&e.name) {
                Some((s, k)) => (s.to_string(), k),
                None => {
                    m.dropped.push(e.name.clone());
                    continue;
                }
            };
            // The TABLE is the authority on the arity, because the table is
            // what wrote the file. A model that answers at a different one is
            // a disagreement about the signature, and reading it into
            // whichever slot the file suggests would file a unary predicate as
            // a perfectly good binary interpretation and then report the unary
            // slot as uninterpreted: a confusing message for a real defect.
            let claimed = match (e.is_relation, e.arity) {
                (true, 1) => Some(SymKind::Unary),
                (true, 2) => Some(SymKind::Binary),
                (false, 0) => Some(SymKind::Constant),
                _ => None,
            };
            if claimed != Some(kind) {
                return Err(IngestError::Unsupported {
                    solver: SOLVER,
                    construct: format!(
                        "{} of arity {}",
                        if e.is_relation { "a relation" } else { "a function" },
                        e.arity
                    ),
                    context: format!(
                        "the symbol {image:?}, mangled to {:?}, which the problem uses as a {} \
                         symbol",
                        e.name,
                        kind.name()
                    ),
                });
            }
            match (e.is_relation, e.arity) {
                (true, 1) => {
                    if e.table.len() != domain {
                        return Err(IngestError::BadWidth {
                            solver: SOLVER,
                            symbol: image,
                            want: domain,
                            got: e.table.len(),
                        });
                    }
                    m.unary.insert(image, e.table.iter().map(|v| *v != 0).collect());
                }
                (true, 2) => {
                    if e.table.len() != domain * domain {
                        return Err(IngestError::BadWidth {
                            solver: SOLVER,
                            symbol: image,
                            want: domain * domain,
                            got: e.table.len(),
                        });
                    }
                    let rows = e
                        .table
                        .chunks(domain)
                        .map(|c| c.iter().map(|v| *v != 0).collect())
                        .collect();
                    m.binary.insert(image, rows);
                }
                (false, 0) => {
                    let v = *e.table.first().ok_or_else(|| IngestError::Syntax {
                        solver: SOLVER,
                        at: image.clone(),
                        why: "a constant with an empty table".into(),
                    })?;
                    if v < 0 || v as usize >= domain {
                        return Err(IngestError::OutOfRange {
                            solver: SOLVER,
                            symbol: image,
                            index: v.max(0) as usize,
                            domain,
                        });
                    }
                    m.consts.insert(image, v as usize);
                }
                // Unreachable: the kind check above admits only the three
                // shapes matched here. Kept as an error rather than an
                // `unreachable!` so that a future edit to that check cannot
                // turn a missed case into a panic in a library.
                (r, a) => {
                    return Err(IngestError::Unsupported {
                        solver: SOLVER,
                        construct: format!(
                            "{} of arity {a}",
                            if r { "a relation" } else { "a function" }
                        ),
                        context: format!("the symbol {image:?}, mangled to {:?}", e.name),
                    });
                }
            }
        }
        m.self_check(SOLVER)?;
        Ok(m)
    }
}
