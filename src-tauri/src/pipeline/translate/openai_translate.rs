use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::json;

use super::TranslationEngine;

const ENDPOINT: &str = "https://api.openai.com/v1/chat/completions";
const MODEL: &str = "gpt-4o-mini";

pub struct OpenAiTranslateEngine {
    api_key: String,
}

impl OpenAiTranslateEngine {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
        }
    }
}

#[derive(Deserialize)]
struct ChatMessage {
    content: String,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct TranslationsPayload {
    translations: Vec<String>,
}

impl TranslationEngine for OpenAiTranslateEngine {
    fn translate(
        &self,
        texts: &[String],
        source_lang: Option<&str>,
        target_lang: &str,
        mut on_progress: Box<dyn FnMut(f32) + Send>,
    ) -> Result<Vec<String>> {
        on_progress(0.1);

        let source_note = source_lang
            .map(|s| format!("The source language is '{s}'."))
            .unwrap_or_else(|| "Detect the source language automatically.".to_string());

        let system_prompt = format!(
            "You translate subtitle lines into language code '{target_lang}'. {source_note} \
             You will receive a JSON array of strings, each one subtitle line. Return a JSON \
             object {{\"translations\": [...]}} with EXACTLY the same number of strings, in the \
             same order, translated. Keep each translation concise and matching the tone of the \
             original. Do not merge or split lines."
        );

        let body = json!({
            "model": MODEL,
            "response_format": {"type": "json_object"},
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": serde_json::to_string(texts).context("serializing input lines")?},
            ],
        });

        let client = reqwest::blocking::Client::new();
        let response = client
            .post(ENDPOINT)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .context("sending request to OpenAI API")?
            .error_for_status()
            .context("OpenAI API returned an error")?;

        on_progress(0.8);

        let parsed: ChatResponse = response.json().context("parsing OpenAI API response")?;
        let content = parsed
            .choices
            .into_iter()
            .next()
            .context("OpenAI API returned no choices")?
            .message
            .content;
        let payload: TranslationsPayload =
            serde_json::from_str(&content).context("parsing OpenAI translation JSON payload")?;

        anyhow::ensure!(
            payload.translations.len() == texts.len(),
            "OpenAI returned {} translations for {} input lines",
            payload.translations.len(),
            texts.len()
        );

        on_progress(1.0);
        Ok(payload.translations)
    }
}
