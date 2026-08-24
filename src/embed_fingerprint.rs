//! Fingerprint of the embedding configuration that produced a vector.
//!
//! # Why this exists
//!
//! `embeddings` stores `text_dim`/`struct_dim`, and `hnsw_index_cache` stores
//! `entries_hash` — a fingerprint over the `(iri, text_vec)` set. Neither
//! records *which model produced the numbers*. Change `[embeddings] model`, or
//! swap provider, to something that emits the same dimension — 768 and 1536 are
//! both crowded — and every existing check still passes: the dimensions match,
//! and `entries_hash` is unchanged because the stored bytes are literally the
//! same bytes. The cache is considered valid and the index now spans two
//! semantically unrelated vector spaces.
//!
//! Two distinct failures follow, and only the first is a mix:
//!
//!   1. `VecStore::persist` opens with `DELETE FROM embeddings` and rewrites the
//!      whole in-memory map, so `load_from_db` (old-model vectors) + `upsert`
//!      (new-model vectors) + `persist` writes out a union that looks internally
//!      consistent.
//!   2. With **zero** new entities, every stored vector is old-model and every
//!      *query* vector is new-model. The index is perfectly coherent and the
//!      comparison is still meaningless. No check over the stored set alone can
//!      see this — which is why strengthening `entries_hash` cannot fix it.
//!
//! Hence a fingerprint over the *configuration*, not over the data.
//!
//! # What goes into it
//!
//! A composite hash over `(provider, model, revision)`, per the design agreed in
//! issue #74. `revision` is an `Option` because the local ONNX provider has no
//! such concept — its stand-in is the identity of the model file itself.
//!
//! | provider | `model` | `revision` |
//! |---|---|---|
//! | `local` | model file name | file identity: size, mtime, sha256 of the head |
//! | `openai`-compatible | resolved model name | none — the API exposes no revision |
//!
//! Hashing the model file *path* would not do: the default download URL and the
//! on-disk filename are both stable across a model change made by replacing the
//! file in place.
//!
//! The revision is a **sha256 of the whole file**, not of a head prefix.
//! `SourceFingerprint` (mtime + size + sha256 of the first 64 KiB) is what
//! `ontology_cache` uses, and it is right there because a missed change costs a
//! re-parse. Here a missed change costs vectors from one model served against
//! queries from another — the very thing this module exists to prevent — and
//! the miss is reachable: two fine-tunes of one architecture have byte-identical
//! sizes and graph protos, and `cp -p` preserves mtime. Measured cost of doing
//! it properly: **1.4 s for the 470 MB default model** (330 MB/s, release, the
//! crate's own sha256), against a `TextEmbedder::load` that already reads and
//! optimises that same file. It is paid once per process, and only by the local
//! provider.
//!
//! Two inputs are included beyond the agreed triple, for the openai-compatible
//! arm only. Both are one line each and either can be dropped without touching
//! anything else:
//!
//!   * **`api_base`** — `provider` is `openai` for every OpenAI-compatible
//!     gateway, so without this, moving `nomic-embed-text` from a local Ollama
//!     to a hosted endpoint keeps the same fingerprint while changing the vector
//!     space. That is the issue's own scenario, one layer down.
//!   * **`dimensions`** — the `dimensions` request parameter truncates output
//!     dimensionality on models that support it. Same model name, different
//!     vectors.
//!
//! # What this deliberately does not cover
//!
//! (Fixed in #92: the tokenizer is hashed into the local arm's revision.
//! What follows describes the defect that used to exist.)
//! The **tokenizer**. Swapping `tokenizer.json` while keeping the same `.onnx`
//! changes the vectors and goes undetected here. It is the same class of defect
//! and the fix is one more `SourceFingerprint` — left out only to keep this
//! change to what issue #74 agreed, rather than decided against. Say the word
//! and it is two lines.

use crate::config::EmbeddingsConfig;

/// Marker used when the local model file cannot be inspected. Deliberately not
/// an empty string: "no model on disk" and "a model whose fingerprint happens
/// to be empty" must never collide.
const UNAVAILABLE: &str = "<unavailable>";

