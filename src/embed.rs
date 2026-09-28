//! ONNX-based text embedding using tract.
//! Loads a sentence-transformer model exported to ONNX format.

use anyhow::{Context, Result};
use std::path::Path;
use std::sync::Arc;
use tokenizers::Tokenizer;
use tract_onnx::prelude::*;

use crate::poincare::l2_normalize;

/// Default embedding model: multilingual MiniLM (384-dim, BERT-style with
/// `token_type_ids`, mean pooling — a drop-in match for this loader's input
/// signature and pooling). Replaces the previous English-only
/// `bge-small-en-v1.5` so that labels in different natural languages embed into
/// a *shared* vector space, which is what makes cross-lingual alignment
/// (`Dog` ↔ `Chien` ↔ `Perro`) possible via the embedding signal in
/// `onto_align`. Override with `[embeddings] model_url/tokenizer_url/model_name`
/// or by switching to the OpenAI provider. The `Xenova/*` repo ships the
/// `onnx/model.onnx` + `tokenizer.json` layout this loader expects.
pub const DEFAULT_MODEL_ONNX_URL: &str =
    "https://huggingface.co/Xenova/paraphrase-multilingual-MiniLM-L12-v2/resolve/main/onnx/model.onnx";
pub const DEFAULT_MODEL_TOKENIZER_URL: &str =
    "https://huggingface.co/Xenova/paraphrase-multilingual-MiniLM-L12-v2/resolve/main/tokenizer.json";
/// On-disk filename for the default downloaded model.
pub const DEFAULT_MODEL_FILENAME: &str = "paraphrase-multilingual-MiniLM-L12-v2.onnx";
/// Legacy English-only model filename. Still loaded as a fallback when present
/// so existing installs that downloaded it before the multilingual switch keep
/// working (with English-only embeddings) until they re-run `init`.
pub const LEGACY_EN_MODEL_FILENAME: &str = "bge-small-en-v1.5.onnx";

// Back-compat aliases for the previous public constant names.
#[deprecated(note = "use DEFAULT_MODEL_ONNX_URL")]
pub const BGE_SMALL_ONNX_URL: &str = DEFAULT_MODEL_ONNX_URL;
#[deprecated(note = "use DEFAULT_MODEL_TOKENIZER_URL")]
pub const BGE_SMALL_TOKENIZER_URL: &str = DEFAULT_MODEL_TOKENIZER_URL;

pub struct TextEmbedder {
    #[allow(clippy::type_complexity)]
    model: SimplePlan<TypedFact, Box<dyn TypedOp>, Graph<TypedFact, Box<dyn TypedOp>>>,
    tokenizer: Tokenizer,
    dim: usize,
}

impl TextEmbedder {
    /// Load an ONNX model and tokenizer from disk.
    pub fn load(model_path: &Path, tokenizer_path: &Path) -> Result<Self> {
        let model = tract_onnx::onnx()
            .model_for_path(model_path)
            .context("Failed to load ONNX model")?
            .into_optimized()
            .context("Failed to optimize model")?
            .into_runnable()
            .context("Failed to create runnable model")?;

        let tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|e| anyhow::anyhow!("Failed to load tokenizer: {e}"))?;

        // Detect output dimension from model
        let output_fact = model.model().output_fact(0)?;
        let dim = output_fact
            .shape
            .as_concrete()
            .and_then(|s| s.last().copied())
            .unwrap_or(384);

        Ok(Self {
            model,
            tokenizer,
            dim,
        })
    }

    /// Embed a single text string. Returns L2-normalized vector.
    pub fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let encoding = self
            .tokenizer
            .encode(text, true)
            .map_err(|e| anyhow::anyhow!("Tokenization failed: {e}"))?;

        let input_ids: Vec<i64> = encoding.get_ids().iter().map(|&id| id as i64).collect();
        let attention_mask: Vec<i64> = encoding
            .get_attention_mask()
            .iter()
            .map(|&m| m as i64)
            .collect();
        let token_type_ids: Vec<i64> =
            encoding.get_type_ids().iter().map(|&t| t as i64).collect();
        let seq_len = input_ids.len();

        let input_ids_tensor =
            tract_ndarray::Array2::from_shape_vec((1, seq_len), input_ids)?;
        let attention_tensor =
            tract_ndarray::Array2::from_shape_vec((1, seq_len), attention_mask.clone())?;
        let type_ids_tensor =
            tract_ndarray::Array2::from_shape_vec((1, seq_len), token_type_ids)?;

        let outputs = self.model.run(tvec![
            input_ids_tensor.into_tensor().into(),
            attention_tensor.into_tensor().into(),
            type_ids_tensor.into_tensor().into(),
        ])?;

        // Get the last hidden state (first output), shape [1, seq_len, dim]
        let output = outputs[0].to_array_view::<f32>()?;

        // Mean pooling with attention mask
        let mut pooled = vec![0.0f32; self.dim];
        let mut mask_sum = 0.0f32;
        for (i, &mask) in attention_mask.iter().enumerate() {
            if mask > 0 {
                let mask_f = mask as f32;
                for j in 0..self.dim {
                    pooled[j] += output[[0, i, j]] * mask_f;
                }
                mask_sum += mask_f;
            }
        }
        if mask_sum > 0.0 {
            for v in &mut pooled {
                *v /= mask_sum;
            }
        }

        Ok(l2_normalize(&pooled))
    }

    /// Embed multiple texts. Returns Vec of L2-normalized vectors.
    pub fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        texts.iter().map(|t| self.embed(t)).collect()
    }

    /// Output dimension of the model.
    pub fn dim(&self) -> usize {
        self.dim
    }
}

