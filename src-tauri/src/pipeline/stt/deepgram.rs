use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

use super::{Segment, SttEngine, Transcript};

const ENDPOINT: &str = "https://api.deepgram.com/v1/listen?model=nova-2&smart_format=true&punctuate=true&utterances=true&detect_language=true";

pub struct DeepgramEngine {
    api_key: String,
}

impl DeepgramEngine {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
        }
    }
}

#[derive(Deserialize)]
struct Utterance {
    start: f64,
    end: f64,
    transcript: String,
}

#[derive(Deserialize)]
struct Channel {
    detected_language: Option<String>,
}

#[derive(Deserialize)]
struct Results {
    channels: Vec<Channel>,
    utterances: Vec<Utterance>,
}

#[derive(Deserialize)]
struct DeepgramResponse {
    results: Results,
}

impl SttEngine for DeepgramEngine {
    fn transcribe(
        &self,
        wav_path: &Path,
        mut on_progress: Box<dyn FnMut(f32) + Send>,
    ) -> Result<Transcript> {
        on_progress(0.05);

        let file_bytes = std::fs::read(wav_path).context("reading WAV file for upload")?;

        on_progress(0.2);

        let client = reqwest::blocking::Client::new();
        let response = client
            .post(ENDPOINT)
            .header("Authorization", format!("Token {}", self.api_key))
            .header("Content-Type", "audio/wav")
            .body(file_bytes)
            .send()
            .context("sending request to Deepgram API")?
            .error_for_status()
            .context("Deepgram API returned an error")?;

        on_progress(0.9);

        let parsed: DeepgramResponse = response.json().context("parsing Deepgram API response")?;

        on_progress(1.0);

        let language = parsed
            .results
            .channels
            .first()
            .and_then(|c| c.detected_language.clone())
            .unwrap_or_else(|| "unknown".to_string());

        Ok(Transcript {
            language,
            segments: parsed
                .results
                .utterances
                .into_iter()
                .map(|u| Segment {
                    start: u.start,
                    end: u.end,
                    text: u.transcript.trim().to_string(),
                })
                .collect(),
        })
    }
}
