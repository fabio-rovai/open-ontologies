//! Compile cache for ontology files.
//!
//! A loaded ontology is serialized as N-Triples (the simplest, fastest-to-parse
//! RDF format) and written to `<cache_dir>/<sha>.nt`. Metadata is stored in
//! the `ontology_cache` SQLite table so we can quickly answer "is the cache
//! still valid for this source file?" without re-parsing.
//!
//! Validity key: `(source_path, mtime_secs, size, sha256(prefix))`.
//! For most workflows the (mtime, size) pair is sufficient and avoids hashing
//! large files; the sha256 of the first 64 KiB is also stored as a tie-breaker.
use anyhow::{anyhow, Context, Result};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use rusqlite::params;

use crate::state::StateDb;

const HASH_PREFIX_BYTES: usize = 64 * 1024;

/// Information about a source ontology file used as the cache validity key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFingerprint {
    pub mtime_secs: i64,
    pub size: u64,
    pub sha_prefix: String,
}

impl SourceFingerprint {
    /// Compute a fingerprint for `path` (mtime, size, sha256 of first 64 KiB).
    pub fn from_path(path: &Path) -> Result<Self> {
        let meta = fs::metadata(path)
            .with_context(|| format!("stat({})", path.display()))?;
        let size = meta.len();
        let mtime_secs = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        // Read up to 64 KiB for hashing — enough to differentiate files
        // with identical mtime/size (rare but possible with `cp -p`).
        let mut file = fs::File::open(path)
            .with_context(|| format!("open({})", path.display()))?;
        let mut buf = vec![0u8; HASH_PREFIX_BYTES.min(size as usize)];
        let n = file.read(&mut buf)?;
        buf.truncate(n);
        let sha_prefix = sha256_hex(&buf);

        Ok(Self { mtime_secs, size, sha_prefix })
    }
}

