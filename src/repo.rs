//! On-disk ontology repository support.
//!
//! A "repo" is one or more host directories (configured via
//! `[general] ontology_dirs` or the `OPEN_ONTOLOGIES_ONTOLOGY_DIRS` env var)
//! that contain RDF/OWL files the server can list and load on demand.
//!
//! This module is dependency-free of the MCP layer — it only deals with
//! filesystem walking, extension filtering, glob matching, and safe path
//! resolution. The MCP tools (`onto_repo_list`, `onto_repo_load`) live in
//! `src/server.rs` and use the helpers below.
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use anyhow::{anyhow, Result};

/// File extensions the RDF parser (`GraphStore::detect_format`) accepts.
/// Lowercased, without the leading dot. Keep in sync with `graph.rs`.
pub const RDF_EXTENSIONS: &[&str] = &[
    "ttl", "turtle", "nt", "ntriples", "rdf", "xml", "owl", "nq", "trig", "jsonld",
];

/// True if `path`'s extension is one of `RDF_EXTENSIONS` (case-insensitive).
pub fn has_rdf_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            let lower = e.to_ascii_lowercase();
            RDF_EXTENSIONS.contains(&lower.as_str())
        })
        .unwrap_or(false)
}

/// One discovered ontology file within a configured repo directory.
#[derive(Debug, Clone)]
pub struct RepoEntry {
    /// Absolute path on disk.
    pub path: PathBuf,
    /// Path relative to the repo dir it was found under.
    pub relative: PathBuf,
    /// Repo dir this entry belongs to.
    pub repo_dir: PathBuf,
    /// Default ontology name (file stem).
    pub name: String,
    pub size: u64,
    pub mtime_secs: i64,
}

/// Minimal fnmatch-style glob: supports `*` and `?` against a single filename
/// (no path separators). This is intentionally tiny so we don't pull in a
/// glob crate just for filtering tool output.
pub fn glob_match(pattern: &str, name: &str) -> bool {
    fn helper(p: &[u8], n: &[u8]) -> bool {
        match (p.first(), n.first()) {
            (None, None) => true,
            (Some(b'*'), _) => {
                // Try consuming zero or more characters from `n`.
                if helper(&p[1..], n) {
                    return true;
                }
                if !n.is_empty() && helper(p, &n[1..]) {
                    return true;
                }
                false
            }
            (Some(b'?'), Some(_)) => helper(&p[1..], &n[1..]),
            (Some(pc), Some(nc)) if pc.eq_ignore_ascii_case(nc) => helper(&p[1..], &n[1..]),
            _ => false,
        }
    }
    helper(pattern.as_bytes(), name.as_bytes())
}

/// Normalize `dir` to an absolute path that lies inside one of the configured
/// repo directories. Returns the resolved path *and* the matching repo dir.
///
/// This is the path-traversal guard: callers cannot pass arbitrary host
/// paths via the `dir` argument of `onto_repo_list`.
pub fn resolve_within_repos(dir: &str, repos: &[PathBuf]) -> Result<(PathBuf, PathBuf)> {
    if repos.is_empty() {
        return Err(anyhow!(
            "no ontology_dirs configured; set [general] ontology_dirs in config.toml or OPEN_ONTOLOGIES_ONTOLOGY_DIRS"
        ));
    }
    let candidate = if Path::new(dir).is_absolute() {
        PathBuf::from(crate::config::expand_tilde(dir))
    } else {
        // Relative paths are resolved against each repo dir in order; the
        // first existing match wins.
        repos
            .iter()
            .map(|r| r.join(dir))
            .find(|p| p.exists())
            .unwrap_or_else(|| repos[0].join(dir))
    };
    let canon = std::fs::canonicalize(&candidate).unwrap_or_else(|_| candidate.clone());

    for repo in repos {
        let repo_canon = std::fs::canonicalize(repo).unwrap_or_else(|_| repo.clone());
        if canon.starts_with(&repo_canon) {
            return Ok((canon, repo_canon));
        }
    }
    Err(anyhow!(
        "directory '{}' is not under any configured ontology_dirs",
        dir
    ))
}

/// Walk a single directory (optionally recursively) collecting RDF files.
fn walk(repo_dir: &Path, start: &Path, recursive: bool, out: &mut Vec<RepoEntry>) {
    let read = match std::fs::read_dir(start) {
        Ok(r) => r,
        Err(_) => return,
    };
    for entry in read.flatten() {
        let path = entry.path();
        let ft = match entry.file_type() {
            Ok(t) => t,
            Err(_) => continue,
        };
        if ft.is_dir() {
            if recursive {
                walk(repo_dir, &path, true, out);
            }
            continue;
        }
        if !ft.is_file() {
            continue;
        }
        if !has_rdf_extension(&path) {
            continue;
        }
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let mtime_secs = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let relative = path
            .strip_prefix(repo_dir)
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|_| path.clone());
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("default")
            .to_string();
        out.push(RepoEntry {
            path: path.clone(),
            relative,
            repo_dir: repo_dir.to_path_buf(),
            name,
            size: meta.len(),
            mtime_secs,
        });
    }
}

