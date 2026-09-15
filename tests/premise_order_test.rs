//! TCB-14: the premise order the engine emits is the order the checker matches,
//! and the two agree BY CONSTRUCTION rather than by a corpus test noticing.
//!
//! `OOCert.checkStep` in `lean/OOCert/Rules.lean` matches a step's premises
//! POSITIONALLY, arm by arm. Until 15 September 2026 the engine passed them in
//! an order written out by hand at each of thirty-one `emit` call sites, and
//! the only thing keeping the two in step was `tests/lean_certificate_test.rs`
//! running the real checker over a corpus. A rule added with its premises in
//! the wrong order produced certificates that failed for a reason no user could
//! act on, and a rule whose corpus coverage was thin could have been wrong for
//! a long time.
//!
//! The engine now computes the premise list and the conclusion from
//! `open_ontologies::reason::BUILTIN_RULES`, so a call site supplies a binding
//! and has no order to get wrong. That moves the question one level up: is THAT
//! table the checker's table? This file answers it by reading the answer out of
//! the checker.
//!
//! It parses the `checkStep` arms — the code, not the prose table in the
//! docstring above them — and rebuilds each arm's premise pattern:
//!
//!   * the arm's premise list `[⟨s, p, o⟩, ⟨p', d, c⟩]` gives the arity and the
//!     positions;
//!   * the conjuncts `p' = p` and `d = V.domain` say which positions are the
//!     same variable and which are fixed vocabulary;
//!   * `st.conclusion = ⟨s, V.type, c⟩` gives the head.
//!
//! Two patterns are then compared up to renaming of variables, because the
//! names are private to each side. A disagreement in ORDER, in ARITY, in which
//! position is fixed, or in WHICH vocabulary term is fixed, is a failure here.
//!
//! What this does NOT check: the four list rules (`cls-int1`, `cls-int2`,
//! `cls-uni`, `cls-oo`). Their premises are a constructor triple followed by an
//! RDF list chain whose length is the length of the list, so they are not a
//! fixed pattern on either side: the checker matches them with `takeChain` and
//! the engine builds the vector explicitly. They are named and counted rather
//! than passed over silently.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use open_ontologies::reason::{Kw, Slot, BUILTIN_RULES, CHAINED_RULES};

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rules_lean() -> String {
    std::fs::read_to_string(repo().join("lean/OOCert/Rules.lean"))
        .expect("lean/OOCert/Rules.lean is tracked by this repository")
}

/// A resolved pattern position: a fixed vocabulary term, or a variable
/// identified by the order of its first appearance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Pos {
    Fixed(&'static str),
    Var(usize),
}

/// A rule's premise patterns and its conclusion, canonically numbered.
type Shape = (Vec<[Pos; 3]>, [Pos; 3]);

