use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single field-to-predicate mapping within a `MappingConfig`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldMapping {
    pub field: String,
    pub predicate: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datatype: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,
    #[serde(default)]
    pub lookup: bool,
}

/// Configuration that describes how structured data rows map to RDF triples.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappingConfig {
    pub base_iri: String,
    pub id_field: String,
    pub class: String,
    pub mappings: Vec<FieldMapping>,
}

const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";

impl MappingConfig {
    /// Generate a naive 1:1 mapping from column headers.
    ///
    /// The first column becomes the `id_field`. Every column gets an
    /// `xsd:string` predicate under `{base_iri}ont#`.
    pub fn from_headers(headers: &[String], base_iri: &str, class: &str) -> Self {
        let id_field = headers
            .first()
            .cloned()
            .unwrap_or_else(|| "id".to_string());

        let mappings = headers
            .iter()
            .map(|h| FieldMapping {
                field: h.clone(),
                predicate: format!("{}ont#{}", base_iri, sanitize_iri(h)),
                datatype: Some(XSD_STRING.to_string()),
                class: None,
                lookup: false,
            })
            .collect();

        Self {
            base_iri: base_iri.to_string(),
            id_field,
            class: class.to_string(),
            mappings,
        }
    }

    /// Convert a single data row to a list of N-Triples lines.
    ///
    /// Produces:
    /// - An `rdf:type` triple linking the subject to `self.class`
    /// - One triple per mapped field that has a non-empty value in the row
    ///
    /// Lookup fields produce IRI objects; typed fields produce typed literals;
    /// everything else produces plain literals.
    pub fn row_to_triples(&self, row: &HashMap<String, String>) -> Vec<String> {
        let subject_id = row
            .get(&self.id_field)
            .filter(|v| !v.is_empty())
            .map(|v| sanitize_iri(v))
            .unwrap_or_else(rand_id);

        let subject = format!("<{}{}>", self.base_iri, subject_id);
        let mut triples = Vec::new();

        // rdf:type triple
        triples.push(format!("{} <{}> <{}> .", subject, RDF_TYPE, self.class));

        for mapping in &self.mappings {
            let value = match row.get(&mapping.field) {
                Some(v) if !v.is_empty() => v,
                _ => continue,
            };

            let object = if mapping.lookup {
                // Lookup field: produce an IRI object
                format!("<{}{}>", self.base_iri, sanitize_iri(value))
            } else if let Some(ref dt) = mapping.datatype {
                // Typed literal
                format!("\"{}\"^^<{}>", escape_ntriples(value), dt)
            } else {
                // Plain literal
                format!("\"{}\"", escape_ntriples(value))
            };

            triples.push(format!("{} <{}> {} .", subject, mapping.predicate, object));
        }

        triples
    }

    /// Convert multiple rows to a single N-Triples string.
    pub fn rows_to_ntriples(&self, rows: &[HashMap<String, String>]) -> String {
        rows.iter()
            .flat_map(|row| self.row_to_triples(row))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Replace spaces and special characters that are invalid in IRIs with underscores.
fn sanitize_iri(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            ' ' | '<' | '>' | '{' | '}' | '|' | '\\' | '^' | '`' | '?' => '_',
            _ => c,
        })
        .collect()
}

/// Escape characters that are special in N-Triples string literals.
fn escape_ntriples(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            '\\' => vec!['\\', '\\'],
            '"' => vec!['\\', '"'],
            '\n' => vec!['\\', 'n'],
            '\r' => vec!['\\', 'r'],
            '\t' => vec!['\\', 't'],
            _ => vec![c],
        })
        .collect()
}

/// Generate a simple random ID based on the current time's sub-second nanos.
fn rand_id() -> String {
    use std::time::SystemTime;
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    format!("_auto_{}", nanos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_iri() {
        assert_eq!(sanitize_iri("hello world"), "hello_world");
        assert_eq!(sanitize_iri("a<b>c"), "a_b_c");
        assert_eq!(sanitize_iri("no_change"), "no_change");
        assert_eq!(sanitize_iri("PEAK_OF_PERFECTION_?"), "PEAK_OF_PERFECTION__");
    }

    #[test]
    fn test_escape_ntriples() {
        assert_eq!(escape_ntriples(r#"say "hi""#), r#"say \"hi\""#);
        assert_eq!(escape_ntriples("line\nnew"), "line\\nnew");
    }

    #[test]
    fn test_rand_id_format() {
        let id = rand_id();
        assert!(id.starts_with("_auto_"));
    }
}
