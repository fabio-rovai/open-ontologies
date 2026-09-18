//! Reading the derivation a prover emits, and re-checking what can honestly be
//! re-checked.
//!
//! Decision 0005 item 5 says an ATP verdict is an ORACLE OPINION, and decision
//! 0006 says a refutation is not a certificate. Neither sentence is retracted
//! here, and this module does not move the boundary: what it does is stop the
//! oracle's answer being a single word. A prover that says `SZS status Theorem`
//! also prints, on request, the derivation it found. That derivation is a
//! finite object. Reading it back turns one unexaminable word into evidence
//! that can be inspected, and some of it into evidence that can be RECOMPUTED.
//!
//! # The three things this earns, in increasing order of what they cost
//!
//! 1. **The prover refuted OUR problem.** Every leaf of the derivation is
//!    matched, by name and by parsed formula, against the problem file this
//!    repository's exporter produced. A prover pointed at the wrong file, a
//!    stale file, or a file whose conjecture was smuggled in as an axiom is
//!    caught here and nowhere else in this codebase. This is the cheapest
//!    check and by some distance the most valuable, because it is the one that
//!    an `AGREE` row in `tools/fol_differential.py` silently assumed.
//! 2. **The derivation is a well-founded DAG ending in `$false`.** Every
//!    parent reference resolves to a node that exists, the parent relation is
//!    acyclic, and the node nothing else cites is the empty clause.
//! 3. **Some steps are replayed.** Binary resolution, factoring, duplicate
//!    literal removal, trivial inequality removal, equality resolution,
//!    associative flattening and the negation of the conjecture are recomputed
//!    from the premises with a small unifier and compared with what the prover
//!    printed. Everything else is NAMED AND COUNTED as unchecked.
//!
//! # What this is NOT
//!
//! It is not a proof of unsatisfiability, and no word in the report says it
//! is. Three separate reasons, all of them live:
//!
//! - **The calculus is not verified.** Even a derivation whose every step this
//!   module recomputes is only a derivation in a calculus whose soundness is
//!   written down nowhere in `lean/`. `Fol.satisfiable_of_check` has a theorem
//!   behind it; this has a Rust program behind it.
//! - **The replayer is not verified either.** The unifier below is ordinary
//!   Rust with ordinary tests. A bug in it makes a wrong step look right.
//! - **Clausification is not checked at all.** `cnf_transformation`,
//!   `ennf_transformation`, `nnf_transformation`, skolemisation, AVATAR
//!   splitting and every SAT-solver step are unchecked and are reported as
//!   unchecked, by rule, with a count. On a real proof they are the majority
//!   of the steps, which is why [`Report::verdict`] reaches
//!   `refutation_partially_replayed` and stops there.
//!
//! The strongest word this module can print is `refutation_fully_replayed`,
//! it requires that NOTHING was left unchecked, and it still is not
//! `unsatisfiable`. The ladder exists so that the distance between what was
//! read and what was recomputed is visible in the output rather than in a
//! footnote.
//!
//! # Why a failed reconstruction is its own word
//!
//! A step whose rule this module implements, whose premises and conclusion are
//! clause-shaped, and which still does not reconstruct is EITHER a defect in
//! the derivation OR a gap in this checker. It is reported as
//! `refutation_step_not_reconstructed` and never as either of those two, for
//! the same reason `tools/fol_differential.py` reports a disagreement and does
//! not adjudicate it. Collapsing it into "unchecked" would let a forged step
//! hide behind a rule name, and collapsing it into "rejected" would claim a
//! defect in someone else's prover on this module's word alone.
//!
//! # Subsumption resolution is checked as binary resolution, and that is exact
//!
//! Vampire prints `forward_subsumption_resolution` for a step whose premises
//! are `C ∨ L` and `D ∨ ¬L'` and whose conclusion is `C`, where `D'σ ⊆ C`. The
//! binary resolvent of those two premises is `C ∪ D'σ`, and `D'σ ⊆ C` makes
//! that exactly `C`. So the conclusion of a subsumption resolution IS the
//! binary resolvent of its premises, and one check covers both rules without
//! weakening either. The same identity is why the check still passes on
//! AVATAR's printed clauses, where the side premise's assertion literals are
//! carried into the conclusion and the classical side condition does not hold
//! of the printed form.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

// ── The term and formula language ──────────────────────────────────────────

/// A first-order term as a prover prints it.
///
/// Deliberately WIDER than [`crate::tptp::Term`], which has no function
/// application because the translation never emits one. A prover's output
/// does: Skolem functions, AVATAR's introduced propositional symbols and
/// TPTP's distinct objects all arrive here. A reader that could not represent
/// them would have to drop them, and a dropped subterm is a different formula.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Term {
    /// A TPTP variable: an upper word. The name is kept verbatim, because
    /// renaming on the way in would make the alpha-equivalence levels in
    /// [`LeafMatch`] impossible to report separately.
    Var(String),
    /// A function application. Arity 0 is a constant.
    Fun(String, Vec<Term>),
}

impl Term {
    fn vars(&self, out: &mut BTreeSet<String>) {
        match self {
            Term::Var(v) => {
                out.insert(v.clone());
            }
            Term::Fun(_, args) => args.iter().for_each(|a| a.vars(out)),
        }
    }

    fn render(&self, s: &mut String) {
        match self {
            Term::Var(v) => s.push_str(v),
            Term::Fun(f, args) => {
                s.push_str(f);
                if !args.is_empty() {
                    s.push('(');
                    for (i, a) in args.iter().enumerate() {
                        if i > 0 {
                            s.push(',');
                        }
                        a.render(s);
                    }
                    s.push(')');
                }
            }
        }
    }
}

/// A first-order formula as a prover prints it.
///
/// `<~>`, `~|` and `~&` are read as `Not(Iff …)`, `Not(Or …)` and
/// `Not(And …)`, and `A <= B` as `Imp(B, A)`. Those are definitional
/// rewritings of the TPTP connectives and not normalisations of the formula:
/// nothing here reorders, flattens or simplifies on the way in, because the
/// leaf check's job is to notice a difference rather than to absorb one.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Formula {
    True,
    False,
    Pred(String, Vec<Term>),
    Eq(Term, Term),
    Not(Box<Formula>),
    And(Box<Formula>, Box<Formula>),
    Or(Box<Formula>, Box<Formula>),
    Imp(Box<Formula>, Box<Formula>),
    Iff(Box<Formula>, Box<Formula>),
    All(String, Box<Formula>),
    Ex(String, Box<Formula>),
}

impl Formula {
    /// A canonical one-line rendering. Used for the problem digest and for
    /// naming a mismatch in a report, never for re-parsing.
    pub fn render(&self) -> String {
        let mut s = String::new();
        self.write(&mut s);
        s
    }

    fn write(&self, s: &mut String) {
        match self {
            Formula::True => s.push_str("$true"),
            Formula::False => s.push_str("$false"),
            Formula::Pred(p, args) => {
                s.push_str(p);
                if !args.is_empty() {
                    s.push('(');
                    for (i, a) in args.iter().enumerate() {
                        if i > 0 {
                            s.push(',');
                        }
                        a.render(s);
                    }
                    s.push(')');
                }
            }
            Formula::Eq(a, b) => {
                a.render(s);
                s.push('=');
                b.render(s);
            }
            Formula::Not(f) => {
                s.push_str("~(");
                f.write(s);
                s.push(')');
            }
            Formula::And(a, b) => binary(s, a, "&", b),
            Formula::Or(a, b) => binary(s, a, "|", b),
            Formula::Imp(a, b) => binary(s, a, "=>", b),
            Formula::Iff(a, b) => binary(s, a, "<=>", b),
            Formula::All(v, f) => {
                let _ = write!(s, "![{v}]:(");
                f.write(s);
                s.push(')');
            }
            Formula::Ex(v, f) => {
                let _ = write!(s, "?[{v}]:(");
                f.write(s);
                s.push(')');
            }
        }
    }

    /// Reassociate `&` and `|` to the right, everywhere.
    ///
    /// Associativity of conjunction and disjunction is not in question, and a
    /// prover's printer is free to bracket an n-ary `&` however it likes. This
    /// exists so that a leaf that differs ONLY in bracketing is reported as
    /// matched at the `associativity_normalised` level rather than as a
    /// mismatch, and so that the level is visible in the report.
    pub fn assoc_normalised(&self) -> Formula {
        fn flat(f: &Formula, and: bool, out: &mut Vec<Formula>) {
            match f {
                Formula::And(a, b) if and => {
                    flat(a, and, out);
                    flat(b, and, out);
                }
                Formula::Or(a, b) if !and => {
                    flat(a, and, out);
                    flat(b, and, out);
                }
                other => out.push(other.assoc_normalised()),
            }
        }
        match self {
            Formula::And(_, _) | Formula::Or(_, _) => {
                let and = matches!(self, Formula::And(_, _));
                let mut parts = Vec::new();
                flat(self, and, &mut parts);
                let mut it = parts.into_iter().rev();
                let mut acc = it.next().expect("a binary node flattens to at least two parts");
                for p in it {
                    acc = if and {
                        Formula::And(Box::new(p), Box::new(acc))
                    } else {
                        Formula::Or(Box::new(p), Box::new(acc))
                    };
                }
                acc
            }
            Formula::Not(f) => Formula::Not(Box::new(f.assoc_normalised())),
            Formula::Imp(a, b) => Formula::Imp(
                Box::new(a.assoc_normalised()),
                Box::new(b.assoc_normalised()),
            ),
            Formula::Iff(a, b) => Formula::Iff(
                Box::new(a.assoc_normalised()),
                Box::new(b.assoc_normalised()),
            ),
            Formula::All(v, f) => Formula::All(v.clone(), Box::new(f.assoc_normalised())),
            Formula::Ex(v, f) => Formula::Ex(v.clone(), Box::new(f.assoc_normalised())),
            other => other.clone(),
        }
    }
}

fn binary(s: &mut String, a: &Formula, op: &str, b: &Formula) {
    s.push('(');
    a.write(s);
    s.push_str(op);
    b.write(s);
    s.push(')');
}