/// Number the variables by first appearance over body-then-head, so two
/// patterns that differ only in what the two sides call their variables become
/// the same value.
fn canonical(atoms: &[[Option<&'static str>; 3]], names: &[[String; 3]]) -> Shape {
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    let mut out: Vec<[Pos; 3]> = Vec::new();
    for (a, n) in atoms.iter().zip(names) {
        let mut row = [Pos::Var(0); 3];
        for i in 0..3 {
            row[i] = match a[i] {
                Some(kw) => Pos::Fixed(kw),
                None => {
                    let next = seen.len();
                    Pos::Var(*seen.entry(n[i].clone()).or_insert(next))
                }
            };
        }
        out.push(row);
    }
    let head = out.pop().expect("body then head");
    (out, head)
}

// ─────────────────────────────────────────────────────────────────────────────
// The Rust side
// ─────────────────────────────────────────────────────────────────────────────

fn rust_shapes() -> BTreeMap<String, BTreeSet<Shape>> {
    let mut out: BTreeMap<String, BTreeSet<Shape>> = BTreeMap::new();
    for r in BUILTIN_RULES {
        let mut atoms: Vec<[Option<&'static str>; 3]> = Vec::new();
        let mut names: Vec<[String; 3]> = Vec::new();
        for a in r.body.iter().chain(std::iter::once(&r.head)) {
            let mut kws = [None; 3];
            let mut ns = [String::new(), String::new(), String::new()];
            for i in 0..3 {
                match a[i] {
                    Slot::Fixed(k) => kws[i] = Some(k.lean_name()),
                    Slot::Var(v) => ns[i] = r.vars[v].to_string(),
                }
            }
            atoms.push(kws);
            names.push(ns);
        }
        out.entry(r.name.to_string())
            .or_default()
            .insert(canonical(&atoms, &names));
    }
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// The Lean side
// ─────────────────────────────────────────────────────────────────────────────

/// `| .rdfs2 => "rdfs2"` for every constructor, out of `def Rule.name`.
fn ctor_to_name(src: &str) -> BTreeMap<String, String> {
    let start = src.find("def Rule.name").expect("Rule.name is defined");
    let end = src[start..].find("def Rule.all").expect("Rule.all follows") + start;
    let body = &src[start..end];
    let mut out = BTreeMap::new();
    for chunk in body.split('|').skip(1) {
        let Some((lhs, rhs)) = chunk.split_once("=>") else { continue };
        let ctor = lhs.trim().trim_start_matches('.').trim();
        let Some(q1) = rhs.find('"') else { continue };
        let rest = &rhs[q1 + 1..];
        let Some(q2) = rest.find('"') else { continue };
        if !ctor.is_empty() {
            out.insert(ctor.to_string(), rest[..q2].to_string());
        }
    }
    assert!(out.len() >= 25, "parsed {} constructors out of Rule.name", out.len());
    out
}

/// The identifier triples inside `⟨a, b, c⟩` groups, in source order.
fn angle_triples(s: &str) -> Vec<[String; 3]> {
    let mut out = Vec::new();
    let mut rest = s;
    while let Some(i) = rest.find('\u{27e8}') {
        rest = &rest[i + '\u{27e8}'.len_utf8()..];
        let Some(j) = rest.find('\u{27e9}') else { break };
        let inner = &rest[..j];
        rest = &rest[j + '\u{27e9}'.len_utf8()..];
        let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
        if parts.len() == 3 {
            out.push([
                parts[0].to_string(),
                parts[1].to_string(),
                parts[2].to_string(),
            ]);
        }
    }
    out
}

/// A tiny union-find over identifier names, so `p' = p` and `d = V.domain` can
/// be applied in any order.
#[derive(Default)]
struct Classes {
    parent: BTreeMap<String, String>,
    constant: BTreeMap<String, &'static str>,
}

impl Classes {
    fn find(&mut self, a: &str) -> String {
        let p = self.parent.get(a).cloned().unwrap_or_else(|| a.to_string());
        if p == a {
            return p;
        }
        let root = self.find(&p);
        self.parent.insert(a.to_string(), root.clone());
        root
    }

    fn union(&mut self, a: &str, b: &str) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra != rb {
            self.parent.insert(ra.clone(), rb.clone());
            if let Some(k) = self.constant.remove(&ra) {
                let prev = self.constant.insert(rb, k);
                assert!(
                    prev.is_none() || prev == Some(k),
                    "{a} and {b} are equated to two different vocabulary terms"
                );
            }
        }
    }

    fn bind_constant(&mut self, a: &str, k: &'static str) {
        let r = self.find(a);
        let prev = self.constant.insert(r, k);
        assert!(prev.is_none() || prev == Some(k), "{a} is two vocabulary terms");
    }

    fn resolve(&mut self, a: &str) -> (Option<&'static str>, String) {
        let r = self.find(a);
        (self.constant.get(&r).copied(), r)
    }
}

fn kw_by_lean_name(n: &str) -> Option<&'static str> {
    const ALL: &[Kw] = &[
        Kw::Type,
        Kw::SubClassOf,
        Kw::SubPropertyOf,
        Kw::Domain,
        Kw::Range,
        Kw::SameAs,
        Kw::InverseOf,
        Kw::TransitiveProperty,
        Kw::SymmetricProperty,
        Kw::EquivalentClass,
        Kw::EquivalentProperty,
        Kw::OnProperty,
        Kw::SomeValuesFrom,
        Kw::AllValuesFrom,
        Kw::HasValue,
    ];
    ALL.iter().map(|k| k.lean_name()).find(|k| *k == n)
}

struct LeanArms {
    shapes: BTreeMap<String, BTreeSet<Shape>>,
    variadic: BTreeSet<String>,
}

fn lean_arms() -> LeanArms {
    let src = rules_lean();
    let names = ctor_to_name(&src);

    let start = src.find("def checkStep").expect("checkStep is defined");
    let end = src[start..]
        .find("def checkAll")
        .expect("checkAll follows checkStep")
        + start;
    let body = &src[start..end];

    let mut shapes: BTreeMap<String, BTreeSet<Shape>> = BTreeMap::new();
    let mut variadic: BTreeSet<String> = BTreeSet::new();

    // Arms of `checkStep` start at column 2. The `| some (...)` arms of the
    // nested matches are indented further, which is what keeps this split
    // honest; `arms_are_all_accounted_for` is the check that it stayed that way.
    for arm in body.split("\n  | ").skip(1) {
        let Some((header, rest)) = arm.split_once("=>") else { continue };
        let header = header.trim();
        if header.starts_with('_') {
            continue; // the catch-all `| _, _ => false`
        }
        let Some((ctor, pattern)) = header.split_once(',') else { continue };
        let ctor = ctor.trim().trim_start_matches('.').trim();
        let name = names
            .get(ctor)
            .unwrap_or_else(|| panic!("checkStep has an arm for `{ctor}`, which Rule.name does not"))
            .clone();
        let pattern = pattern.trim();

        if !(pattern.starts_with('[') && pattern.ends_with(']')) {
            // `⟨c, io, l⟩ :: ps`, a list rule.
            variadic.insert(name);
            continue;
        }

        let premises = angle_triples(pattern);
        assert!(!premises.is_empty(), "{name}: no premises parsed from {pattern}");

        // The arm body, up to the start of the next arm.
        let mut cls = Classes::default();
        for (lhs, rhs) in equalities(rest) {
            match rhs.strip_prefix("V.") {
                Some(v) => {
                    let kw = kw_by_lean_name(v).unwrap_or_else(|| {
                        panic!("{name}: `V.{v}` has no `Kw` in the Rust table")
                    });
                    cls.bind_constant(&lhs, kw);
                }
                None => cls.union(&lhs, &rhs),
            }
        }

        let concl = conclusions(rest);
        assert!(!concl.is_empty(), "{name}: no `st.conclusion = ⟨..⟩` found");

        for c in &concl {
            let mut atoms: Vec<[Option<&'static str>; 3]> = Vec::new();
            let mut idents: Vec<[String; 3]> = Vec::new();
            for t in premises.iter().chain(std::iter::once(c)) {
                let mut kws = [None; 3];
                let mut ns = [String::new(), String::new(), String::new()];
                for i in 0..3 {
                    // A position written `V.type` directly is fixed; otherwise
                    // ask the equality classes.
                    if let Some(v) = t[i].strip_prefix("V.") {
                        kws[i] = Some(kw_by_lean_name(v).unwrap_or_else(|| {
                            panic!("{name}: `V.{v}` has no `Kw` in the Rust table")
                        }));
                    } else {
                        let (k, root) = cls.resolve(&t[i]);
                        kws[i] = k;
                        ns[i] = root;
                    }
                }
                atoms.push(kws);
                idents.push(ns);
            }
            shapes
                .entry(name.clone())
                .or_default()
                .insert(canonical(&atoms, &idents));
        }
    }

    LeanArms { shapes, variadic }
}

/// `a = b` and `a = V.foo` conjuncts. The right-hand side of
/// `st.conclusion = ⟨..⟩` is an angle group and never matches.
fn equalities(body: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let bytes: Vec<char> = body.chars().collect();
    let is_ident = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '\'' || c == '.';
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == '=' {
            // Not `==`, not `=>`, not `≠`.
            let next = bytes.get(i + 1).copied().unwrap_or(' ');
            let prev = if i == 0 { ' ' } else { bytes[i - 1] };
            if next == '>' || next == '=' || prev == '=' || prev == '!' {
                i += 1;
                continue;
            }
            let mut a = i;
            while a > 0 && bytes[a - 1] == ' ' {
                a -= 1;
            }
            let end_l = a;
            while a > 0 && is_ident(bytes[a - 1]) {
                a -= 1;
            }
            let lhs: String = bytes[a..end_l].iter().collect();
            let mut b = i + 1;
            while b < bytes.len() && bytes[b] == ' ' {
                b += 1;
            }
            let start_r = b;
            while b < bytes.len() && is_ident(bytes[b]) {
                b += 1;
            }
            let rhs: String = bytes[start_r..b].iter().collect();
            if !lhs.is_empty() && !rhs.is_empty() && !lhs.starts_with("st.") {
                out.push((lhs, rhs));
            }
            i = b;
            continue;
        }
        i += 1;
    }
    out
}

/// Every `st.conclusion = ⟨a, b, c⟩` in an arm. More than one when the rule
/// licenses more than one conclusion from the same premise.
fn conclusions(body: &str) -> Vec<[String; 3]> {
    let mut out = Vec::new();
    let mut rest = body;
    while let Some(i) = rest.find("st.conclusion = ") {
        rest = &rest[i + "st.conclusion = ".len()..];
        let group = angle_triples(rest);
        if let Some(first) = group.first() {
            out.push(first.clone());
        }
    }
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// The tests
// ─────────────────────────────────────────────────────────────────────────────

/// The parser above is itself a parser, and a parser that silently returned
/// nothing would make every assertion below vacuous.
#[test]
fn the_checker_arms_are_recovered() {
    let arms = lean_arms();
    assert_eq!(
        arms.shapes.len() + arms.variadic.len(),
        27 - 2 + 4,
        "recovered {} fixed-arity arms and {} list arms",
        arms.shapes.len(),
        arms.variadic.len()
    );
    for expect in ["rdfs2", "prp-trp", "cls-avf", "scm-avf2", "scm-rng2"] {
        assert!(arms.shapes.contains_key(expect), "{expect} missing");
    }
    // The rule with two conclusions from one premise really does come back with
    // two shapes, so the `∨` in that arm is being read.
    assert_eq!(arms.shapes["scm-eqc1"].len(), 2);
    assert_eq!(arms.shapes["scm-eqp1"].len(), 2);
    // And a two-premise rule really does come back with two premises, so the
    // arity is being read rather than defaulted.
    let (body, _) = arms.shapes["rdfs2"].iter().next().unwrap();
    assert_eq!(body.len(), 2);
    let (body, _) = arms.shapes["scm-svf1"].iter().next().unwrap();
    assert_eq!(body.len(), 5);
}

/// The list rules are exactly the ones the Rust table also declines to hold,
/// and nothing fell between the two sets.
#[test]
fn the_variadic_arms_are_the_chained_rules() {
    let arms = lean_arms();
    let expected: BTreeSet<String> = CHAINED_RULES.iter().map(|s| s.to_string()).collect();
    assert_eq!(arms.variadic, expected);

    let fixed: BTreeSet<String> = rust_shapes().keys().cloned().collect();
    let all: BTreeSet<String> = fixed.union(&expected).cloned().collect();
    // Every name the checker knows is in one bucket or the other.
    let src = rules_lean();
    let checker_names: BTreeSet<String> = ctor_to_name(&src).into_values().collect();
    assert_eq!(all, checker_names, "a rule is in neither table");
}

/// TCB-14. The premise ORDER the engine emits is the order the checker matches,
/// for every fixed-arity rule, derived from both sides rather than asserted.
#[test]
fn the_emitted_premise_order_is_the_checkers_order() {
    let lean = lean_arms().shapes;
    let rust = rust_shapes();

    assert_eq!(
        rust.keys().collect::<Vec<_>>(),
        lean.keys().collect::<Vec<_>>(),
        "the two tables cover different rules"
    );

    for (name, want) in &lean {
        let got = &rust[name];
        assert_eq!(
            got, want,
            "\n{name}: the engine's premise pattern is not the checker's.\n  \
             engine: {got:#?}\n  checker: {want:#?}"
        );
    }
}

/// The inverted rule stays inverted. `scm-avf2` concludes `c2 subClassOf c1`
/// where its three siblings conclude `c1 subClassOf c2`, and a table that got
/// it the natural way round would emit a step no model supports.
/// `OOCert.the_natural_avf2_direction_is_not_entailed` is the refutation, and
/// this is the check that the Rust did not quietly normalise it.
#[test]
fn scm_avf2_concludes_the_other_way_round() {
    let rust = rust_shapes();
    let (b1, h1) = rust["scm-svf2"].iter().next().unwrap().clone();
    let (b2, h2) = rust["scm-avf2"].iter().next().unwrap().clone();
    // Same body shape up to the vocabulary term, opposite conclusion.
    assert_eq!(b1.len(), b2.len());
    assert_ne!(
        h1, h2,
        "scm-avf2 concludes in the same direction as scm-svf2, which is unsound"
    );
    let flip = [h2[2], h2[1], h2[0]];
    assert_eq!(h1, flip, "scm-avf2's conclusion is not the reverse of scm-svf2's");
}