/// Unified text embedder that dispatches to either the local ONNX model or
/// an OpenAI-compatible HTTP API, selected by configuration.
// clippy::large_enum_variant fires here (Local is ~1.4 KB against ~100 bytes for
// OpenAI) and the lint had never run, because nothing in CI compiled the
// `embeddings` feature. Boxing is not the right answer for this one: the value
// is built once at startup and immediately parked in an `Arc` (server.rs), then
// only ever borrowed — it is never moved in a hot path or stored in a
// collection. The box would buy an indirection on every embed call and change a
// public type, to save a single move of 1.4 KB at boot.
#[allow(clippy::large_enum_variant)]
pub enum TextEmbedderProvider {
    Local(TextEmbedder),
    OpenAI(crate::embed_remote::OpenAIEmbedder),
}

/// Default directory the local ONNX model and tokenizer are looked up in.
fn default_model_dir() -> Option<std::path::PathBuf> {
    dirs::home_dir().map(|h| h.join(".open-ontologies/models"))
}

/// Resolve the ONNX model file the local provider will load.
///
/// Extracted from `TextEmbedderProvider::from_config` so the embedding
/// fingerprint (`crate::embed_fingerprint`) hashes *the file that is actually
/// loaded*. Two copies of this resolution could drift, and a fingerprint over
/// the wrong file is worse than no fingerprint: it would report "unchanged"
/// while the loader picked up a different model.
///
/// Returns the path whether or not it exists — the caller decides. The legacy
/// fallback only wins when it is the file present on disk.
pub fn resolve_local_model_path(
    cfg: &crate::config::EmbeddingsConfig,
) -> Option<std::path::PathBuf> {
    cfg.model_path
        .clone()
        .map(|p| std::path::PathBuf::from(crate::config::expand_tilde(&p)))
        .or_else(|| {
            default_model_dir().map(|d| {
                // Prefer the multilingual default; fall back to a legacy
                // English model only if it is the one present on disk
                // (older installs).
                let preferred = d.join(DEFAULT_MODEL_FILENAME);
                if preferred.exists() {
                    return preferred;
                }
                let legacy = d.join(LEGACY_EN_MODEL_FILENAME);
                if legacy.exists() { legacy } else { preferred }
            })
        })
}

/// Resolve the tokenizer file the local provider will load. Companion to
/// [`resolve_local_model_path`].
pub fn resolve_local_tokenizer_path(
    cfg: &crate::config::EmbeddingsConfig,
) -> Option<std::path::PathBuf> {
    cfg.tokenizer_path
        .clone()
        .map(|p| std::path::PathBuf::from(crate::config::expand_tilde(&p)))
        .or_else(|| default_model_dir().map(|d| d.join("tokenizer.json")))
}

impl TextEmbedderProvider {
    /// Build a provider from runtime configuration. Returns `Ok(None)` when
    /// the configured provider cannot be initialised (e.g. local model files
    /// missing) so the server can start without embedding tools wired up.
    pub fn from_config(cfg: &crate::config::EmbeddingsConfig) -> anyhow::Result<Option<Self>> {
        let provider = crate::config::resolve_embeddings_provider(cfg);
        match provider.as_str() {
            "openai" | "openai-compatible" | "remote" | "http" => {
                let api_base = crate::config::resolve_embeddings_api_base(cfg);
                let api_key = crate::config::resolve_embeddings_api_key(cfg);
                let model = crate::config::resolve_embeddings_model(cfg);
                let timeout = std::time::Duration::from_secs(
                    cfg.request_timeout_secs.unwrap_or(30).max(1),
                );
                let embedder = crate::embed_remote::OpenAIEmbedder::new(
                    &api_base,
                    api_key,
                    model,
                    cfg.dimensions,
                    timeout,
                )?;
                Ok(Some(Self::OpenAI(embedder)))
            }
            "local" | "" | "onnx" => {
                let model_path = resolve_local_model_path(cfg);
                let tokenizer_path = resolve_local_tokenizer_path(cfg);

                match (model_path, tokenizer_path) {
                    (Some(m), Some(t)) if m.exists() && t.exists() => {
                        let local = TextEmbedder::load(&m, &t)?;
                        Ok(Some(Self::Local(local)))
                    }
                    _ => Ok(None),
                }
            }
            other => anyhow::bail!(
                "unknown embeddings provider '{}': expected 'local' or 'openai'",
                other
            ),
        }
    }

