use std::path::Path;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use serde_json::json;

use super::{Segment, SttEngine, Transcript};

const BASE_URL: &str = "https://api.assemblyai.com/v2";
const POLL_INTERVAL: Duration = Duration::from_secs(3);
const MAX_POLLS: u32 = 200; // ~10 minutes

pub struct AssemblyAiEngine {
    api_key: String,
}

impl AssemblyAiEngine {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
        }
    }
}

#[derive(Deserialize)]
struct UploadResponse {
    upload_url: String,
}

#[derive(Deserialize)]
struct TranscriptCreated {
    id: String,
}

#[derive(Deserialize)]
struct TranscriptStatus {
    status: String,
    language_code: Option<String>,
    error: Option<String>,
}

#[derive(Deserialize)]
struct SentencesResponse {
    sentences: Vec<Sentence>,
}

#[derive(Deserialize)]
struct Sentence {
    text: String,
    start: i64,
    end: i64,
}

impl SttEngine for AssemblyAiEngine {
    fn transcribe(
        &self,
        wav_path: &Path,
        mut on_progress: Box<dyn FnMut(f32) + Send>,
    ) -> Result<Transcript> {
        let client = reqwest::blocking::Client::new();

        on_progress(0.05);
        let file_bytes = std::fs::read(wav_path).context("reading WAV file for upload")?;
        let upload: UploadResponse = client
            .post(format!("{BASE_URL}/upload"))
            .header("authorization", &self.api_key)
            .body(file_bytes)
            .send()
            .context("uploading audio to AssemblyAI")?
            .error_for_status()
            .context("AssemblyAI upload returned an error")?
            .json()
            .context("parsing AssemblyAI upload response")?;

        on_progress(0.2);
        let created: TranscriptCreated = client
            .post(format!("{BASE_URL}/transcript"))
            .header("authorization", &self.api_key)
            .json(&json!({
                "audio_url": upload.upload_url,
                "language_detection": true,
            }))
            .send()
            .context("creating AssemblyAI transcript job")?
            .error_for_status()
            .context("AssemblyAI transcript creation returned an error")?
            .json()
            .context("parsing AssemblyAI transcript creation response")?;

        let status_url = format!("{BASE_URL}/transcript/{}", created.id);
        let mut language = "unknown".to_string();
        let mut completed = false;
        for i in 0..MAX_POLLS {
            sleep(POLL_INTERVAL);
            let status: TranscriptStatus = client
                .get(&status_url)
                .header("authorization", &self.api_key)
                .send()
                .context("polling AssemblyAI transcript status")?
                .error_for_status()
                .context("AssemblyAI status check returned an error")?
                .json()
                .context("parsing AssemblyAI status response")?;

            on_progress(0.2 + 0.6 * (i as f32 / MAX_POLLS as f32).min(1.0));

            match status.status.as_str() {
                "completed" => {
                    language = status.language_code.unwrap_or(language);
                    completed = true;
                    break;
                }
                "error" => {
                    bail!(
                        "AssemblyAI transcription failed: {}",
                        status.error.unwrap_or_else(|| "unknown error".to_string())
                    );
                }
                _ => continue,
            }
        }
        anyhow::ensure!(completed, "AssemblyAI transcription timed out");

        on_progress(0.85);
        let sentences: SentencesResponse = client
            .get(format!("{status_url}/sentences"))
            .header("authorization", &self.api_key)
            .send()
            .context("fetching AssemblyAI sentences")?
            .error_for_status()
            .context("AssemblyAI sentences endpoint returned an error")?
            .json()
            .context("parsing AssemblyAI sentences response")?;

        on_progress(1.0);

        Ok(Transcript {
            language,
            segments: sentences
                .sentences
                .into_iter()
                .map(|s| Segment {
                    start: s.start as f64 / 1000.0,
                    end: s.end as f64 / 1000.0,
                    text: s.text.trim().to_string(),
                })
                .collect(),
        })
    }
}
