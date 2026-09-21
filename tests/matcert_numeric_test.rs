//! A numeric claim, and the two different sentences this repository can say
//! about one.
//!
//! The split is the whole design and these tests exist to keep it. Recomputing
//! over the integers earns `MatCert.mul_of_check` and the word `certified`.
//! Freivalds is O(n²) and probabilistic, so it earns an OPINION with its bound
//! printed. Floating point earns a tolerance and nothing else, because a proof
//! over the reals says nothing about IEEE-754.
//!
//! If those three ever collapse into one word, the cheapest of them will be
//! read as the strongest, which is exactly the laundering this repository is
//! built to refuse.

mod common;

use std::path::PathBuf;

use open_ontologies::matcert::{check, freivalds, freivalds_f64, write_cert, Checked, IntMatrix};

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("oo-matcert-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// [[1,2],[3,4]] × [[5,6],[7,8]] = [[19,22],[43,50]], by hand.
fn a() -> IntMatrix { vec![vec![1, 2], vec![3, 4]] }
fn b() -> IntMatrix { vec![vec![5, 6], vec![7, 8]] }
fn c_right() -> IntMatrix { vec![vec![19, 22], vec![43, 50]] }
fn c_wrong() -> IntMatrix { vec![vec![19, 22], vec![43, 51]] }

fn skip() -> bool {
    common::skip_unless(
        open_ontologies::matcert::find_checker().is_some(),
        "the numeric checker (oo-matcert)",
        "run `lake build oo-matcert` in lean/, or point $OO_MATCERT at a built one",
    )
}

#[test]
fn a_correct_product_earns_the_theorem() {
    if skip() {
        return;
    }
    match check(&a(), &b(), &c_right(), &dir("right")).expect("runs") {
        Checked::Accepted(token, out) => {
            assert_eq!(token.theorem(), "MatCert.mul_of_check");
            assert!(out.contains("product_checked"), "{out}");
        }
        Checked::Refused { exit, output } => panic!("refused a correct product: {exit} {output}"),
        Checked::Absent(w) => panic!("{w}"),
    }
}

#[test]
fn one_wrong_entry_is_refused_and_names_no_theorem() {
    if skip() {
        return;
    }
    match check(&a(), &b(), &c_wrong(), &dir("wrong")).expect("runs") {
        Checked::Refused { exit, output } => {
            assert_eq!(exit, 1, "a rejection is exit 1: {output}");
            assert!(
                !output.contains("MatCert.mul_of_check"),
                "a refusal must not name the theorem it did not discharge: {output}"
            );
        }
        Checked::Accepted(..) => panic!("accepted a product that is wrong in one entry"),
        Checked::Absent(w) => panic!("{w}"),
    }
}

#[test]
fn ragged_input_is_refused_before_anything_is_written() {
    let ragged: IntMatrix = vec![vec![1, 2], vec![3]];
    let e = write_cert(&ragged, &b(), &c_right(), &dir("ragged")).expect_err("refused");
    assert!(
        e.to_string().contains("padded"),
        "the refusal has to say why padding would be wrong: {e}"
    );
}

#[test]
fn freivalds_refutes_a_wrong_product_with_certainty() {
    // A refutation from Freivalds is certain even though its acceptance is not:
    // one separating vector settles it.
    let js = freivalds(&a(), &b(), &c_wrong(), 40, 7);
    assert_eq!(js["verdict"], "refuted_by_this_engine", "{js}");
    assert!(js["means"].as_str().unwrap().contains("certain"), "{js}");
}

#[test]
fn freivalds_accepts_with_a_bound_and_never_with_a_theorem() {
    let js = freivalds(&a(), &b(), &c_right(), 40, 7);
    assert_eq!(js["verdict"], "survived_freivalds");
    assert_eq!(js["error_bound"], "2^-40", "the bound belongs in the report, not a docstring");
    let s = js.to_string();
    // The theorem name appears only where it was earned, so a grep for it
    // finds acceptances and nothing else.
    assert!(!s.contains("MatCert.mul_of_check"), "no theorem covers the probabilistic path");
    // The VERDICT is what a consumer reads and switches on. It is the field
    // that must not wear a certified word; the prose around it is free to say
    // "not a certificate", and has to.
    let verdict = js["verdict"].as_str().unwrap();
    for earned in ["certified", "checked", "proved", "product_checked"] {
        assert_ne!(verdict, earned, "the probabilistic verdict must not be a word a proof earns");
    }
    assert!(js["means"].as_str().unwrap().contains("not a certificate"), "{s}");
    assert!(s.contains("OPINION of this engine"), "it must say whose word this is");
}

#[test]
fn the_same_seed_gives_the_same_answer() {
    // A report nobody can reproduce is not evidence.
    let x = freivalds(&a(), &b(), &c_right(), 12, 99);
    let y = freivalds(&a(), &b(), &c_right(), 12, 99);
    assert_eq!(x, y);
}

#[test]
fn the_float_path_reports_a_tolerance_and_refuses_the_word() {
    let af = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
    let bf = vec![vec![5.0, 6.0], vec![7.0, 8.0]];
    let cf = vec![vec![19.0, 22.0], vec![43.0, 50.0]];
    let js = freivalds_f64(&af, &bf, &cf, 8, 1e-9);
    assert_eq!(js["verdict"], "within_tolerance", "{js}");
    assert!(js["tolerance"].is_number(), "the tolerance travels with the answer: {js}");
    let s = js.to_string();
    assert!(!s.contains("MatCert.mul_of_check") && !s.contains("certified"), "{s}");
    assert!(
        s.contains("says nothing about IEEE-754"),
        "the reason floating point cannot be certified has to be IN the report: {s}"
    );
}

#[test]
fn the_three_verdicts_are_three_different_words() {
    // The guard on the whole design. If the cheap check and the proved one
    // ever share a word, the cheap one will be read as the strong one.
    let certified_path = check(&a(), &b(), &c_right(), &dir("words"));
    let probabilistic = freivalds(&a(), &b(), &c_right(), 8, 3).to_string();
    let floating = freivalds_f64(
        &[vec![1.0]], &[vec![1.0]], &[vec![1.0]], 2, 1e-9,
    ).to_string();

    assert!(probabilistic.contains("survived_freivalds"));
    assert!(floating.contains("within_tolerance"));
    assert!(!probabilistic.contains("within_tolerance"));
    assert!(!floating.contains("survived_freivalds"));
    if let Ok(Checked::Accepted(_, out)) = certified_path {
        assert!(out.contains("product_checked"));
        assert!(!out.contains("survived_freivalds") && !out.contains("within_tolerance"));
    }
}