/// Human-readable description of the embedding configuration.
///
/// This is the string that gets hashed, and it is also what should be logged on
/// a mismatch — a bare hash tells an operator that something changed, this tells
/// them *what*. Field order is fixed; changing it changes every fingerprint, so
/// treat it as a stored format.
pub fn describe(cfg: &EmbeddingsConfig) -> String {
    let provider = crate::config::resolve_embeddings_provider(cfg);

    match provider.as_str() {
        "openai" | "openai-compatible" | "remote" | "http" => {
            let api_base = crate::config::resolve_embeddings_api_base(cfg);
            let model = crate::config::resolve_embeddings_model(cfg);
            let dimensions = cfg
                .dimensions
                .map(|d| d.to_string())
                .unwrap_or_else(|| "default".to_string());
            // `revision=none`: the OpenAI-compatible API exposes no model
            // revision. The slot is kept so the format does not shift the day
            // one appears.
            format!(
                "provider=openai\napi_base={api_base}\nmodel={model}\ndimensions={dimensions}\nrevision=none"
            )
        }
        _ => {
            // Everything else is the local ONNX arm, including the empty string
            // and "onnx" — mirroring `TextEmbedderProvider::from_config`, which
            // rejects an unknown provider before anything reaches here.
            let path = crate::embed::resolve_local_model_path(cfg);
            let name = path
                .as_ref()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| UNAVAILABLE.to_string());
            let revision = path
                .as_ref()
                .and_then(|p| crate::cache::sha256_file_hex(p).ok())
                .map(|sha| format!("sha256={sha}"))
                .unwrap_or_else(|| UNAVAILABLE.to_string());
            // The tokenizer is part of the model as far as the vectors are
            // concerned: swapping tokenizer.json alone changes what every
            // string embeds to, while the .onnx bytes and therefore the
            // revision above stay identical. Without this hash the guard
            // accepts vectors from one tokenizer against queries from
            // another, which is the same silent mixing of vector spaces the
            // fingerprint exists to prevent. A tokenizer is a few hundred KB,
            // so the cost is not measurable beside the model's own hash.
            let tokenizer = crate::embed::resolve_local_tokenizer_path(cfg)
                .and_then(|p| crate::cache::sha256_file_hex(&p).ok())
                .map(|sha| format!("sha256={sha}"))
                .unwrap_or_else(|| UNAVAILABLE.to_string());
            format!("provider=local\nmodel={name}\nrevision={revision}\ntokenizer={tokenizer}")
        }
    }
}

