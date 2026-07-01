pub mod cloud_deepl;
pub mod languages;
pub mod local_nllb;
pub mod nllb_models;

pub use cloud_deepl::CloudDeepLEngine;
pub use local_nllb::LocalNllbEngine;

pub trait TranslationEngine {
    /// Translates a batch of texts, in order. `source_lang` is `None` for auto-detect.
    fn translate(
        &self,
        texts: &[String],
        source_lang: Option<&str>,
        target_lang: &str,
        on_progress: Box<dyn FnMut(f32) + Send>,
    ) -> anyhow::Result<Vec<String>>;
}
