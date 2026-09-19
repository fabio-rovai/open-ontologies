//! The W3C SHACL test suite, run against the VERIFIED evaluator in `lean/Shacl/`.
//!
//! # What this measures, and why it exists
//!
//! `tests/w3c_shacl_conformance_test.rs` measures the Rust engine, which compiles
//! every SHACL constraint into a SPARQL query and hands it to Oxigraph. That path
//! cannot be verified without verifying SPARQL, and no verified SPARQL engine
//! exists. The usual conclusion is that a verified SHACL validator is therefore out
//! of reach.
//!
//! The hole in that argument is that SHACL Core does not need SPARQL. The
//! Recommendation says of its own SPARQL definitions that they "represent potential
//! validators ... included for illustration purposes only and have no formal status
//! otherwise"; the normative validators are the prose ones in Appendix D. So
//! `lean/Shacl/Spec.lean` states those conditions directly over a finite graph,
//! `lean/Shacl/Eval.lean` decides them, and `Shacl.validate_spec` proves the two
//! agree. There is no SPARQL engine anywhere in that path.
//!
//! This file is what stops that from being an admirable claim. It runs the verified
//! evaluator over the same 120 tests, under the same comparison, in the same four
//! buckets, and prints the number.
//!
//! # A zero in the FAIL column is the claim, not the size of the PASS column
//!
//! The Lean development covers every SHACL Core constraint component, all four
//! target forms, and four of the six path forms. Everything else is REFUSED by the
//! compiler in `lean/Shacl/Compile.lean`, which lands in UNDETERMINED. A zero FAIL
//! count is the shape this must have: a FAIL is the verified evaluator giving an
//! answer that disagrees with the Working Group, which means either the
//! specification in `lean/Shacl/Spec.lean` misreads the Recommendation or the
//! compiler misreads a shapes graph.
//!
//! What is still refused, and why, each named where it lives:
//!
//! * `sh:sparql` and user-defined constraint components, 22 of the 120 tests. That
//!   is the SHACL extension mechanism and it needs a SPARQL evaluator. Avoiding
//!   SPARQL is the entire reason this development can exist, so these are out of
//!   scope rather than unfinished.
//! * `sh:zeroOrMorePath` and `sh:oneOrMorePath`, 4 tests. Every other path form is
//!   decided by a recursion over the path itself; these two are transitive closures
//!   over the DATA graph. See the `Path` type in `lean/Shacl/Shape.lean`.
//! * A path node carrying two path forms at once, 2 tests. Deliberate: SHACL allows
//!   exactly one, the shapes graph is ill formed, and no reading of it is chosen.
//! * A `sh:pattern` whose lexical form carries a backslash, 1 test. Nothing in this
//!   development decodes an N-Triples escape, so it cannot tell a literal backslash
//!   from an escape sequence.
//!
//! The Rust engine's own number is printed alongside at run time, read from its
//! committed baseline rather than repeated here, because a number typed into
//! prose goes stale silently and this one did within a day. The comparison is
//! not apples to apples either way: the two answer different SETS of tests.
//! What is comparable is how many of the 120 each ANSWERS and how many of those
//! it gets wrong.
//!
//! And FAIL at 0 claims less than it looks. It is 0 on the tests this evaluator
//! answers, under a lens that ignores sh:resultPath, sh:value, sh:resultSeverity,
//! sh:resultMessage, result multiplicity and blank-node identity. It is 0 on
//! this suite, which is not the same as 0 against the Recommendation: the suite
//! is the only gate on the compiler, so a constraint the suite never exercises
//! is not checked by anything here.
//!
//! # The comparison, which is the same one the other harness uses
//!
//! COMPARED: the `sh:conforms` verdict, and the SET of
//! `(focus node, sh:sourceShape, sh:sourceConstraintComponent)` triples.
//!
//! IGNORED, each for the same reason as in `w3c_shacl_conformance_test.rs`:
//! `sh:resultPath` and `sh:value` (blank-node path structures need graph
//! isomorphism), `sh:resultSeverity` (this validator never emits one and ignores
//! `sh:severity`; see `lean/Shacl/Compile.lean`), `sh:resultMessage`, multiplicity
//! (sets, not multisets), and blank node identity (canonicalised to `_:`).
//!
//! Keeping the lens identical is deliberate. Two numbers produced under two
//! different lenses cannot be compared, and the comparison is the point.
//!
//! # Buckets
//!
//! PASS         verdict and result set both agree; or the test expects
//!              `sht:Failure` and the bridge reported an error.
//! FAIL         the evaluator answered and the answer is wrong.
//! UNDETERMINED exit 3: the compiler refused the shapes graph, or the evaluator
//!              declined to judge a literal, to order two terms, or to read an
//!              escaped spelling. Not a pass and not a failure. NOT credit.
//! ERROR        exit 2, or a graph that would not load or re-serialise.
//!
//! # The gate
//!
//! A committed baseline in `tests/fixtures/shaclcore/baseline.json`, ratcheted:
//! PASS may not drop, FAIL may not rise, and no named PASS may regress. Re-cut with
//! `OO_SHACL_CORE_UPDATE_BASELINE=1 cargo test --test shacl_core_verified_test`.
//!
//! `the_bridge_refuses_what_it_cannot_check` feeds the bridge six shapes graphs it
//! must refuse and three data graphs whose verdicts must differ, and requires the
//! exact exit code and a reason that names the thing refused. Its own docstring
//! records the runs in which it was seen to fail.

mod common;


use oxigraph::io::{RdfFormat, RdfParser};
use oxigraph::sparql::{QueryResults, SparqlEvaluator};
use oxigraph::store::Store;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// The base every relative IRI in the suite resolves against. Identical to the
/// other harness: the resolved IRIs end up inside `sh:sourceShape` on both sides,
/// so a base that varied with the checkout would make the comparison
/// machine-dependent.
const SUITE_BASE: &str = "https://w3c.github.io/data-shapes/data-shapes-test-suite/tests/";

/// The upstream commit `tests/w3c-shacl/` was taken from.
const SUITE_COMMIT: &str = "94d8bc2bd4fc4fdc6f2964d1ec4a892329e05f06";

