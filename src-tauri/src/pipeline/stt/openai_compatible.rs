use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

use super::{Segment, SttEngine, Transcript};

/// Transcription via any OpenAI-Whisper-compatible `/audio/transcriptions` endpoint.
/// Covers both OpenAI itself and Groq, which expose an identical request/response shape.
pub struct OpenAiCompatibleWhisperEngine {
    endpoint: &'static str,
    model: &'static str,
    api_key: String,
}

impl OpenAiCompatibleWhisperEngine {
    pub fn groq(api_key: impl Into<String>) -> Self {
        Self {
            endpoint: "https://api.groq.com/openai/v1/audio/transcriptions",
            model: "whisper-large-v3-turbo",
            api_key: api_key.into(),
        }
    }

    pub fn openai(api_key: impl Into<String>) -> Self {
        Self {
            endpoint: "https://api.openai.com/v1/audio/transcriptions",
            model: "whisper-1",
            api_key: api_key.into(),
        }
    }
}

#[derive(Deserialize)]
struct ApiSegment {
    start: f64,
    end: f64,
    text: String,
}

#[derive(Deserialize)]
struct ApiResponse {
    language: String,
    segments: Vec<ApiSegment>,
}

impl SttEngine for OpenAiCompatibleWhisperEngine {
    fn transcribe(
        &self,
        wav_path: &Path,
        mut on_progress: Box<dyn FnMut(f32) + Send>,
    ) -> Result<Transcript> {
        on_progress(0.05);

        let file_bytes = std::fs::read(wav_path).context("reading WAV file for upload")?;
        let part = reqwest::blocking::multipart::Part::bytes(file_bytes)
            .file_name("audio.wav")
            .mime_str("audio/wav")
            .context("building multipart file part")?;
        let form = reqwest::blocking::multipart::Form::new()
            .part("file", part)
            .text("model", self.model)
            .text("response_format", "verbose_json");

        on_progress(0.2);

        let client = reqwest::blocking::Client::new();
        let response = client
            .post(self.endpoint)
            .bearer_auth(&self.api_key)
            .multipart(form)
            .send()
            .with_context(|| format!("sending request to {}", self.endpoint))?
            .error_for_status()
            .with_context(|| format!("{} returned an error", self.endpoint))?;

        on_progress(0.9);

        let parsed: ApiResponse = response.json().context("parsing API response")?;

        on_progress(1.0);

        Ok(Transcript {
            language: parsed.language,
            segments: parsed
                .segments
                .into_iter()
                .map(|s| Segment {
                    start: s.start,
                    end: s.end,
                    text: s.text.trim().to_string(),
                })
                .collect(),
        })
    }
}
