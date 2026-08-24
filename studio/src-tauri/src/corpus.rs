//! Corpus ingestion: read a folder of documents, build the knowledge graph.
//!
//! Wraps `demo/corpus_pipeline.py` and streams its output to the UI as
//! `corpus-progress` events, so the five stages (read, extract, verify, load,
//! scan) appear live rather than as a spinner.
//!
//! The pipeline is deliberately a separate process: it talks to the same
//! engine over MCP that Studio does, so what the UI shows afterwards is the
//! real merged store, not a private copy.

use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager};

pub struct CorpusState {
    /// Arc so the flag can be cleared from the reader thread without holding
    /// a borrow of the AppHandle across statements.
    pub running: Arc<Mutex<bool>>,
}

/// Pick a Python interpreter that actually exists.
///
/// A Finder-launched app inherits a minimal PATH, and a bare "python3" then
/// resolves to the system interpreter rather than whichever one has the
/// pipeline's optional dependencies. Candidates are tried in order of
/// specificity so an explicit choice always wins.
fn resolve_python() -> String {
    if let Ok(explicit) = std::env::var("ONTO_PYTHON") {
        if !explicit.trim().is_empty() {
            return explicit;
        }
    }
    let root = repo_root();
    let candidates = [
        root.join(".venv/bin/python3"),
        root.join("venv/bin/python3"),
        PathBuf::from(std::env::var("HOME").unwrap_or_default()).join("scratch/.venv/bin/python3"),
    ];
    for c in candidates {
        if c.exists() {
            return c.to_string_lossy().into_owned();
        }
    }
    "python3".into()
}

/// Single-quote a value for safe inclusion in a shell command.
fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

/// Repository root, derived from the crate location at compile time.
pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Corpora offered in the UI. The third element is a document count and MUST
/// match what the corpus that populates `demo/corpus/dcat-us` actually
/// contains: it is not computed from disk here because that corpus does not
/// exist in this tree yet, only the pipeline that will read it does.
#[tauri::command]
pub fn corpus_presets() -> Vec<(String, String, usize)> {
    vec![(
        "dcat-us".to_string(),
        "DCAT-US 3.0 profile documents and the W3C DCAT conformance clause".to_string(),
        6,
    )]
}

/// Run the pipeline over `folder`. When `live` is false the cached extraction
/// is reused, which makes the demo instant; when true it re-runs extraction
/// against the configured model endpoint.
#[tauri::command]
pub fn ingest_corpus(
    app: tauri::AppHandle,
    folder: String,
    live: bool,
) -> Result<(), String> {
    let running_flag = app.state::<CorpusState>().running.clone();
    {
        let mut running = running_flag.lock().map_err(|e| e.to_string())?;
        if *running {
            return Err("an ingestion is already running".into());
        }
        *running = true;
    }

    // Prefer the app's resource directory when `demo/` has actually been
    // bundled into it (today only the sidecar's `dist/` and its
    // `package.json` are, per `tauri.conf.json`); fall back to the
    // development checkout the same way `chat::resolve_sidecar_entry` does
    // for the agent sidecar. That keeps this consistent with the rest of the
    // crate rather than assuming the internal branch's directory layout, and
    // it starts working the moment `demo/` is added to `resources` without a
    // second code path to maintain.
    let root = app
        .path()
        .resource_dir()
        .ok()
        .filter(|dir| dir.join("demo").is_dir())
        .unwrap_or_else(repo_root);
    let script = root.join("demo").join("ontology_from_docs.py");
    if !script.exists() {
        *running_flag.lock().unwrap() = false;
        return Err(format!("pipeline not found at {}", script.display()));
    }

    // An app launched from Finder inherits a minimal PATH, so a bare "python3"
    // resolves to /usr/bin/python3 rather than whichever interpreter actually
    // has the pipeline's dependencies installed. Run through a LOGIN shell so
    // the user's real PATH applies, and let ONTO_PYTHON override outright.
    let python = resolve_python();
    let cached = if live { "" } else { "--cached" };
    let command = format!(
        "{} {} {} --corpus {}",
        shell_quote(&python),
        shell_quote(&script.to_string_lossy()),
        cached,
        shell_quote(&folder),
    );

    let mut child = Command::new("/bin/zsh")
        .arg("-lc")
        .arg(&command)
        .current_dir(&root)
        // Strip ANSI colour: the pane renders plain text.
        .env("NO_COLOR", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to start pipeline: {e}"))?;

    let stdout = child.stdout.take().ok_or("no stdout")?;
    let stderr = child.stderr.take().ok_or("no stderr")?;

    // stderr MUST be drained. Leaving it piped and unread can block the child
    // once the pipe buffer fills, and it is where the actual error message
    // lives: without this the UI can only report "finished with errors".
    let err_handle = app.clone();
    std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            let _ = err_handle.emit("corpus-progress", format!("stderr: {line}"));
        }
    });

    let handle = app.clone();
    let running_flag = running_flag.clone();
    let normalise_root = root.clone();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            let _ = handle.emit("corpus-progress", line);
        }
        let status = child.wait();
        let ok = status.map(|s| s.success()).unwrap_or(false);

        // Post-build normalisation, before anything loads the store.
        //
        // The pipeline REGENERATES _store.ttl on every build, which discards
        // anything appended to it. Access control is derived from the document
        // headers rather than produced by the extractor, so without this it
        // vanishes the moment the graph is rebuilt: the role switcher would
        // silently stop filtering, which is the worst possible failure because
        // it looks like it is working. Running it here ties the ACL triples to
        // the build that produced the store they annotate.
        if ok {
            for script in ["demo/acl_normalise.py", "demo/corpus_text.py"] {
                let path = normalise_root.join(script);
                if !path.exists() {
                    continue;
                }
                let out = Command::new("/bin/zsh")
                    .arg("-lc")
                    .arg(format!(
                        "{} {}",
                        shell_quote(&resolve_python()),
                        shell_quote(&path.to_string_lossy())
                    ))
                    .current_dir(&normalise_root)
                    .output();
                match out {
                    Ok(o) if o.status.success() => {
                        if let Some(line) = String::from_utf8_lossy(&o.stdout).lines().next() {
                            let _ = handle.emit("corpus-progress", line.to_string());
                        }
                    }
                    Ok(o) => {
                        let _ = handle.emit(
                            "corpus-progress",
                            format!("{script} failed: {}", String::from_utf8_lossy(&o.stderr)),
                        );
                    }
                    Err(e) => {
                        let _ = handle.emit("corpus-progress", format!("{script} failed: {e}"));
                    }
                }
            }
        }

        let _ = handle.emit("corpus-done", ok);
        if let Ok(mut running) = running_flag.lock() {
            *running = false;
        }
    });

    Ok(())
}

