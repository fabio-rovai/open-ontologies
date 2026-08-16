//! Reference Open Ontologies plugin (ABI v1): lints class labels for
//! convention violations. Demonstrates the full contract — manifest,
//! allocation, packed-pointer returns, and consuming injected SPARQL
//! bindings.
//!
//! Invoke as:
//!
//! ```text
//! onto_plugin_call plugin=label-case-lint tool=lint_labels \
//!   sparql='SELECT ?class ?label WHERE { ?class a <http://www.w3.org/2002/07/owl#Class> ; <http://www.w3.org/2000/01/rdf-schema#label> ?label }'
//! ```

use serde_json::{json, Value};

const MANIFEST: &str = r#"{
  "name": "label-case-lint",
  "version": "0.1.0",
  "tools": [{
    "name": "lint_labels",
    "description": "Checks class labels from injected SPARQL bindings (?label, plus any subject var for attribution): flags empty labels, leading/trailing/double whitespace, and labels not starting with an uppercase letter"
  }]
}"#;

#[no_mangle]
pub extern "C" fn oo_abi_version() -> i32 {
    1
}

#[no_mangle]
pub extern "C" fn oo_alloc(len: i32) -> i32 {
    let layout = std::alloc::Layout::from_size_align(len.max(1) as usize, 1).unwrap();
    unsafe { std::alloc::alloc(layout) as i32 }
}

/// Pack a byte buffer into the ABI's `(ptr << 32) | len` return form. The
/// buffer is leaked on purpose: the instance is torn down after each call.
fn pack(bytes: Vec<u8>) -> i64 {
    let len = bytes.len() as u64;
    let ptr = Vec::leak(bytes).as_ptr() as u32 as u64;
    ((ptr << 32) | len) as i64
}

#[no_mangle]
pub extern "C" fn oo_describe() -> i64 {
    pack(MANIFEST.as_bytes().to_vec())
}

#[no_mangle]
pub extern "C" fn oo_call(ptr: i32, len: i32) -> i64 {
    let input = unsafe { std::slice::from_raw_parts(ptr as *const u8, len as usize) };
    let doc: Value = match serde_json::from_slice(input) {
        Ok(v) => v,
        Err(e) => return pack(json!({"error": format!("invalid input JSON: {e}")}).to_string().into_bytes()),
    };
    match doc["tool"].as_str() {
        Some("lint_labels") => pack(lint_labels(&doc).to_string().into_bytes()),
        other => pack(json!({"error": format!("unknown tool {other:?}")}).to_string().into_bytes()),
    }
}

fn lint_labels(doc: &Value) -> Value {
    let Some(rows) = doc["bindings"].as_array() else {
        return json!({"error": "no bindings — pass a `sparql` SELECT with a ?label variable to onto_plugin_call"});
    };
    let mut issues = Vec::new();
    for row in rows {
        let Some(label) = row["label"].as_str() else { continue };
        let label = label.trim_start_matches('"');
        let label = label.split('"').next().unwrap_or(label);
        let subject = row
            .as_object()
            .and_then(|o| o.iter().find(|(k, _)| *k != "label"))
            .and_then(|(_, v)| v.as_str())
            .unwrap_or("?");
        let mut problems = Vec::new();
        if label.is_empty() {
            problems.push("empty label");
        } else {
            if label != label.trim() {
                problems.push("leading/trailing whitespace");
            }
            if label.contains("  ") {
                problems.push("double whitespace");
            }
            if label.chars().next().is_some_and(|c| c.is_lowercase()) {
                problems.push("class label should start uppercase");
            }
        }
        if !problems.is_empty() {
            issues.push(json!({"subject": subject, "label": label, "problems": problems}));
        }
    }
    json!({"ok": true, "checked": rows.len(), "issues": issues})
}
