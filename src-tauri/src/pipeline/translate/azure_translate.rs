use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::json;

use super::TranslationEngine;

const ENDPOINT: &str = "https://api.cognitive.microsofttranslator.com/translate";

pub struct AzureTranslateEngine {
    api_key: String,
    region: String,
}

impl AzureTranslateEngine {
    pub fn new(api_key: impl Into<String>, region: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            region: region.into(),
        }
    }
}

#[derive(Deserialize)]
struct Translation {
    text: String,
}

#[derive(Deserialize)]
struct TranslationResult {
    translations: Vec<Translation>,
}

impl TranslationEngine for AzureTranslateEngine {
    fn translate(
        &self,
        texts: &[String],
        source_lang: Option<&str>,
        target_lang: &str,
        mut on_progress: Box<dyn FnMut(f32) + Send>,
    ) -> Result<Vec<String>> {
        on_progress(0.1);

        let mut query = vec![("api-version", "3.0"), ("to", target_lang)];
        if let Some(src) = source_lang {
            query.push(("from", src));
        }

        let body: Vec<_> = texts.iter().map(|t| json!({ "Text": t })).collect();

        let client = reqwest::blocking::Client::new();
        let response = client
            .post(ENDPOINT)
            .query(&query)
            .header("Ocp-Apim-Subscription-Key", &self.api_key)
            .header("Ocp-Apim-Subscription-Region", &self.region)
            .json(&body)
            .send()
            .context("sending request to Azure Translator API")?
            .error_for_status()
            .context("Azure Translator API returned an error")?;

        on_progress(0.8);

        let parsed: Vec<TranslationResult> = response
            .json()
            .context("parsing Azure Translator API response")?;

        on_progress(1.0);

        Ok(parsed
            .into_iter()
            .filter_map(|r| r.translations.into_iter().next().map(|t| t.text))
            .collect())
    }
}