/// The store the last build produced, for the frontend to feed through the
/// plan -> enforce -> apply cycle in ITS OWN MCP session. Lineage is
/// session-scoped in the engine, so events recorded by the pipeline's session
/// are invisible to the Studio; the trail the Lineage panel shows must be
/// created by the session that reads it.
#[tauri::command]
pub fn read_store() -> Result<String, String> {
    let path = repo_root().join("demo/derived/_store.ttl");
    std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))
}

/// The per-document VOCABULARIES the last build wrote. Alignment operates on
/// class declarations, so instance graphs always produce zero candidates;
/// the fragments are where the duplicate concepts live.
#[tauri::command]
pub fn list_graphs() -> Vec<String> {
    let dir = repo_root().join("demo/derived");
    // The full derived vocabulary first: aligning it against the curated
    // store is the richest source of duplicate-concept candidates.
    let full = dir.join("_ontology.ttl");
    let mut head: Vec<String> = if full.exists() {
        vec![full.to_string_lossy().into_owned()]
    } else {
        Vec::new()
    };
    let mut out: Vec<String> = std::fs::read_dir(&dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| {
                    p.file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| n.ends_with(".vocab.ttl"))
                })
                .filter_map(|p| p.to_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    out.sort();
    head.extend(out);
    head
}

/// The build's decision ledger: every automated resolution (typing, renames,
/// inversions, spam drops, review referrals) as JSONL, newest last.
#[tauri::command]
pub fn read_decisions() -> String {
    std::fs::read_to_string(repo_root().join("demo/derived/_decisions.jsonl")).unwrap_or_default()
}

/// Revert a typing decision: put `from` back in place of `to` on one subject
/// in one document graph. The ledger gets a revert entry rather than a
/// deletion, because an audit trail that forgets is not an audit trail.
#[tauri::command]
pub fn revert_type(doc: String, subject: String, from: String, to: String) -> Result<(), String> {
    let path = repo_root().join(format!("demo/derived/{doc}.data.ttl"));
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;

    let mut out: Vec<String> = Vec::new();
    let mut in_block = false;
    let mut done = false;
    for line in text.lines() {
        let mut l = line.to_string();
        if let Some(rest) = l.strip_prefix(&format!(":{subject}")) {
            let boundary = rest.chars().next().map_or(true, |c| !c.is_alphanumeric() && c != '_');
            if boundary {
                in_block = true;
            }
        } else if l.starts_with(':') {
            in_block = false;
        }
        if in_block && !done {
            let pat = format!("a :{to}");
            if l.contains(&pat) {
                l = l.replacen(&pat, &format!("a :{from}"), 1);
                done = true;
            }
        }
        out.push(l);
    }
    if !done {
        return Err(format!("no `a :{to}` under :{subject} in {doc}"));
    }
    std::fs::write(&path, out.join("\n")).map_err(|e| e.to_string())?;

    let ledger = repo_root().join("demo/derived/_decisions.jsonl");
    let entry = format!(
        "{{\"kind\": \"reverted\", \"doc\": \"{doc}\", \"subject\": \"{subject}\", \"from\": \"{to}\", \"to\": \"{from}\", \"how\": \"human revert in the resolution panel\"}}\n"
    );
    use std::io::Write;
    if let Ok(mut fh) = std::fs::OpenOptions::new().create(true).append(true).open(&ledger) {
        let _ = fh.write_all(entry.as_bytes());
    }
    Ok(())
}

/// Saved ontologies in ~/.open-ontologies, for the Open control: a save
/// without a load is a dead end.
#[tauri::command]
pub fn list_saved() -> Vec<String> {
    let dir = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join(".open-ontologies");
    let mut out: Vec<String> = std::fs::read_dir(&dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("ttl"))
                .filter_map(|p| p.file_stem().and_then(|n| n.to_str()).map(String::from))
                .collect()
        })
        .unwrap_or_default();
    out.sort();
    out
}

/// Native file picker for opening an ontology: starts in the folder where
/// saves land, but the whole disk is reachable. Returns the absolute path.
/// Sync command: Tauri runs it on the main thread, which macOS dialogs need.
#[tauri::command]
pub fn pick_ontology_file() -> Option<String> {
    let start = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join(".open-ontologies");
    rfd::FileDialog::new()
        .add_filter("RDF / OWL", &["ttl", "nt", "rdf", "owl", "trig"])
        .set_directory(&start)
        .pick_file()
        .and_then(|p| p.to_str().map(String::from))
}