/// Entries reachable from the root manifest at `SUITE_COMMIT`. The same 120 the
/// other harness walks, so the denominators are the same number.
const EXPECTED_ENTRIES: usize = 120;

const PREFIXES: &str = r#"
PREFIX mf:   <http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#>
PREFIX sht:  <http://www.w3.org/ns/shacl-test#>
PREFIX sh:   <http://www.w3.org/ns/shacl#>
PREFIX rdf:  <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
"#;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn lean_dir() -> PathBuf {
    repo().join("lean")
}

fn suite_root() -> PathBuf {
    repo().join("tests").join("w3c-shacl")
}

fn fixtures() -> PathBuf {
    repo().join("tests").join("fixtures").join("shaclcore")
}

fn baseline_path() -> PathBuf {
    fixtures().join("baseline.json")
}

/// `.current_dir(lean_dir())` is not cosmetic: elan resolves the toolchain from the
/// working directory's `lean-toolchain`, and the crate root has none in its
/// ancestry. See the same note in `tests/lean_certificate_test.rs`, which was
/// written after this exact probe reported lake as missing in CI.
fn lake_available() -> bool {
    Command::new("lake")
        .arg("--version")
        .current_dir(lean_dir())
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn skip() -> bool {
    common::skip_unless(
        lake_available() && suite_root().join("manifest.ttl").is_file(),
        "lake (the Lean 4 build tool) and the vendored W3C SHACL suite at tests/w3c-shacl/",
        "install elan from https://github.com/leanprover/elan; the suite is normally committed",
    )
}

/// Build the verified evaluator once per test binary.
///
/// `lake build oo-shacl` compiles `lean/Shacl/` in full, and compiling it IS
/// checking the proofs: a `sorry`, a `native_decide`, or an axiom list that no
/// longer matches the `#guard_msgs` pins fails here. So a green run of this file
/// entails a green proof check, and a failure is a failure rather than a skip.
///
/// The plain `lake build` used by `tests/lean_certificate_test.rs` does NOT build
/// this target: `lean/lakefile.toml` lists `defaultTargets = ["OOCert", "oo-cert",
/// "oo-horn"]` and this task was not permitted to edit that line. Naming the target
/// explicitly is the workaround, and the consequence is that a bare `lake build` in
/// `lean/` does not check these proofs. Adding `"Shacl"` and `"oo-shacl"` to
/// `defaultTargets` is the fix and is reported rather than made.
fn evaluator() -> &'static Path {
    static BUILT: OnceLock<PathBuf> = OnceLock::new();
    BUILT.get_or_init(|| {
        let out = Command::new("lake")
            .args(["build", "Shacl", "oo-shacl"])
            .current_dir(lean_dir())
            .output()
            .expect("run lake build Shacl oo-shacl");
        assert!(
            out.status.success(),
            "lake build Shacl oo-shacl failed. The proofs are part of this build, so this is a \
             proof failure, not a build inconvenience:\n{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        let exe = lean_dir().join(".lake").join("build").join("bin").join("oo-shacl");
        assert!(exe.exists(), "oo-shacl binary missing at {}", exe.display());
        exe
    })
}

fn scratch() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let dir = std::env::temp_dir().join(format!("oo-shacl-core-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Running the bridge
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct Verdict {
    code: i32,
    json: serde_json::Value,
    raw: String,
}

fn run_bridge(data: &Path, shapes: &Path) -> Verdict {
    let out = Command::new(evaluator())
        .arg("validate")
        .arg(data)
        .arg(shapes)
        .output()
        .expect("run oo-shacl");
    let raw = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let json = serde_json::from_str(raw.lines().next().unwrap_or("")).unwrap_or(serde_json::Value::Null);
    Verdict { code: out.status.code().unwrap_or(-1), json, raw }
}

// ─────────────────────────────────────────────────────────────────────────────
// Manifest walk
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ResultKey {
    focus: String,
    shape: String,
    component: String,
}

impl std::fmt::Display for ResultKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {}, {})", short(&self.focus), short(&self.shape), short(&self.component))
    }
}

#[derive(Debug, Clone)]
enum Expectation {
    Failure,
    Report { conforms: bool, results: BTreeSet<ResultKey> },
}

#[derive(Debug, Clone)]
struct TestCase {
    name: String,
    data: PathBuf,
    shapes: PathBuf,
    expected: Expectation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Outcome {
    Pass,
    Fail(String),
    Undetermined(String),
    Error(String),
}

impl Outcome {
    fn bucket(&self) -> &'static str {
        match self {
            Outcome::Pass => "PASS",
            Outcome::Fail(_) => "FAIL",
            Outcome::Undetermined(_) => "UNDETERMINED",
            Outcome::Error(_) => "ERROR",
        }
    }
    fn detail(&self) -> &str {
        match self {
            Outcome::Pass => "",
            Outcome::Fail(d) | Outcome::Undetermined(d) | Outcome::Error(d) => d,
        }
    }
}

fn short(iri: &str) -> String {
    if iri.starts_with('"') || iri.starts_with("_:") || !iri.contains(['#', '/']) {
        return iri.to_string();
    }
    iri.rsplit(['#', '/']).next().unwrap_or(iri).to_string()
}

/// Canonical comparison form. IRIs lose their angle brackets, blank nodes collapse
/// to `_:`, and a literal is passed through an Oxigraph store once. Applied to both
/// sides, so the two are symmetric.
///
/// # Why a literal goes through a store
///
/// Oxigraph encodes an XSD-typed literal as a native value and hands back the
/// CANONICAL lexical form on the way out, so `"4.0"^^xsd:decimal` comes out of a
/// store as `"4"^^xsd:decimal`. The expected side of every comparison is read out of
/// a `Store` with SPARQL, so it has already been through that; the produced side
/// comes straight from the validator, which is now fed a term-faithful graph (see
/// `to_ntriples`) and so has not. Putting both through the same step is what keeps
/// the lens symmetric. Canonicalisation is idempotent, so the expected side is
/// unchanged by it.
///
/// What it costs, stated rather than discovered: two lexical forms of one value
/// within one datatype become one comparison key, so a validator that reported
/// `"4.0"^^xsd:decimal` where the Working Group wrote `"4"^^xsd:decimal` is not
/// caught here. Two DIFFERENT datatypes never merge, so `"4"^^xsd:integer` and
/// `"4.0"^^xsd:decimal` stay distinct and a validator confusing those two is still
/// caught.
fn canon(term: &str) -> String {
    let t = term.trim();
    if t.starts_with("_:") {
        return "_:".to_string();
    }
    if let Some(inner) = t.strip_prefix('<').and_then(|s| s.strip_suffix('>')) {
        return inner.to_string();
    }
    if t.starts_with('"') {
        return literal_canon(t);
    }
    t.to_string()
}

