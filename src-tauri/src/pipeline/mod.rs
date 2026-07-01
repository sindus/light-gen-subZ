pub mod audio_extract;
pub mod segmentation;
pub mod stt;
pub mod subtitle_writer;

use std::path::Path;

use anyhow::{Context, Result};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::models;
use stt::{LocalWhisperEngine, SttEngine};

#[derive(Debug, Clone, Serialize)]
pub struct PipelineProgress {
    pub stage: String,
    pub fraction: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct PipelineOutput {
    pub srt_path: String,
    pub srt_content: String,
    pub language: String,
}

fn emit_progress(app: &AppHandle, stage: &str, fraction: f32) {
    let _ = app.emit(
        "pipeline-progress",
        PipelineProgress {
            stage: stage.to_string(),
            fraction,
        },
    );
}

/// Orchestrates the full pipeline: audio extraction -> local transcription -> segmentation -> SRT writing.
/// Writes `<source_name>.srt` next to the input file and returns its content.
pub async fn run(app: AppHandle, input_path: String) -> Result<PipelineOutput> {
    let input = Path::new(&input_path);
    anyhow::ensure!(input.exists(), "file not found: {input_path}");

    emit_progress(&app, "download_model", 0.0);
    let model = models::default_model();
    let model_app = app.clone();
    let model_path = models::ensure_model_downloaded(&model, move |frac| {
        emit_progress(&model_app, "download_model", frac);
    })
    .await
    .context("downloading whisper model")?;

    emit_progress(&app, "extract_audio", 0.0);
    let cache_dir = app
        .path()
        .app_cache_dir()
        .context("resolving app cache directory")?;
    let wav_path =
        audio_extract::extract_to_wav(input, &cache_dir).context("extracting audio via ffmpeg")?;
    emit_progress(&app, "extract_audio", 1.0);

    emit_progress(&app, "transcribe", 0.0);
    let engine = LocalWhisperEngine::new(model_path);
    let transcribe_app = app.clone();
    let transcript = tauri::async_runtime::spawn_blocking(move || {
        engine.transcribe(
            &wav_path,
            Box::new(move |frac| emit_progress(&transcribe_app, "transcribe", frac)),
        )
    })
    .await
    .context("transcription task")??;
    emit_progress(&app, "transcribe", 1.0);

    emit_progress(&app, "write_subtitles", 0.0);
    let cues = segmentation::build_cues(&transcript.segments);
    let srt_content = subtitle_writer::to_srt(&cues);

    let srt_path = input.with_extension("srt");
    std::fs::write(&srt_path, &srt_content).context("writing .srt file")?;
    emit_progress(&app, "write_subtitles", 1.0);

    Ok(PipelineOutput {
        srt_path: srt_path.to_string_lossy().to_string(),
        srt_content,
        language: transcript.language,
    })
}
