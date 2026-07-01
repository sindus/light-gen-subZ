use std::path::PathBuf;

use anyhow::{Context, Result};
use keyring::Entry;
use serde::{Deserialize, Serialize};

const SERVICE_NAME: &str = "light-gen-subZ";

pub const GROQ_API_KEY: &str = "groq_api_key";
pub const DEEPL_API_KEY: &str = "deepl_api_key";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SttEngineChoice {
    #[default]
    Local,
    Cloud,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranslationEngineChoice {
    #[default]
    None,
    Local,
    Cloud,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Settings {
    pub stt_engine: SttEngineChoice,
    pub translation_engine: TranslationEngineChoice,
}

fn settings_path() -> Result<PathBuf> {
    let dir = dirs::config_dir()
        .context("could not determine the user's config directory")?
        .join("light-gen-subZ");
    std::fs::create_dir_all(&dir).context("creating config directory")?;
    Ok(dir.join("settings.json"))
}

pub fn load_settings() -> Result<Settings> {
    let path = settings_path()?;
    if !path.exists() {
        return Ok(Settings::default());
    }
    let raw = std::fs::read_to_string(&path).context("reading settings file")?;
    serde_json::from_str(&raw).context("parsing settings file")
}

pub fn save_settings(settings: &Settings) -> Result<()> {
    let path = settings_path()?;
    let raw = serde_json::to_string_pretty(settings).context("serializing settings")?;
    std::fs::write(path, raw).context("writing settings file")
}

fn keyring_entry(key_name: &str) -> Result<Entry> {
    Entry::new(SERVICE_NAME, key_name).context("creating keyring entry")
}

pub fn set_api_key(key_name: &str, value: &str) -> Result<()> {
    keyring_entry(key_name)?
        .set_password(value)
        .context("storing API key in keyring")
}

pub fn get_api_key(key_name: &str) -> Result<Option<String>> {
    match keyring_entry(key_name)?.get_password() {
        Ok(password) => Ok(Some(password)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e).context("reading API key from keyring"),
    }
}

pub fn has_api_key(key_name: &str) -> bool {
    get_api_key(key_name).ok().flatten().is_some()
}