/// One literal through an Oxigraph store and back. Anything that will not make the
/// round trip is handed back untouched, so a term this cannot parse turns into a
/// disagreement rather than into a silent match.
fn literal_canon(lit: &str) -> String {
    let Ok(store) = Store::new() else { return lit.to_string() };
    let line = format!("<urn:x-oo-canon:s> <urn:x-oo-canon:p> {lit} .\n");
    let parser =
        RdfParser::from_format(RdfFormat::NTriples).for_reader(Cursor::new(line.into_bytes()));
    for quad in parser {
        let Ok(quad) = quad else { return lit.to_string() };
        if store.insert(&quad).is_err() {
            return lit.to_string();
        }
    }
    match store.iter().next() {
        Some(Ok(q)) => q.object.to_string(),
        _ => lit.to_string(),
    }
}

fn rows(store: &Store, query: &str) -> anyhow::Result<Vec<HashMap<String, String>>> {
    let mut out = Vec::new();
    if let QueryResults::Solutions(solutions) =
        SparqlEvaluator::new().parse_query(query)?.on_store(store).execute()?
    {
        let vars: Vec<String> =
            solutions.variables().iter().map(|v| v.as_str().to_string()).collect();
        for solution in solutions {
            let solution = solution?;
            let mut row = HashMap::new();
            for v in &vars {
                if let Some(t) = solution.get(v.as_str()) {
                    row.insert(v.clone(), t.to_string());
                }
            }
            out.push(row);
        }
    }
    Ok(out)
}

fn cell(row: &HashMap<String, String>, var: &str) -> String {
    row.get(var).cloned().unwrap_or_else(|| "(none)".to_string())
}

fn rel(path: &Path) -> String {
    path.strip_prefix(suite_root())
        .unwrap_or(path)
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join("/")
}

fn base_of(path: &Path) -> String {
    format!("{SUITE_BASE}{}", rel(path))
}

fn path_of(iri: &str) -> Option<PathBuf> {
    iri.strip_prefix(SUITE_BASE).map(|r| suite_root().join(r))
}

fn load_graph(path: &Path) -> anyhow::Result<Store> {
    let text = std::fs::read_to_string(path)?;
    let store = Store::new()?;
    let parser = RdfParser::from_format(RdfFormat::Turtle)
        .with_base_iri(base_of(path))?
        .for_reader(Cursor::new(text.as_bytes()));
    for quad in parser {
        store.insert(&quad?)?;
    }
    Ok(store)
}

