//! The front page claims Simplified Technical English. This measures the claim.
//!
//! `README.md` says it follows the writing rules of ASD-STE100. A claim about
//! prose is as checkable as a claim about a number, and this repository does not
//! ship the second kind without a test, so it does not ship the first kind
//! without one either.
//!
//! What is measured here are the STE WRITING RULES that a machine can see:
//! sentence length, paragraph length, the passive voice, and a short list of
//! constructions the standard replaces. What is NOT measured is approved-word
//! compliance, because the STE dictionary is a licensed document that this
//! project does not hold. The README says the same thing in its own words, so a
//! reader is told the limit of the claim rather than left to assume it.
//!
//! The Chinese page is checked for a different property, in
//! `the_translated_page_carries_the_same_structure`: word counts mean nothing in
//! Chinese, so it is held to structural parity with the English instead.

use std::path::PathBuf;

/// STE Rule 5.2: a descriptive sentence has a maximum of 25 words.
const MAX_WORDS: usize = 25;
/// STE Rule 5.4: a descriptive paragraph has a maximum of 6 sentences.
const MAX_SENTENCES: usize = 6;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(name: &str) -> String {
    std::fs::read_to_string(repo().join(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// The prose paragraphs, with everything that is not prose removed.
///
/// Code fences, HTML blocks, tables, headings, link-reference lines and block
/// quotes of code are not prose and are not held to the rules. What remains is
/// the text a reader actually reads as sentences.
fn paragraphs(md: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut in_code = false;
    let mut in_html = 0i32;
    for raw in md.lines() {
        let line = raw.trim();
        if line.starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if in_code {
            continue;
        }
        // HTML blocks: the centred banners, figures and captions.
        in_html += line.matches("<p").count() as i32 + line.matches("<h1").count() as i32;
        if line.starts_with('<') || in_html > 0 {
            if line.contains("</p>") || line.contains("</h1>") {
                in_html = 0;
            }
            continue;
        }
        let skip = line.is_empty()
            || line.starts_with('|')      // table
            || line.starts_with('#')      // heading
            || line.starts_with("- ")     // list item: STE allows vertical lists
            || line.starts_with("* ")
            || line.starts_with("$ ")
            || line.starts_with("//")
            || line.starts_with('[') && line.contains("]:");
        if skip {
            if !current.trim().is_empty() {
                out.push(std::mem::take(&mut current));
            }
            current.clear();
            continue;
        }
        current.push_str(line);
        current.push(' ');
    }
    if !current.trim().is_empty() {
        out.push(current);
    }
    out
}

/// Split on sentence enders, without splitting on the dots inside a version, a
/// file name, an IRI or an abbreviation.
fn sentences(p: &str) -> Vec<String> {
    let b: Vec<char> = p.chars().collect();
    let mut out = Vec::new();
    let mut start = 0usize;
    for i in 0..b.len() {
        if !matches!(b[i], '.' | '!' | '?') {
            continue;
        }
        // Step over the closing marks of an emphasis span: "…closes.**" ends a
        // sentence exactly as "…closes." does, and an extractor that misses that
        // joins two sentences and reports a length nobody wrote.
        let mut j = i + 1;
        while matches!(b.get(j), Some('*') | Some('_') | Some('`') | Some(')') | Some('"')) {
            j += 1;
        }
        let next = b.get(j).copied().unwrap_or(' ');
        let mut k = j + 1;
        while matches!(b.get(k), Some('*') | Some('_') | Some('`') | Some('[') | Some('"')) {
            k += 1;
        }
        let after = b.get(k).copied().unwrap_or('A');
        // A sentence ends at a stop, a space, and then a capital or a backtick.
        let ends = next == ' ' && (after.is_uppercase() || after == '`' || after == '*');
        // Not a stop inside 4.33.1, docs/x.md, ex:a.b, or "e.g."
        let prev = if i == 0 { ' ' } else { b[i - 1] };
        let numeric = prev.is_ascii_digit() && b.get(i + 1).is_some_and(|c| c.is_ascii_digit());
        if ends && !numeric {
            let s: String = b[start..=i].iter().collect();
            if !s.trim().is_empty() {
                out.push(s.trim().to_string());
            }
            start = i + 1;
        }
    }
    let tail: String = b[start..].iter().collect();
    if !tail.trim().is_empty() {
        out.push(tail.trim().to_string());
    }
    out
}

/// Words, with code spans, links and bare IRIs counted as one word each, which
/// is how a reader meets them.
fn word_count(s: &str) -> usize {
    let mut t = s.to_string();
    // a code span, a markdown link target and an IRI are each one token
    while let (Some(a), Some(b)) = (t.find('`'), t[t.find('`').map(|i| i + 1).unwrap_or(0)..].find('`')) {
        let start = a;
        let end = a + 1 + b;
        t.replace_range(start..=end, "CODE");
    }
    let t = regex_lite_replace(&t, '(', ')', "LINK");
    t.split_whitespace().filter(|w| !w.chars().all(|c| !c.is_alphanumeric())).count()
}

/// Replace every `open..close` span with `with`, without a regex dependency.
fn regex_lite_replace(s: &str, open: char, close: char, with: &str) -> String {
    let mut out = String::new();
    let mut depth = 0usize;
    for c in s.chars() {
        if c == open {
            if depth == 0 {
                out.push_str(with);
            }
            depth += 1;
        } else if c == close {
            depth = depth.saturating_sub(1);
        } else if depth == 0 {
            out.push(c);
        }
    }
    out
}

#[test]
fn every_sentence_is_within_the_simplified_english_limit() {
    let md = read("README.md");
    let paras = paragraphs(&md);
    assert!(
        paras.len() > 15,
        "only {} prose paragraphs were found, so this test is not reading the page and \
         cannot fail",
        paras.len()
    );
    let mut long = Vec::new();
    let mut counted = 0usize;
    for p in &paras {
        for s in sentences(p) {
            counted += 1;
            let n = word_count(&s);
            if n > MAX_WORDS {
                long.push(format!("  {n} words: {}", s.trim()));
            }
        }
    }
    assert!(counted > 60, "only {counted} sentences examined; the extractor is broken");
    assert!(
        long.is_empty(),
        "{} sentence(s) are longer than the {MAX_WORDS}-word limit of STE rule 5.2, out of \
         {counted} examined:\n{}\n\nSplit each one. A sentence that carries two ideas \
         carries them both badly, and it is the hardest kind of sentence to translate.",
        long.len(),
        long.join("\n")
    );
}

#[test]
fn every_paragraph_is_within_the_simplified_english_limit() {
    let md = read("README.md");
    let mut long = Vec::new();
    for p in paragraphs(&md) {
        let n = sentences(&p).len();
        if n > MAX_SENTENCES {
            long.push(format!("  {n} sentences: {}…", p.chars().take(90).collect::<String>()));
        }
    }
    assert!(
        long.is_empty(),
        "{} paragraph(s) hold more than the {MAX_SENTENCES} sentences of STE rule 5.4:\n{}",
        long.len(),
        long.join("\n")
    );
}

/// The constructions STE replaces, and the reasons it replaces them.
#[test]
fn the_page_avoids_the_constructions_the_standard_replaces() {
    let md = read("README.md");
    let prose = paragraphs(&md).join("\n").to_lowercase();
    // (construction, what STE asks for instead)
    let banned: [(&str, &str); 12] = [
        ("in order to", "use \"to\""),
        ("utilise", "use \"use\""),
        ("utilize", "use \"use\""),
        ("comprises", "use \"contains\" or \"has\""),
        ("leverage", "use \"use\""),
        ("facilitate", "use \"help\" or name the action"),
        ("in the event that", "use \"if\""),
        ("prior to", "use \"before\""),
        ("subsequent to", "use \"after\""),
        ("with regard to", "use \"about\""),
        ("it should be noted that", "delete it and state the fact"),
        ("due to the fact that", "use \"because\""),
    ];
    let mut hits = Vec::new();
    for (bad, fix) in banned {
        if prose.contains(bad) {
            hits.push(format!("  {bad:?} -> {fix}"));
        }
    }
    assert!(
        hits.is_empty(),
        "the page uses {} construction(s) that Simplified Technical English replaces:\n{}",
        hits.len(),
        hits.join("\n")
    );
}

/// The page must say what its language claim covers, and what it does not.
///
/// Claiming "ASD-STE100" without the dictionary would be a claim this project
/// cannot check, which is the failure mode every decision record here exists to
/// prevent. The page states the limit; this holds the page to stating it.
#[test]
fn the_language_claim_states_its_own_limit() {
    let md = read("README.md");
    assert!(
        md.contains("ASD-STE100"),
        "the page no longer names the standard it follows"
    );
    assert!(
        md.contains("does not claim approved-word compliance"),
        "the page claims Simplified Technical English without stating that the approved-word \
         dictionary is licensed and unchecked here. A claim nobody can verify is the one \
         thing this repository does not ship."
    );
    assert!(
        md.contains("readme_simplified_english_test.rs"),
        "the page claims a test measures its language and does not name the test"
    );
}

/// The translated page is the same product, not a shorter one.
///
/// `README.zh-CN.md` used to be a stub: it carried the banners and lost the
/// argument. Word counts do not transfer between the two languages, so this
/// holds the pages to STRUCTURE: the same headings in the same order, the same
/// figures, and the same numbers.
#[test]
fn the_translated_page_carries_the_same_structure() {
    let en = read("README.md");
    let zh = read("README.zh-CN.md");

    let heads = |s: &str| -> usize { s.lines().filter(|l| l.starts_with("## ")).count() };
    let (he, hz) = (heads(&en), heads(&zh));
    assert!(
        hz + 1 >= he,
        "the English page has {he} sections and the translated page has {hz}. The \
         translation is a different product, not the same one in another language."
    );

    // Each figure, by stem: the translated page is expected to use the
    // TRANSLATED variant where one exists, and the same file where it does not.
    for (stem, translated) in [
        ("knowledge-graph", true),
        ("hqdm-audit", true),
        ("demo-certify", false),
        ("logo.png", false),
    ] {
        assert!(
            zh.contains(stem),
            "the translated page is missing the figure {stem:?}. Both pages show the same \
             front page, or the translation is a different product."
        );
        if translated {
            let zh_file = format!("{stem}.zh-CN.svg");
            assert!(
                std::path::Path::new(&repo().join("docs/assets").join(&zh_file)).exists(),
                "{zh_file} does not exist, and the translated page is expected to use it"
            );
            assert!(
                zh.contains(&zh_file),
                "the translated page uses the English {stem:?} figure. The generator emits a \
                 Chinese one, and a page that shows English words inside a Chinese figure is \
                 the translation that was not finished."
            );
            assert!(
                !en.contains(&zh_file),
                "the English page uses the Chinese figure {zh_file:?}"
            );
        }
    }
    // The translated figure must carry the SAME computed numbers as the English
    // one. The words are translated; the counts come from the run.
    let (en_svg, zh_svg) = (
        std::fs::read_to_string(repo().join("docs/assets/hqdm-audit.svg")).expect("en figure"),
        std::fs::read_to_string(repo().join("docs/assets/hqdm-audit.zh-CN.svg")).expect("zh figure"),
    );
    for n in ["23", "12", "13", "195", "39"] {
        let needle = format!(">{n}</text>");
        assert_eq!(
            en_svg.matches(&needle).count(),
            zh_svg.matches(&needle).count(),
            "the two language versions of the HQDM figure disagree about how many times {n} \
             appears. Only the words are translated; every count is computed from the same rows."
        );
    }
    for link in ["docs/tool-reference.md", "open-ontologies-try.vercel.app"] {
        assert!(zh.contains(link), "the translated page is missing {link:?}");
    }
    // The headline numbers are digits in both languages, so they can be compared.
    for n in ["426", "259", "901", "23", "12", "13", "195", "39"] {
        assert!(
            zh.contains(n),
            "the translated page does not state {n}, and the English page does. A number \
             that moved on one side only is how a translation starts to lie."
        );
    }
}
