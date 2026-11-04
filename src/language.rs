// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

//! Natural-language handling for ontology labels.
//!
//! RDF labels carry an optional BCP-47 language tag (`"Dog"@en`). Before this
//! module, the alignment engine matched the raw N-Triples string returned by
//! `Term::to_string()`, which (a) silently mangled tagged literals
//! (`"Dog"@en` survived `trim_matches('"')` as `Dog"@en`) and (b) gave callers
//! no way to either pin matching to a preferred language or opt into fully
//! multilingual matching.
//!
//! This module parses the language tag off the literal and provides a small,
//! configurable policy so callers can keep all languages (default — pairs with
//! a multilingual embedding model for cross-lingual alignment) or restrict to a
//! preferred set. Untagged / datatyped literals are always treated as
//! language-neutral and retained regardless of policy.

/// A label with its optional (lowercased) BCP-47 language tag.
///
/// `lang == None` means a plain or datatyped literal — language-neutral, always
/// retained under any policy.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Label {
    pub text: String,
    pub lang: Option<String>,
}

impl Label {
    pub fn new(text: impl Into<String>, lang: Option<String>) -> Self {
        Self {
            text: text.into(),
            lang: lang.map(|l| l.trim().to_lowercase()).filter(|l| !l.is_empty()),
        }
    }

    /// Human/LLM-readable form: `"text@lang"` when tagged, else just `"text"`.
    /// Used to surface the language of borderline alignment candidates so the
    /// reviewing orchestrator can judge cross-lingual pairs.
    pub fn tagged(&self) -> String {
        match &self.lang {
            Some(lang) => format!("{}@{}", self.text, lang),
            None => self.text.clone(),
        }
    }
}

/// Parse an oxigraph `Term::to_string()` literal into its lexical value and
/// language tag.
///
/// Handles the N-Triples literal forms:
///  - `"value"@en`            → `(value, Some("en"))`
///  - `"value"^^<datatype>`   → `(value, None)`
///  - `"value"`               → `(value, None)`
///  - bare (IRI / local name) → `(raw, None)`
///
/// The closing quote is located as the last `"` in the string, which is correct
/// for well-formed literals because neither the `@lang` suffix nor the
/// `^^<datatype>` suffix contains a double quote. Common escapes inside the
/// lexical value (`\"`, `\\`, `\n`, `\t`, `\r`) are unescaped.
pub fn parse_literal(raw: &str) -> Label {
    let s = raw.trim();
    if !s.starts_with('"') {
        // Bare IRI or unquoted local name — no language tag.
        return Label::new(s.to_string(), None);
    }
    if let Some(close) = s.rfind('"')
        && close > 0
    {
        let value = unescape(&s[1..close]);
        let suffix = &s[close + 1..];
        let lang = suffix.strip_prefix('@').map(|l| l.to_string());
        return Label::new(value, lang);
    }
    // Degenerate input (e.g. a lone `"`). Fall back to a best-effort strip.
    Label::new(s.trim_matches('"').to_string(), None)
}

/// Unescape the common N-Triples/Turtle string escapes that appear inside a
/// literal's lexical form. Anything else is passed through verbatim.
fn unescape(s: &str) -> String {
    if !s.contains('\\') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Should this label be retained under the given preferred-language policy?
///
/// An empty `preferred` list means "keep all languages" (multilingual mode).
/// Untagged labels (`lang == None`) are always kept. Otherwise the label's tag
/// must be present in `preferred`. Comparison is case-insensitive on the prefix
/// so that `"en"` matches `"en-GB"` / `"en-us"` etc.
pub fn label_matches_policy(label: &Label, preferred: &[String]) -> bool {
    if preferred.is_empty() {
        return true;
    }
    match &label.lang {
        None => true,
        Some(lang) => preferred.iter().any(|p| {
            let p = p.to_lowercase();
            lang == &p || lang.starts_with(&format!("{p}-"))
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tagged_literal() {
        let l = parse_literal("\"Dog\"@en");
        assert_eq!(l.text, "Dog");
        assert_eq!(l.lang.as_deref(), Some("en"));
    }

    #[test]
    fn parse_region_tag_is_lowercased() {
        let l = parse_literal("\"Colour\"@en-GB");
        assert_eq!(l.text, "Colour");
        assert_eq!(l.lang.as_deref(), Some("en-gb"));
    }

    #[test]
    fn parse_plain_literal() {
        let l = parse_literal("\"Dog\"");
        assert_eq!(l.text, "Dog");
        assert_eq!(l.lang, None);
    }

    #[test]
    fn parse_typed_literal_has_no_lang() {
        let l = parse_literal("\"42\"^^<http://www.w3.org/2001/XMLSchema#integer>");
        assert_eq!(l.text, "42");
        assert_eq!(l.lang, None);
    }

    #[test]
    fn parse_non_english_value() {
        assert_eq!(parse_literal("\"Chien\"@fr").text, "Chien");
        assert_eq!(parse_literal("\"Hund\"@de").lang.as_deref(), Some("de"));
        // Non-Latin scripts survive intact.
        assert_eq!(parse_literal("\"犬\"@ja").text, "犬");
    }

    #[test]
    fn parse_bare_value() {
        let l = parse_literal("Dog");
        assert_eq!(l.text, "Dog");
        assert_eq!(l.lang, None);
    }

    #[test]
    fn unescape_quotes_and_backslashes() {
        let l = parse_literal("\"say \\\"hi\\\"\"@en");
        assert_eq!(l.text, "say \"hi\"");
        assert_eq!(l.lang.as_deref(), Some("en"));
    }

    #[test]
    fn policy_empty_keeps_all() {
        assert!(label_matches_policy(&Label::new("Chien", Some("fr".into())), &[]));
        assert!(label_matches_policy(&Label::new("Dog", None), &[]));
    }

    #[test]
    fn policy_untagged_always_kept() {
        let pref = vec!["en".to_string()];
        assert!(label_matches_policy(&Label::new("Dog", None), &pref));
    }

    #[test]
    fn policy_filters_by_preferred() {
        let pref = vec!["en".to_string(), "cy".to_string()];
        assert!(label_matches_policy(&Label::new("Dog", Some("en".into())), &pref));
        assert!(label_matches_policy(&Label::new("Ci", Some("cy".into())), &pref));
        assert!(!label_matches_policy(&Label::new("Chien", Some("fr".into())), &pref));
    }

    #[test]
    fn policy_region_subtag_matches_base() {
        let pref = vec!["en".to_string()];
        assert!(label_matches_policy(&Label::new("Colour", Some("en-gb".into())), &pref));
    }
}