fn collect_tests() -> anyhow::Result<Vec<TestCase>> {
    let mut seen: BTreeSet<PathBuf> = BTreeSet::new();
    let mut out: Vec<TestCase> = Vec::new();
    walk(&suite_root().join("manifest.ttl"), &mut seen, &mut out)?;
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

fn walk(
    manifest: &Path,
    seen: &mut BTreeSet<PathBuf>,
    out: &mut Vec<TestCase>,
) -> anyhow::Result<()> {
    let canonical = std::fs::canonicalize(manifest).unwrap_or_else(|_| manifest.to_path_buf());
    if !seen.insert(canonical) {
        return Ok(());
    }
    let store = load_graph(manifest)?;

    let entries: Vec<String> = rows(
        &store,
        &format!(
            "{PREFIXES}
             SELECT ?e WHERE {{ ?m mf:entries/rdf:rest*/rdf:first ?e . ?e a sht:Validate . }}"
        ),
    )?
    .into_iter()
    .filter_map(|r| r.get("e").cloned())
    .collect();
    for entry in entries {
        out.push(read_entry(&store, manifest, &entry)?);
    }

    let mut includes: Vec<String> =
        rows(&store, &format!("{PREFIXES} SELECT ?i WHERE {{ ?m mf:include ?i }}"))?
            .into_iter()
            .filter_map(|r| r.get("i").map(|s| canon(s)))
            .collect();
    includes.sort();
    includes.dedup();
    for inc in includes {
        match path_of(&inc) {
            Some(p) if p.is_file() => walk(&p, seen, out)?,
            _ => anyhow::bail!("{}: mf:include leaves the vendored suite: {inc}", rel(manifest)),
        }
    }
    Ok(())
}

fn read_entry(store: &Store, manifest: &Path, entry: &str) -> anyhow::Result<TestCase> {
    let action = rows(
        store,
        &format!(
            "{PREFIXES}
             SELECT ?d ?s WHERE {{ {entry} mf:action ?a . ?a sht:dataGraph ?d ; sht:shapesGraph ?s . }}"
        ),
    )?;
    let Some(a) = action.first() else {
        anyhow::bail!("{}: entry {entry} has no mf:action naming both graphs", rel(manifest));
    };
    let data = path_of(&canon(&cell(a, "d")))
        .ok_or_else(|| anyhow::anyhow!("{}: sht:dataGraph outside the suite", rel(manifest)))?;
    let shapes = path_of(&canon(&cell(a, "s")))
        .ok_or_else(|| anyhow::anyhow!("{}: sht:shapesGraph outside the suite", rel(manifest)))?;

    let result_term = rows(store, &format!("{PREFIXES} SELECT ?r WHERE {{ {entry} mf:result ?r }}"))?
        .first()
        .map(|r| cell(r, "r"))
        .unwrap_or_else(|| "(none)".to_string());

    let expected = if canon(&result_term) == "http://www.w3.org/ns/shacl-test#Failure" {
        Expectation::Failure
    } else {
        let conforms = rows(
            store,
            &format!("{PREFIXES} SELECT ?c WHERE {{ {entry} mf:result ?r . ?r sh:conforms ?c }}"),
        )?
        .first()
        .map(|r| {
            let c = cell(r, "c");
            c.starts_with("\"true\"") || c == "true"
        })
        .ok_or_else(|| {
            anyhow::anyhow!(
                "{}: entry {entry} has neither sh:conforms nor sht:Failure",
                rel(manifest)
            )
        })?;

        let mut results = BTreeSet::new();
        for row in rows(
            store,
            &format!(
                "{PREFIXES}
                 SELECT ?f ?sh ?c WHERE {{
                     {entry} mf:result ?r . ?r sh:result ?res .
                     OPTIONAL {{ ?res sh:focusNode ?f }}
                     OPTIONAL {{ ?res sh:sourceShape ?sh }}
                     OPTIONAL {{ ?res sh:sourceConstraintComponent ?c }}
                 }}"
            ),
        )? {
            results.insert(ResultKey {
                focus: canon(&cell(&row, "f")),
                shape: canon(&cell(&row, "sh")),
                component: canon(&cell(&row, "c")),
            });
        }
        Expectation::Report { conforms, results }
    };

    let mut name = rel(manifest);
    name.truncate(name.len().saturating_sub(".ttl".len()));
    let local = short(&canon(entry));
    if !name.ends_with(&local) {
        name = format!("{name}::{local}");
    }
    Ok(TestCase { name, data, shapes, expected })
}

// ─────────────────────────────────────────────────────────────────────────────
// Running one test
// ─────────────────────────────────────────────────────────────────────────────

/// Turtle to N-Triples with the file's IRI as base. The Lean bridge reads
/// N-Triples only, on purpose: prefix and base resolution are exactly the kind of
/// work that is easy to get subtly wrong, and doing it here rather than in Lean
/// keeps it out of the verified path AND makes this harness resolve IRIs the same
/// way the expected reports do.
///
/// This is the one part of the pipeline whose correctness is assumed rather than
/// proved or measured. A mis-resolved IRI would show up as a disagreement, not as a
/// false pass, because the expected side resolves through the same code.
///
/// # It goes parser to serialiser, and no longer through the engine's store
///
/// It used to load the Turtle into `GraphStore` and serialise that. **That round
/// trip does not preserve RDF terms.** Oxigraph encodes an XSD-typed literal as a
/// native value and hands back the CANONICAL lexical form on the way out, so
/// `"1"^^xsd:boolean` in a shapes file arrived at the validator as
/// `"true"^^xsd:boolean`. SHACL reads several parameter values as terms rather
/// than as values, and `core/property/uniqueLang-002` is the Working Group's test
/// for exactly that distinction: it says in a comment that `"1"^^xsd:boolean` must
/// NOT activate `sh:uniqueLang`. Under the old conversion no validator could pass
/// it, because the distinction was destroyed before the validator ran.
///
/// The finding is about `GraphStore`, which this task does not own, so it is
/// reported rather than fixed there: **`GraphStore::load_turtle` followed by
/// `GraphStore::serialize("ntriples")` is not term-preserving.** Anything in `src/`
/// that round-trips a graph through the store and then compares terms, SHACL
/// validation included, inherits that.
///
/// Parsing is still Oxigraph's, so base and prefix resolution are unchanged.
/// Triples are deduplicated and sorted here because the store used to do both, and
/// the compiler in `lean/Shacl/Compile.lean` reads `sh:path` as "exactly one
/// object" and would refuse a shapes graph that merely repeated a triple.
fn to_ntriples(path: &Path) -> Result<String, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("unreadable: {e}"))?;
    let parser = RdfParser::from_format(RdfFormat::Turtle)
        .with_base_iri(base_of(path))
        .map_err(|e| format!("has an unusable base IRI: {e}"))?
        .for_reader(Cursor::new(text.into_bytes()));
    let mut lines: BTreeSet<String> = BTreeSet::new();
    for quad in parser {
        let q = quad.map_err(|e| format!("does not parse: {}", first_line(&e.to_string())))?;
        lines.insert(format!("{} {} {} .", q.subject, q.predicate, q.object));
    }
    Ok(lines.into_iter().collect::<Vec<_>>().join("\n"))
}

/// The outcome of one test, plus how many targeted shapes the compiler found.
///
/// The shape count is not decoration. A test whose expected report is
/// `sh:conforms true` PASSES for free if the compiler found no targeted shape at
/// all, and counting those as credit is exactly the silent success this repository
/// exists to catch. They are counted separately and printed on their own line.
fn run_case(tc: &TestCase, idx: usize) -> (Outcome, Option<u64>) {
    let data_nt = match to_ntriples(&tc.data) {
        Ok(s) => s,
        Err(e) => return (Outcome::Error(format!("data graph {e}")), None),
    };
    let shapes_nt = match to_ntriples(&tc.shapes) {
        Ok(s) => s,
        Err(e) => return (Outcome::Error(format!("shapes graph {e}")), None),
    };
    let dp = scratch().join(format!("data-{idx}.nt"));
    let sp = scratch().join(format!("shapes-{idx}.nt"));
    if let Err(e) = std::fs::write(&dp, &data_nt) {
        return (Outcome::Error(format!("cannot write scratch data graph: {e}")), None);
    }
    if let Err(e) = std::fs::write(&sp, &shapes_nt) {
        return (Outcome::Error(format!("cannot write scratch shapes graph: {e}")), None);
    }

    // `OO_SHACL_DUMP=core/property/uniqueLang-002 cargo test --test
    // shacl_core_verified_test -- --nocapture` prints the exact N-Triples the
    // validator was handed for one test. It is here because a disagreement is as
    // likely to be the conversion above distorting a term as it is to be the
    // evaluator, and reading the actual input is the only way to tell which. It
    // found the `GraphStore` round trip documented on `to_ntriples`.
    if std::env::var("OO_SHACL_DUMP").as_deref() == Ok(tc.name.as_str()) {
        eprintln!("---- {} data ----\n{data_nt}\n---- shapes ----\n{shapes_nt}", tc.name);
    }
    let v = run_bridge(&dp, &sp);
    let shapes_found = v.json["shapes"].as_u64();
    let outcome = match v.code {
        3 => {
            let reason = v.json["reason"].as_str().unwrap_or("unnamed").to_string();
            Outcome::Undetermined(first_line(&reason))
        }
        2 => {
            let reason = v.json["reason"].as_str().unwrap_or(&v.raw).to_string();
            // The suite's rule: a test whose mf:result is sht:Failure passes if the
            // validation reported a failure. Exit 2 is that report.
            match tc.expected {
                Expectation::Failure => Outcome::Pass,
                _ => Outcome::Error(first_line(&reason)),
            }
        }
        0 => compare_verdict(tc, &v),
        other => Outcome::Error(format!("unexpected exit code {other}: {}", first_line(&v.raw))),
    };
    (outcome, shapes_found)
}

