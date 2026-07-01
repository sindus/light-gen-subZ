use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};

/// Converts a video or audio file to mono 16kHz WAV via ffmpeg (expected on PATH),
/// writes it to the app's cache directory, and returns the resulting WAV path.
pub fn extract_to_wav(input_path: &Path, cache_dir: &Path) -> Result<PathBuf> {
    std::fs::create_dir_all(cache_dir).context("creating cache directory")?;

    let stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("audio");
    let out_path = cache_dir.join(format!("{stem}.wav"));

    let status = Command::new("ffmpeg")
        .arg("-y") // overwrite the output file if it already exists
        .arg("-i")
        .arg(input_path)
        .args(["-ar", "16000", "-ac", "1", "-c:a", "pcm_s16le"])
        .arg(&out_path)
        .status()
        .context("failed to launch ffmpeg — is it installed and on the PATH?")?;

    if !status.success() {
        bail!("ffmpeg failed with code {:?}", status.code());
    }

    Ok(out_path)
}