/// Equality up to renaming of BOUND variables. Free variables must agree by
/// name, because a free variable in a TPTP clause is implicitly universally
/// quantified over the clause and its name is the only thing linking its
/// occurrences.
pub fn alpha_eq(a: &Formula, b: &Formula) -> bool {
    fn go(a: &Formula, b: &Formula, l: &mut Vec<String>, r: &mut Vec<String>) -> bool {
        // A bound variable is identified by its distance from the binder, so a
        // name that shadows an outer binder resolves to the inner one, which
        // is what `rposition` gives.
        fn same_term(x: &Term, y: &Term, l: &[String], r: &[String]) -> bool {
            match (x, y) {
                (Term::Var(u), Term::Var(v)) => {
                    match (l.iter().rposition(|n| n == u), r.iter().rposition(|n| n == v)) {
                        (Some(i), Some(j)) => i == j,
                        (None, None) => u == v,
                        _ => false,
                    }
                }
                (Term::Fun(f, xs), Term::Fun(g, ys)) => {
                    f == g
                        && xs.len() == ys.len()
                        && xs.iter().zip(ys).all(|(x, y)| same_term(x, y, l, r))
                }
                _ => false,
            }
        }
        match (a, b) {
            (Formula::True, Formula::True) | (Formula::False, Formula::False) => true,
            (Formula::Pred(p, xs), Formula::Pred(q, ys)) => {
                p == q
                    && xs.len() == ys.len()
                    && xs.iter().zip(ys).all(|(x, y)| same_term(x, y, l, r))
            }
            (Formula::Eq(x1, y1), Formula::Eq(x2, y2)) => {
                same_term(x1, x2, l, r) && same_term(y1, y2, l, r)
            }
            (Formula::Not(f), Formula::Not(g)) => go(f, g, l, r),
            (Formula::And(f1, g1), Formula::And(f2, g2))
            | (Formula::Or(f1, g1), Formula::Or(f2, g2))
            | (Formula::Imp(f1, g1), Formula::Imp(f2, g2))
            | (Formula::Iff(f1, g1), Formula::Iff(f2, g2)) => {
                go(f1, f2, l, r) && go(g1, g2, l, r)
            }
            (Formula::All(u, f), Formula::All(v, g)) | (Formula::Ex(u, f), Formula::Ex(v, g)) => {
                l.push(u.clone());
                r.push(v.clone());
                let ok = go(f, g, l, r);
                l.pop();
                r.pop();
                ok
            }
            _ => false,
        }
    }
    go(a, b, &mut Vec::new(), &mut Vec::new())
}

// ── Lexer ──────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq)]
enum Tok {
    LParen,
    RParen,
    LBrack,
    RBrack,
    Comma,
    Dot,
    Colon,
    Op(&'static str),
    /// A lower word, a dollar word or a number. Names, roles, rules and
    /// predicate symbols all arrive as this.
    Word(String),
    /// A single-quoted TPTP atom, with `\'` and `\\` already unescaped. This
    /// is how every IRI in this repository's export travels.
    Quoted(String),
    /// A double-quoted TPTP DISTINCT OBJECT. Kept with its quotes so that it
    /// can never compare equal to a single-quoted atom of the same text: a
    /// distinct object is pairwise unequal to every other by fiat, which is
    /// exactly the trap decision 0005 records for the TPTP writer.
    Distinct(String),
    Var(String),
}

/// Every operator token, longest-first where one is a prefix of another:
/// `!=` before `!`, `<=>` before `<=`, `~|` before `~`. The lexer scans this
/// list in order and takes the first hit, so the order is the disambiguation.
const OPS: [&str; 13] =
    ["<=>", "<~>", "~|", "~&", "!=", "=>", "<=", "&", "|", "~", "=", "!", "?"];

fn lex(src: &str) -> Result<Vec<Tok>, String> {
    let b: Vec<char> = src.chars().collect();
    let mut i = 0usize;
    let mut out = Vec::new();
    while i < b.len() {
        let c = b[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        // `%` is TPTP's line comment. `#` is E's: it prefixes every line of
        // its own commentary, including the `# SZS output start` marker and
        // the `# SZS output end` one that closes the block, so a reader that
        // did not skip it would choke on the last line of every E proof. A
        // `#` inside an IRI is safe, because a quoted atom is lexed by the
        // branch below and this one never sees the inside of one.
        if c == '%' || c == '#' {
            while i < b.len() && b[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if c == '/' && i + 1 < b.len() && b[i + 1] == '*' {
            i += 2;
            while i + 1 < b.len() && !(b[i] == '*' && b[i + 1] == '/') {
                i += 1;
            }
            i = (i + 2).min(b.len());
            continue;
        }
        if c == '\'' || c == '"' {
            let quote = c;
            let mut s = String::new();
            i += 1;
            loop {
                if i >= b.len() {
                    return Err(format!("unterminated {quote} at end of input"));
                }
                if b[i] == '\\' && i + 1 < b.len() {
                    s.push(b[i + 1]);
                    i += 2;
                    continue;
                }
                if b[i] == quote {
                    i += 1;
                    break;
                }
                s.push(b[i]);
                i += 1;
            }
            out.push(if quote == '\'' { Tok::Quoted(s) } else { Tok::Distinct(format!("\"{s}\"")) });
            continue;
        }
        match c {
            '(' => {
                out.push(Tok::LParen);
                i += 1;
                continue;
            }
            ')' => {
                out.push(Tok::RParen);
                i += 1;
                continue;
            }
            '[' => {
                out.push(Tok::LBrack);
                i += 1;
                continue;
            }
            ']' => {
                out.push(Tok::RBrack);
                i += 1;
                continue;
            }
            ',' => {
                out.push(Tok::Comma);
                i += 1;
                continue;
            }
            '.' => {
                out.push(Tok::Dot);
                i += 1;
                continue;
            }
            ':' => {
                out.push(Tok::Colon);
                i += 1;
                continue;
            }
            _ => {}
        }
        // `!=` must be tried before `!`, `<=>` before `<=`, and `~|` before
        // `~`; OPS is in that order and is scanned in it.
        if let Some(op) = OPS.iter().find(|op| {
            b[i..].len() >= op.len() && b[i..i + op.len()].iter().collect::<String>() == **op
        }) {
            out.push(Tok::Op(op));
            i += op.len();
            continue;
        }
        if c == '$' || c.is_ascii_alphabetic() || c == '_' || c.is_ascii_digit() {
            let start = i;
            if c == '$' {
                i += 1;
                if i < b.len() && b[i] == '$' {
                    i += 1;
                }
            }
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == '_') {
                i += 1;
            }
            let w: String = b[start..i].iter().collect();
            let first = w.chars().find(|ch| ch.is_ascii_alphabetic());
            if first.is_some_and(|ch| ch.is_ascii_uppercase()) && !w.starts_with('$') {
                out.push(Tok::Var(w));
            } else {
                out.push(Tok::Word(w));
            }
            continue;
        }
        if c == '+' || c == '-' {
            let start = i;
            i += 1;
            while i < b.len() && b[i].is_ascii_digit() {
                i += 1;
            }
            if i == start + 1 {
                return Err(format!("unexpected character {c:?}"));
            }
            out.push(Tok::Word(b[start..i].iter().collect()));
            continue;
        }
        return Err(format!("unexpected character {c:?}"));
    }
    Ok(out)
}

// ── The annotated-formula list ─────────────────────────────────────────────

/// One `inference(RULE, [status(…)], [parents])` record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Inference {
    pub rule: String,
    pub parents: Vec<Parent>,
}

/// A parent of an inference step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Parent {
    /// A reference to another annotated formula, by name.
    Named(String),
    /// An inference record written INLINE in the parent position, which E does
    /// routinely. It has a rule and parents of its own and NO FORMULA, so
    /// nothing about it or about the step it feeds can be replayed. It is
    /// given a synthetic node so that its own parent references are still
    /// resolved and its rule is still counted.
    Inline(Box<Inference>),
    /// Anything else in a parent position, kept verbatim so a report can name
    /// it rather than pretend it was understood.
    Other(String),
}

/// The provenance a prover attaches to an annotated formula.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
    /// `file('path', name)`. The second argument is the name the formula had
    /// in the problem file, which is what makes the leaf check possible.
    File { path: String, name: Option<String> },
    Inference(Inference),
    /// `introduced(definition, …)`. A symbol the prover invented. It is NOT
    /// from the problem, it is not derived from the problem, and no
    /// conservativity argument is checked here.
    Introduced { rule: String, detail: String },
    /// A bare name, which some printers use for a trivial re-statement.
    Name(String),
    /// A source this reader did not understand, kept verbatim.
    Other(String),
}

/// One `fof(…)` or `cnf(…)` line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Annotated {
    pub name: String,
    pub role: String,
    pub formula: Formula,
    pub source: Option<Source>,
}

/// A general term, the TPTP grammar's catch-all for annotations.
#[derive(Clone, Debug, PartialEq, Eq)]
enum GTerm {
    Word(String),
    Func(String, Vec<GTerm>),
    List(Vec<GTerm>),
    Other(String),
}

impl GTerm {
    fn word(&self) -> Option<&str> {
        match self {
            GTerm::Word(w) | GTerm::Func(w, _) => Some(w),
            _ => None,
        }
    }
    fn render(&self) -> String {
        match self {
            GTerm::Word(w) => w.clone(),
            GTerm::Other(w) => w.clone(),
            GTerm::Func(f, args) => {
                format!("{f}({})", args.iter().map(GTerm::render).collect::<Vec<_>>().join(","))
            }
            GTerm::List(xs) => {
                format!("[{}]", xs.iter().map(GTerm::render).collect::<Vec<_>>().join(","))
            }
        }
    }
}