/// Compare a produced verdict with the expected one, under the lens the module
/// header states.
fn compare_verdict(tc: &TestCase, v: &Verdict) -> Outcome {
    let Some(actual_conforms) = v.json["conforms"].as_bool() else {
        return Outcome::Error(format!("exit 0 without a conforms field: {}", v.raw));
    };
    let actual: BTreeSet<ResultKey> = v.json["results"]
        .as_array()
        .map(|rs| {
            rs.iter()
                .map(|r| ResultKey {
                    focus: canon(r["focus"].as_str().unwrap_or("(none)")),
                    shape: canon(r["sourceShape"].as_str().unwrap_or("(none)")),
                    component: canon(r["sourceConstraintComponent"].as_str().unwrap_or("(none)")),
                })
                .collect()
        })
        .unwrap_or_default();
    match &tc.expected {
        Expectation::Failure => Outcome::Fail(format!(
            "verdict: the suite requires a reported failure for this shapes graph; the verified \
             evaluator returned conforms={actual_conforms}"
        )),
        Expectation::Report { conforms, results } => {
            if actual_conforms != *conforms {
                return Outcome::Fail(format!(
                    "verdict: expected conforms={conforms}, got {actual_conforms} ({} expected \
                     result(s), {} produced)",
                    results.len(),
                    actual.len()
                ));
            }
            if actual == *results {
                return Outcome::Pass;
            }
            let missing: Vec<String> = results.difference(&actual).map(|k| k.to_string()).collect();
            let extra: Vec<String> = actual.difference(results).map(|k| k.to_string()).collect();
            Outcome::Fail(format!(
                "results: verdict agrees (conforms={conforms}) but the result set differs; {} \
                 missing {}, {} unexpected {}",
                missing.len(),
                preview(&missing),
                extra.len(),
                preview(&extra)
            ))
        }
    }
}

fn first_line(s: &str) -> String {
    s.lines().next().unwrap_or(s).to_string()
}

fn preview(items: &[String]) -> String {
    if items.is_empty() {
        return "[]".to_string();
    }
    let shown: Vec<&str> = items.iter().take(2).map(String::as_str).collect();
    if items.len() > 2 {
        format!("[{}, ...]", shown.join("; "))
    } else {
        format!("[{}]", shown.join("; "))
    }
}

/// The short reason a test was undetermined, clustered so that the next piece of
/// work is chosen by evidence. "not implemented: sh:pattern" is a different job
/// from "recursive shapes".
fn refusal_class(detail: &str) -> String {
    if let Some(i) = detail.find("the constraint parameter ") {
        let rest = &detail[i + "the constraint parameter ".len()..];
        let name = rest.split(' ').next().unwrap_or(rest);
        return format!("parameter not implemented: {}", short(&canon(name)));
    }
    if detail.contains("extension mechanism") {
        return "SHACL extension mechanism (sh:sparql / constraint component)".to_string();
    }
    if detail.contains("recursive") {
        return "recursive shapes".to_string();
    }
    if detail.contains("more than one path form") {
        return "path node with more than one path form (ill-formed shapes graph)".to_string();
    }
    if detail.contains("lexical space") {
        return "datatype lexical space not implemented".to_string();
    }
    if detail.contains("value comparison") {
        return "value comparison: no rule orders these two terms".to_string();
    }
    if detail.contains("string length") {
        return "string length: the spelling carries a backslash escape".to_string();
    }
    if detail.contains("carries a backslash") {
        return "sh:pattern: the pattern carries a backslash escape".to_string();
    }
    if detail.contains("regular-expression construct") {
        let i = detail.find('\'').map(|i| i + 1).unwrap_or(0);
        let c = detail[i..].chars().next().unwrap_or('?');
        return format!("sh:pattern: the regular-expression construct '{c}' is not implemented");
    }
    if detail.contains("is not implemented") {
        let i = detail.rfind(": ").map(|i| i + 2).unwrap_or(0);
        return format!("path form: {}", &detail[i..]);
    }
    if detail.contains("malformed RDF list") {
        return "malformed RDF list".to_string();
    }
    first_line(detail)
}

// ─────────────────────────────────────────────────────────────────────────────
// The run
// ─────────────────────────────────────────────────────────────────────────────