/// Composite fingerprint of the embedding configuration, as lowercase hex.
///
/// Stored on `embeddings` (where it detects mixed vector spaces) and on
/// `hnsw_index_cache` (where it detects a stale index). A mismatch invalidates
/// *alongside* `entries_hash`, never instead of it: the two catch different
/// things and both are cheap.
pub fn fingerprint(cfg: &EmbeddingsConfig) -> String {
    crate::cache::sha256_hex(describe(cfg).as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn local_cfg(model_path: Option<&str>) -> EmbeddingsConfig {
        EmbeddingsConfig {
            provider: Some("local".into()),
            model_path: model_path.map(|s| s.to_string()),
            ..Default::default()
        }
    }

    fn openai_cfg(model: &str, api_base: Option<&str>) -> EmbeddingsConfig {
        EmbeddingsConfig {
            provider: Some("openai".into()),
            model: Some(model.into()),
            api_base: api_base.map(|s| s.to_string()),
            ..Default::default()
        }
    }

    /// `cargo test` runs the tests of one binary on several threads, and the
    /// environment is process-wide: without this, one test clearing
    /// `OPEN_ONTOLOGIES_EMBEDDINGS_MODEL` between another's two `fingerprint`
    /// calls makes that other test fail for no reason of its own. The RAII
    /// guard below restores state but grants no mutual exclusion, which is a
    /// different thing.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// Guard the env-var overrides the resolvers read, so a developer's shell
    /// cannot make these pass or fail for reasons unrelated to the code, and
    /// hold [`ENV_LOCK`] for the duration so no sibling test observes the
    /// mutation.
    struct EnvGuard {
        saved: Vec<(&'static str, Option<String>)>,
        /// Held for its `Drop`, never read: it is what serialises these tests
        /// against each other. Named rather than `_0` so that intent survives.
        #[allow(dead_code)]
        lock: std::sync::MutexGuard<'static, ()>,
    }
    impl EnvGuard {
        fn clear() -> Self {
            let keys = [
                "OPEN_ONTOLOGIES_EMBEDDINGS_PROVIDER",
                "OPEN_ONTOLOGIES_EMBEDDINGS_API_BASE",
                "OPEN_ONTOLOGIES_EMBEDDINGS_MODEL",
            ];
            // Take the lock BEFORE reading or mutating anything: saving the
            // old values while another test is mid-mutation would restore its
            // scratch state as if it were ours.
            // Poisoning only means an earlier test panicked while holding it;
            // the environment is still ours to restore, so take it either way.
            let lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            let saved = keys
                .iter()
                .map(|k| (*k, std::env::var(k).ok()))
                .collect::<Vec<_>>();
            for (k, _) in &saved {
                unsafe { std::env::remove_var(k) };
            }
            Self { saved, lock }
        }
    }
    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for (k, v) in &self.saved {
                match v {
                    Some(val) => unsafe { std::env::set_var(k, val) },
                    None => unsafe { std::env::remove_var(k) },
                }
            }
        }
    }

    #[test]
    fn a_different_tokenizer_changes_the_fingerprint() {
        // The defect this guards: same .onnx, different tokenizer.json, same
        // fingerprint, so vectors from one tokenizer were accepted against
        // queries from another.
        let dir = std::env::temp_dir().join(format!("oo-fp-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let model = dir.join("model.onnx");
        std::fs::write(&model, b"same model bytes").unwrap();
        let tok = dir.join("tokenizer.json");

        let cfg = EmbeddingsConfig {
            provider: Some("local".into()),
            model_path: Some(model.to_string_lossy().into_owned()),
            tokenizer_path: Some(tok.to_string_lossy().into_owned()),
            ..Default::default()
        };

        std::fs::write(&tok, b"{\"vocab\":\"one\"}").unwrap();
        let first = fingerprint(&cfg);
        std::fs::write(&tok, b"{\"vocab\":\"two\"}").unwrap();
        let second = fingerprint(&cfg);

        std::fs::remove_dir_all(&dir).ok();
        assert_ne!(
            first, second,
            "swapping the tokenizer must change the fingerprint"
        );
    }

    #[test]
    fn the_same_configuration_hashes_the_same_way() {
        let _g = EnvGuard::clear();
        let a = fingerprint(&openai_cfg("text-embedding-3-small", None));
        let b = fingerprint(&openai_cfg("text-embedding-3-small", None));
        assert_eq!(a, b);
    }

    #[test]
    fn swapping_the_model_changes_the_fingerprint() {
        // The issue's headline case: two models of identical dimension.
        let _g = EnvGuard::clear();
        let small = fingerprint(&openai_cfg("text-embedding-3-small", None));
        let other = fingerprint(&openai_cfg("nomic-embed-text", None));
        assert_ne!(small, other);
    }

    #[test]
    fn swapping_the_provider_changes_the_fingerprint() {
        let _g = EnvGuard::clear();
        let remote = fingerprint(&openai_cfg("text-embedding-3-small", None));
        let local = fingerprint(&local_cfg(Some("/nonexistent/model.onnx")));
        assert_ne!(remote, local);
    }

    #[test]
    fn the_same_model_name_on_a_different_gateway_is_a_different_space() {
        // `provider` is "openai" for every OpenAI-compatible gateway, so
        // without api_base this pair collides — and moving a model between a
        // local Ollama and a hosted endpoint silently reuses its vectors.
        let _g = EnvGuard::clear();
        let ollama = fingerprint(&openai_cfg(
            "nomic-embed-text",
            Some("http://localhost:11434/v1"),
        ));
        let hosted = fingerprint(&openai_cfg(
            "nomic-embed-text",
            Some("https://api.example.com/v1"),
        ));
        assert_ne!(ollama, hosted);
    }

    #[test]
    fn truncating_dimensionality_changes_the_fingerprint() {
        let _g = EnvGuard::clear();
        let mut truncated = openai_cfg("text-embedding-3-large", None);
        truncated.dimensions = Some(256);
        assert_ne!(
            fingerprint(&openai_cfg("text-embedding-3-large", None)),
            fingerprint(&truncated)
        );
    }

    #[test]
    fn an_env_override_is_reflected() {
        // The fingerprint has to describe the *effective* configuration. One
        // taken from the config file alone would report "unchanged" for a
        // deployment that overrides the model by environment.
        let _g = EnvGuard::clear();
        let cfg = openai_cfg("text-embedding-3-small", None);
        let from_file = fingerprint(&cfg);
        unsafe { std::env::set_var("OPEN_ONTOLOGIES_EMBEDDINGS_MODEL", "text-embedding-3-large") };
        let overridden = fingerprint(&cfg);
        unsafe { std::env::remove_var("OPEN_ONTOLOGIES_EMBEDDINGS_MODEL") };
        assert_ne!(from_file, overridden);
    }

    #[test]
    fn replacing_the_model_file_in_place_changes_the_fingerprint() {
        // The reason the *path* is not enough: the default download URL and the
        // on-disk filename are both stable across a model swap done by
        // overwriting the file.
        let _g = EnvGuard::clear();
        let dir = std::env::temp_dir().join(format!("oo-fp-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let model = dir.join("model.onnx");

        std::fs::write(&model, b"first model weights").unwrap();
        let cfg = local_cfg(Some(model.to_str().unwrap()));
        let before = fingerprint(&cfg);

        // Same path, same name, different contents.
        std::fs::write(&model, b"second model weights, same length!").unwrap();
        let after = fingerprint(&cfg);

        std::fs::remove_dir_all(&dir).ok();
        assert_ne!(before, after, "a model replaced in place went undetected");
    }

    #[test]
    fn a_same_size_swap_with_preserved_mtime_is_still_detected() {
        // The case a head-prefix fingerprint misses, and it is not contrived:
        // two fine-tunes of one architecture have byte-identical sizes AND
        // byte-identical ONNX graph protos, `cp -p` preserves mtime, and the
        // weights that differ can sit past the first 64 KiB. Under
        // (mtime, size, sha256-of-head) the two are indistinguishable.
        let _g = EnvGuard::clear();
        let dir = std::env::temp_dir().join(format!("oo-fp3-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let model = dir.join("model.onnx");

        // 96 KiB: a shared 64 KiB head, then weights.
        const HEAD: usize = 64 * 1024;
        let mut a = vec![0xABu8; HEAD];
        a.extend(std::iter::repeat_n(0x01u8, 32 * 1024));
        let mut b = vec![0xABu8; HEAD];
        b.extend(std::iter::repeat_n(0x02u8, 32 * 1024));
        assert_eq!(a.len(), b.len(), "the scenario requires identical sizes");
        assert_eq!(a[..HEAD], b[..HEAD], "the scenario requires an identical head");

        let cfg = local_cfg(Some(model.to_str().unwrap()));

        std::fs::write(&model, &a).unwrap();
        let mtime = std::fs::metadata(&model).unwrap().modified().unwrap();
        let before = fingerprint(&cfg);

        // `cp -p`: same bytes count, same modification time.
        std::fs::write(&model, &b).unwrap();
        std::fs::File::open(&model)
            .unwrap()
            .set_modified(mtime)
            .unwrap();

        let after = fingerprint(&cfg);
        std::fs::remove_dir_all(&dir).ok();

        assert_ne!(
            before, after,
            "a same-size, same-mtime model swap differing only past the head \
             went undetected — this is the corruption path the module exists \
             to close"
        );
    }

    #[test]
    fn a_missing_model_file_does_not_collide_with_a_real_one() {
        let _g = EnvGuard::clear();
        let absent = fingerprint(&local_cfg(Some("/nonexistent/definitely-not-here.onnx")));
        let dir = std::env::temp_dir().join(format!("oo-fp2-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let model = dir.join("definitely-not-here.onnx");
        std::fs::write(&model, b"weights").unwrap();
        let present = fingerprint(&local_cfg(Some(model.to_str().unwrap())));
        std::fs::remove_dir_all(&dir).ok();
        assert_ne!(absent, present);
    }

    #[test]
    fn describe_is_readable_enough_to_log() {
        // `describe` is what gets logged on a mismatch; a bare hash would tell
        // an operator that something changed but not what.
        let _g = EnvGuard::clear();
        let d = describe(&openai_cfg("text-embedding-3-small", None));
        assert!(d.contains("provider=openai"), "{d}");
        assert!(d.contains("model=text-embedding-3-small"), "{d}");
        assert!(d.contains("revision=none"), "{d}");
    }
}
