use std::path::PathBuf;

use anyhow::{Context, Result};
use futures_util::StreamExt;

const HF_BASE: &str = "https://huggingface.co/Xenova/nllb-200-distilled-600M/resolve/main";

pub struct NllbFiles {
    pub encoder: PathBuf,
    pub decoder: PathBuf,
    pub tokenizer: PathBuf,
}

fn nllb_dir() -> Result<PathBuf> {
    let dir = dirs::data_dir()
        .context("could not determine the user's data directory")?
        .join("light-gen-subZ")
        .join("translate-models")
        .join("nllb-200-distilled-600M");
    std::fs::create_dir_all(&dir).context("creating translation model directory")?;
    Ok(dir)
}

/// Downloads the local NLLB translation model (encoder + decoder + tokenizer, ~900MB total)
/// if not already present, reporting overall progress (0.0-1.0) across all three files.
pub async fn ensure_nllb_downloaded(mut on_progress: impl FnMut(f32)) -> Result<NllbFiles> {
    let dir = nllb_dir()?;
    let files = [
        (
            "onnx/encoder_model_quantized.onnx",
            dir.join("encoder.onnx"),
        ),
        (
            "onnx/decoder_model_quantized.onnx",
            dir.join("decoder.onnx"),
        ),
        ("tokenizer.json", dir.join("tokenizer.json")),
    ];

    let total_files = files.len();
    for (i, (remote, dest)) in files.iter().enumerate() {
        if !dest.exists() {
            download_file(&format!("{HF_BASE}/{remote}"), dest, |frac| {
                on_progress((i as f32 + frac) / total_files as f32);
            })
            .await?;
        }
        on_progress((i as f32 + 1.0) / total_files as f32);
    }

    Ok(NllbFiles {
        encoder: dir.join("encoder.onnx"),
        decoder: dir.join("decoder.onnx"),
        tokenizer: dir.join("tokenizer.json"),
    })
}

async fn download_file(
    url: &str,
    dest: &std::path::Path,
    mut on_progress: impl FnMut(f32),
) -> Result<()> {
    let tmp_path = dest.with_extension("part");
    let response = reqwest::get(url)
        .await
        .with_context(|| format!("request to {url} failed"))?
        .error_for_status()
        .with_context(|| format!("HTTP error response for {url}"))?;

    let total_size = response.content_length().unwrap_or(0);
    let mut file = tokio::fs::File::create(&tmp_path)
        .await
        .context("creating temporary download file")?;
    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();

    use tokio::io::AsyncWriteExt;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("error while streaming the download")?;
        file.write_all(&chunk)
            .await
            .context("writing downloaded file")?;
        downloaded += chunk.len() as u64;
        if total_size > 0 {
            on_progress(downloaded as f32 / total_size as f32);
        }
    }
    file.flush().await.context("flushing downloaded file")?;

    tokio::fs::rename(&tmp_path, dest)
        .await
        .context("renaming downloaded file")?;

    Ok(())
}