struct Run {
    cases: Vec<TestCase>,
    outcomes: Vec<Outcome>,
    counts: Counts,
    /// Tests where the verdict agreed even if the result set did not. The suite
    /// calls this partial compliance.
    partial: usize,
    /// PASSes earned with no targeted shape compiled at all. Free credit, counted
    /// so it can be subtracted by anyone who reads the number.
    vacuous_passes: Vec<String>,
    /// PASSes where the expected report is NON-EMPTY, so the evaluator had to
    /// produce the right violations at the right nodes with the right components,
    /// not merely stay silent. The strongest number in this file.
    substantive_passes: usize,
    /// Tests expecting `sht:Failure`, and how many of them landed in each bucket.
    /// A refusal is not a reported failure and is not credited as one.
    failure_expected: usize,
    failure_undetermined: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
struct Counts {
    total: usize,
    pass: usize,
    fail: usize,
    undetermined: usize,
    error: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct Baseline {
    note: String,
    suite_commit: String,
    counts: Counts,
    partial_compliance: usize,
    outcomes: BTreeMap<String, String>,
}

fn run() -> &'static Run {
    static RUN: OnceLock<Run> = OnceLock::new();
    RUN.get_or_init(|| {
        let cases = collect_tests().expect("walk the vendored W3C SHACL manifests");
        let mut outcomes = Vec::with_capacity(cases.len());
        let mut counts = Counts { total: cases.len(), ..Default::default() };
        let mut partial = 0usize;
        let mut vacuous_passes = Vec::new();
        let mut substantive_passes = 0usize;
        let mut failure_expected = 0usize;
        let mut failure_undetermined = 0usize;
        for (i, tc) in cases.iter().enumerate() {
            let (o, shapes_found) = run_case(tc, i);
            match &o {
                Outcome::Pass => {
                    counts.pass += 1;
                    partial += 1;
                    if shapes_found == Some(0) {
                        vacuous_passes.push(tc.name.clone());
                    }
                    if let Expectation::Report { results, .. } = &tc.expected
                        && !results.is_empty()
                    {
                        substantive_passes += 1;
                    }
                }
                Outcome::Fail(d) => {
                    counts.fail += 1;
                    if d.starts_with("results:") {
                        partial += 1;
                    }
                }
                Outcome::Undetermined(_) => {
                    counts.undetermined += 1;
                    if matches!(tc.expected, Expectation::Failure) {
                        failure_undetermined += 1;
                    }
                }
                Outcome::Error(_) => counts.error += 1,
            }
            if matches!(tc.expected, Expectation::Failure) {
                failure_expected += 1;
            }
            outcomes.push(o);
        }
        Run {
            cases,
            outcomes,
            counts,
            partial,
            vacuous_passes,
            substantive_passes,
            failure_expected,
            failure_undetermined,
        }
    })
}

fn outcome_map(r: &Run) -> BTreeMap<String, String> {
    r.cases
        .iter()
        .zip(r.outcomes.iter())
        .map(|(c, o)| (c.name.clone(), o.bucket().to_string()))
        .collect()
}

fn ratchet_verdict(
    base: &Counts,
    base_outcomes: &BTreeMap<String, String>,
    now: &Counts,
    now_outcomes: &BTreeMap<String, String>,
) -> Vec<String> {
    let mut broken = Vec::new();
    if now.pass < base.pass {
        broken.push(format!("PASS dropped from {} to {}", base.pass, now.pass));
    }
    if now.fail > base.fail {
        broken.push(format!("FAIL rose from {} to {}", base.fail, now.fail));
    }
    let regressed: Vec<&str> = base_outcomes
        .iter()
        .filter(|(n, b)| {
            b.as_str() == "PASS" && now_outcomes.get(*n).map(String::as_str) != Some("PASS")
        })
        .map(|(n, _)| n.as_str())
        .collect();
    if !regressed.is_empty() {
        broken.push(format!(
            "{} test(s) that passed in the baseline no longer pass: {}",
            regressed.len(),
            regressed.join(", ")
        ));
    }
    broken
}

fn pct(n: usize, d: usize) -> f64 {
    if d == 0 { 0.0 } else { 100.0 * n as f64 / d as f64 }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

/// The bridge refuses what it cannot check, and says which thing it refused.
///
/// This gate has been seen to fail. Deleting the unknown-parameter check in
/// `lean/Shacl/Compile.lean` (replacing its condition with `false`) and running this
/// file produced:
///
/// ```text
/// test the_bridge_refuses_what_it_cannot_check ... FAILED
///   unsupported-shapes.nt: expected exit 3 (undetermined), got 0.
///   {"status":"verdict","conforms":true,"shapes":1,...,"results":[]}
/// test the_verified_evaluator_does_not_regress ... FAILED
///   FAIL rose from 0 to 36
/// ```
///
/// That is the failure mode this whole layer exists to catch: a validator reporting
/// `conforms` about constraints it never evaluated. Both gates caught it.
///
/// It was seen to fail a second time, without anyone arranging it. `sh:pattern` was
/// implemented, `unsupported-shapes.nt` had used `sh:pattern "^A"` as its example of
/// something the compiler could not read, and the fixture stopped being refused:
///
/// ```text
/// test the_bridge_refuses_what_it_cannot_check ... FAILED
///   unsupported-shapes.nt: expected exit 3 (undetermined), got 0.
///   {"status":"verdict","conforms":true,"shapes":1,"data_triples":2,"results":[]}
/// ```
///
/// The fixture now uses `sh:prefixes`, and two more were added beside it, one per
/// remaining way to decline: an unimplemented path form and a regular expression
/// outside the proved subset.
#[test]
fn the_bridge_refuses_what_it_cannot_check() {
    if skip() {
        return;
    }
    let f = fixtures();
    let shapes = f.join("ok-shapes.nt");

    // A verdict, both ways, on the same shapes graph. If these two agreed the
    // bridge would not be measuring anything.
    let good = run_bridge(&f.join("conforming-data.nt"), &shapes);
    assert_eq!(good.code, 0, "conforming fixture did not produce a verdict: {}", good.raw);
    assert_eq!(good.json["conforms"], serde_json::json!(true), "{}", good.raw);

    let bad = run_bridge(&f.join("violating-data.nt"), &shapes);
    assert_eq!(bad.code, 0, "violating fixture did not produce a verdict: {}", bad.raw);
    assert_eq!(bad.json["conforms"], serde_json::json!(false), "{}", bad.raw);
    let comps: BTreeSet<String> = bad.json["results"]
        .as_array()
        .expect("results array")
        .iter()
        .map(|r| short(&canon(r["sourceConstraintComponent"].as_str().unwrap_or(""))))
        .collect();
    assert!(
        comps.contains("MinCountConstraintComponent") && comps.contains("MaxCountConstraintComponent"),
        "expected both count components, got {comps:?} in {}",
        bad.raw
    );

    // sh:maxCount counts value nodes, not statements. The same triple twice is one
    // value node, so this must conform.
    let dup = run_bridge(&f.join("duplicate-data.nt"), &shapes);
    assert_eq!(dup.code, 0, "{}", dup.raw);
    assert_eq!(
        dup.json["conforms"],
        serde_json::json!(true),
        "a repeated triple is one value node, so sh:maxCount 1 must hold: {}",
        dup.raw
    );

    // Seven refusals, each naming what it refused. One per way the pipeline can
    // decline: an unknown parameter, an unimplemented path form in each of its
    // two SPELLINGS, a regular expression outside the proved subset, recursion,
    // the extension mechanism, and a datatype whose lexical space is not
    // implemented.
    //
    // The IRI-spelled path is #205 and it is the reason the pair is listed
    // rather than one of them: the compiler tested the spelling before the path
    // forms, so a named path node carrying `sh:zeroOrMorePath` was read as a
    // plain predicate and answered with a verdict, while its blank-node twin was
    // refused. Two shapes graphs saying the same thing, one checked and one not,
    // and no way for the author to tell which they had written.
    let refusals: [(&str, &str, &str); 7] = [
        ("conforming-data.nt", "unsupported-shapes.nt", "shacl#prefixes"),
        ("conforming-data.nt", "unsupported-path-shapes.nt", "zeroOrMorePath"),
        ("conforming-data.nt", "unsupported-path-iri-shapes.nt", "zeroOrMorePath"),
        ("conforming-data.nt", "unsupported-regex-shapes.nt", "'+'"),
        ("conforming-data.nt", "recursive-shapes.nt", "recursive"),
        ("conforming-data.nt", "component-shapes.nt", "extension mechanism"),
        ("unknown-datatype-data.nt", "unknown-datatype-shapes.nt", "lexical space"),
    ];
    for (data, shp, needle) in refusals {
        let v = run_bridge(&f.join(data), &f.join(shp));
        assert_eq!(
            v.code, 3,
            "{shp}: expected exit 3 (undetermined), got {}. A validator that answers here is \
             answering about constraints it did not evaluate.\n{}",
            v.code, v.raw
        );
        assert_eq!(v.json["status"], serde_json::json!("undetermined"), "{}", v.raw);
        let reason = v.json["reason"].as_str().unwrap_or("");
        assert!(
            reason.contains(needle),
            "{shp}: the refusal must name what was refused; expected {needle:?} in {reason:?}"
        );
    }

    // The property behind the pair above, asserted directly: the two spellings
    // must give the SAME answer. Listing both in the table would still pass if
    // some future change made both of them answer rather than both refuse.
    let by_bnode = run_bridge(&f.join("conforming-data.nt"), &f.join("unsupported-path-shapes.nt"));
    let by_iri = run_bridge(&f.join("conforming-data.nt"), &f.join("unsupported-path-iri-shapes.nt"));
    assert_eq!(
        by_bnode.code, by_iri.code,
        "the same shape spelled two ways gave exit {} and exit {}. Whichever is right, a \
         shapes author cannot be expected to know that the spelling decided it.\n{}\n{}",
        by_bnode.code, by_iri.code, by_bnode.raw, by_iri.raw
    );
    assert_eq!(
        by_bnode.json["status"], by_iri.json["status"],
        "{}\n{}", by_bnode.raw, by_iri.raw
    );

    // A file that is not N-Triples is exit 2, which is a different answer again.
    let broken = run_bridge(&f.join("malformed.nt"), &shapes);
    assert_eq!(broken.code, 2, "a parse failure must be exit 2, not a verdict: {}", broken.raw);
    assert_eq!(broken.json["status"], serde_json::json!("error"), "{}", broken.raw);
}

/// The suite is present and whole, so the denominator cannot shrink silently.
#[test]
fn the_vendored_suite_is_whole() {
    if skip() {
        return;
    }
    let r = run();
    assert!(
        r.cases.len() >= EXPECTED_ENTRIES,
        "the manifest walk found only {} sht:Validate entries; upstream {SUITE_COMMIT} carries \
         {EXPECTED_ENTRIES}. A shrunken suite makes the number below meaningless.",
        r.cases.len()
    );
    let names: BTreeSet<&str> = r.cases.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names.len(), r.cases.len(), "two entries share a baseline key");
}

/// The headline: what the verified evaluator scores, and where the rest went.
/// The other harness's counts, from the baseline it commits. Returns None rather
/// than a guess if the file is absent or its shape changes.
fn rust_engine_baseline() -> Option<(u64, u64, u64)> {
    let raw = std::fs::read_to_string(repo().join("tests").join("w3c_shacl_baseline.json")).ok()?;
    let v: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let counts = v.get("counts").unwrap_or(&v);
    Some((
        counts.get("pass")?.as_u64()?,
        counts.get("fail")?.as_u64()?,
        counts.get("undetermined")?.as_u64()?,
    ))
}

#[test]
fn the_verified_conformance_report() {
    if skip() {
        return;
    }
    let r = run();
    let c = r.counts;

    eprintln!("\n====== W3C SHACL Core, the VERIFIED evaluator (lean/Shacl/) ======");
    eprintln!("suite commit {SUITE_COMMIT}\n");
    eprintln!("  total         {:>4}", c.total);
    eprintln!("  PASS          {:>4}  ({:.1}%)", c.pass, pct(c.pass, c.total));
    eprintln!(
        "  FAIL          {:>4}  ({:.1}%)  <- wrong answer from a proved evaluator, the serious bucket",
        c.fail,
        pct(c.fail, c.total)
    );
    eprintln!(
        "  UNDETERMINED  {:>4}  ({:.1}%)  <- exit 3, refused and said why. NOT credit.",
        c.undetermined,
        pct(c.undetermined, c.total)
    );
    eprintln!("  ERROR         {:>4}  ({:.1}%)", c.error, pct(c.error, c.total));
    eprintln!(
        "\n  partial compliance (sh:conforms agrees, result set ignored): {} of {} ({:.1}%)",
        r.partial,
        c.total,
        pct(r.partial, c.total)
    );
    eprintln!(
        "\n  of those passes, {} required the evaluator to produce the RIGHT violations at the\n  \
         right nodes with the right constraint components, not merely the right silence.",
        r.substantive_passes
    );
    eprintln!(
        "  {} were earned with no targeted shape compiled at all. That is the correct answer\n  \
         when every shape in the graph is deactivated and free credit otherwise, so they are\n  \
         named:",
        r.vacuous_passes.len()
    );
    for n in &r.vacuous_passes {
        eprintln!("      {n}");
    }
    eprintln!(
        "  {} test(s) expect sht:Failure; {} of those were refused rather than failed, and a\n  \
         refusal is NOT credited as a reported failure.",
        r.failure_expected, r.failure_undetermined
    );
    // Read from the other harness's committed baseline rather than typed here.
    // These constants were typed once and went stale within a day: they said the
    // Rust engine was at 36/24/60 when it was at 35/17/68, inside a report whose
    // entire value is that its numbers are measured. A number standing next to
    // the screen that shows it has to be derived from the same source, or it
    // becomes a claim nothing checks.
    match rust_engine_baseline() {
        Some((rp, rf, ru)) => eprintln!(
            "\n  for comparison, the SPARQL-backed Rust engine under its own harness at the same\n  \
             suite commit, read from tests/w3c_shacl_baseline.json: PASS {rp} / FAIL {rf} /\n  \
             UNDETERMINED {ru}. It ANSWERS {} of the 120 and is wrong on {rf}; this one answers\n  \
             {} and is wrong on none of those it answers.",
            rp + rf,
            c.pass + c.fail
        ),
        None => eprintln!(
            "\n  the Rust engine's baseline could not be read, so no comparison is printed.\n  \
             A missing number is better than a stale one."
        ),
    }

    // Where the undetermined tests went, clustered, with every test named under
    // its cluster so the next piece of work can be picked by evidence rather than
    // by guessing which tests a cluster holds.
    let mut clusters: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (tc, o) in r.cases.iter().zip(r.outcomes.iter()) {
        if let Outcome::Undetermined(d) = o {
            clusters.entry(refusal_class(d)).or_default().push(tc.name.clone());
        }
    }
    if !clusters.is_empty() {
        eprintln!("\n  UNDETERMINED by reason, with every test named under its reason:");
        let mut rows: Vec<(&String, &Vec<String>)> = clusters.iter().collect();
        rows.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(b.0)));
        for (why, names) in rows {
            eprintln!("    {:>4}  {why}", names.len());
            for n in names {
                eprintln!("            {n}");
            }
        }
    }

    // Every failure, by name. There should not be many and each one is a defect.
    let fails: Vec<(&TestCase, &Outcome)> = r
        .cases
        .iter()
        .zip(r.outcomes.iter())
        .filter(|(_, o)| matches!(o, Outcome::Fail(_)))
        .collect();
    if !fails.is_empty() {
        eprintln!("\n  FAIL, every one:");
        for (tc, o) in &fails {
            eprintln!("    {}\n        {}", tc.name, o.detail());
        }
    }
    let errs: Vec<(&TestCase, &Outcome)> = r
        .cases
        .iter()
        .zip(r.outcomes.iter())
        .filter(|(_, o)| matches!(o, Outcome::Error(_)))
        .collect();
    if !errs.is_empty() {
        eprintln!("\n  ERROR, every one:");
        for (tc, o) in &errs {
            eprintln!("    {}\n        {}", tc.name, o.detail());
        }
    }
    eprintln!();
}