/// List RDF files under one repo directory.
pub fn list_one(repo_dir: &Path, start: &Path, recursive: bool) -> Vec<RepoEntry> {
    let mut out = Vec::new();
    walk(repo_dir, start, recursive, &mut out);
    // Stable order: by relative path.
    out.sort_by(|a, b| a.relative.cmp(&b.relative));
    out
}

/// List RDF files across all configured repo directories.
pub fn list_all(repos: &[PathBuf], recursive: bool) -> Vec<RepoEntry> {
    let mut out = Vec::new();
    for repo in repos {
        let canon = std::fs::canonicalize(repo).unwrap_or_else(|_| repo.clone());
        walk(&canon, &canon, recursive, &mut out);
    }
    out.sort_by(|a, b| a.repo_dir.cmp(&b.repo_dir).then(a.relative.cmp(&b.relative)));
    out
}

/// Resolve an `onto_repo_load` `name` argument to an absolute path that
/// lies inside one of the configured repo directories.
///
/// Resolution order:
///  1. If `name` is an absolute path, it must already lie under a repo dir.
///  2. If `name` contains a path separator, treat as a relative path under
///     each repo dir; first existing match wins.
///  3. Otherwise treat as a bare stem and recursively search every repo dir
///     for a file whose stem matches and whose extension is in
///     `RDF_EXTENSIONS`. If multiple files match, return an error listing
///     all candidates.
pub fn resolve_load_target(name: &str, repos: &[PathBuf]) -> Result<PathBuf> {
    if repos.is_empty() {
        return Err(anyhow!(
            "no ontology_dirs configured; set [general] ontology_dirs in config.toml or OPEN_ONTOLOGIES_ONTOLOGY_DIRS"
        ));
    }
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("name must not be empty"));
    }

    // Case 1 & 2: looks like a path.
    let looks_like_path = trimmed.contains('/')
        || trimmed.contains('\\')
        || Path::new(trimmed).is_absolute()
        || has_rdf_extension(Path::new(trimmed));

    if looks_like_path {
        let expanded = crate::config::expand_tilde(trimmed);
        let candidate_paths: Vec<PathBuf> = if Path::new(&expanded).is_absolute() {
            vec![PathBuf::from(&expanded)]
        } else {
            repos.iter().map(|r| r.join(&expanded)).collect()
        };
        for cand in &candidate_paths {
            if !cand.exists() || !cand.is_file() {
                continue;
            }
            let canon = std::fs::canonicalize(cand).unwrap_or_else(|_| cand.clone());
            for repo in repos {
                let repo_canon = std::fs::canonicalize(repo).unwrap_or_else(|_| repo.clone());
                if canon.starts_with(&repo_canon) {
                    return Ok(canon);
                }
            }
            return Err(anyhow!(
                "path '{}' is outside the configured ontology_dirs",
                cand.display()
            ));
        }
        return Err(anyhow!("no file matching '{}' found in ontology_dirs", trimmed));
    }

    // Case 3: bare stem — recursive search across all repos.
    let entries = list_all(repos, true);
    let mut matches: Vec<&RepoEntry> = entries.iter().filter(|e| e.name == trimmed).collect();
    match matches.len() {
        0 => Err(anyhow!(
            "no ontology with name '{}' found in configured ontology_dirs",
            trimmed
        )),
        1 => Ok(matches.remove(0).path.clone()),
        _ => {
            let paths: Vec<String> = matches
                .iter()
                .map(|e| e.path.display().to_string())
                .collect();
            Err(anyhow!(
                "ambiguous name '{}' matches {} files: {}",
                trimmed,
                paths.len(),
                paths.join(", ")
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rdf_extension_recognized() {
        assert!(has_rdf_extension(Path::new("a.ttl")));
        assert!(has_rdf_extension(Path::new("a.TTL")));
        assert!(has_rdf_extension(Path::new("a.owl")));
        assert!(has_rdf_extension(Path::new("a.jsonld")));
        assert!(!has_rdf_extension(Path::new("a.txt")));
        assert!(!has_rdf_extension(Path::new("noext")));
    }

    #[test]
    fn glob_basic() {
        assert!(glob_match("*.ttl", "foo.ttl"));
        assert!(glob_match("*", "anything"));
        assert!(glob_match("foo?.ttl", "foo1.ttl"));
        assert!(!glob_match("*.ttl", "foo.nt"));
        assert!(glob_match("FOO*", "foo_bar"));
    }
}