/// A small, dependency-free SHA-256 implementation (FIPS 180-4).
/// We avoid pulling in a new crate just for the cache key.
fn sha256_hex(input: &[u8]) -> String {
    let digest = sha256(input);
    let mut s = String::with_capacity(64);
    for b in digest.iter() {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

fn sha256(input: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
        0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
        0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
        0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
        0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];

    // Pre-processing: padding
    let bit_len = (input.len() as u64).wrapping_mul(8);
    let mut msg = Vec::with_capacity(input.len() + 64);
    msg.extend_from_slice(input);
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in msg.chunks(64) {
        let mut w = [0u32; 64];
        for (i, word) in chunk.chunks(4).enumerate() {
            w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
            (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }

    let mut out = [0u8; 32];
    for (i, word) in h.iter().enumerate() {
        out[i * 4..(i + 1) * 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

/// One row from the `ontology_cache` table.
#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub name: String,
    pub source_path: String,
    pub source_mtime: i64,
    pub source_size: u64,
    pub source_sha: String,
    pub cache_path: String,
    pub triple_count: usize,
    pub compiled_at: String,
    pub last_access_at: String,
}

/// Manages the on-disk N-Triples cache plus its SQLite metadata.
pub struct CacheManager {
    cache_dir: PathBuf,
    db: StateDb,
}

impl CacheManager {
    pub fn new(cache_dir: PathBuf, db: StateDb) -> Result<Self> {
        fs::create_dir_all(&cache_dir)
            .with_context(|| format!("create cache dir {}", cache_dir.display()))?;
        Ok(Self { cache_dir, db })
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// Look up a cache entry by ontology name.
    pub fn get(&self, name: &str) -> Result<Option<CacheEntry>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT name, source_path, source_mtime, source_size, source_sha, \
                    cache_path, triple_count, compiled_at, last_access_at \
             FROM ontology_cache WHERE name = ?1",
        )?;
        let mut rows = stmt.query(params![name])?;
        if let Some(row) = rows.next()? {
            Ok(Some(CacheEntry {
                name: row.get(0)?,
                source_path: row.get(1)?,
                source_mtime: row.get(2)?,
                source_size: row.get::<_, i64>(3)? as u64,
                source_sha: row.get(4)?,
                cache_path: row.get(5)?,
                triple_count: row.get::<_, i64>(6)? as usize,
                compiled_at: row.get(7)?,
                last_access_at: row.get(8)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Determine whether `entry` is still valid for the on-disk source file.
    /// Returns `Ok(true)` only if the source's fingerprint matches.
    pub fn is_fresh(&self, entry: &CacheEntry) -> Result<bool> {
        let path = Path::new(&entry.source_path);
        if !path.exists() {
            return Ok(false);
        }
        let fp = SourceFingerprint::from_path(path)?;
        Ok(fp.mtime_secs == entry.source_mtime
            && fp.size == entry.source_size
            && fp.sha_prefix == entry.source_sha)
    }

    /// Build a cache file path for a given source key.
    pub fn cache_path_for(&self, name: &str, sha: &str) -> PathBuf {
        // Use both the name and the sha so renames don't collide and we can
        // garbage-collect stale entries by `name`.
        let safe = name
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect::<String>();
        self.cache_dir.join(format!("{}.{}.nt", safe, &sha[..sha.len().min(16)]))
    }

    /// Atomically write `content` to `path` (writes to `<path>.tmp` then renames).
    pub fn atomic_write(path: &Path, content: &str) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).ok();
        }
        let tmp = path.with_extension("nt.tmp");
        fs::write(&tmp, content)
            .with_context(|| format!("write {}", tmp.display()))?;
        fs::rename(&tmp, path)
            .with_context(|| format!("rename {} -> {}", tmp.display(), path.display()))?;
        Ok(())
    }

    /// Insert or update a cache entry.
    pub fn upsert(
        &self,
        name: &str,
        source_path: &str,
        fp: &SourceFingerprint,
        cache_path: &Path,
        triple_count: usize,
    ) -> Result<()> {
        let conn = self.db.conn();
        conn.execute(
            "INSERT INTO ontology_cache \
                (name, source_path, source_mtime, source_size, source_sha, cache_path, \
                 triple_count, compiled_at, last_access_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, datetime('now'), datetime('now')) \
             ON CONFLICT(name) DO UPDATE SET \
                source_path = excluded.source_path, \
                source_mtime = excluded.source_mtime, \
                source_size = excluded.source_size, \
                source_sha = excluded.source_sha, \
                cache_path = excluded.cache_path, \
                triple_count = excluded.triple_count, \
                compiled_at = excluded.compiled_at, \
                last_access_at = excluded.last_access_at",
            params![
                name,
                source_path,
                fp.mtime_secs,
                fp.size as i64,
                fp.sha_prefix,
                cache_path.to_string_lossy(),
                triple_count as i64,
            ],
        )?;
        Ok(())
    }

    /// Touch `last_access_at` for an entry.
    pub fn touch(&self, name: &str) -> Result<()> {
        let conn = self.db.conn();
        conn.execute(
            "UPDATE ontology_cache SET last_access_at = datetime('now') WHERE name = ?1",
            params![name],
        )?;
        Ok(())
    }

    /// Remove a cache entry and its on-disk file.
    pub fn remove(&self, name: &str) -> Result<()> {
        let entry = self.get(name)?;
        if let Some(e) = &entry {
            let _ = fs::remove_file(&e.cache_path);
        }
        let conn = self.db.conn();
        conn.execute("DELETE FROM ontology_cache WHERE name = ?1", params![name])?;
        Ok(())
    }

    /// List all cache entries.
    pub fn list(&self) -> Result<Vec<CacheEntry>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT name, source_path, source_mtime, source_size, source_sha, \
                    cache_path, triple_count, compiled_at, last_access_at \
             FROM ontology_cache ORDER BY last_access_at DESC",
        )?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(CacheEntry {
                name: row.get(0)?,
                source_path: row.get(1)?,
                source_mtime: row.get(2)?,
                source_size: row.get::<_, i64>(3)? as u64,
                source_sha: row.get(4)?,
                cache_path: row.get(5)?,
                triple_count: row.get::<_, i64>(6)? as usize,
                compiled_at: row.get(7)?,
                last_access_at: row.get(8)?,
            });
        }
        Ok(out)
    }
}

/// Convenience: derive a default ontology name from a source path
/// (basename without extension), used when the caller doesn't supply one.
pub fn derive_name(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "default".to_string())
}

/// Validate a shape used by tests and the registry.
pub fn require_path(path: &Option<String>) -> Result<&str> {
    match path {
        Some(p) if !p.is_empty() => Ok(p),
        _ => Err(anyhow!("path required")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_known_vector() {
        // FIPS 180-2 example: "abc" -> ba7816bf...
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn fingerprint_changes_when_content_changes() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("x.ttl");
        fs::write(&p, "@prefix : <http://example.org/> . :a a :B .").unwrap();
        let fp1 = SourceFingerprint::from_path(&p).unwrap();
        // Sleep just enough to bump mtime; on filesystems with 1s resolution
        // we also rewrite different content so size/sha differs even at same mtime.
        std::thread::sleep(std::time::Duration::from_millis(1100));
        fs::write(&p, "@prefix : <http://example.org/> . :a a :C .").unwrap();
        let fp2 = SourceFingerprint::from_path(&p).unwrap();
        assert_ne!(fp1, fp2);
    }

    #[test]
    fn derive_name_strips_extension() {
        assert_eq!(derive_name("/tmp/foo.ttl"), "foo");
        assert_eq!(derive_name("foo.bar.ttl"), "foo.bar");
        assert_eq!(derive_name(""), "default");
    }
}
