pub mod audio_extract;
pub mod segmentation;
pub mod stt;
pub mod subtitle_writer;
pub mod translate;

use std::path::Path;

use anyhow::{Context, Result};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::config::{self, SttEngineChoice, TranslationEngineChoice};
use crate::models;
use stt::{CloudWhisperEngine, LocalWhisperEngine, Segment, SttEngine, Transcript};
use translate::{CloudDeepLEngine, LocalNllbEngine, TranslationEngine};

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

    let settings = config::load_settings().context("loading settings")?;

    emit_progress(&app, "extract_audio", 0.0);
    let cache_dir = app
        .path()
        .app_cache_dir()
        .context("resolving app cache directory")?;
    let wav_path =
        audio_extract::extract_to_wav(input, &cache_dir).context("extracting audio via ffmpeg")?;
    emit_progress(&app, "extract_audio", 1.0);

    emit_progress(&app, "transcribe", 0.0);
    let transcribe_app = app.clone();
    let transcript: Transcript = match settings.stt_engine {
        SttEngineChoice::Local => {
            emit_progress(&app, "download_model", 0.0);
            let model = models::default_model();
            let model_app = app.clone();
            let model_path = models::ensure_model_downloaded(&model, move |frac| {
                emit_progress(&model_app, "download_model", frac);
            })
            .await
            .context("downloading whisper model")?;
            emit_progress(&app, "download_model", 1.0);

            let engine = LocalWhisperEngine::new(model_path);
            tauri::async_runtime::spawn_blocking(move || {
                engine.transcribe(
                    &wav_path,
                    Box::new(move |frac| emit_progress(&transcribe_app, "transcribe", frac)),
                )
            })
            .await
            .context("transcription task")??
        }
        SttEngineChoice::Cloud => {
            let api_key = config::get_api_key(config::GROQ_API_KEY)
                .context("reading Groq API key")?
                .context("no Groq API key configured — add one in Settings")?;

            let engine = CloudWhisperEngine::new(api_key);
            tauri::async_runtime::spawn_blocking(move || {
                engine.transcribe(
                    &wav_path,
                    Box::new(move |frac| emit_progress(&transcribe_app, "transcribe", frac)),
                )
            })
            .await
            .context("transcription task")??
        }
    };
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

#[derive(Debug, Clone, Serialize)]
pub struct TranslationOutput {
    pub srt_path: String,
    pub srt_content: String,
}

/// Translates an existing SRT's cue text (keeping timestamps), writing
/// `<source_name>.<target_lang>.srt` next to it.
pub async fn translate(
    app: AppHandle,
    srt_path: String,
    srt_content: String,
    source_lang: Option<String>,
    target_lang: String,
) -> Result<TranslationOutput> {
    let settings = config::load_settings().context("loading settings")?;

    let cues = subtitle_writer::parse_srt(&srt_content);
    anyhow::ensure!(!cues.is_empty(), "no subtitle cues to translate");
    let texts: Vec<String> = cues.iter().map(|c| c.text.clone()).collect();

    emit_progress(&app, "translate", 0.0);
    let translate_app = app.clone();
    let translated_texts: Vec<String> = match settings.translation_engine {
        TranslationEngineChoice::Cloud => {
            let api_key = config::get_api_key(config::DEEPL_API_KEY)
                .context("reading DeepL API key")?
                .context("no DeepL API key configured — add one in Settings")?;
            let engine = CloudDeepLEngine::new(api_key);
            let target_lang = target_lang.clone();
            tauri::async_runtime::spawn_blocking(move || {
                engine.translate(
                    &texts,
                    source_lang.as_deref(),
                    &target_lang,
                    Box::new(move |frac| emit_progress(&translate_app, "translate", frac)),
                )
            })
            .await
            .context("translation task")??
        }
        TranslationEngineChoice::Local => {
            emit_progress(&app, "download_translation_model", 0.0);
            let model_app = app.clone();
            let files = translate::nllb_models::ensure_nllb_downloaded(move |frac| {
                emit_progress(&model_app, "download_translation_model", frac);
            })
            .await
            .context("downloading local translation model")?;
            emit_progress(&app, "download_translation_model", 1.0);

            let target_lang = target_lang.clone();
            tauri::async_runtime::spawn_blocking(move || -> Result<Vec<String>> {
                let engine =
                    LocalNllbEngine::load(&files.encoder, &files.decoder, &files.tokenizer)
                        .context("loading local translation model")?;
                engine.translate(
                    &texts,
                    source_lang.as_deref(),
                    &target_lang,
                    Box::new(move |frac| emit_progress(&translate_app, "translate", frac)),
                )
            })
            .await
            .context("translation task")??
        }
        TranslationEngineChoice::None => {
            anyhow::bail!("translation is disabled — enable it in Settings first");
        }
    };
    emit_progress(&app, "translate", 1.0);

    anyhow::ensure!(
        translated_texts.len() == cues.len(),
        "translation engine returned {} results for {} cues",
        translated_texts.len(),
        cues.len()
    );

    let translated_cues: Vec<Segment> = cues
        .iter()
        .zip(translated_texts)
        .map(|(cue, text)| Segment {
            start: cue.start,
            end: cue.end,
            text,
        })
        .collect();
    let translated_srt = subtitle_writer::to_srt(&translated_cues);

    let input = Path::new(&srt_path);
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("subtitles");
    let out_path = input.with_file_name(format!("{stem}.{target_lang}.srt"));
    std::fs::write(&out_path, &translated_srt).context("writing translated .srt file")?;

    Ok(TranslationOutput {
        srt_path: out_path.to_string_lossy().to_string(),
        srt_content: translated_srt,
    })
}