struct Parser {
    toks: Vec<Tok>,
    at: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.at)
    }
    fn bump(&mut self) -> Option<Tok> {
        let t = self.toks.get(self.at).cloned();
        if t.is_some() {
            self.at += 1;
        }
        t
    }
    fn eat(&mut self, t: &Tok) -> bool {
        if self.peek() == Some(t) {
            self.at += 1;
            true
        } else {
            false
        }
    }
    fn expect(&mut self, t: &Tok) -> Result<(), String> {
        if self.eat(t) {
            Ok(())
        } else {
            Err(format!("expected {:?}, found {:?}", t, self.peek()))
        }
    }

    fn name(&mut self) -> Result<String, String> {
        match self.bump() {
            Some(Tok::Word(w)) | Some(Tok::Quoted(w)) => Ok(w),
            other => Err(format!("expected a name, found {other:?}")),
        }
    }

    fn term(&mut self) -> Result<Term, String> {
        match self.bump() {
            Some(Tok::Var(v)) => Ok(Term::Var(v)),
            Some(Tok::Distinct(d)) => Ok(Term::Fun(d, Vec::new())),
            Some(Tok::Word(w)) | Some(Tok::Quoted(w)) => {
                let mut args = Vec::new();
                if self.eat(&Tok::LParen) {
                    loop {
                        args.push(self.term()?);
                        if self.eat(&Tok::Comma) {
                            continue;
                        }
                        self.expect(&Tok::RParen)?;
                        break;
                    }
                }
                Ok(Term::Fun(w, args))
            }
            other => Err(format!("expected a term, found {other:?}")),
        }
    }

    /// A unitary formula: a bracketed formula, a negation, a quantification,
    /// or an atom. TPTP makes a quantifier's body unitary, so the body is
    /// parsed here rather than at the binary level.
    fn unit(&mut self) -> Result<Formula, String> {
        match self.peek().cloned() {
            Some(Tok::LParen) => {
                self.at += 1;
                let f = self.formula(0)?;
                self.expect(&Tok::RParen)?;
                Ok(f)
            }
            Some(Tok::Op("~")) => {
                self.at += 1;
                Ok(Formula::Not(Box::new(self.unit()?)))
            }
            Some(Tok::Op(q @ ("!" | "?"))) => {
                self.at += 1;
                self.expect(&Tok::LBrack)?;
                let mut vars = Vec::new();
                loop {
                    match self.bump() {
                        Some(Tok::Var(v)) => vars.push(v),
                        other => return Err(format!("expected a variable, found {other:?}")),
                    }
                    // TFF writes `X : $i`; the sort is read and discarded,
                    // because this reader is about FOF and an untyped reading
                    // of a typed binder is the same binder.
                    if self.eat(&Tok::Colon) {
                        self.bump();
                    }
                    if self.eat(&Tok::Comma) {
                        continue;
                    }
                    break;
                }
                self.expect(&Tok::RBrack)?;
                self.expect(&Tok::Colon)?;
                let body = self.unit()?;
                Ok(vars.into_iter().rev().fold(body, |acc, v| {
                    if q == "!" {
                        Formula::All(v, Box::new(acc))
                    } else {
                        Formula::Ex(v, Box::new(acc))
                    }
                }))
            }
            Some(Tok::Word(w)) if w == "$true" => {
                self.at += 1;
                Ok(Formula::True)
            }
            Some(Tok::Word(w)) if w == "$false" => {
                self.at += 1;
                Ok(Formula::False)
            }
            _ => {
                let lhs = self.term()?;
                match self.peek().cloned() {
                    Some(Tok::Op("=")) => {
                        self.at += 1;
                        Ok(Formula::Eq(lhs, self.term()?))
                    }
                    Some(Tok::Op("!=")) => {
                        self.at += 1;
                        Ok(Formula::Not(Box::new(Formula::Eq(lhs, self.term()?))))
                    }
                    _ => match lhs {
                        Term::Fun(p, args) => Ok(Formula::Pred(p, args)),
                        Term::Var(v) => {
                            Err(format!("a bare variable {v} is not a formula in FOF"))
                        }
                    },
                }
            }
        }
    }

    fn formula(&mut self, min: u8) -> Result<Formula, String> {
        let mut left = self.unit()?;
        loop {
            let (op, prec) = match self.peek() {
                Some(Tok::Op(o @ ("<=>" | "<~>"))) => (*o, 10u8),
                Some(Tok::Op(o @ ("=>" | "<="))) => (*o, 20),
                Some(Tok::Op(o @ ("|" | "~|"))) => (*o, 30),
                Some(Tok::Op(o @ ("&" | "~&"))) => (*o, 40),
                _ => break,
            };
            if prec < min {
                break;
            }
            self.at += 1;
            // Right-associative throughout. `&` and `|` are associative, so
            // the choice is arbitrary for them; `=>` is right-associative in
            // TPTP. `Form::conj` in src/tptp.rs right-associates too, which is
            // what makes a leaf of this repository's own export come back
            // structurally identical rather than merely equivalent.
            let right = self.formula(prec)?;
            left = match op {
                "<=>" => Formula::Iff(Box::new(left), Box::new(right)),
                "<~>" => Formula::Not(Box::new(Formula::Iff(Box::new(left), Box::new(right)))),
                "=>" => Formula::Imp(Box::new(left), Box::new(right)),
                "<=" => Formula::Imp(Box::new(right), Box::new(left)),
                "|" => Formula::Or(Box::new(left), Box::new(right)),
                "~|" => Formula::Not(Box::new(Formula::Or(Box::new(left), Box::new(right)))),
                "&" => Formula::And(Box::new(left), Box::new(right)),
                "~&" => Formula::Not(Box::new(Formula::And(Box::new(left), Box::new(right)))),
                _ => unreachable!("the operator table above lists every arm"),
            };
        }
        Ok(left)
    }

    /// A general term. Never fails: anything it does not recognise is consumed
    /// as balanced tokens and returned as [`GTerm::Other`], so one odd
    /// annotation cannot make a whole derivation unreadable.
    fn general(&mut self) -> GTerm {
        match self.peek().cloned() {
            Some(Tok::LBrack) => {
                self.at += 1;
                let mut xs = Vec::new();
                if self.eat(&Tok::RBrack) {
                    return GTerm::List(xs);
                }
                loop {
                    xs.push(self.general());
                    if self.eat(&Tok::Comma) {
                        continue;
                    }
                    let _ = self.eat(&Tok::RBrack);
                    break;
                }
                GTerm::List(xs)
            }
            Some(Tok::Word(w)) | Some(Tok::Quoted(w)) => {
                self.at += 1;
                if self.eat(&Tok::LParen) {
                    let mut args = Vec::new();
                    if self.eat(&Tok::RParen) {
                        return GTerm::Func(w, args);
                    }
                    loop {
                        args.push(self.general());
                        if self.eat(&Tok::Comma) {
                            continue;
                        }
                        let _ = self.eat(&Tok::RParen);
                        break;
                    }
                    let mut g = GTerm::Func(w, args);
                    if self.eat(&Tok::Colon) {
                        let inner = self.general();
                        g = GTerm::Other(format!("{}:{}", g.render(), inner.render()));
                    }
                    return g;
                }
                if self.eat(&Tok::Colon) {
                    let inner = self.general();
                    return GTerm::Other(format!("{w}:{}", inner.render()));
                }
                GTerm::Word(w)
            }
            Some(Tok::Var(v)) | Some(Tok::Distinct(v)) => {
                self.at += 1;
                GTerm::Word(v)
            }
            _ => {
                // Balanced skip. Stops at a top-level comma or a closing
                // bracket, which is where a general term always ends.
                let start = self.at;
                let mut depth = 0i32;
                while let Some(t) = self.peek() {
                    match t {
                        Tok::LParen | Tok::LBrack => depth += 1,
                        Tok::RParen | Tok::RBrack => {
                            if depth == 0 {
                                break;
                            }
                            depth -= 1;
                        }
                        Tok::Comma if depth == 0 => break,
                        Tok::Dot if depth == 0 => break,
                        _ => {}
                    }
                    self.at += 1;
                }
                if self.at == start {
                    self.at += 1;
                }
                GTerm::Other(format!("{:?}", &self.toks[start..self.at.min(self.toks.len())]))
            }
        }
    }
}

fn parents_of(g: &GTerm) -> Vec<Parent> {
    let items: &[GTerm] = match g {
        GTerm::List(xs) => xs,
        single => std::slice::from_ref(single),
    };
    items
        .iter()
        .map(|p| match p {
            GTerm::Word(w) => Parent::Named(w.clone()),
            GTerm::Func(f, args) if f == "inference" => match inference_of(args) {
                Some(inf) => Parent::Inline(Box::new(inf)),
                None => Parent::Other(p.render()),
            },
            other => Parent::Other(other.render()),
        })
        .collect()
}

fn inference_of(args: &[GTerm]) -> Option<Inference> {
    let rule = args.first()?.word()?.to_string();
    let parents = args.get(2).map(parents_of).unwrap_or_default();
    Some(Inference { rule, parents })
}

fn source_of(g: &GTerm) -> Source {
    match g {
        GTerm::Func(f, args) if f == "file" => Source::File {
            path: args.first().map(GTerm::render).unwrap_or_default(),
            name: args.get(1).and_then(|n| n.word()).map(|s| s.to_string()),
        },
        GTerm::Func(f, args) if f == "inference" => match inference_of(args) {
            Some(inf) => Source::Inference(inf),
            None => Source::Other(g.render()),
        },
        GTerm::Func(f, args) if f == "introduced" => Source::Introduced {
            rule: args.first().and_then(|r| r.word()).unwrap_or("introduced").to_string(),
            detail: args.iter().skip(1).map(GTerm::render).collect::<Vec<_>>().join(","),
        },
        GTerm::Word(w) => Source::Name(w.clone()),
        other => Source::Other(other.render()),
    }
}

/// The SZS status the prover printed, if it printed one.
///
/// The LAST status line wins: E prints an input status before its result, and
/// taking the first would report the wrong one. The word is ECHOED and never
/// acted on as though it were evidence.
pub fn szs_status(text: &str) -> Option<String> {
    let mut last = None;
    for line in text.lines() {
        if let Some(i) = line.find("SZS status") {
            let rest = line[i + "SZS status".len()..].trim_start();
            let word: String =
                rest.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
            if !word.is_empty() {
                last = Some(word);
            }
        }
    }
    last
}

/// Read every `fof(…)` / `cnf(…)` annotated formula out of a prover's output.
///
/// When the text carries an `SZS output start` / `SZS output end` pair, ONLY
/// what lies between them is read. E prints its saturation state before the
/// proof, in the same `cnf(…)` syntax and with names it reuses, and a reader
/// that swallowed those would build a derivation out of clauses the prover had
/// abandoned.
pub fn parse_derivation(text: &str) -> Result<Vec<Annotated>, String> {
    let body = match (text.find("SZS output start"), text.find("SZS output end")) {
        (Some(a), Some(b)) if b > a => {
            let after = &text[a..];
            let nl = after.find('\n').map(|n| a + n + 1).unwrap_or(a);
            &text[nl..b]
        }
        _ => text,
    };
    parse_annotated_list(body)
}

/// Read every annotated formula in a plain TPTP file. Used for the problem
/// side, where there is no SZS block.
pub fn parse_problem(text: &str) -> Result<Vec<Annotated>, String> {
    parse_annotated_list(text)
}

fn parse_annotated_list(text: &str) -> Result<Vec<Annotated>, String> {
    let toks = lex(text)?;
    let mut p = Parser { toks, at: 0 };
    let mut out = Vec::new();
    while let Some(t) = p.peek().cloned() {
        let kind = match &t {
            Tok::Word(w) => w.clone(),
            _ => {
                return Err(format!(
                    "expected `fof` or `cnf` at the start of an annotated formula, found {t:?}"
                ));
            }
        };
        match kind.as_str() {
            "fof" | "cnf" => {}
            "tff" | "thf" | "tcf" | "tpi" => {
                return Err(format!(
                    "`{kind}` is outside this reader's language. Only untyped first-order (fof \
                     and cnf) is read, because that is what src/tptp.rs emits and what the \
                     adequacy theorem is about"
                ));
            }
            other => return Err(format!("unknown annotated-formula kind `{other}`")),
        }
        p.at += 1;
        p.expect(&Tok::LParen)?;
        let name = p.name()?;
        p.expect(&Tok::Comma)?;
        let role = p.name()?;
        p.expect(&Tok::Comma)?;
        let formula = p.formula(0)?;
        let mut source = None;
        if p.eat(&Tok::Comma) {
            source = Some(source_of(&p.general()));
            // Useful-info fields, e.g. E's trailing `['proof']`. Read and
            // discarded: nothing in them is load-bearing here.
            while p.eat(&Tok::Comma) {
                let _ = p.general();
            }
        }
        p.expect(&Tok::RParen)?;
        p.expect(&Tok::Dot)?;
        out.push(Annotated { name, role, formula, source });
    }
    Ok(out)
}