    /// Embed a single text string. Async because the OpenAI variant performs
    /// an HTTP request; the local variant just runs CPU-bound work.
    pub async fn embed(&self, text: &str) -> anyhow::Result<Vec<f32>> {
        match self {
            Self::Local(e) => e.embed(text),
            Self::OpenAI(e) => e.embed(text).await,
        }
    }

    /// Output dimension of the embedding vectors.
    pub fn dim(&self) -> usize {
        match self {
            Self::Local(e) => e.dim(),
            Self::OpenAI(e) => e.dim(),
        }
    }

    /// Short provider identifier ("local" or "openai") for diagnostics.
    pub fn provider_name(&self) -> &'static str {
        match self {
            Self::Local(_) => "local",
            Self::OpenAI(_) => "openai",
        }
    }
}

// ─── One provider per process, not one per session (#262) ───────────────────

/// Everything a server session needs out of the embedding configuration, built
/// once per process instead of once per session.
///
/// # Why this is shared
///
/// The HTTP arm hands `OpenOntologiesServer::new_with_repo_options` to
/// `StreamableHttpService::new` as a per-session factory, so everything that
/// constructor builds is built again for every MCP session that connects. Two
/// of those things are expensive and neither can legitimately differ between
/// sessions handed the same configuration:
///
///   * the provider, whose local arm runs the tract optimize pass over a 470 MB
///     ONNX graph and then holds that graph resident for as long as it lives,
///     and
///   * the configuration fingerprint, whose local arm is a sha256 over that
///     same file plus the tokenizer.
///
/// Sharing them turns "N sessions, N copies of the model" into one copy behind
/// an `Arc`. Nothing in the shared value is mutable: `TextEmbedderProvider`
/// takes `&self` all the way down to `SimplePlan::run` and `Tokenizer::encode`,
/// and the `OpenAIEmbedder` arm keeps its only interior mutability in an
/// `AtomicUsize`. That is what makes one instance servable to every session at
/// once, and it is worth stating rather than assuming, because a provider
/// holding per-session state could not be shared at all.
///
/// # Why the fingerprint travels with it
///
/// `embed_fingerprint` exists to notice that the model behind a set of stored
/// vectors changed, and it answers by reading the file on disk. If the provider
/// were shared but the fingerprint recomputed per session, a model file
/// replaced in place while the process ran would hand a later session the NEW
/// fingerprint while it embeds with the OLD model still resident in the shared
/// `Arc`. That stamps one model's identity onto another model's vectors, which
/// is the exact corruption the fingerprint was added to prevent. Taking both at
/// the same instant and caching them together keeps the label attached to the
/// thing it labels.
///
/// `embed_fingerprint::fingerprint` itself is deliberately left uncached. A
/// direct caller is asking what is on disk now, and that module's own tests
/// replace a model in place and require the answer to change.
#[derive(Clone)]
pub struct SharedEmbeddings {
    /// The provider, or `None` when the configuration produced no usable one,
    /// which is the ordinary case for a deployment whose local model files were
    /// never downloaded.
    pub provider: Option<Arc<TextEmbedderProvider>>,
    /// The error [`TextEmbedderProvider::from_config`] returned, if it returned
    /// one. A string because `anyhow::Error` is not `Clone`, and separate from
    /// `provider: None` because "no model on disk" and "this configuration is
    /// wrong" are different things to tell an operator.
    pub init_error: Option<String>,
    /// [`crate::embed_fingerprint::describe`] over the same configuration, read
    /// at the same moment as the provider.
    pub description: String,
    /// The hash of [`Self::description`], which is what
    /// [`crate::embed_fingerprint::fingerprint`] returns for this
    /// configuration.
    pub fingerprint: String,
}

type SharedCache = std::sync::Mutex<std::collections::HashMap<String, SharedEmbeddings>>;

static SHARED: std::sync::OnceLock<SharedCache> = std::sync::OnceLock::new();

