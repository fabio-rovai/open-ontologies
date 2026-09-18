//! The pure core of the certificate boundary, over bytes and nothing else.
//!
//! `docs/trusted-computing-base.md` says that every Lean theorem in `lean/` is
//! conditional on the Rust engine writing down the truth, and that the code
//! deciding whether it does is a handful of small, total, pure functions. This
//! module is those functions. `src/reason.rs` and `src/tableaux.rs` call them;
//! they are not a copy.
//!
//! Everything here is `u8`, `&[u8]`, `Vec<u8>`, `usize`, `bool` and one
//! three-constructor enum. No `str`, no `String`, no trait beyond what the
//! primitives carry, no type from any other crate. That is not a style rule.
//! `aeneas/oo-boundary` reaches THIS FILE with `#[path]` and Aeneas translates
//! it into a pure functional model in Lean 4; Aeneas handles a subset of safe
//! Rust that stops at `str` and at anything reaching into `oxrdf` or `oxiri`.
//! Widening a signature here to `&str` deletes the Lean model of that function.
//! See `docs/aeneas-boundary.md`.
//!
//! The `&str` wrappers live at the call sites and are `as_bytes()` and nothing
//! else. Every separator this module tests for is ASCII, and no ASCII byte can
//! occur inside a multi-byte UTF-8 sequence, so a byte-level test and a
//! `char`-level test agree on every `&str` and the wrapper costs nothing.
//! `tcb_wrappers_agree_with_the_char_level_predicates` in `src/reason.rs` is
//! the property test that pins that sentence.

/// Which position of a certificate line refused to be written.
///
/// Deliberately a bare enum and not an `anyhow::Error`. A formatted error
/// inside a writer is what put CBMC inside the formatting machinery and killed
/// the first `parse_pat` Kani harness, and it would put Aeneas inside
/// `core::fmt` too. The message a user reads is built by the caller, from this
/// and the term.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Position {
    Subject,
    Predicate,
    Object,
}

/// Can this triple be written by an RDF serialiser?
///
/// A literal cannot be a subject and only an IRI can be a predicate, in every
/// RDF 1.1 serialisation. Terms arrive in their N-Triples spelling, so a
/// literal begins with `"` and an IRI with `<`. The history of the defect this
/// guard closed is on `reason::writable_triple`, which is the `&str` wrapper.
///
/// Note what it says about the EMPTY subject: `"".starts_with('"')` is false,
/// so an empty subject passes this guard. That is not a hole, because
/// `term_fits_the_format` refuses an empty term outright and every certificate
/// path runs both, but it is the kind of edge the bounded harness never saw:
/// `writable_triple_decides_both_positions` fixes the term length at four
/// bytes. `OOBoundary.writable_triple_bytes_eq` covers every length including
/// zero.
pub fn writable_triple_bytes(subject: &[u8], predicate: &[u8]) -> bool {
    !starts_with_byte(subject, b'"') && starts_with_byte(predicate, b'<')
}

/// Whether `s` begins with the byte `b`. False on the empty slice.
pub fn starts_with_byte(s: &[u8], b: u8) -> bool {
    !s.is_empty() && s[0] == b
}

/// The half of [`term_fits_the_format`] that is about the FORMAT and not about
/// N-Triples: a field is non-empty and carries no separator.
///
/// `horn.tsv` also writes variable names, which are not terms and have no
/// N-Triples spelling, and this is what they have to satisfy.
pub fn field_fits_the_format(f: &[u8]) -> bool {
    if f.is_empty() {
        return false;
    }
    let mut i = 0;
    while i < f.len() {
        if f[i] == b'\t' || f[i] == b'\n' || f[i] == b'\r' {
            return false;
        }
        i += 1;
    }
    true
}

/// Whether a term can be written to a certificate file without forging it.
///
/// This is TCB-4 and TCB-5, enforced here rather than observed of `oxrdf`.
/// The reasoning is on `reason::term_fits_the_format`, the `&str` wrapper.
pub fn term_fits_the_format(t: &[u8]) -> bool {
    if !field_fits_the_format(t) {
        return false;
    }
    t[0] == b'<' || t[0] == b'"' || (t[0] == b'_' && t.len() > 1 && t[1] == b':')
}

/// A name that survives the round trip through `axioms.tsv` and `model.tsv`.
///
/// TCB-25. The concept grammar is SPACE separated and the files are TAB
/// separated, so the DL path is exposed to a space as well as to the three
/// separators `field_fits_the_format` refuses. The two predicates are
/// deliberately separate functions rather than one with a flag: the set of
/// bytes that breaks a format is a property of that format.
pub fn name_is_safe_bytes(name: &[u8]) -> bool {
    if name.is_empty() {
        return false;
    }
    let mut i = 0;
    while i < name.len() {
        if name[i] == b' ' || name[i] == b'\t' || name[i] == b'\n' || name[i] == b'\r' {
            return false;
        }
        i += 1;
    }
    true
}

/// Append one line of `asserted.tsv`: `s TAB p TAB o NEWLINE`.
///
/// This is the whole of that file's grammar, and it is the narrowest point of
/// the trusted computing base: `OOCert.certificate_sound` is conditional on the
/// asserted graph and `OOCert.Parse.parseTriples` recovers it by splitting on
/// those two characters.
///
/// Every term is checked BEFORE anything is appended, so a refusal leaves `out`
/// byte for byte as it was. A writer that appended a subject and then refused
/// the object would leave a half-line in the buffer, which is the same defect
/// the non-atomic materialiser had.
pub fn push_asserted_line_bytes(
    out: &mut Vec<u8>,
    s: &[u8],
    p: &[u8],
    o: &[u8],
) -> Result<(), Position> {
    if !term_fits_the_format(s) {
        return Err(Position::Subject);
    }
    if !term_fits_the_format(p) {
        return Err(Position::Predicate);
    }
    if !term_fits_the_format(o) {
        return Err(Position::Object);
    }
    out.extend_from_slice(s);
    out.push(b'\t');
    out.extend_from_slice(p);
    out.push(b'\t');
    out.extend_from_slice(o);
    out.push(b'\n');
    Ok(())
}

/// Append a triple as three further fields of a line already begun, the shape
/// `derivations.tsv` and `horn.tsv` use after their header fields. Same guard
/// and same all-or-nothing discipline as [`push_asserted_line_bytes`].
pub fn push_triple_fields_bytes(
    out: &mut Vec<u8>,
    s: &[u8],
    p: &[u8],
    o: &[u8],
) -> Result<(), Position> {
    if !term_fits_the_format(s) {
        return Err(Position::Subject);
    }
    if !term_fits_the_format(p) {
        return Err(Position::Predicate);
    }
    if !term_fits_the_format(o) {
        return Err(Position::Object);
    }
    out.push(b'\t');
    out.extend_from_slice(s);
    out.push(b'\t');
    out.extend_from_slice(p);
    out.push(b'\t');
    out.extend_from_slice(o);
    Ok(())
}