/// The problem this repository's exporter emits, embedded into the reader's
/// wider language.
///
/// Used by `tests/tstp_parser_test.rs` to pin the reader against the emitter:
/// parsing `fof::form(f)` must give back exactly `from_tptp_form(f)`. A reader
/// that silently normalised on the way in would pass a leaf check it should
/// have failed, which is the one thing this module must not do.
pub fn from_tptp_form(f: &crate::tptp::Form) -> Formula {
    use crate::tptp::{Form, Term as T, sym};
    fn term(t: &T) -> Term {
        match t {
            T::Var(n) => Term::Var(format!("X{n}")),
            T::Const(a) => Term::Fun(sym::constant(a), Vec::new()),
        }
    }
    match f {
        Form::App1(p, t) => Formula::Pred(sym::p1(p), vec![term(t)]),
        Form::App2(p, t, u) => Formula::Pred(sym::p2(p), vec![term(t), term(u)]),
        Form::Eq(t, u) => Formula::Eq(term(t), term(u)),
        Form::Tru => Formula::True,
        Form::Fls => Formula::False,
        Form::Neg(g) => Formula::Not(Box::new(from_tptp_form(g))),
        Form::And(g, h) => {
            Formula::And(Box::new(from_tptp_form(g)), Box::new(from_tptp_form(h)))
        }
        Form::Or(g, h) => Formula::Or(Box::new(from_tptp_form(g)), Box::new(from_tptp_form(h))),
        Form::Imp(g, h) => {
            Formula::Imp(Box::new(from_tptp_form(g)), Box::new(from_tptp_form(h)))
        }
        Form::All(n, g) => Formula::All(format!("X{n}"), Box::new(from_tptp_form(g))),
        Form::Ex(n, g) => Formula::Ex(format!("X{n}"), Box::new(from_tptp_form(g))),
    }
}

// ── Clause view, unification, and the resolvent ────────────────────────────

/// An atom in a clause. Equality is stored with its arguments in a canonical
/// order, because `s = t` and `t = s` are the same literal and a set
/// comparison that treated them as two would refuse correct steps.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Atom {
    Pred(String, Vec<Term>),
    Eq(Term, Term),
}

impl Atom {
    fn eq_canon(a: Term, b: Term) -> Atom {
        if a <= b { Atom::Eq(a, b) } else { Atom::Eq(b, a) }
    }
    fn vars(&self, out: &mut BTreeSet<String>) {
        match self {
            Atom::Pred(_, args) => args.iter().for_each(|a| a.vars(out)),
            Atom::Eq(a, b) => {
                a.vars(out);
                b.vars(out);
            }
        }
    }
    fn render(&self) -> String {
        let mut s = String::new();
        match self {
            Atom::Pred(p, args) => {
                s.push_str(p);
                if !args.is_empty() {
                    s.push('(');
                    for (i, a) in args.iter().enumerate() {
                        if i > 0 {
                            s.push(',');
                        }
                        a.render(&mut s);
                    }
                    s.push(')');
                }
            }
            Atom::Eq(a, b) => {
                a.render(&mut s);
                s.push('=');
                b.render(&mut s);
            }
        }
        s
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Lit {
    positive: bool,
    atom: Atom,
}

impl Lit {
    fn render(&self) -> String {
        if self.positive { self.atom.render() } else { format!("~{}", self.atom.render()) }
    }
}

/// A formula seen as a clause: a set of literals under an implicit universal
/// prefix.
///
/// `$false` disjuncts are DROPPED and a `$true` disjunct makes the whole thing
/// a tautology. Both are ordinary clause normalisations and both are needed on
/// real output: Vampire prints AVATAR conclusions as `$false | (spl1 | ~spl2)`,
/// where the `$false` is the empty clause part and the rest is the assertion.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Clause {
    lits: Vec<Lit>,
    tautology: bool,
}

impl Clause {
    fn render(&self) -> String {
        if self.tautology {
            return "$true".into();
        }
        if self.lits.is_empty() {
            return "$false".into();
        }
        self.lits.iter().map(Lit::render).collect::<Vec<_>>().join(" | ")
    }
    fn normalise(mut lits: Vec<Lit>, tautology: bool) -> Clause {
        lits.sort();
        lits.dedup();
        Clause { lits, tautology }
    }
    fn vars(&self) -> BTreeSet<String> {
        let mut v = BTreeSet::new();
        self.lits.iter().for_each(|l| l.atom.vars(&mut v));
        v
    }
}

/// Read a formula as a clause, or say why it is not one.
fn clause_view(f: &Formula) -> Result<Clause, &'static str> {
    fn body<'a>(f: &'a Formula, bound: &mut Vec<&'a str>) -> &'a Formula {
        match f {
            Formula::All(v, g) => {
                bound.push(v);
                body(g, bound)
            }
            other => other,
        }
    }
    fn lits(f: &Formula, out: &mut Vec<Lit>, taut: &mut bool) -> Result<(), &'static str> {
        match f {
            Formula::Or(a, b) => {
                lits(a, out, taut)?;
                lits(b, out, taut)
            }
            Formula::False => Ok(()),
            Formula::True => {
                *taut = true;
                Ok(())
            }
            Formula::Pred(p, args) => {
                out.push(Lit { positive: true, atom: Atom::Pred(p.clone(), args.clone()) });
                Ok(())
            }
            Formula::Eq(a, b) => {
                out.push(Lit { positive: true, atom: Atom::eq_canon(a.clone(), b.clone()) });
                Ok(())
            }
            Formula::Not(g) => match &**g {
                Formula::Pred(p, args) => {
                    out.push(Lit { positive: false, atom: Atom::Pred(p.clone(), args.clone()) });
                    Ok(())
                }
                Formula::Eq(a, b) => {
                    out.push(Lit {
                        positive: false,
                        atom: Atom::eq_canon(a.clone(), b.clone()),
                    });
                    Ok(())
                }
                Formula::True => Ok(()),
                Formula::False => {
                    *taut = true;
                    Ok(())
                }
                _ => Err("a negation over a compound formula is not a literal"),
            },
            _ => Err("a connective other than disjunction appears in the clause body"),
        }
    }
    let mut bound = Vec::new();
    let inner = body(f, &mut bound);
    if matches!(inner, Formula::Ex(_, _) | Formula::All(_, _)) {
        return Err("a quantifier appears below the universal prefix");
    }
    let mut out = Vec::new();
    let mut taut = false;
    lits(inner, &mut out, &mut taut)?;
    Ok(Clause::normalise(out, taut))
}

type Subst = HashMap<String, Term>;

fn walk(t: &Term, s: &Subst) -> Term {
    match t {
        Term::Var(v) => match s.get(v) {
            Some(b) => walk(b, s),
            None => t.clone(),
        },
        Term::Fun(f, args) => Term::Fun(f.clone(), args.iter().map(|a| walk(a, s)).collect()),
    }
}

fn occurs(v: &str, t: &Term, s: &Subst) -> bool {
    match walk(t, s) {
        Term::Var(u) => u == v,
        Term::Fun(_, args) => args.iter().any(|a| occurs(v, a, s)),
    }
}

/// Syntactic first-order unification with an occurs check. No theory, no
/// ordering, no AC: a most general unifier or nothing.
fn unify(a: &Term, b: &Term, s: &mut Subst) -> bool {
    let (x, y) = (walk(a, s), walk(b, s));
    match (&x, &y) {
        (Term::Var(u), Term::Var(v)) if u == v => true,
        (Term::Var(u), other) | (other, Term::Var(u)) => {
            if occurs(u, other, s) {
                return false;
            }
            s.insert(u.clone(), other.clone());
            true
        }
        (Term::Fun(f, xs), Term::Fun(g, ys)) => {
            f == g && xs.len() == ys.len() && xs.iter().zip(ys).all(|(p, q)| unify(p, q, s))
        }
    }
}

fn unify_atoms(a: &Atom, b: &Atom) -> Option<Subst> {
    let mut s = Subst::new();
    match (a, b) {
        (Atom::Pred(p, xs), Atom::Pred(q, ys)) => {
            if p != q || xs.len() != ys.len() {
                return None;
            }
            for (x, y) in xs.iter().zip(ys) {
                if !unify(x, y, &mut s) {
                    return None;
                }
            }
            Some(s)
        }
        (Atom::Eq(x1, y1), Atom::Eq(x2, y2)) => {
            // Equality is symmetric, so both orientations are tried. The
            // canonical storage order is a comparison convenience and must not
            // become a restriction on what unifies.
            for (p, q) in [(x2, y2), (y2, x2)] {
                let mut t = Subst::new();
                if unify(x1, p, &mut t) && unify(y1, q, &mut t) {
                    return Some(t);
                }
            }
            None
        }
        _ => None,
    }
}

fn apply_atom(a: &Atom, s: &Subst) -> Atom {
    match a {
        Atom::Pred(p, args) => Atom::Pred(p.clone(), args.iter().map(|t| walk(t, s)).collect()),
        Atom::Eq(x, y) => Atom::eq_canon(walk(x, s), walk(y, s)),
    }
}

fn apply_clause(c: &[Lit], s: &Subst) -> Vec<Lit> {
    c.iter().map(|l| Lit { positive: l.positive, atom: apply_atom(&l.atom, s) }).collect()
}

/// Rename every variable of a clause with a prefix nothing else uses, so two
/// premises cannot share a variable by accident.
fn rename_apart(c: &Clause, tag: &str) -> Clause {
    let mut s = Subst::new();
    for v in c.vars() {
        s.insert(v.clone(), Term::Var(format!("{tag}{v}")));
    }
    Clause::normalise(apply_clause(&c.lits, &s), c.tautology)
}

/// Are two clauses the same up to a BIJECTIVE renaming of their variables?
///
/// The fast path is exact set equality, which is what nearly every real step
/// hits because provers keep their variable names. The slow path pairs
/// literals with backtracking under a budget; the budget is reported rather
/// than silently treated as a mismatch, because "too big to check" and "does
/// not check out" are different facts.
fn variant(a: &Clause, b: &Clause) -> Result<bool, &'static str> {
    if a.tautology != b.tautology {
        return Ok(false);
    }
    if a.tautology {
        return Ok(true);
    }
    if a.lits == b.lits {
        return Ok(true);
    }
    if a.lits.len() != b.lits.len() {
        return Ok(false);
    }
    const BUDGET: u32 = 200_000;
    let mut used = vec![false; b.lits.len()];
    let mut fwd: HashMap<String, String> = HashMap::new();
    let mut bwd: HashMap<String, String> = HashMap::new();
    let mut spent = 0u32;
    let ok = pair(&a.lits, &b.lits, 0, &mut used, &mut fwd, &mut bwd, &mut spent, BUDGET);
    match ok {
        Some(v) => Ok(v),
        None => Err("the variant check ran out of budget"),
    }
}

