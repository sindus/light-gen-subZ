use std::path::Path;

use anyhow::{Context, Result};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use super::{Segment, SttEngine, Transcript};

pub struct LocalWhisperEngine {
    model_path: std::path::PathBuf,
}

impl LocalWhisperEngine {
    pub fn new(model_path: impl Into<std::path::PathBuf>) -> Self {
        Self {
            model_path: model_path.into(),
        }
    }
}

impl SttEngine for LocalWhisperEngine {
    fn transcribe(
        &self,
        wav_path: &Path,
        mut on_progress: Box<dyn FnMut(f32) + Send>,
    ) -> Result<Transcript> {
        let samples = read_wav_mono_f32(wav_path)?;

        let model_path = self
            .model_path
            .to_str()
            .context("model path is not valid UTF-8")?;
        let ctx = WhisperContext::new_with_params(model_path, WhisperContextParameters::default())
            .context("failed to load whisper model")?;
        let mut state = ctx
            .create_state()
            .context("failed to create whisper state")?;

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(None); // auto-detect language
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_progress_callback_safe(move |percent: i32| {
            on_progress(percent as f32 / 100.0);
        });

        state
            .full(params, &samples)
            .context("whisper transcription failed")?;

        let num_segments = state.full_n_segments().context("reading segment count")?;
        let mut segments = Vec::with_capacity(num_segments as usize);
        for i in 0..num_segments {
            let text = state
                .full_get_segment_text(i)
                .context("reading segment text")?;
            let t0 = state.full_get_segment_t0(i).context("reading t0")?;
            let t1 = state.full_get_segment_t1(i).context("reading t1")?;
            segments.push(Segment {
                start: t0 as f64 / 100.0,
                end: t1 as f64 / 100.0,
                text: text.trim().to_string(),
            });
        }

        let language = state
            .full_lang_id_from_state()
            .ok()
            .and_then(|id| whisper_rs::get_lang_str(id))
            .unwrap_or("unknown")
            .to_string();

        Ok(Transcript { language, segments })
    }
}

/// Reads a 16-bit mono WAV (produced by ffmpeg) and returns f32 samples normalized to [-1, 1].
fn read_wav_mono_f32(path: &Path) -> Result<Vec<f32>> {
    let mut reader = hound::WavReader::open(path).context("opening WAV file")?;
    let spec = reader.spec();
    anyhow::ensure!(
        spec.channels == 1 && spec.sample_rate == 16000,
        "WAV must be mono 16kHz (got: {} channels, {}Hz)",
        spec.channels,
        spec.sample_rate
    );

    let samples: Result<Vec<f32>, _> = match spec.sample_format {
        hound::SampleFormat::Int => reader
            .samples::<i16>()
            .map(|s| s.map(|v| v as f32 / i16::MAX as f32))
            .collect(),
        hound::SampleFormat::Float => reader.samples::<f32>().collect(),
    };
    samples.context("reading WAV samples")
}
