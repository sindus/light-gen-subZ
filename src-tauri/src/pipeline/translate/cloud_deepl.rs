use anyhow::{Context, Result};
use serde::Deserialize;

use super::TranslationEngine;

#[derive(Deserialize)]
struct DeepLTranslation {
    text: String,
}

#[derive(Deserialize)]
struct DeepLResponse {
    translations: Vec<DeepLTranslation>,
}

pub struct CloudDeepLEngine {
    api_key: String,
}

impl CloudDeepLEngine {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
        }
    }

    /// DeepL uses separate hosts for free-tier and paid keys (free keys end in ":fx").
    fn endpoint(&self) -> &'static str {
        if self.api_key.ends_with(":fx") {
            "https://api-free.deepl.com/v2/translate"
        } else {
            "https://api.deepl.com/v2/translate"
        }
    }
}

/// DeepL requires a region variant for some target languages (plain source codes work fine).
fn deepl_target_lang(code: &str) -> String {
    match code.to_ascii_lowercase().as_str() {
        "en" => "EN-US".to_string(),
        "pt" => "PT-PT".to_string(),
        other => other.to_ascii_uppercase(),
    }
}

impl TranslationEngine for CloudDeepLEngine {
    fn translate(
        &self,
        texts: &[String],
        source_lang: Option<&str>,
        target_lang: &str,
        mut on_progress: Box<dyn FnMut(f32) + Send>,
    ) -> Result<Vec<String>> {
        on_progress(0.1);

        let target_lang = deepl_target_lang(target_lang);
        let source_lang = source_lang.map(|s| s.to_ascii_uppercase());

        let mut params: Vec<(&str, &str)> = texts.iter().map(|t| ("text", t.as_str())).collect();
        params.push(("target_lang", &target_lang));
        if let Some(src) = &source_lang {
            params.push(("source_lang", src));
        }

        let client = reqwest::blocking::Client::new();
        let response = client
            .post(self.endpoint())
            .header("Authorization", format!("DeepL-Auth-Key {}", self.api_key))
            .form(&params)
            .send()
            .context("sending request to DeepL API")?
            .error_for_status()
            .context("DeepL API returned an error")?;

        on_progress(0.8);

        let parsed: DeepLResponse = response.json().context("parsing DeepL API response")?;

        on_progress(1.0);

        Ok(parsed.translations.into_iter().map(|t| t.text).collect())
    }
}
