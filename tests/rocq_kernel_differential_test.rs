//! A third proof assistant, the same certificate format, and the question of whether
//! three formalisations say the same thing.
//!
//! `lean/OOCert/Horn.lean`, `isabelle/OO_Check.thy` and `rocq/theories/Checker.v` are
//! three formalisations of one Horn certificate checker.
//! `tests/cross_kernel_differential_test.rs` runs the first two over one corpus. This
//! file runs the FIRST AND THIRD over a corpus of its own and requires them to agree,
//! except on one named cause.
//!
//! # What agreement here is, and is not
//!
//! The same caveat the Lean-Isabelle comparison carries applies here unchanged and is
//! repeated rather than cross-referenced, because a caveat nobody reads is not a caveat.
//! Checking a Horn certificate is PURELY SYNTACTIC. Neither checker consults its own
//! semantics: no interpretation, no domain, no truth. Two checkers built on contradictory
//! model theories agree on every certificate and on every forgery. Agreement here is
//! evidence about the FILE FORMAT and the CHECKING DISCIPLINE, and it is not a proof of
//! anything.
//!
//! The evidence that the DEFINITIONS agree is elsewhere: in which rule arms each side can
//! discharge and from which conditions (`rocq/theories/Builtin.v` against
//! `lean/OOCert/HornBuiltin.lean`), and in whether those conditions describe anything at
//! all (`rocq/theories/Witness.v`).
//!
//! # What this comparison found
//!
//! TWO things, and they are different in kind.
//!
//! **One divergence, one cause, no soundness content: AN EMPTY LINE.** Lean skips an
//! empty line in all three input files. Rocq refuses one, anywhere, in any of the three
//! (`R-PARSE-2` in `rocq/theories/Parse.v`). So a certificate file that ends with two
//! newlines is accepted by one verified checker and refused by the other, and the same
//! goes for the rules table and the asserted graph. Neither is unsound: an empty line is
//! not a step, not a triple and not a rule, so skipping it derives nothing and refusing
//! it asserts nothing. What is defective is the FORMAT, which never said whether an empty
//! line is a line. The quarantine below is keyed on the CAUSE, computed from the bytes,
//! and not on which mutation produced the row, because a quarantine keyed on the edit
//! hides the rows nobody designed.
//!
//! The sharp detail, and the reason this is worth a decision record rather than a shrug:
//! Lean's tolerance is for the EMPTY string exactly. A line holding one space, one tab or
//! one carriage return is refused by both. So the leniency does not cover the case that
//! motivates leniency, which is a file checked out with CRLF endings, whose blank lines
//! are `\r` and not empty. See
//! `docs/decisions/0015-a-blank-line-is-a-line-or-it-is-not.md`.
//!
//! **One defect, in the NEW checker, found by running it beside the old one.** A
//! certificate citing rule `99999999999999999999` made the Rocq checker die of a
//! `Stack_overflow`: Rocq's `nat` is unary, the parser built the number before looking at
//! it, and OCaml reports an uncaught exception with exit status 2, which is this tool's
//! code for a parse error. A crash was wearing a verdict's clothes, and the only reason
//! anybody looked was that Lean answered 1 where Rocq answered 2. Both halves are fixed:
//! `R-STEP-3` and `R-PARSE-8` put the count fields in binary, and the driver now exits 70
//! on any exception so that a crash can never again be read as a refusal. `a_crash_is_not
//! _a_verdict` below is the regression.
//!
//! Nothing on either side was adjusted to make a divergence go away. The empty-line
//! divergence is open and reported.

mod common;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn horn_fixture(name: &str) -> PathBuf {
    repo().join("tests").join("fixtures").join("horn").join(name)
}

// ---------------------------------------------------------------------------
// The two checkers
// ---------------------------------------------------------------------------

