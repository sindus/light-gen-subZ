use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use futures_util::StreamExt;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, serde::Serialize)]
pub struct ModelInfo {
    pub name: &'static str,
    pub filename: &'static str,
    pub url: &'static str,
    pub approx_size_mb: u32,
}

/// Known ggml models (whisper.cpp), hosted on the official HuggingFace repo.
/// `small-q5_1` is the MVP's default model: good accuracy/speed/size tradeoff.
pub fn known_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            name: "base",
            filename: "ggml-base-q5_1.bin",
            url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base-q5_1.bin",
            approx_size_mb: 60,
        },
        ModelInfo {
            name: "small",
            filename: "ggml-small-q5_1.bin",
            url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small-q5_1.bin",
            approx_size_mb: 190,
        },
        ModelInfo {
            name: "medium",
            filename: "ggml-medium-q5_0.bin",
            url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium-q5_0.bin",
            approx_size_mb: 540,
        },
    ]
}

pub fn default_model() -> ModelInfo {
    known_models()
        .into_iter()
        .find(|m| m.name == "small")
        .expect("default model 'small' must exist in known_models()")
}

pub fn models_dir() -> Result<PathBuf> {
    let dir = dirs::data_dir()
        .context("could not determine the user's data directory")?
        .join("light-gen-subZ")
        .join("models");
    std::fs::create_dir_all(&dir).context("creating models directory")?;
    Ok(dir)
}

pub fn local_path(model: &ModelInfo) -> Result<PathBuf> {
    Ok(models_dir()?.join(model.filename))
}

/// Downloads the model if not already present locally, reporting progress (0.0-1.0).
/// Writes a .sha256 sidecar file to detect corruption on a future re-download.
pub async fn ensure_model_downloaded(
    model: &ModelInfo,
    mut on_progress: impl FnMut(f32),
) -> Result<PathBuf> {
    let dest = local_path(model)?;
    if dest.exists() {
        return Ok(dest);
    }

    let tmp_path = dest.with_extension("part");
    let response = reqwest::get(model.url)
        .await
        .with_context(|| format!("request to {} failed", model.url))?
        .error_for_status()
        .with_context(|| format!("HTTP error response for {}", model.url))?;

    let total_size = response.content_length().unwrap_or(0);
    let mut file = tokio::fs::File::create(&tmp_path)
        .await
        .context("creating temporary download file")?;
    let mut hasher = Sha256::new();
    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();

    use tokio::io::AsyncWriteExt;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("error while streaming the download")?;
        hasher.update(&chunk);
        file.write_all(&chunk)
            .await
            .context("writing downloaded file")?;
        downloaded += chunk.len() as u64;
        if total_size > 0 {
            on_progress(downloaded as f32 / total_size as f32);
        }
    }
    file.flush().await.context("flushing downloaded file")?;

    tokio::fs::rename(&tmp_path, &dest)
        .await
        .context("renaming downloaded model file")?;

    let checksum = format!("{:x}", hasher.finalize());
    write_checksum_sidecar(&dest, &checksum)?;

    Ok(dest)
}

fn write_checksum_sidecar(model_path: &Path, checksum: &str) -> Result<()> {
    let sidecar = model_path.with_extension("sha256");
    std::fs::write(sidecar, checksum).context("writing checksum file")
}
