use std::path::Path;
use std::sync::Mutex;

use anyhow::{Context, Result};
use ort::session::Session;
use ort::value::Tensor;
use tokenizers::Tokenizer;

use super::languages::flores_code_for;
use super::TranslationEngine;

const EOS_TOKEN_ID: i64 = 2;
const MAX_NEW_TOKENS: usize = 200;

/// Local translation via an NLLB-200 model exported to ONNX (encoder/decoder, no KV cache —
/// each decoding step recomputes the full decoder pass, which is simple and fast enough for
/// short subtitle lines).
pub struct LocalNllbEngine {
    encoder: Mutex<Session>,
    decoder: Mutex<Session>,
    tokenizer: Tokenizer,
}

impl LocalNllbEngine {
    pub fn load(encoder_path: &Path, decoder_path: &Path, tokenizer_path: &Path) -> Result<Self> {
        let encoder = Session::builder()
            .context("creating encoder session builder")?
            .commit_from_file(encoder_path)
            .context("loading encoder model")?;
        let decoder = Session::builder()
            .context("creating decoder session builder")?
            .commit_from_file(decoder_path)
            .context("loading decoder model")?;
        let tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|e| anyhow::anyhow!("loading tokenizer: {e}"))?;
        Ok(Self {
            encoder: Mutex::new(encoder),
            decoder: Mutex::new(decoder),
            tokenizer,
        })
    }

    fn translate_one(&self, text: &str, src_flores: &str, tgt_flores: &str) -> Result<String> {
        let src_lang_id = self
            .tokenizer
            .token_to_id(src_flores)
            .with_context(|| format!("unknown source language code: {src_flores}"))?
            as i64;
        let tgt_lang_id = self
            .tokenizer
            .token_to_id(tgt_flores)
            .with_context(|| format!("unknown target language code: {tgt_flores}"))?
            as i64;

        // The tokenizer's built-in post-processor bakes in a fixed source language, so we
        // tokenize without special tokens and add the source/eos tokens ourselves.
        let encoding = self
            .tokenizer
            .encode(text, false)
            .map_err(|e| anyhow::anyhow!("tokenizing text: {e}"))?;

        let mut input_ids: Vec<i64> = vec![src_lang_id];
        input_ids.extend(encoding.get_ids().iter().map(|&id| id as i64));
        input_ids.push(EOS_TOKEN_ID);
        let seq_len = input_ids.len();
        let attention_mask: Vec<i64> = vec![1; seq_len];

        let (enc_shape, encoder_hidden_states) = {
            let mut encoder = self.encoder.lock().unwrap();
            let input_ids_tensor =
                Tensor::from_array((vec![1i64, seq_len as i64], input_ids.clone()))?;
            let attention_mask_tensor =
                Tensor::from_array((vec![1i64, seq_len as i64], attention_mask.clone()))?;
            let outputs = encoder.run(ort::inputs![
                "input_ids" => input_ids_tensor,
                "attention_mask" => attention_mask_tensor,
            ])?;
            let (shape, data) = outputs["last_hidden_state"].try_extract_tensor::<f32>()?;
            (shape.iter().copied().collect::<Vec<i64>>(), data.to_vec())
        };

        // Greedy decode: [eos, target_lang] primes the decoder (matches NLLB's
        // decoder_start_token_id + forced_bos_token_id generation convention).
        let mut decoder_ids: Vec<i64> = vec![EOS_TOKEN_ID, tgt_lang_id];
        let mut decoder = self.decoder.lock().unwrap();
        for _ in 0..MAX_NEW_TOKENS {
            let dec_len = decoder_ids.len();
            let decoder_input_tensor =
                Tensor::from_array((vec![1i64, dec_len as i64], decoder_ids.clone()))?;
            let enc_hidden_tensor =
                Tensor::from_array((enc_shape.clone(), encoder_hidden_states.clone()))?;
            let enc_mask_tensor =
                Tensor::from_array((vec![1i64, seq_len as i64], attention_mask.clone()))?;

            let outputs = decoder.run(ort::inputs![
                "encoder_attention_mask" => enc_mask_tensor,
                "input_ids" => decoder_input_tensor,
                "encoder_hidden_states" => enc_hidden_tensor,
            ])?;
            let (logits_shape, logits) = outputs["logits"].try_extract_tensor::<f32>()?;
            let vocab_size = logits_shape[2] as usize;
            let last_pos_start = (dec_len - 1) * vocab_size;
            let last_logits = &logits[last_pos_start..last_pos_start + vocab_size];
            let next_id = last_logits
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                .map(|(i, _)| i as i64)
                .context("decoder produced no logits")?;

            if next_id == EOS_TOKEN_ID {
                break;
            }
            decoder_ids.push(next_id);
        }

        let generated: Vec<u32> = decoder_ids[2..].iter().map(|&id| id as u32).collect();
        self.tokenizer
            .decode(&generated, true)
            .map(|s| s.trim().to_string())
            .map_err(|e| anyhow::anyhow!("decoding generated tokens: {e}"))
    }
}

impl TranslationEngine for LocalNllbEngine {
    /// `source_lang` must be a known language code — unlike the cloud engine, NLLB has no
    /// auto-detection; callers should pass the language already detected during transcription.
    fn translate(
        &self,
        texts: &[String],
        source_lang: Option<&str>,
        target_lang: &str,
        mut on_progress: Box<dyn FnMut(f32) + Send>,
    ) -> Result<Vec<String>> {
        let tgt_flores = flores_code_for(target_lang)
            .with_context(|| format!("unsupported target language: {target_lang}"))?;
        let src_flores = source_lang
            .context("local translation requires a known source language")
            .and_then(|code| {
                flores_code_for(code)
                    .with_context(|| format!("unsupported source language: {code}"))
            })?;

        let total = texts.len().max(1);
        let mut results = Vec::with_capacity(texts.len());
        for (i, text) in texts.iter().enumerate() {
            results.push(self.translate_one(text, src_flores, tgt_flores)?);
            on_progress((i + 1) as f32 / total as f32);
        }
        Ok(results)
    }
}
