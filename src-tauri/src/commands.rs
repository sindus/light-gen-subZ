use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;

use crate::models;
use crate::pipeline::{self, PipelineOutput};

const MEDIA_EXTENSIONS: &[&str] = &[
    "mp4", "mkv", "mov", "avi", "webm", "mp3", "wav", "m4a", "flac", "ogg",
];

#[tauri::command]
pub async fn pick_file(app: AppHandle) -> Option<String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .add_filter("Video / Audio", MEDIA_EXTENSIONS)
        .pick_file(move |file_path| {
            let _ = tx.send(file_path);
        });
    rx.await
        .ok()
        .flatten()
        .and_then(|f| f.into_path().ok())
        .map(|p| p.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn run_pipeline(app: AppHandle, input_path: String) -> Result<PipelineOutput, String> {
    pipeline::run(app, input_path)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_models() -> Vec<models::ModelInfo> {
    models::known_models()
}

#[tauri::command]
pub fn save_subtitle(dest_path: String, content: String) -> Result<(), String> {
    std::fs::write(&dest_path, content).map_err(|e| e.to_string())
}