#[allow(clippy::too_many_arguments)]
fn pair(
    a: &[Lit],
    b: &[Lit],
    i: usize,
    used: &mut Vec<bool>,
    fwd: &mut HashMap<String, String>,
    bwd: &mut HashMap<String, String>,
    spent: &mut u32,
    budget: u32,
) -> Option<bool> {
    if i == a.len() {
        return Some(true);
    }
    for j in 0..b.len() {
        if used[j] {
            continue;
        }
        *spent += 1;
        if *spent > budget {
            return None;
        }
        let (f0, b0) = (fwd.clone(), bwd.clone());
        if match_lit(&a[i], &b[j], fwd, bwd) {
            used[j] = true;
            match pair(a, b, i + 1, used, fwd, bwd, spent, budget) {
                None => return None,
                Some(true) => return Some(true),
                Some(false) => {}
            }
            used[j] = false;
        }
        *fwd = f0;
        *bwd = b0;
    }
    Some(false)
}

fn match_lit(
    x: &Lit,
    y: &Lit,
    fwd: &mut HashMap<String, String>,
    bwd: &mut HashMap<String, String>,
) -> bool {
    if x.positive != y.positive {
        return false;
    }
    match (&x.atom, &y.atom) {
        (Atom::Pred(p, xs), Atom::Pred(q, ys)) => {
            p == q
                && xs.len() == ys.len()
                && xs.iter().zip(ys).all(|(u, v)| match_term(u, v, fwd, bwd))
        }
        (Atom::Eq(x1, y1), Atom::Eq(x2, y2)) => {
            let (f0, b0) = (fwd.clone(), bwd.clone());
            if match_term(x1, x2, fwd, bwd) && match_term(y1, y2, fwd, bwd) {
                return true;
            }
            *fwd = f0;
            *bwd = b0;
            match_term(x1, y2, fwd, bwd) && match_term(y1, x2, fwd, bwd)
        }
        _ => false,
    }
}

fn match_term(
    x: &Term,
    y: &Term,
    fwd: &mut HashMap<String, String>,
    bwd: &mut HashMap<String, String>,
) -> bool {
    match (x, y) {
        (Term::Var(u), Term::Var(v)) => {
            match (fwd.get(u).cloned(), bwd.get(v).cloned()) {
                (Some(a), Some(b)) => a == *v && b == *u,
                (None, None) => {
                    fwd.insert(u.clone(), v.clone());
                    bwd.insert(v.clone(), u.clone());
                    true
                }
                _ => false,
            }
        }
        (Term::Fun(f, xs), Term::Fun(g, ys)) => {
            f == g
                && xs.len() == ys.len()
                && xs.iter().zip(ys).all(|(u, v)| match_term(u, v, fwd, bwd))
        }
        _ => false,
    }
}

// ── The derivation ─────────────────────────────────────────────────────────

/// How closely a leaf of the derivation matched the problem formula it names.
///
/// Three levels rather than a boolean, because they cost different amounts of
/// belief. `Identical` means the prover echoed our formula unchanged.
/// `AlphaEquivalent` means it renamed bound variables, which E does on every
/// axiom it reads. `AssociativityNormalised` means it also rebracketed an
/// n-ary `&` or `|`. Nothing below that is accepted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LeafMatch {
    Identical,
    AlphaEquivalent,
    AssociativityNormalised,
}

impl LeafMatch {
    pub fn name(self) -> &'static str {
        match self {
            LeafMatch::Identical => "identical",
            LeafMatch::AlphaEquivalent => "alpha_equivalent",
            LeafMatch::AssociativityNormalised => "associativity_normalised",
        }
    }
}

fn leaf_match(proof: &Formula, problem: &Formula) -> Option<LeafMatch> {
    if proof == problem {
        return Some(LeafMatch::Identical);
    }
    if alpha_eq(proof, problem) {
        return Some(LeafMatch::AlphaEquivalent);
    }
    if alpha_eq(&proof.assoc_normalised(), &problem.assoc_normalised()) {
        return Some(LeafMatch::AssociativityNormalised);
    }
    None
}

/// A leaf that is not a formula of the problem. The reason is carried because
/// "the name is not in the problem" and "the name is there and the formula is
/// different" are different accusations.
#[derive(Clone, Debug, serde::Serialize)]
pub struct LeafFailure {
    pub node: String,
    pub why: String,
}

/// A symbol or a formula the prover invented. Not from the problem, not
/// derived from it, and nothing here checks that the extension is
/// conservative.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Introduced {
    pub node: String,
    pub rule: String,
    pub detail: String,
}

/// A rule this checker did not replay, with the count and the reason.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Unchecked {
    pub rule: String,
    pub count: usize,
    pub why: String,
}

/// A step whose rule IS implemented, whose shapes DO apply, and which still
/// did not reconstruct. Either the derivation is wrong or this checker is
/// incomplete for that rule; the report says both and decides neither.
#[derive(Clone, Debug, serde::Serialize)]
pub struct NotReconstructed {
    pub node: String,
    pub rule: String,
    pub expected: String,
    pub found: String,
}

/// What a checked run of this module establishes, field by field, with nothing
/// collapsed into anything else.
///
/// The shape follows `fol_solve::Outcome`, which decision 0006 item 4 fixes at
/// five never-collapsed fields: what the oracle said, what it was asked, what
/// the checker did, the one verdict word, and the reading that rides on the
/// unproved translation. The same discipline applies here and for the same
/// reason: a reader who has never opened a decision record still must not be
/// able to mistake an oracle's word for a checked one.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Report {
    /// What the prover said about itself. ECHOED, UNTRUSTED, and never used to
    /// decide anything below.
    pub szs_status: Option<String>,
    /// Why the problem file could not be read, when it could not.
    pub problem_error: Option<String>,
    /// Why the derivation could not be read, when it could not.
    pub derivation_error: Option<String>,

    pub problem_formulas: usize,
    /// FNV-1a 64 over the canonical rendering of the PARSED problem, so a
    /// report can be tied to the file it is about. It identifies and does not
    /// commit, which is decision 0006 item 7 in the same words.
    pub problem_digest: String,

    pub derivation_nodes: usize,
    /// Every parent reference resolves and the parent relation is acyclic.
    pub derivation_wellformed: bool,
    pub wellformedness_failures: Vec<String>,
    /// Some node is the empty clause and nothing cites it.
    pub root_is_false: bool,
    pub nodes_outside_the_refutation: usize,

    pub leaves_total: usize,
    /// Every leaf reachable from the empty clause is a formula of the problem,
    /// under the name the prover gave for it and with the role the problem
    /// gave it.
    pub leaves_match_problem: bool,
    pub leaf_match_levels: BTreeMap<&'static str, usize>,
    pub leaf_failures: Vec<LeafFailure>,
    pub leaves_introduced: Vec<Introduced>,

    /// The conjecture's name in the problem, when the problem has one.
    pub conjecture_in_problem: Option<String>,
    /// The refutation uses it.
    pub conjecture_used: bool,
    /// The step that negates it was replayed: the child IS the negation of the
    /// problem's conjecture and of nothing else.
    pub conjecture_negation_checked: bool,
    /// `axioms_and_the_negated_conjecture` or `axioms_alone`. Null unless the
    /// structure holds, because it is a statement about a derivation this
    /// module has read and not about a theory.
    pub what_was_refuted: Option<&'static str>,

    pub steps_total: usize,
    pub steps_checked: usize,
    pub steps_checked_by_rule: BTreeMap<String, usize>,
    pub steps_unchecked: Vec<Unchecked>,
    pub steps_not_reconstructed: Vec<NotReconstructed>,

    /// The one word. See [`verdict_means`].
    pub verdict: &'static str,
}

/// One sentence per word, so a report is readable without this file open
/// beside it.
pub fn verdict_means(v: &str) -> &'static str {
    match v {
        "problem_unparsed" => "the TPTP problem could not be read, so there was nothing to \
                               check the derivation against. Not a statement about the prover",
        "derivation_unparsed" => "the prover's output could not be read as a TSTP derivation. \
                                  Run the prover with a proof-printing option (vampire \
                                  `--proof tptp`, eprover `--proof-object`); without one there \
                                  is only a word",
        "no_refutation_offered" => "the output contains no empty clause, so there is no \
                                    refutation to check. This is the normal outcome of a \
                                    satisfiable problem, a timeout or a give-up, and it is \
                                    evidence of nothing in either direction",
        "derivation_rejected" => "THE CHECKER SAID NO. Either a parent reference does not \
                                  resolve, or the parent relation has a cycle, or a leaf of the \
                                  refutation is not a formula of the problem. A refutation of \
                                  something else is not a refutation of this",
        "refutation_step_not_reconstructed" => "the structure holds and every leaf is ours, and \
                                                at least one step whose rule this checker \
                                                implements did not reconstruct from its \
                                                premises. EITHER the derivation is wrong OR \
                                                this checker is incomplete for that rule. This \
                                                module does not decide which, and the step is \
                                                named",
        "refutation_structure_checked" => "the derivation is a well-founded DAG from the \
                                           problem's own formulas to the empty clause, and NO \
                                           inference step was replayed. It rules out a prover \
                                           answering about a different file; it says nothing \
                                           about whether the steps are sound",
        "refutation_partially_replayed" => "the structure holds and SOME steps were recomputed \
                                            from their premises. The unchecked rules are named \
                                            and counted. This is the normal outcome, because \
                                            clausification and AVATAR splitting are not checked",
        "refutation_fully_replayed" => "the structure holds and EVERY step was recomputed. This \
                                        is the strongest word here and it is still NOT a proof \
                                        of unsatisfiability: the calculus's soundness is not \
                                        machine-checked anywhere in lean/, and the replayer in \
                                        src/tstp.rs is ordinary unverified Rust",
        _ => "unrecognised verdict",
    }
}

/// A node of the derivation as the checker sees it.
struct Node {
    role: String,
    /// `None` for a synthetic node standing for an inline inference record,
    /// which carries a rule and parents but no formula of its own.
    formula: Option<Formula>,
    source: Option<Source>,
    rule: Option<String>,
    parents: Vec<String>,
}

/// The reasons a step is not replayed, written once so a report cannot end up
/// with two phrasings for the same fact.
const WHY_NO_CHECK: &str =
    "this checker implements no replay for this rule. Clausification, Skolemisation, AVATAR \
     splitting and every SAT-solver step are in this class";
const WHY_NOT_CLAUSE: &str =
    "a premise or the conclusion is not clause-shaped, so the clause-level replay does not \
     apply to this occurrence";
const WHY_NO_FORMULA: &str =
    "a premise is an inline inference record and carries no formula of its own, so neither it \
     nor the step it feeds can be replayed";
const WHY_ARITY: &str = "the rule was given a number of premises this replay does not handle";
const WHY_BUDGET: &str = "the clause is larger than the variant check's backtracking budget";