/// How many times the expensive build has actually run, as opposed to being
/// served from the cache.
static BUILD_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Identity of a configuration, for cache purposes.
///
/// Keyed rather than held in a single process-wide cell because
/// `new_with_repo_options` takes a configuration per call. One cell would hand
/// the second, differently configured server the first one's model and report
/// it as that server's own, and serving vectors from silently the wrong model
/// is a worse failure than the one being fixed here. The crate's own tests
/// build differently configured servers inside one process, so this is not a
/// hypothetical.
///
/// Built from RESOLVED values rather than from the struct, because every
/// resolver consults an environment variable before the config field. All of it
/// is cheap: environment reads, string work, and the `exists()` calls that
/// choose between the default and legacy model filenames. Nothing here opens
/// the model.
fn cache_key(cfg: &crate::config::EmbeddingsConfig) -> String {
    let provider = crate::config::resolve_embeddings_provider(cfg);
    match provider.as_str() {
        "openai" | "openai-compatible" | "remote" | "http" => {
            let api_base = crate::config::resolve_embeddings_api_base(cfg);
            let model = crate::config::resolve_embeddings_model(cfg);
            let dimensions = cfg
                .dimensions
                .map(|d| d.to_string())
                .unwrap_or_else(|| "default".to_string());
            let timeout = cfg.request_timeout_secs.unwrap_or(30).max(1);
            // Two deployments differing only by credential do need separate
            // clients, so the key has to cover the key. It is hashed rather
            // than stored so that the credential does not sit in a map key
            // which any future `Debug` over this cache would print.
            let api_key_hash = {
                use std::hash::{Hash, Hasher};
                let mut h = std::collections::hash_map::DefaultHasher::new();
                crate::config::resolve_embeddings_api_key(cfg).hash(&mut h);
                h.finish()
            };
            format!(
                "openai\u{1f}{api_base}\u{1f}{model}\u{1f}{dimensions}\u{1f}{timeout}\u{1f}{api_key_hash}"
            )
        }
        "local" | "" | "onnx" => {
            let model = resolve_local_model_path(cfg)
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default();
            let tokenizer = resolve_local_tokenizer_path(cfg)
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default();
            format!("local\u{1f}{model}\u{1f}{tokenizer}")
        }
        // `from_config` refuses this one. The refusal is cached like any other
        // outcome so that a misconfigured deployment does not re-derive the
        // same error for every session that connects.
        other => format!("unsupported\u{1f}{other}"),
    }
}

/// Build the shared embedding state for `cfg`, or return the copy this process
/// already built for an identical configuration.
///
/// This is the whole of the fix for #262. Every session gets a clone of one
/// `Arc`, so the ONNX optimize pass runs once and one copy of the model is
/// resident however many sessions are open.
pub fn shared_embeddings(cfg: &crate::config::EmbeddingsConfig) -> SharedEmbeddings {
    let key = cache_key(cfg);
    let cache = SHARED.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()));
    // A poisoned lock here means an earlier build panicked. This map is a plain
    // cache and a panic cannot have left it half written, so take the data back
    // rather than turning one panic into a server that can never embed again.
    let mut guard = cache.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(hit) = guard.get(&key) {
        return hit.clone();
    }

    // Built with the lock held, so a second session arriving during the optimize
    // pass waits for that pass instead of starting a second one. It waits no
    // longer than it would have spent doing its own load, and it does not end up
    // holding a second copy of the model.
    BUILD_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    // Described before the provider is loaded, which is the order this ran in
    // when it ran per session: the fingerprint was taken before `from_config`.
    let description = crate::embed_fingerprint::describe(cfg);
    let fingerprint = crate::cache::sha256_hex(description.as_bytes());
    let (provider, init_error) = match TextEmbedderProvider::from_config(cfg) {
        Ok(Some(p)) => (Some(Arc::new(p)), None),
        Ok(None) => (None, None),
        Err(e) => (None, Some(e.to_string())),
    };

    let built = SharedEmbeddings {
        provider,
        init_error,
        description,
        fingerprint,
    };
    guard.insert(key, built.clone());
    built
}

/// How many times [`shared_embeddings`] has actually built a provider over the
/// life of this process, rather than serving one it already had.
///
/// This is the test signal for #262, and it is public because the test that
/// needs it drives whole `OpenOntologiesServer` constructions from outside the
/// crate. One session cannot distinguish shared from per-session, because the
/// first session builds under either arrangement. Two sessions over one
/// configuration must move this counter by exactly one.
pub fn shared_embeddings_builds() -> u64 {
    BUILD_COUNT.load(std::sync::atomic::Ordering::Relaxed)
}

/// Download a file from URL to a local path.
pub async fn download_model_file(url: &str, dest: &Path) -> Result<()> {
    let client = reqwest::Client::new();
    let resp = client
        .get(url)
        .send()
        .await
        .context("Failed to download model")?;

    if !resp.status().is_success() {
        anyhow::bail!("Download failed with status: {}", resp.status());
    }

    let bytes = resp.bytes().await?;
    std::fs::write(dest, &bytes).context("Failed to write model file")?;

    Ok(())
}