/// The ratchet. PASS may not drop, FAIL may not rise, no named PASS may regress.
///
/// Seen to fail. `langMatches` in `lean/Shacl/Term.lean` implements BCP 47 basic
/// filtering, under which the range `en` matches the tag `en-NZ`. Dropping the
/// subtag-boundary half of it, so that only an exact tag matches, compiles, proves,
/// and passes every witness, because nothing in the Lean development pins that
/// behaviour. This gate is what caught it:
///
/// ```text
/// test the_verified_evaluator_does_not_regress ... FAILED
///   the verified SHACL evaluator regressed against .../baseline.json:
///     PASS dropped from 91 to 90
///     FAIL rose from 0 to 1
///     1 test(s) that passed in the baseline no longer pass: core/property/languageIn-001
/// ```
///
/// That is the division of labour this file exists for. A proof cannot tell you that
/// `Shacl/Spec.lean` misreads the Recommendation, because the proof is about the
/// agreement between the specification and the evaluator, not about the prose. The
/// Working Group's suite is the only thing that can, and it did.
#[test]
fn the_verified_evaluator_does_not_regress() {
    if skip() {
        return;
    }
    let r = run();
    let now_outcomes = outcome_map(r);

    if std::env::var("OO_SHACL_CORE_UPDATE_BASELINE").as_deref() == Ok("1") {
        let baseline = Baseline {
            note: "W3C SHACL Core conformance of the VERIFIED evaluator in lean/Shacl/, \
                   ratcheted. PASS may not drop, FAIL may not rise, and no named PASS may \
                   regress. Regenerate with OO_SHACL_CORE_UPDATE_BASELINE=1 cargo test --test \
                   shacl_core_verified_test. UNDETERMINED is a refusal, not a pass; moving a \
                   test out of it lands as a PASS and the ratchet counts that."
                .to_string(),
            suite_commit: SUITE_COMMIT.to_string(),
            counts: r.counts,
            partial_compliance: r.partial,
            outcomes: now_outcomes.clone(),
        };
        std::fs::create_dir_all(fixtures()).unwrap();
        std::fs::write(baseline_path(), serde_json::to_string_pretty(&baseline).unwrap()).unwrap();
        eprintln!("baseline rewritten at {}", baseline_path().display());
        return;
    }

    let text = std::fs::read_to_string(baseline_path()).unwrap_or_else(|e| {
        panic!(
            "no baseline at {}: {e}. Cut one with OO_SHACL_CORE_UPDATE_BASELINE=1 cargo test \
             --test shacl_core_verified_test",
            baseline_path().display()
        )
    });
    let base: Baseline = serde_json::from_str(&text).expect("parse the committed baseline");
    assert_eq!(
        base.suite_commit, SUITE_COMMIT,
        "the baseline was cut against a different suite commit; a suite refresh and an \
         evaluator change must not be confusable in a diff"
    );

    let broken = ratchet_verdict(&base.counts, &base.outcomes, &r.counts, &now_outcomes);
    assert!(
        broken.is_empty(),
        "the verified SHACL evaluator regressed against {}:\n  {}\n\nIf the change is \
         deliberate, re-cut the baseline and say why in the commit message.",
        baseline_path().display(),
        broken.join("\n  ")
    );
    eprintln!(
        "ratchet ok: PASS {} (baseline {}), FAIL {} (baseline {})",
        r.counts.pass, base.counts.pass, r.counts.fail, base.counts.fail
    );
}