/// Read a problem and a prover's output, and say exactly what checks out.
pub fn check(problem_text: &str, proof_text: &str) -> Report {
    let mut r = Report {
        szs_status: szs_status(proof_text),
        problem_error: None,
        derivation_error: None,
        problem_formulas: 0,
        problem_digest: String::new(),
        derivation_nodes: 0,
        derivation_wellformed: false,
        wellformedness_failures: Vec::new(),
        root_is_false: false,
        nodes_outside_the_refutation: 0,
        leaves_total: 0,
        leaves_match_problem: false,
        leaf_match_levels: BTreeMap::new(),
        leaf_failures: Vec::new(),
        leaves_introduced: Vec::new(),
        conjecture_in_problem: None,
        conjecture_used: false,
        conjecture_negation_checked: false,
        what_was_refuted: None,
        steps_total: 0,
        steps_checked: 0,
        steps_checked_by_rule: BTreeMap::new(),
        steps_unchecked: Vec::new(),
        steps_not_reconstructed: Vec::new(),
        verdict: "problem_unparsed",
    };

    // ── The problem ────────────────────────────────────────────────────────
    let problem = match parse_problem(problem_text) {
        Ok(p) => p,
        Err(e) => {
            r.problem_error = Some(e);
            return r;
        }
    };
    let mut by_name: HashMap<&str, &Annotated> = HashMap::new();
    let mut canonical = String::new();
    for a in &problem {
        by_name.insert(a.name.as_str(), a);
        let _ = writeln!(canonical, "{}\t{}", a.role, a.formula.render());
        if a.role == "conjecture" {
            r.conjecture_in_problem = Some(a.name.clone());
        }
    }
    r.problem_formulas = problem.len();
    r.problem_digest =
        crate::tptp::checkfmt::hex16(crate::tptp::checkfmt::fnv1a64(canonical.trim_end()));

    // ── The derivation ─────────────────────────────────────────────────────
    let annotated = match parse_derivation(proof_text) {
        Ok(a) => a,
        Err(e) => {
            r.derivation_error = Some(e);
            r.verdict = "derivation_unparsed";
            return r;
        }
    };
    if annotated.is_empty() {
        r.derivation_error = Some(
            "the output carries no annotated formula. A prover asked for a verdict and not for \
             a proof prints one line and no derivation; run it with vampire `--proof tptp` or \
             eprover `--proof-object`"
                .into(),
        );
        r.verdict = "derivation_unparsed";
        return r;
    }

    let mut nodes: HashMap<String, Node> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    let mut duplicates: Vec<String> = Vec::new();
    for a in &annotated {
        let mut parents = Vec::new();
        let mut rule = None;
        if let Some(Source::Inference(inf)) = &a.source {
            rule = Some(inf.rule.clone());
            flatten_parents(&a.name, &inf.parents, &mut parents, &mut nodes, &mut order);
        }
        if nodes.contains_key(&a.name) {
            duplicates.push(a.name.clone());
            continue;
        }
        order.push(a.name.clone());
        nodes.insert(
            a.name.clone(),
            Node {
                role: a.role.clone(),
                formula: Some(a.formula.clone()),
                source: a.source.clone(),
                rule,
                parents,
            },
        );
    }
    r.derivation_nodes = nodes.len();

    // Every parent resolves.
    let mut dangling: Vec<String> = Vec::new();
    for name in &order {
        for p in &nodes[name].parents {
            if !nodes.contains_key(p) {
                dangling.push(format!("{name} cites {p}, which is not in the derivation"));
            }
        }
    }
    // The parent relation is acyclic.
    let cycle = find_cycle(&order, &nodes);

    for d in &duplicates {
        r.wellformedness_failures
            .push(format!("the name {d} is used by more than one annotated formula, so a \
                           parent reference to it is ambiguous"));
    }
    r.wellformedness_failures.extend(dangling.iter().cloned());
    if let Some(c) = &cycle {
        r.wellformedness_failures
            .push(format!("the parent relation has a cycle: {}", c.join(" -> ")));
    }
    r.derivation_wellformed = r.wellformedness_failures.is_empty();

    // ── The root, and what is reachable from it ────────────────────────────
    let cited: HashSet<&str> =
        order.iter().flat_map(|n| nodes[n].parents.iter().map(String::as_str)).collect();
    let false_roots: Vec<String> = order
        .iter()
        .filter(|n| !cited.contains(n.as_str()))
        .filter(|n| nodes[*n].formula.as_ref() == Some(&Formula::False))
        .cloned()
        .collect();
    r.root_is_false = !false_roots.is_empty();

    let mut reachable: BTreeSet<String> = BTreeSet::new();
    if r.derivation_wellformed {
        let mut stack: Vec<String> = false_roots.clone();
        while let Some(n) = stack.pop() {
            if !reachable.insert(n.clone()) {
                continue;
            }
            for p in &nodes[&n].parents {
                stack.push(p.clone());
            }
        }
    }
    r.nodes_outside_the_refutation = nodes.len().saturating_sub(reachable.len());

    if !r.root_is_false {
        r.verdict = if r.derivation_wellformed { "no_refutation_offered" } else { "derivation_rejected" };
        return r;
    }
    if !r.derivation_wellformed {
        r.verdict = "derivation_rejected";
        return r;
    }

    // ── Leaves ─────────────────────────────────────────────────────────────
    let mut leaves_ok = true;
    // Keyed on the rule AND the reason, not on the rule alone. Two
    // occurrences of `rw` can be unchecked for different reasons, and a map
    // that kept only the first reason would report the other occurrences under
    // a sentence that is not true of them.
    let mut unchecked: BTreeMap<(String, &'static str), usize> = BTreeMap::new();
    for name in &reachable {
        let node = &nodes[name];
        if node.rule.is_some() {
            continue;
        }
        r.leaves_total += 1;
        match &node.source {
            Some(Source::Introduced { rule, detail }) => {
                r.leaves_introduced.push(Introduced {
                    node: name.clone(),
                    rule: rule.clone(),
                    detail: detail.clone(),
                });
                *unchecked
                    .entry((
                        format!("introduced:{rule}"),
                        "the prover invented this formula and its symbols. It is not from the \
                         problem, and no conservativity argument is checked here",
                    ))
                    .or_insert(0) += 1;
            }
            Some(Source::File { name: Some(orig), .. }) => {
                check_leaf(name, orig, node, &by_name, &mut r, &mut leaves_ok);
            }
            _ => {
                // No usable name. Fall back to matching the formula itself
                // against the problem, and say that is what happened.
                let f = node.formula.as_ref();
                let hit = f.and_then(|f| {
                    problem.iter().find_map(|a| leaf_match(f, &a.formula).map(|m| (a, m)))
                });
                match hit {
                    Some((a, m)) => {
                        *r.leaf_match_levels.entry(m.name()).or_insert(0) += 1;
                        if a.role == "conjecture" {
                            r.conjecture_used = true;
                        }
                    }
                    None => {
                        leaves_ok = false;
                        r.leaf_failures.push(LeafFailure {
                            node: name.clone(),
                            why: format!(
                                "the leaf carries no `file(…)` provenance this reader \
                                 understands and its formula `{}` is not a formula of the \
                                 problem",
                                f.map(Formula::render).unwrap_or_else(|| "?".into())
                            ),
                        });
                    }
                }
            }
        }
    }
    r.leaves_match_problem = leaves_ok;

    // ── Steps ──────────────────────────────────────────────────────────────
    for name in &reachable {
        let node = &nodes[name];
        let Some(rule) = node.rule.clone() else { continue };
        r.steps_total += 1;
        match check_step(name, &rule, node, &nodes, &by_name, &mut r) {
            StepResult::Checked(kind) => {
                r.steps_checked += 1;
                *r.steps_checked_by_rule.entry(kind.to_string()).or_insert(0) += 1;
            }
            StepResult::Unchecked(why) => {
                *unchecked.entry((rule.clone(), why)).or_insert(0) += 1;
            }
            StepResult::NotReconstructed(nr) => r.steps_not_reconstructed.push(nr),
        }
    }
    r.steps_unchecked = unchecked
        .into_iter()
        .map(|((rule, why), count)| Unchecked { rule, count, why: why.to_string() })
        .collect();

    // ── The verdict ────────────────────────────────────────────────────────
    r.what_was_refuted = Some(if r.conjecture_used {
        "axioms_and_the_negated_conjecture"
    } else {
        "axioms_alone"
    });
    r.verdict = if !r.leaves_match_problem {
        "derivation_rejected"
    } else if !r.steps_not_reconstructed.is_empty() {
        "refutation_step_not_reconstructed"
    } else if r.steps_unchecked.is_empty() && r.steps_checked == r.steps_total {
        "refutation_fully_replayed"
    } else if r.steps_checked > 0 {
        "refutation_partially_replayed"
    } else {
        "refutation_structure_checked"
    };
    r
}

/// Give every inline inference record a node of its own, so that its parent
/// references are still resolved and its rule is still counted.
fn flatten_parents(
    owner: &str,
    parents: &[Parent],
    out: &mut Vec<String>,
    nodes: &mut HashMap<String, Node>,
    order: &mut Vec<String>,
) {
    for (i, p) in parents.iter().enumerate() {
        match p {
            Parent::Named(n) => out.push(n.clone()),
            Parent::Other(o) => {
                let name = format!("{owner}::other{i}");
                order.push(name.clone());
                nodes.insert(
                    name.clone(),
                    Node {
                        role: "plain".into(),
                        formula: None,
                        source: Some(Source::Other(o.clone())),
                        rule: Some(format!("unreadable_parent:{o}")),
                        parents: Vec::new(),
                    },
                );
                out.push(name);
            }
            Parent::Inline(inf) => {
                let name = format!("{owner}::inline{i}");
                let mut inner = Vec::new();
                flatten_parents(&name, &inf.parents, &mut inner, nodes, order);
                order.push(name.clone());
                nodes.insert(
                    name.clone(),
                    Node {
                        role: "plain".into(),
                        formula: None,
                        source: None,
                        rule: Some(inf.rule.clone()),
                        parents: inner,
                    },
                );
                out.push(name);
            }
        }
    }
}

fn find_cycle(order: &[String], nodes: &HashMap<String, Node>) -> Option<Vec<String>> {
    #[derive(Clone, Copy, PartialEq)]
    enum Mark {
        White,
        Grey,
        Black,
    }
    let mut mark: HashMap<String, Mark> =
        order.iter().map(|n| (n.clone(), Mark::White)).collect();
    let mut path: Vec<String> = Vec::new();
    // Iterative depth-first search. A derivation from a real prover can be
    // thousands of nodes deep and a recursive walk would overflow the stack on
    // the one input that matters.
    for start in order {
        if mark.get(start) != Some(&Mark::White) {
            continue;
        }
        let mut stack: Vec<(String, usize)> = vec![(start.clone(), 0)];
        mark.insert(start.clone(), Mark::Grey);
        path.push(start.clone());
        while let Some((node, idx)) = stack.pop() {
            let parents = nodes.get(&node).map(|n| n.parents.clone()).unwrap_or_default();
            if idx < parents.len() {
                stack.push((node, idx + 1));
                let child = parents[idx].clone();
                match mark.get(&child) {
                    Some(Mark::Grey) => {
                        let mut c = path.clone();
                        c.push(child);
                        return Some(c);
                    }
                    Some(Mark::White) => {
                        mark.insert(child.clone(), Mark::Grey);
                        path.push(child.clone());
                        stack.push((child, 0));
                    }
                    _ => {}
                }
            } else {
                mark.insert(node, Mark::Black);
                path.pop();
            }
        }
    }
    None
}

fn check_leaf(
    name: &str,
    orig: &str,
    node: &Node,
    by_name: &HashMap<&str, &Annotated>,
    r: &mut Report,
    leaves_ok: &mut bool,
) {
    let Some(problem_formula) = by_name.get(orig) else {
        *leaves_ok = false;
        r.leaf_failures.push(LeafFailure {
            node: name.to_string(),
            why: format!(
                "the leaf says it came from `{orig}` in the problem file, and the problem has \
                 no formula of that name. The prover was given a different file"
            ),
        });
        return;
    };
    let Some(f) = &node.formula else {
        *leaves_ok = false;
        r.leaf_failures.push(LeafFailure {
            node: name.to_string(),
            why: "the leaf carries no formula".into(),
        });
        return;
    };
    match leaf_match(f, &problem_formula.formula) {
        None => {
            *leaves_ok = false;
            r.leaf_failures.push(LeafFailure {
                node: name.to_string(),
                why: format!(
                    "the leaf says it is `{orig}` from the problem, and it is a different \
                     formula. Problem: {}. Derivation: {}",
                    problem_formula.formula.render(),
                    f.render()
                ),
            });
        }
        Some(m) => {
            if node.role != problem_formula.role {
                // A conjecture presented as an axiom would make the refutation
                // prove nothing about entailment, so this is a rejection and
                // not a note.
                *leaves_ok = false;
                r.leaf_failures.push(LeafFailure {
                    node: name.to_string(),
                    why: format!(
                        "the leaf is `{orig}` from the problem with role `{}`, and the problem \
                         gives it role `{}`. A conjecture used as an axiom refutes nothing",
                        node.role, problem_formula.role
                    ),
                });
                return;
            }
            *r.leaf_match_levels.entry(m.name()).or_insert(0) += 1;
            if problem_formula.role == "conjecture" {
                r.conjecture_used = true;
            }
        }
    }
}

enum StepResult {
    Checked(&'static str),
    Unchecked(&'static str),
    NotReconstructed(NotReconstructed),
}

/// The rules this module replays, and the shape of each replay.
///
/// Anything not named here is unchecked and is reported as unchecked. The list
/// is short on purpose: a rule is in it only when the check is exact.
fn check_step(
    name: &str,
    rule: &str,
    node: &Node,
    nodes: &HashMap<String, Node>,
    by_name: &HashMap<&str, &Annotated>,
    r: &mut Report,
) -> StepResult {
    let Some(child) = &node.formula else {
        return StepResult::Unchecked(WHY_NO_FORMULA);
    };
    let premises: Vec<Option<&Formula>> =
        node.parents.iter().map(|p| nodes[p].formula.as_ref()).collect();

    match rule {
        // The negation of the conjecture. Checked exactly, and it is the step
        // that makes the refutation say something about ENTAILMENT rather than
        // about the axioms alone.
        "negated_conjecture" | "assume_negation" => {
            if premises.len() != 1 {
                return StepResult::Unchecked(WHY_ARITY);
            }
            let Some(parent) = premises[0] else {
                return StepResult::Unchecked(WHY_NO_FORMULA);
            };
            let negated = Formula::Not(Box::new(parent.clone()));
            if leaf_match(child, &negated).is_some() {
                let parent_name = &node.parents[0];
                let is_conjecture = nodes
                    .get(parent_name)
                    .is_some_and(|p| p.role == "conjecture")
                    || by_name
                        .get(parent_name.as_str())
                        .is_some_and(|a| a.role == "conjecture");
                if is_conjecture {
                    r.conjecture_negation_checked = true;
                }
                StepResult::Checked("negation_of_the_conjecture")
            } else {
                StepResult::NotReconstructed(NotReconstructed {
                    node: name.to_string(),
                    rule: rule.to_string(),
                    expected: negated.render(),
                    found: child.render(),
                })
            }
        }

        // Binary resolution, and the simplification rules whose conclusion IS
        // the binary resolvent. See the module docstring for why subsumption
        // resolution is exactly this check and not a weaker one.
        "resolution"
        | "subsumption_resolution"
        | "forward_subsumption_resolution"
        | "backward_subsumption_resolution" => {
            if premises.len() != 2 {
                return StepResult::Unchecked(WHY_ARITY);
            }
            let (Some(p0), Some(p1)) = (premises[0], premises[1]) else {
                return StepResult::Unchecked(WHY_NO_FORMULA);
            };
            let (Ok(a), Ok(b), Ok(c)) = (clause_view(p0), clause_view(p1), clause_view(child))
            else {
                return StepResult::Unchecked(WHY_NOT_CLAUSE);
            };
            match resolvent_matches(&a, &b, &c) {
                Ok(true) => StepResult::Checked("binary_resolvent"),
                Ok(false) => StepResult::NotReconstructed(NotReconstructed {
                    node: name.to_string(),
                    rule: rule.to_string(),
                    expected: format!(
                        "a binary resolvent of `{}` and `{}`",
                        a.render(),
                        b.render()
                    ),
                    found: c.render(),
                }),
                Err(_) => StepResult::Unchecked(WHY_BUDGET),
            }
        }

        // Factoring: two literals of one premise unified and merged.
        "factoring" | "factor" => one_premise(name, rule, &premises, child, |p, c| {
            for i in 0..p.lits.len() {
                for j in (i + 1)..p.lits.len() {
                    if p.lits[i].positive != p.lits[j].positive {
                        continue;
                    }
                    if let Some(s) = unify_atoms(&p.lits[i].atom, &p.lits[j].atom) {
                        let cand = Clause::normalise(apply_clause(&p.lits, &s), p.tautology);
                        if variant(&cand, c)? {
                            return Ok(true);
                        }
                    }
                }
            }
            Ok(false)
        })
        .map_checked("factor"),

        // The clause is the same set of literals as its premise. Covers
        // duplicate literal removal and Vampire's `flattening`, which
        // reassociates a disjunction and changes nothing else.
        "duplicate_literal_removal" | "flattening" => {
            one_premise(name, rule, &premises, child, variant).map_checked("same_clause")
        }

        // Literals `s != s` with syntactically identical sides, deleted.
        "trivial_inequality_removal" => one_premise(name, rule, &premises, child, |p, c| {
            let kept: Vec<Lit> = p
                .lits
                .iter()
                .filter(|l| !matches!(&l.atom, Atom::Eq(x, y) if !l.positive && x == y))
                .cloned()
                .collect();
            variant(&Clause::normalise(kept, p.tautology), c)
        })
        .map_checked("trivial_inequality_removed"),

        // `C ∨ s ≠ t` with σ = mgu(s, t) gives `Cσ`.
        "equality_resolution" => one_premise(name, rule, &premises, child, |p, c| {
            for (i, l) in p.lits.iter().enumerate() {
                if l.positive {
                    continue;
                }
                let Atom::Eq(x, y) = &l.atom else { continue };
                let mut s = Subst::new();
                if !unify(x, y, &mut s) {
                    continue;
                }
                let rest: Vec<Lit> =
                    p.lits.iter().enumerate().filter(|(k, _)| *k != i).map(|(_, l)| l.clone()).collect();
                let cand = Clause::normalise(apply_clause(&rest, &s), p.tautology);
                if variant(&cand, c)? {
                    return Ok(true);
                }
            }
            Ok(false)
        })
        .map_checked("equality_resolved"),

        _ => StepResult::Unchecked(WHY_NO_CHECK),
    }
}

/// The outcome of a one-premise replay before it is given a name.
enum OneOf {
    Ok,
    No(NotReconstructed),
    Skip(&'static str),
}

impl OneOf {
    fn map_checked(self, kind: &'static str) -> StepResult {
        match self {
            OneOf::Ok => StepResult::Checked(kind),
            OneOf::Skip(why) => StepResult::Unchecked(why),
            OneOf::No(nr) => StepResult::NotReconstructed(nr),
        }
    }
}

fn one_premise(
    name: &str,
    rule: &str,
    premises: &[Option<&Formula>],
    child: &Formula,
    f: impl Fn(&Clause, &Clause) -> Result<bool, &'static str>,
) -> OneOf {
    if premises.len() != 1 {
        return OneOf::Skip(WHY_ARITY);
    }
    let Some(p) = premises[0] else {
        return OneOf::Skip(WHY_NO_FORMULA);
    };
    let (Ok(pc), Ok(cc)) = (clause_view(p), clause_view(child)) else {
        return OneOf::Skip(WHY_NOT_CLAUSE);
    };
    match f(&pc, &cc) {
        Ok(true) => OneOf::Ok,
        Ok(false) => OneOf::No(NotReconstructed {
            node: name.to_string(),
            rule: rule.to_string(),
            expected: format!("a `{rule}` conclusion of `{}`", pc.render()),
            found: cc.render(),
        }),
        Err(why) => OneOf::Skip(why),
    }
}

/// Is `c` a binary resolvent of `a` and `b`?
///
/// Every complementary literal pair is tried, on standardised-apart copies.
/// The resolvent is taken as a SET, which is what makes the check agree with
/// a prover that factors the resolvent on the way out.
fn resolvent_matches(a: &Clause, b: &Clause, c: &Clause) -> Result<bool, &'static str> {
    let a = rename_apart(a, "_a");
    let b = rename_apart(b, "_b");
    for (i, la) in a.lits.iter().enumerate() {
        for (j, lb) in b.lits.iter().enumerate() {
            if la.positive == lb.positive {
                continue;
            }
            let Some(s) = unify_atoms(&la.atom, &lb.atom) else { continue };
            let mut rest: Vec<Lit> = Vec::new();
            rest.extend(a.lits.iter().enumerate().filter(|(k, _)| *k != i).map(|(_, l)| l.clone()));
            rest.extend(b.lits.iter().enumerate().filter(|(k, _)| *k != j).map(|(_, l)| l.clone()));
            let cand =
                Clause::normalise(apply_clause(&rest, &s), a.tautology || b.tautology);
            if variant(&cand, c)? {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

/// The JSON one check reports, with the vocabulary written out beside the
/// verdict, the way `fol_solve::outcome_json` does.
pub fn report_json(r: &Report) -> serde_json::Value {
    serde_json::json!({
        "szs_status": r.szs_status,
        "szs_status_means": "WHAT THE PROVER SAID ABOUT ITSELF. Echoed, untrusted, and used to \
                             decide nothing in this report",
        "problem_error": r.problem_error,
        "derivation_error": r.derivation_error,
        "problem_formulas": r.problem_formulas,
        "problem_digest": r.problem_digest,
        "derivation_nodes": r.derivation_nodes,
        "derivation_wellformed": r.derivation_wellformed,
        "wellformedness_failures": r.wellformedness_failures,
        "root_is_false": r.root_is_false,
        "nodes_outside_the_refutation": r.nodes_outside_the_refutation,
        "leaves_total": r.leaves_total,
        "leaves_match_problem": r.leaves_match_problem,
        "leaf_match_levels": r.leaf_match_levels,
        "leaf_failures": r.leaf_failures,
        "leaves_introduced": r.leaves_introduced,
        "conjecture_in_problem": r.conjecture_in_problem,
        "conjecture_used": r.conjecture_used,
        "conjecture_negation_checked": r.conjecture_negation_checked,
        "what_was_refuted": r.what_was_refuted,
        "steps_total": r.steps_total,
        "steps_checked": r.steps_checked,
        "steps_checked_by_rule": r.steps_checked_by_rule,
        "steps_unchecked": r.steps_unchecked,
        "steps_unchecked_total": r.steps_unchecked.iter().map(|u| u.count).sum::<usize>(),
        "steps_not_reconstructed": r.steps_not_reconstructed,
        "verdict": r.verdict,
        "verdict_means": verdict_means(r.verdict),
        "checked_by": "src/tstp.rs, an UNVERIFIED Rust replayer. lean/ is not involved and no \
                       theorem is cited. A refutation remains an ORACLE OPINION (decision \
                       0005); what this adds is that the opinion is now about a derivation \
                       somebody can read, over the problem this engine emitted",
    })
}

// ── Driving a prover ───────────────────────────────────────────────────────

/// Which prover to run. Both read TPTP FOF and both print TSTP on request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Prover {
    Vampire,
    Eprover,
}

impl Prover {
    pub fn parse(s: &str) -> anyhow::Result<Prover> {
        match s.to_ascii_lowercase().as_str() {
            "vampire" => Ok(Prover::Vampire),
            "eprover" | "e" => Ok(Prover::Eprover),
            other => anyhow::bail!("unknown prover {other:?}; expected `vampire` or `eprover`"),
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Prover::Vampire => "vampire",
            Prover::Eprover => "eprover",
        }
    }
    /// The argument vector, WITH the proof-printing option.
    ///
    /// It is not optional here. A run without it returns an SZS word and no
    /// derivation, which is the state this module exists to leave behind.
    pub fn argv(self, file: &Path, secs: u32) -> Vec<String> {
        let f = file.display().to_string();
        match self {
            Prover::Vampire => {
                vec!["--proof".into(), "tptp".into(), "-t".into(), format!("{secs}"), f]
            }
            Prover::Eprover => vec![
                "--auto".into(),
                "--tptp3-format".into(),
                "--proof-object".into(),
                format!("--cpu-limit={secs}"),
                f,
            ],
        }
    }
    pub fn install_line(self) -> &'static str {
        match self {
            Prover::Vampire => "install Vampire: `brew install vampire` (macOS) or \
                                https://github.com/vprover/vampire",
            Prover::Eprover => "install E: `brew install eprover` (macOS), \
                                `apt-get install eprover`, or https://github.com/eprover/eprover",
        }
    }
    pub fn available(self) -> bool {
        which(self.name()).is_some()
    }
}

fn which(bin: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths).find_map(|dir| {
            let p = dir.join(bin);
            if p.is_file() { Some(p) } else { None }
        })
    })
}

/// How to run the pipeline.
#[derive(Clone, Debug)]
pub struct ProveOptions {
    pub prover: Prover,
    /// Seconds per prover invocation.
    pub timeout_secs: u32,
}

impl Default for ProveOptions {
    fn default() -> Self {
        ProveOptions { prover: Prover::Vampire, timeout_secs: 30 }
    }
}

fn run_prover(prover: Prover, file: &Path, out: &Path, secs: u32) -> std::io::Result<String> {
    let f = std::fs::File::create(out)?;
    let f2 = f.try_clone()?;
    let mut cmd = Command::new(prover.name());
    cmd.args(prover.argv(file, secs))
        .stdout(Stdio::from(f))
        .stderr(Stdio::from(f2))
        .stdin(Stdio::null());
    let mut child = cmd.spawn()?;
    let start = Instant::now();
    let limit = Duration::from_secs((secs as u64).max(1) + 15);
    loop {
        match child.try_wait()? {
            Some(_) => break,
            None => {
                if start.elapsed() > limit {
                    let _ = child.kill();
                    let _ = child.wait();
                    break;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    }
    std::fs::read_to_string(out)
}

/// One problem: write it, run the prover, check what comes back.
fn prove_one(
    problem_text: &str,
    dir: &Path,
    stem: &str,
    opts: &ProveOptions,
) -> anyhow::Result<Report> {
    std::fs::create_dir_all(dir)?;
    let p = dir.join(format!("{stem}.p"));
    std::fs::write(&p, problem_text)?;
    let out = dir.join(format!("{stem}.tstp"));
    let text = run_prover(opts.prover, &p, &out, opts.timeout_secs)?;
    Ok(check(problem_text, &text))
}

/// Run the prover over the loaded ontology and, optionally, one problem per
/// goal, and check every derivation it prints.
///
/// The shape is `fol_solve::solve_export`'s, deliberately: the two are the
/// refutation half and the model half of the same question and a reader should
/// not have to learn two report formats.
pub fn prove_export(
    graph: &std::sync::Arc<crate::graph::GraphStore>,
    dir: &Path,
    opts: &ProveOptions,
    goals: Option<&Path>,
    goals_skip_columns: usize,
) -> anyhow::Result<String> {
    if !opts.prover.available() {
        return Ok(serde_json::json!({
            "prover": opts.prover.name(),
            "skipped": format!(
                "{} is not on PATH, so no derivation was produced and nothing was checked. \
                 That is the ABSENCE of a second opinion and not agreement with one. {}",
                opts.prover.name(),
                opts.prover.install_line()
            ),
        })
        .to_string());
    }
    let triples = graph.all_triples()?;
    let read = crate::tptp::read_graph(triples);
    std::fs::create_dir_all(dir)?;

    let base = crate::tptp::FolProblem::build(&read.axioms, None)?;
    let ontology = prove_one(&base.to_tptp(), &dir.join("ontology"), "ontology", opts)?;

    let mut counts: BTreeMap<&'static str, usize> = BTreeMap::new();
    *counts.entry(ontology.verdict).or_default() += 1;
    let mut goal_reports = Vec::new();
    let mut not_asked: Vec<serde_json::Value> = Vec::new();

    if let Some(path) = goals {
        let text = std::fs::read_to_string(path)?;
        for (i, line) in text.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let cols: Vec<&str> = line.split('\t').collect();
            if cols.len() < goals_skip_columns + 3 {
                not_asked.push(serde_json::json!({
                    "line": line,
                    "why": format!("fewer than {} tab-separated columns", goals_skip_columns + 3),
                }));
                continue;
            }
            let (s, p, o) = (
                cols[goals_skip_columns],
                cols[goals_skip_columns + 1],
                cols[goals_skip_columns + 2],
            );
            let ax = match crate::tptp::triple_as_axiom(&read, s, p, o) {
                Ok(ax) => ax,
                Err(why) => {
                    not_asked.push(serde_json::json!({"triple": [s, p, o], "why": why}));
                    continue;
                }
            };
            let gp = crate::tptp::FolProblem::build(&read.axioms, Some(&ax))?;
            let rep = prove_one(
                &gp.to_tptp(),
                &dir.join(format!("goal_{i:05}")),
                &format!("goal_{i:05}"),
                opts,
            )?;
            *counts.entry(rep.verdict).or_default() += 1;
            goal_reports.push(serde_json::json!({
                "triple": [s, p, o],
                "report": report_json(&rep),
            }));
        }
    }

    let rejected = counts.get("derivation_rejected").copied().unwrap_or(0);
    let unreconstructed = counts.get("refutation_step_not_reconstructed").copied().unwrap_or(0);

    Ok(serde_json::json!({
        "prover": opts.prover.name(),
        "timeout_secs": opts.timeout_secs,
        "dir": dir.display().to_string(),
        "ontology": report_json(&ontology),
        "goals": goal_reports,
        "not_asked": not_asked,
        "not_asked_means": "no axiom form in OwlLean/Syntax.lean corresponds to the triple, so \
                            the question could not be put at all. Listed rather than skipped, \
                            so a run cannot report a clean sweep over a subset nobody chose",
        "verdict_counts": counts,
        "stop_the_line": rejected + unreconstructed,
        "stop_the_line_means": "a derivation was REJECTED (a dangling parent, a cycle, or a \
                                leaf that is not a formula of the problem this engine emitted), \
                                or a step whose rule this checker implements did not \
                                reconstruct. The first means the prover refuted something other \
                                than what it was given; the second is either a defect in the \
                                derivation or a gap in this checker and needs a human. The \
                                command exits non-zero on either",
        "not_certified": "NOTHING HERE IS CERTIFIED. A refutation is an ORACLE OPINION \
                          (decision 0005) and stays one: the calculus's soundness is not \
                          machine-checked anywhere in lean/, and this replayer is ordinary \
                          Rust. What the check adds is that the opinion is now about a \
                          derivation that was read, whose leaves are the formulas this engine \
                          emitted, and some of whose steps were recomputed. The MODEL \
                          direction is the one that can be certified; see decision 0006 and \
                          onto_fol_model",
    })
    .to_string())
}

/// Check a problem file and a recorded prover output, with no store and no
/// prover run. This is what `tools/fol_differential.py` calls once it already
/// has both.
pub fn check_files(problem: &Path, proof: &Path) -> anyhow::Result<String> {
    let p = std::fs::read_to_string(problem)
        .map_err(|e| anyhow::anyhow!("cannot read problem {}: {e}", problem.display()))?;
    let d = std::fs::read_to_string(proof)
        .map_err(|e| anyhow::anyhow!("cannot read proof {}: {e}", proof.display()))?;
    let r = check(&p, &d);
    let mut v = report_json(&r);
    if let Some(o) = v.as_object_mut() {
        o.insert("problem".into(), serde_json::json!(problem.display().to_string()));
        o.insert("proof".into(), serde_json::json!(proof.display().to_string()));
        o.insert(
            "stop_the_line".into(),
            serde_json::json!(usize::from(matches!(
                r.verdict,
                "derivation_rejected" | "refutation_step_not_reconstructed"
            ))),
        );
    }
    Ok(v.to_string())
}