fn lake_available() -> bool {
    Command::new("lake").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

fn rocq_available() -> bool {
    Command::new("rocq").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

fn lean_checker() -> Option<&'static Path> {
    static P: OnceLock<Option<PathBuf>> = OnceLock::new();
    P.get_or_init(|| {
        let bin = repo().join("lean").join(".lake").join("build").join("bin").join("oo-horn");
        if bin.exists() {
            return Some(bin);
        }
        if !lake_available() {
            return None;
        }
        let ok = Command::new("lake")
            .current_dir(repo().join("lean"))
            .args(["build", "oo-horn"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ok && bin.exists() { Some(bin) } else { None }
    })
    .as_deref()
}

fn rocq_checker() -> Option<&'static Path> {
    static P: OnceLock<Option<PathBuf>> = OnceLock::new();
    P.get_or_init(|| {
        let bin = repo().join("rocq").join("build").join("oo-horn-rocq");
        if bin.exists() {
            return Some(bin);
        }
        if !rocq_available() {
            return None;
        }
        let ok = Command::new("bash")
            .arg(repo().join("rocq").join("build.sh"))
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ok && bin.exists() { Some(bin) } else { None }
    })
    .as_deref()
}

fn skip() -> bool {
    let lean = lean_checker().is_some();
    let rocq = rocq_checker().is_some();
    let what = match (lean, rocq) {
        (true, true) => return false,
        (false, true) => "the Lean checker (lake build oo-horn, in lean/)",
        (true, false) => "the Rocq checker (rocq/build.sh)",
        (false, false) => "both checkers: lake and rocq",
    };
    common::skip_unless(false, what, "install the toolchain it names and re-run; a differential with one half missing measures nothing")
}

/// 0 accepted, 1 rejected, 2 unreadable or unparseable, and anything else is a bug in
/// the checker rather than a verdict about the file. The fourth case is why this is an
/// enum and not an integer: an exit code nobody enumerated is exactly how a
/// `Stack_overflow` came to be read as a parse error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Outcome {
    Accept,
    Reject,
    Unparseable,
    Broken(i32),
}

fn run(bin: &Path, rules: &Path, asserted: &Path, cert: &Path) -> Outcome {
    let out = Command::new(bin)
        .arg("check")
        .arg(rules)
        .arg(asserted)
        .arg(cert)
        .output()
        .unwrap_or_else(|e| panic!("cannot run {}: {e}", bin.display()));
    match out.status.code() {
        Some(0) => Outcome::Accept,
        Some(1) => Outcome::Reject,
        Some(2) => Outcome::Unparseable,
        Some(n) => Outcome::Broken(n),
        None => Outcome::Broken(-1),
    }
}

// ---------------------------------------------------------------------------
// The corpus
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct Case {
    name: String,
    rules: Vec<u8>,
    asserted: Vec<u8>,
    cert: Vec<u8>,
}

fn read(p: PathBuf) -> Vec<u8> {
    std::fs::read(&p).unwrap_or_else(|e| panic!("cannot read {}: {e}", p.display()))
}

/// Every (rule table, asserted graph, certificate) triple the repository ships, crossed.
/// The three rule tables matter because two of them do not contain the rule the
/// certificates cite, which exercises the out-of-range path on real bytes.
fn bases() -> Vec<Case> {
    let tables = [
        ("builtin", "builtin_rules.tsv"),
        ("short", "short_rules.tsv"),
        ("user", "user_rules.tsv"),
    ];
    let pairs = [
        ("good", "asserted.tsv", "good.tsv"),
        ("bad_binding", "asserted.tsv", "bad_binding.tsv"),
        ("bad_conclusion", "asserted.tsv", "bad_conclusion.tsv"),
        ("bad_index", "asserted.tsv", "bad_index.tsv"),
        ("bad_premise", "asserted.tsv", "bad_premise.tsv"),
        ("bad_self", "asserted.tsv", "bad_self.tsv"),
        ("self_support", "deep/self_support_asserted.tsv", "deep/self_support_cert.tsv"),
        ("mutual", "deep/mutual_asserted.tsv", "deep/mutual_cert.tsv"),
        ("mutual_seeded", "deep/mutual_seeded_asserted.tsv", "deep/mutual_cert.tsv"),
    ];
    let mut out = Vec::new();
    for (tname, tfile) in tables {
        for (pname, afile, cfile) in pairs {
            out.push(Case {
                name: format!("{tname}/{pname}"),
                rules: read(horn_fixture(tfile)),
                asserted: read(horn_fixture(afile)),
                cert: read(horn_fixture(cfile)),
            });
        }
    }
    out
}

const ALIEN: &str = "<http://ex.invalid/never-derived>";

/// Byte-level edits, applied to each of the three files in turn. These are the ones that
/// ask lexical questions: what is a line, what is a field, what is the end of a file.
/// A named mutation of a file's bytes. Named because the differential reports
/// which edit produced a divergence, and a row that cannot say what it did to
/// the input is not a finding anyone can act on.
type ByteEdit = (&'static str, fn(&[u8]) -> Vec<u8>);

/// The same, for an edit that rewrites one step in place.
type StepEdit = (&'static str, fn(&mut Step));

fn byte_edits() -> Vec<ByteEdit> {
    fn trailing_blank(b: &[u8]) -> Vec<u8> {
        let mut v = b.to_vec();
        v.push(b'\n');
        v
    }
    fn leading_blank(b: &[u8]) -> Vec<u8> {
        let mut v = vec![b'\n'];
        v.extend_from_slice(b);
        v
    }
    fn interior_blank(b: &[u8]) -> Vec<u8> {
        match b.iter().position(|c| *c == b'\n') {
            Some(i) => {
                let mut v = b[..=i].to_vec();
                v.push(b'\n');
                v.extend_from_slice(&b[i + 1..]);
                v
            }
            None => b.to_vec(),
        }
    }
    fn no_final_newline(b: &[u8]) -> Vec<u8> {
        let mut v = b.to_vec();
        while v.last() == Some(&b'\n') {
            v.pop();
        }
        v
    }
    fn crlf(b: &[u8]) -> Vec<u8> {
        String::from_utf8_lossy(b).replace('\n', "\r\n").into_bytes()
    }
    fn emptied(_b: &[u8]) -> Vec<u8> {
        Vec::new()
    }
    fn blank_only(_b: &[u8]) -> Vec<u8> {
        b"\n\n".to_vec()
    }
    fn trailing_tab(b: &[u8]) -> Vec<u8> {
        let mut v = no_final_newline(b);
        v.push(b'\t');
        v.push(b'\n');
        v
    }
    fn space_line(b: &[u8]) -> Vec<u8> {
        let mut v = b.to_vec();
        v.extend_from_slice(b" \n");
        v
    }
    fn cr_line(b: &[u8]) -> Vec<u8> {
        let mut v = b.to_vec();
        v.extend_from_slice(b"\r\n");
        v
    }
    fn dup_first_line(b: &[u8]) -> Vec<u8> {
        match b.iter().position(|c| *c == b'\n') {
            Some(i) => {
                let mut v = b[..=i].to_vec();
                v.extend_from_slice(b);
                v
            }
            None => b.to_vec(),
        }
    }
    fn drop_first_line(b: &[u8]) -> Vec<u8> {
        match b.iter().position(|c| *c == b'\n') {
            Some(i) => b[i + 1..].to_vec(),
            None => Vec::new(),
        }
    }
    fn tab_to_space(b: &[u8]) -> Vec<u8> {
        let mut v = b.to_vec();
        if let Some(i) = v.iter().position(|c| *c == b'\t') {
            v[i] = b' ';
        }
        v
    }
    fn halved(b: &[u8]) -> Vec<u8> {
        b[..b.len() / 2].to_vec()
    }
    vec![
        ("trailing_blank_line", trailing_blank),
        ("leading_blank_line", leading_blank),
        ("interior_blank_line", interior_blank),
        ("no_final_newline", no_final_newline),
        ("crlf", crlf),
        ("emptied", emptied),
        ("blank_lines_only", blank_only),
        ("trailing_empty_field", trailing_tab),
        ("line_of_one_space", space_line),
        ("line_of_one_cr", cr_line),
        ("duplicated_first_line", dup_first_line),
        ("dropped_first_line", drop_first_line),
        ("tab_became_space", tab_to_space),
        ("truncated_in_half", halved),
    ]
}

/// A certificate step, split on tabs and reassembled. This is the test's own reading of
/// the format and it is deliberately NOT either checker's: it only has to be good enough
/// to build interesting mutants, and a mutant it gets wrong is still a pair of bytes both
/// checkers have to answer for.
struct Step {
    idx: String,
    bind: Vec<(String, String)>,
    concl: [String; 3],
    prems: Vec<[String; 3]>,
}

fn parse_step(line: &str) -> Option<Step> {
    let f: Vec<&str> = line.split('\t').collect();
    if f.len() < 5 {
        return None;
    }
    let k: usize = f[1].parse().ok()?;
    if f.len() < 2 + 2 * k + 3 {
        return None;
    }
    let bind = (0..k).map(|i| (f[2 + 2 * i].to_string(), f[3 + 2 * i].to_string())).collect();
    let rest = &f[2 + 2 * k..];
    let concl = [rest[0].to_string(), rest[1].to_string(), rest[2].to_string()];
    let tail = &rest[3..];
    if !tail.len().is_multiple_of(3) {
        return None;
    }
    let prems = tail
        .chunks(3)
        .map(|c| [c[0].to_string(), c[1].to_string(), c[2].to_string()])
        .collect();
    Some(Step { idx: f[0].to_string(), bind, concl, prems })
}

fn render_step(s: &Step) -> String {
    let mut f = vec![s.idx.clone(), s.bind.len().to_string()];
    for (k, v) in &s.bind {
        f.push(k.clone());
        f.push(v.clone());
    }
    f.extend(s.concl.iter().cloned());
    for p in &s.prems {
        f.extend(p.iter().cloned());
    }
    f.join("\t")
}

/// Structural edits to the FIRST step of a certificate. These are the ones that ask
/// checking questions: what is a binding, what is a premise list, what is an index.
fn step_edits() -> Vec<StepEdit> {
    fn permute_premises(s: &mut Step) {
        if s.prems.len() >= 2 {
            s.prems.swap(0, 1);
        }
    }
    fn drop_premise(s: &mut Step) {
        s.prems.pop();
    }
    fn dup_premise(s: &mut Step) {
        if let Some(p) = s.prems.first().cloned() {
            s.prems.push(p);
        }
    }
    fn no_premises(s: &mut Step) {
        s.prems.clear();
    }
    fn dup_key_different(s: &mut Step) {
        if let Some((k, _)) = s.bind.first().cloned() {
            s.bind.push((k, ALIEN.to_string()));
        }
    }
    fn dup_key_same(s: &mut Step) {
        if let Some(p) = s.bind.first().cloned() {
            s.bind.push(p);
        }
    }
    fn drop_binding(s: &mut Step) {
        s.bind.pop();
    }
    fn extra_binding(s: &mut Step) {
        s.bind.push(("never_mentioned".to_string(), ALIEN.to_string()));
    }
    fn index_zero(s: &mut Step) {
        s.idx = "0".to_string();
    }
    fn index_last(s: &mut Step) {
        s.idx = "26".to_string();
    }
    fn index_past_end(s: &mut Step) {
        s.idx = "27".to_string();
    }
    fn index_astronomical(s: &mut Step) {
        s.idx = "99999999999999999999".to_string();
    }
    fn index_leading_zero(s: &mut Step) {
        s.idx = format!("0{}", s.idx);
    }
    fn alien_conclusion(s: &mut Step) {
        s.concl[2] = ALIEN.to_string();
    }
    fn alien_premise(s: &mut Step) {
        if let Some(p) = s.prems.first_mut() {
            p[0] = ALIEN.to_string();
        }
    }
    fn swap_conclusion_and_premise(s: &mut Step) {
        if let Some(p) = s.prems.first_mut() {
            std::mem::swap(&mut s.concl, p);
        }
    }
    vec![
        ("permuted_premises", permute_premises),
        ("dropped_premise", drop_premise),
        ("duplicated_premise", dup_premise),
        ("no_premises", no_premises),
        ("duplicate_binding_key_different_value", dup_key_different),
        ("duplicate_binding_key_same_value", dup_key_same),
        ("dropped_binding", drop_binding),
        ("binding_for_an_unmentioned_variable", extra_binding),
        ("index_zero", index_zero),
        ("index_last_row", index_last),
        ("index_one_past_the_end", index_past_end),
        ("index_astronomical", index_astronomical),
        ("index_with_a_leading_zero", index_leading_zero),
        ("alien_conclusion", alien_conclusion),
        ("alien_premise", alien_premise),
        ("conclusion_swapped_with_premise", swap_conclusion_and_premise),
    ]
}

fn mutate_first_step(cert: &[u8], f: fn(&mut Step)) -> Option<Vec<u8>> {
    let text = String::from_utf8(cert.to_vec()).ok()?;
    let mut lines: Vec<String> = text.lines().map(|s| s.to_string()).collect();
    if lines.is_empty() {
        return None;
    }
    let mut s = parse_step(&lines[0])?;
    f(&mut s);
    lines[0] = render_step(&s);
    let mut out = lines.join("\n");
    out.push('\n');
    Some(out.into_bytes())
}

fn corpus() -> Vec<Case> {
    let mut out = Vec::new();
    for base in bases() {
        out.push(base.clone());
        for (ename, edit) in byte_edits() {
            out.push(Case {
                name: format!("{}+rules.{ename}", base.name),
                rules: edit(&base.rules),
                asserted: base.asserted.clone(),
                cert: base.cert.clone(),
            });
            out.push(Case {
                name: format!("{}+asserted.{ename}", base.name),
                rules: base.rules.clone(),
                asserted: edit(&base.asserted),
                cert: base.cert.clone(),
            });
            out.push(Case {
                name: format!("{}+cert.{ename}", base.name),
                rules: base.rules.clone(),
                asserted: base.asserted.clone(),
                cert: edit(&base.cert),
            });
        }
        for (ename, edit) in step_edits() {
            if let Some(cert) = mutate_first_step(&base.cert, edit) {
                out.push(Case {
                    name: format!("{}+step.{ename}", base.name),
                    rules: base.rules.clone(),
                    asserted: base.asserted.clone(),
                    cert,
                });
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// The one quarantine, keyed on the cause
// ---------------------------------------------------------------------------

/// True when some input file carries an EMPTY line, which is the one lexical question the
/// format never answered and the one place these two checkers part company.
///
/// Computed from the bytes and not from the name of the edit that produced them. That
/// distinction is the whole design of this function: the Lean-Isabelle comparison found
/// its most interesting row through a fuzzer's typo rather than through a designed
/// mutation, and a quarantine keyed on the edit would have hidden exactly that row.
/// Several edits here produce an empty line without being named for one, `truncated_in_half`
/// and `dropped_first_line` among them.
fn has_empty_line(b: &[u8]) -> bool {
    let text = String::from_utf8_lossy(b);
    let mut parts: Vec<&str> = text.split('\n').collect();
    // A single trailing newline terminates the last line rather than starting an empty
    // one, and both checkers agree about that.
    if parts.last() == Some(&"") {
        parts.pop();
    }
    parts.iter().any(|l| l.is_empty())
}

fn empty_line_cause(c: &Case) -> Option<&'static str> {
    if has_empty_line(&c.rules) {
        Some("the rules table contains an empty line")
    } else if has_empty_line(&c.asserted) {
        Some("the asserted graph contains an empty line")
    } else if has_empty_line(&c.cert) {
        Some("the certificate contains an empty line")
    } else {
        None
    }
}

/// One directory per test, because `cargo test` runs them in parallel and they all write
/// three files with the same three names. Sharing one directory made the unmutated base
/// case report a parse error, which read exactly like a finding and was a race.
fn scratch(who: &str) -> PathBuf {
    let d = std::env::temp_dir().join("oo-rocq-differential").join(who);
    std::fs::create_dir_all(&d).expect("scratch dir");
    d
}

fn write_case(dir: &Path, c: &Case) -> (PathBuf, PathBuf, PathBuf) {
    let r = dir.join("rules.tsv");
    let a = dir.join("asserted.tsv");
    let d = dir.join("cert.tsv");
    std::fs::write(&r, &c.rules).expect("write rules");
    std::fs::write(&a, &c.asserted).expect("write asserted");
    std::fs::write(&d, &c.cert).expect("write cert");
    (r, a, d)
}

// ---------------------------------------------------------------------------
// The tests
// ---------------------------------------------------------------------------

#[test]
fn the_skip_says_which_half_is_missing() {
    if skip() {
        return;
    }
    assert!(lean_checker().is_some() && rocq_checker().is_some());
}

/// THE COMPARISON.
#[test]
fn the_two_kernels_agree_except_on_one_named_cause() {
    if skip() {
        return;
    }
    let lean = lean_checker().expect("checked above");
    let rocq = rocq_checker().expect("checked above");
    let dir = scratch("corpus");

    let cases = corpus();
    assert!(
        cases.len() > 500,
        "the corpus collapsed to {} rows, which means the generator is broken rather \
         than that the format got simple",
        cases.len()
    );

    let mut both_accept = 0usize;
    let mut both_reject = 0usize;
    let mut both_unparseable = 0usize;
    let mut quarantined: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut broken: Vec<String> = Vec::new();
    let mut unexplained: Vec<String> = Vec::new();
    let mut empty_line_rows = 0usize;

    for c in &cases {
        let (r, a, d) = write_case(&dir, c);
        let l = run(lean, &r, &a, &d);
        let q = run(rocq, &r, &a, &d);

        if let Outcome::Broken(n) = l {
            broken.push(format!("{}: the LEAN checker exited {n}", c.name));
        }
        if let Outcome::Broken(n) = q {
            broken.push(format!("{}: the ROCQ checker exited {n}", c.name));
        }

        let cause = empty_line_cause(c);
        if cause.is_some() {
            empty_line_rows += 1;
        }

        if l == q {
            match l {
                Outcome::Accept => both_accept += 1,
                Outcome::Reject => both_reject += 1,
                Outcome::Unparseable => both_unparseable += 1,
                Outcome::Broken(_) => {}
            }
            continue;
        }
        match cause {
            Some(why) => *quarantined.entry(why).or_default() += 1,
            None => unexplained.push(format!("{}: lean {l:?}, rocq {q:?}", c.name)),
        }
    }

    let divergent: usize = quarantined.values().sum();
    println!("\n=== Lean against Rocq, over {} rows ===", cases.len());
    println!("both accept:            {both_accept}");
    println!("both reject:            {both_reject}");
    println!("both unparseable:       {both_unparseable}");
    println!("divergent, explained:   {divergent}");
    for (why, n) in &quarantined {
        println!("    {n:>5}  {why}");
    }
    println!("divergent, unexplained: {}", unexplained.len());
    println!("rows carrying an empty line at all: {empty_line_rows}");
    println!(
        "    of which the two kernels still agreed: {}",
        empty_line_rows.saturating_sub(divergent)
    );
    println!("checker crashes:        {}", broken.len());

    assert!(
        broken.is_empty(),
        "a checker exited with a code that is not a verdict:\n  {}\n\nAn exit code \
         nobody enumerated is how a Stack_overflow came to be read as a parse error. \
         This is a bug in the checker, not a finding about the format.",
        broken.join("\n  ")
    );

    assert!(
        unexplained.is_empty(),
        "The two kernels disagreed on rows with no empty line in them:\n  {}\n\nThis is \
         a NEW finding and it belongs in a report before it belongs in a patch. Do not \
         edit either formalisation to make it go away, and in particular do not edit \
         rocq/ to agree with lean/: the only thing a third formalisation is for is \
         disagreeing.",
        unexplained.join("\n  ")
    );

    // Agreement bought by refusing everything is worth nothing, so the floor is on the
    // column that cannot be bought that way.
    assert!(
        both_accept >= 20,
        "only {both_accept} rows were ACCEPTED by both kernels. Agreement is cheap when \
         one side refuses everything, so this floor is what makes the count above mean \
         anything."
    );
    assert!(
        divergent > 0,
        "the empty-line divergence reported in this file's header did not reproduce. \
         Either a checker changed, in which case delete the quarantine and say so in \
         docs/decisions/0015, or the corpus stopped generating empty lines, in which \
         case the generator is broken."
    );
}

/// The divergence, pinned on the smallest input that shows it, so that a change in either
/// checker turns this red rather than silently emptying the quarantine above.
#[test]
fn known_divergence_an_empty_line_is_a_line_here_and_not_there() {
    if skip() {
        return;
    }
    let lean = lean_checker().expect("checked above");
    let rocq = rocq_checker().expect("checked above");
    let dir = scratch("empty-line");

    let base = Case {
        name: "good".into(),
        rules: read(horn_fixture("builtin_rules.tsv")),
        asserted: read(horn_fixture("asserted.tsv")),
        cert: read(horn_fixture("good.tsv")),
    };

    // The unmutated triple agrees, so the divergence below is about the edit.
    let (r, a, d) = write_case(&dir, &base);
    assert_eq!(run(lean, &r, &a, &d), Outcome::Accept);
    assert_eq!(run(rocq, &r, &a, &d), Outcome::Accept);

    for (what, mutated) in [
        ("certificate", Case { cert: [base.cert.clone(), b"\n".to_vec()].concat(), ..base.clone() }),
        ("asserted graph", Case { asserted: [base.asserted.clone(), b"\n".to_vec()].concat(), ..base.clone() }),
        ("rules table", Case { rules: [base.rules.clone(), b"\n".to_vec()].concat(), ..base.clone() }),
    ] {
        let (r, a, d) = write_case(&dir, &mutated);
        assert_eq!(
            run(lean, &r, &a, &d),
            Outcome::Accept,
            "Lean used to skip an empty line in the {what}. If it no longer does, the \
             two kernels now agree and the quarantine should go."
        );
        assert_eq!(
            run(rocq, &r, &a, &d),
            Outcome::Unparseable,
            "Rocq used to refuse an empty line in the {what} (R-PARSE-2). If it no \
             longer does, somebody edited the third formalisation to agree with the \
             first, which is the one thing it must never be edited for."
        );
    }

    // And the sharp half: the tolerance is for the EMPTY string exactly, so it does not
    // cover a file whose blank lines carry a carriage return, which is what a CRLF
    // checkout produces. Both refuse those.
    for filler in [" ", "\t", "\r"] {
        let mutated = Case {
            cert: [base.cert.clone(), filler.as_bytes().to_vec(), b"\n".to_vec()].concat(),
            ..base.clone()
        };
        let (r, a, d) = write_case(&dir, &mutated);
        assert_eq!(run(lean, &r, &a, &d), Outcome::Unparseable, "filler {filler:?}");
        assert_eq!(run(rocq, &r, &a, &d), Outcome::Unparseable, "filler {filler:?}");
    }
}

/// The regression for the defect this comparison found in the Rocq checker.
///
/// An index of twenty digits used to overflow the stack while the number was being built
/// out of unary successors, and OCaml's uncaught-exception status is 2, which this tool
/// uses for a parse error. So the crash reported a verdict. Both halves are pinned: the
/// answer is now a rejection, and it arrives in well under a second.
#[test]
fn a_crash_is_not_a_verdict() {
    if skip() {
        return;
    }
    let lean = lean_checker().expect("checked above");
    let rocq = rocq_checker().expect("checked above");
    let dir = scratch("astronomical-index");

    let cert = std::fs::read_to_string(horn_fixture("good.tsv")).expect("good.tsv");
    let mut step = parse_step(cert.lines().next().expect("one step")).expect("parses");
    step.idx = "99999999999999999999".to_string();
    let c = Case {
        name: "astronomical index".into(),
        rules: read(horn_fixture("builtin_rules.tsv")),
        asserted: read(horn_fixture("asserted.tsv")),
        cert: format!("{}\n", render_step(&step)).into_bytes(),
    };
    let (r, a, d) = write_case(&dir, &c);

    let started = std::time::Instant::now();
    let q = run(rocq, &r, &a, &d);
    let took = started.elapsed();

    assert_eq!(
        q,
        Outcome::Reject,
        "a rule index no table can contain is a REJECTION (R-CHK-4). Exit 2 here means \
         the checker crashed and the crash is wearing a parse error's exit code, which \
         is the defect this test exists for."
    );
    assert_eq!(run(lean, &r, &a, &d), Outcome::Reject);
    assert!(
        took.as_secs() < 10,
        "the Rocq checker took {took:?} on a twenty-digit index. The count fields are \
         supposed to be binary (R-PARSE-8); if this is slow again, something went back \
         to unary."
    );
}

/// The extracted binary against the kernel.
///
/// `rocq/theories/Fixtures.v` computes the verdict for the shipped fixtures INSIDE the
/// Rocq kernel, so those lines are what the proved checker does. Extraction is trusted
/// and not verified, so this is the only handle there is on it: the binary must give the
/// same answers on the same bytes.
#[test]
fn the_extracted_binary_matches_the_verdicts_the_kernel_computed() {
    if skip() {
        return;
    }
    let rocq = rocq_checker().expect("checked above");

    // Each row is a theorem in rocq/theories/Fixtures.v, named in the comment.
    let expected: &[(&str, &str, &str, Outcome)] = &[
        // good_is_accepted_with_the_absolute_verdict
        ("builtin_rules.tsv", "asserted.tsv", "good.tsv", Outcome::Accept),
        // bad_binding_is_rejected
        ("builtin_rules.tsv", "asserted.tsv", "bad_binding.tsv", Outcome::Reject),
        // bad_conclusion_is_rejected
        ("builtin_rules.tsv", "asserted.tsv", "bad_conclusion.tsv", Outcome::Reject),
        // bad_index_is_rejected
        ("builtin_rules.tsv", "asserted.tsv", "bad_index.tsv", Outcome::Reject),
        // bad_premise_is_rejected
        ("builtin_rules.tsv", "asserted.tsv", "bad_premise.tsv", Outcome::Reject),
        // bad_self_is_rejected
        ("builtin_rules.tsv", "asserted.tsv", "bad_self.tsv", Outcome::Reject),
        // deep_self_support_is_rejected
        (
            "builtin_rules.tsv",
            "deep/self_support_asserted.tsv",
            "deep/self_support_cert.tsv",
            Outcome::Reject,
        ),
        // deep_mutual_support_is_rejected
        ("builtin_rules.tsv", "deep/mutual_asserted.tsv", "deep/mutual_cert.tsv", Outcome::Reject),
        // deep_mutual_seeded_is_accepted
        (
            "builtin_rules.tsv",
            "deep/mutual_seeded_asserted.tsv",
            "deep/mutual_cert.tsv",
            Outcome::Accept,
        ),
    ];

    let mut wrong = Vec::new();
    for (r, a, c, want) in expected {
        let got = run(rocq, &horn_fixture(r), &horn_fixture(a), &horn_fixture(c));
        if got != *want {
            wrong.push(format!("{c}: kernel says {want:?}, extracted binary says {got:?}"));
        }
    }
    assert!(
        wrong.is_empty(),
        "The extracted checker disagrees with the Rocq kernel on the shipped \
         fixtures:\n  {}\n\nThe kernel is right by construction, so this is an \
         extraction bug and it is the one thing rocq/theories/Fixtures.v exists to \
         catch.",
        wrong.join("\n  ")
    );
}

/// A certificate over a table nobody discharged must never earn the word a certificate
/// over the built-ins earns. Decision 0003 requires it of the Lean; the third
/// formalisation has to carry its own copy or the separation is only as good as one
/// implementation.
#[test]
fn a_user_rule_never_earns_the_absolute_verdict_in_rocq() {
    if skip() {
        return;
    }
    let rocq = rocq_checker().expect("checked above");
    let dir = scratch("user-verdict");

    // The built-in table with one extra row appended is not the built-in table.
    let mut rules = read(horn_fixture("builtin_rules.tsv"));
    rules.extend_from_slice(
        b"mine\t1\t?s\t<http://ex.org/p>\t?o\t?s\t<http://ex.org/q>\t?o\n",
    );
    let c = Case {
        name: "builtins plus one".into(),
        rules,
        asserted: read(horn_fixture("asserted.tsv")),
        cert: read(horn_fixture("good.tsv")),
    };
    let (r, a, d) = write_case(&dir, &c);

    let out = Command::new(rocq).arg("check").arg(&r).arg(&a).arg(&d).output().expect("run");
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    assert_eq!(out.status.code(), Some(0), "the certificate still checks: {text}");
    assert!(
        text.contains("entailed_under_supplied_rules"),
        "a table that is not the built-in one must earn the relativised verdict, and \
         this run said: {text}"
    );
    assert!(
        !text.contains("\"verdict\":\"entailed\""),
        "the absolute verdict was awarded to a table nothing discharged: {text}"
    );

    // And the built-in table itself still earns the absolute one, or the check above
    // would be satisfied by a checker that never says `entailed` at all.
    let base = Case {
        name: "builtins".into(),
        rules: read(horn_fixture("builtin_rules.tsv")),
        asserted: read(horn_fixture("asserted.tsv")),
        cert: read(horn_fixture("good.tsv")),
    };
    let (r, a, d) = write_case(&dir, &base);
    let out = Command::new(rocq).arg("check").arg(&r).arg(&a).arg(&d).output().expect("run");
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(
        text.contains("\"verdict\":\"entailed\""),
        "the built-in table no longer earns the absolute verdict, so the guard above \
         proves nothing: {text}"
    );
}

/// The fourteen, counted from the source text rather than from a comment.
///
/// `rocq/theories/Builtin.v` claims that exactly fourteen of the twenty-seven built-in
/// arms consume a BACKWARD RDFS condition, and names them. That number is the size of the
/// gap between what RDFS states and what the absolute verdict rests on, so it is quoted in
/// `rocq/README.md` and in the pull request that added it. A number quoted in three places
/// and checked in none is how a claim goes stale, and this repository has the receipts.
///
/// Each arm carries the conditions it uses as explicit hypotheses (`R-INT-4`), so the
/// dependency is in the STATEMENT and this can be read off the file with a regular
/// expression rather than inferred from a proof. Needs no toolchain: it reads the source.
#[test]
fn the_fourteen_arms_that_need_a_backward_condition_are_the_fourteen_named() {
    let src = std::fs::read_to_string(repo().join("rocq").join("theories").join("Builtin.v"))
        .expect("rocq/theories/Builtin.v must be readable");

    // Every `Lemma arm_... : <statement>.` up to the `Proof.` that follows it.
    let mut arms: Vec<(String, bool)> = Vec::new();
    let mut rest = src.as_str();
    while let Some(i) = rest.find("\nLemma arm_") {
        let after = &rest[i + 1..];
        let Some(colon) = after.find(" : ") else { break };
        let name = after[..colon].trim_start_matches("Lemma ").to_string();
        let Some(proof) = after.find("\nProof.") else { break };
        let statement = &after[colon..proof];
        arms.push((name, statement.contains("_bwd")));
        rest = &after[proof..];
    }

    assert_eq!(
        arms.len(),
        27,
        "found {} arms in Builtin.v, not 27. Either the built-in table changed, in which \
         case docs and README need changing with it, or this scan broke.",
        arms.len()
    );

    let with_bwd: Vec<&str> =
        arms.iter().filter(|(_, b)| *b).map(|(n, _)| n.as_str()).collect();

    let expected = [
        "arm_rdfs5",
        "arm_rdfs11",
        "arm_scm_eqc1a",
        "arm_scm_eqc1b",
        "arm_scm_eqp1a",
        "arm_scm_eqp1b",
        "arm_scm_svf1",
        "arm_scm_svf2",
        "arm_scm_avf1",
        "arm_scm_avf2",
        "arm_scm_dom1",
        "arm_scm_dom2",
        "arm_scm_rng1",
        "arm_scm_rng2",
    ];

    let mut got = with_bwd.clone();
    got.sort_unstable();
    let mut want = expected.to_vec();
    want.sort_unstable();

    assert_eq!(
        got,
        want,
        "\nBuiltin.v's R-BLT-2 names fourteen arms that consume a backward RDFS condition. \
         The statements say: {with_bwd:?}\n\nThis number is the size of the gap between \
         what RDFS states and what the absolute verdict rests on, and it is quoted in \
         rocq/README.md. Correct the prose, or correct the proof, but do not leave them \
         disagreeing."
    );

    // The other thirteen must not be empty either, or `_bwd` could have stopped appearing
    // for a reason that has nothing to do with the claim.
    assert_eq!(arms.len() - with_bwd.len(), 13);
}

/// Every decision record this directory cites exists, and every number it cites is that
/// record's number.
///
/// This exists because it was needed, twice. `rocq/README.md` shipped a link to a record
/// named `0012-a-premise-list-is-evidence-or-it-is-decoration`, a filename that has never
/// existed, left over from a draft in which the record had a different name. Then the record
/// itself was renumbered to 0015 when main gained a 0012 of its own, and three files under
/// `rocq/` went on citing the old number, which by then named a DIFFERENT decision about
/// concurrency. A stale cross-reference that resolves to the wrong record is worse than one
/// that resolves to nothing, because nothing about it looks broken.
///
/// The example above is spelled without its `docs/decisions/` prefix ON PURPOSE. This file
/// is scanned by the check it contains, so a literal path written here as an illustration is
/// a path the check has to resolve, and the first run of this test failed on its own
/// docstring. `ci_gate_coverage_test.rs` hit the same wall and solved it by excluding
/// itself; excluding this file would stop it checking its own citation of 0015, which is a
/// live reference, so the example is defanged instead.
///
/// Two things are checked, and the second is the one the renumber missed: that a cited path
/// exists, and that a bare "decision NNNN" names a record that exists. Needs no toolchain.
#[test]
fn every_decision_this_directory_cites_exists() {
    let decisions = repo().join("docs").join("decisions");
    let numbers: std::collections::BTreeSet<String> = std::fs::read_dir(&decisions)
        .expect("docs/decisions/ must be readable")
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().to_str().map(str::to_owned))
        .filter(|n| n.ends_with(".md") && n.len() > 4)
        .map(|n| n[..4].to_string())
        .filter(|n| n.chars().all(|c| c.is_ascii_digit()))
        .collect();
    assert!(
        numbers.len() > 5,
        "only {} decision records found, so this scan is broken rather than the tree tidy",
        numbers.len()
    );

    let mut sources: Vec<PathBuf> = vec![repo().join("tests").join("rocq_kernel_differential_test.rs")];
    for dir in [repo().join("rocq"), repo().join("rocq").join("theories"), repo().join("rocq").join("driver")] {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for e in entries.filter_map(|e| e.ok()) {
            let p = e.path();
            if p.is_file() {
                sources.push(p);
            }
        }
    }

    let mut bad = Vec::new();
    for src in &sources {
        let Ok(text) = std::fs::read_to_string(src) else { continue };
        let name = src.strip_prefix(repo()).unwrap_or(src).display().to_string();

        // A cited PATH must resolve.
        for (i, _) in text.match_indices("docs/decisions/") {
            let rest = &text[i + "docs/decisions/".len()..];
            let end = rest.find(['`', ']', ')', ' ', '\n', '"']).unwrap_or(rest.len());
            let file = &rest[..end];
            if file.ends_with(".md") && !decisions.join(file).exists() {
                bad.push(format!("{name}: cites docs/decisions/{file}, which does not exist"));
            }
        }

        // A bare "decision NNNN" must name a record that exists.
        let lower = text.to_lowercase();
        for (i, _) in lower.match_indices("decision ") {
            let rest = &lower[i + "decision ".len()..];
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if digits.len() == 4 && !numbers.contains(&digits) {
                bad.push(format!("{name}: cites decision {digits}, and no such record exists"));
            }
        }
    }

    bad.sort();
    bad.dedup();
    assert!(
        bad.is_empty(),
        "Dangling or stale decision references:\n  {}\n\nA reference that resolves to the \
         wrong record looks fine and says something false, which is why this checks the \
         number and not only the link.",
        bad.join("\n  ")
    );
}

