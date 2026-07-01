use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

use super::{Segment, SttEngine, Transcript};

const GROQ_ENDPOINT: &str = "https://api.groq.com/openai/v1/audio/transcriptions";
const GROQ_MODEL: &str = "whisper-large-v3-turbo";

pub struct CloudWhisperEngine {
    api_key: String,
}

impl CloudWhisperEngine {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
        }
    }
}

#[derive(Deserialize)]
struct GroqSegment {
    start: f64,
    end: f64,
    text: String,
}

#[derive(Deserialize)]
struct GroqResponse {
    language: String,
    segments: Vec<GroqSegment>,
}

impl SttEngine for CloudWhisperEngine {
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
            .text("model", GROQ_MODEL)
            .text("response_format", "verbose_json");

        on_progress(0.2);

        let client = reqwest::blocking::Client::new();
        let response = client
            .post(GROQ_ENDPOINT)
            .bearer_auth(&self.api_key)
            .multipart(form)
            .send()
            .context("sending request to Groq API")?
            .error_for_status()
            .context("Groq API returned an error")?;

        on_progress(0.9);

        let parsed: GroqResponse = response.json().context("parsing Groq API response")?;

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
