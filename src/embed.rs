//! ONNX-based text embedding using tract.
//! Loads a sentence-transformer model exported to ONNX format.

use anyhow::{Context, Result};
use std::path::Path;
use tokenizers::Tokenizer;
use tract_onnx::prelude::*;

use crate::poincare::l2_normalize;

/// Model download URLs for bge-small-en-v1.5
pub const BGE_SMALL_ONNX_URL: &str =
    "https://huggingface.co/BAAI/bge-small-en-v1.5/resolve/main/onnx/model.onnx";
pub const BGE_SMALL_TOKENIZER_URL: &str =
    "https://huggingface.co/BAAI/bge-small-en-v1.5/resolve/main/tokenizer.json";

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
pub enum TextEmbedderProvider {
    Local(TextEmbedder),
    OpenAI(crate::embed_remote::OpenAIEmbedder),
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
                let default_model_dir =
                    dirs::home_dir().map(|h| h.join(".open-ontologies/models"));

                let model_path = cfg
                    .model_path
                    .clone()
                    .map(|p| std::path::PathBuf::from(crate::config::expand_tilde(&p)))
                    .or_else(|| {
                        default_model_dir
                            .as_ref()
                            .map(|d| d.join("bge-small-en-v1.5.onnx"))
                    });

                let tokenizer_path = cfg
                    .tokenizer_path
                    .clone()
                    .map(|p| std::path::PathBuf::from(crate::config::expand_tilde(&p)))
                    .or_else(|| {
                        default_model_dir
                            .as_ref()
                            .map(|d| d.join("tokenizer.json"))
                    });

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
