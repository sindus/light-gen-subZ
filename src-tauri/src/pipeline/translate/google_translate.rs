use anyhow::Context;
use anyhow::Result;
use serde::Deserialize;
use serde_json::json;

use super::TranslationEngine;

const ENDPOINT: &str = "https://translation.googleapis.com/language/translate/v2";

pub struct GoogleTranslateEngine {
    api_key: String,
}

impl GoogleTranslateEngine {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
        }
    }
}

#[derive(Deserialize)]
struct TranslationEntry {
    #[serde(rename = "translatedText")]
    translated_text: String,
}

#[derive(Deserialize)]
struct TranslateData {
    translations: Vec<TranslationEntry>,
}

#[derive(Deserialize)]
struct GoogleResponse {
    data: TranslateData,
}

impl TranslationEngine for GoogleTranslateEngine {
    fn translate(
        &self,
        texts: &[String],
        source_lang: Option<&str>,
        target_lang: &str,
        mut on_progress: Box<dyn FnMut(f32) + Send>,
    ) -> Result<Vec<String>> {
        on_progress(0.1);

        let mut body = json!({
            "q": texts,
            "target": target_lang,
            "format": "text",
        });
        if let Some(src) = source_lang {
            body["source"] = json!(src);
        }

        let client = reqwest::blocking::Client::new();
        let response = client
            .post(ENDPOINT)
            .query(&[("key", &self.api_key)])
            .json(&body)
            .send()
            .context("sending request to Google Translate API")?
            .error_for_status()
            .context("Google Translate API returned an error")?;

        on_progress(0.8);

        let parsed: GoogleResponse = response
            .json()
            .context("parsing Google Translate API response")?;

        on_progress(1.0);

        Ok(parsed
            .data
            .translations
            .into_iter()
            .map(|t| t.translated_text)
            .collect())
    }
}
