//! Supported repeated wildcards must not stall repository listing.
use open_ontologies::repo::glob_match;

fn reference(pattern: &[u8], name: &[u8]) -> bool {
    match (pattern.first(), name.first()) {
        (None, None) => true,
        (Some(b'*'), _) => {
            reference(&pattern[1..], name) || (!name.is_empty() && reference(pattern, &name[1..]))
        }
        (Some(b'?'), Some(_)) => reference(&pattern[1..], &name[1..]),
        (Some(pc), Some(nc)) if pc.eq_ignore_ascii_case(nc) => reference(&pattern[1..], &name[1..]),
        _ => false,
    }
}

fn words(alphabet: &[u8], maximum: usize) -> Vec<String> {
    let mut all = vec![String::new()];
    let mut level = vec![String::new()];
    for _ in 0..maximum {
        level = level
            .into_iter()
            .flat_map(|prefix| {
                alphabet.iter().map(move |c| {
                    let mut word = prefix.clone();
                    word.push(char::from(*c));
                    word
                })
            })
            .collect();
        all.extend(level.iter().cloned());
    }
    all
}

#[test]
fn bounded_matcher_keeps_existing_wildcard_and_case_behavior() {
    for pattern in words(b"ab*?", 5) {
        for name in words(b"aB", 4) {
            assert_eq!(
                glob_match(&pattern, &name),
                reference(pattern.as_bytes(), name.as_bytes()),
                "pattern {pattern:?}, name {name:?}"
            );
        }
    }
    assert!(glob_match("*.TTL", "ontology.ttl"));
    assert!(glob_match("schema?.ttl", "schema1.ttl"));
    assert!(!glob_match("schema?.ttl", "schema12.ttl"));
    // Preserve the existing byte contract rather than adding Unicode semantics.
    assert!(!glob_match("?.ttl", "é.ttl"));
    assert!(glob_match("??.ttl", "é.ttl"));
}

#[test]
fn a_short_repeated_star_pattern_finishes_on_a_short_filename() {
    let pattern = format!("{}b.ttl", "*a".repeat(24));
    let name = format!("{}.ttl", "a".repeat(32));
    let (sender, receiver) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || sender.send(glob_match(&pattern, &name)).unwrap());
    let result = receiver
        .recv_timeout(std::time::Duration::from_secs(2))
        .expect(
            "repository filtering exceeded two seconds for a 53-byte pattern and 36-byte filename",
        );
    assert!(!result);
    worker.join().unwrap();
}
