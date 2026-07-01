import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { save } from "@tauri-apps/plugin-dialog";
import "./App.css";

type Stage =
  | "download_model"
  | "extract_audio"
  | "transcribe"
  | "write_subtitles"
  | "download_translation_model"
  | "translate";

type PipelineProgress = {
  stage: Stage;
  fraction: number;
};

type PipelineOutput = {
  srt_path: string;
  srt_content: string;
  language: string;
};

type TranslationOutput = {
  srt_path: string;
  srt_content: string;
};

type Cue = {
  index: number;
  start: string;
  end: string;
  text: string;
};

type SttEngineChoice = "local" | "cloud";
type TranslationEngineChoice = "none" | "local" | "cloud";

type Settings = {
  stt_engine: SttEngineChoice;
  translation_engine: TranslationEngineChoice;
};

type LanguageInfo = {
  code: string;
  flores_code: string;
  name: string;
};

const TRANSCRIBE_STAGES: { key: Stage; label: string }[] = [
  { key: "download_model", label: "Model" },
  { key: "extract_audio", label: "Audio" },
  { key: "transcribe", label: "Transcript" },
  { key: "write_subtitles", label: "Subtitles" },
];

function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

function parseSrt(content: string): Cue[] {
  return content
    .trim()
    .split(/\r?\n\r?\n/)
    .map((block) => {
      const lines = block.split(/\r?\n/);
      const [index, timing, ...textLines] = lines;
      const [start, end] = (timing ?? "").split(" --> ");
      return {
        index: Number(index) || 0,
        start: (start ?? "").trim(),
        end: (end ?? "").trim(),
        text: textLines.join(" ").trim(),
      };
    })
    .filter((cue) => cue.text.length > 0);
}

function CueList({ cues }: { cues: Cue[] }) {
  return (
    <ol className="cue-list">
      {cues.map((cue) => (
        <li className="cue" key={cue.index}>
          <span className="cue-index">{cue.index}</span>
          <div className="cue-body">
            <span className="cue-time">
              {cue.start} <span className="cue-arrow">→</span> {cue.end}
            </span>
            <p className="cue-text">{cue.text}</p>
          </div>
        </li>
      ))}
    </ol>
  );
}

function App() {
  const [inputPath, setInputPath] = useState<string | null>(null);
  const [progress, setProgress] = useState<PipelineProgress | null>(null);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<PipelineOutput | null>(null);

  const [settings, setSettings] = useState<Settings | null>(null);
  const [languages, setLanguages] = useState<LanguageInfo[]>([]);
  const [showSettings, setShowSettings] = useState(false);
  const [groqKeyInput, setGroqKeyInput] = useState("");
  const [deeplKeyInput, setDeeplKeyInput] = useState("");
  const [hasGroqKey, setHasGroqKey] = useState(false);
  const [hasDeeplKey, setHasDeeplKey] = useState(false);

  const [sourceLang, setSourceLang] = useState("auto");
  const [targetLang, setTargetLang] = useState("en");
  const [translating, setTranslating] = useState(false);
  const [translationError, setTranslationError] = useState<string | null>(null);
  const [translation, setTranslation] = useState<TranslationOutput | null>(null);

  useEffect(() => {
    const unlisten = listen<PipelineProgress>("pipeline-progress", (event) => {
      setProgress(event.payload);
    });
    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  useEffect(() => {
    invoke<Settings>("get_settings").then(setSettings);
    invoke<LanguageInfo[]>("list_languages").then(setLanguages);
    invoke<boolean>("has_api_key", { keyName: "groq_api_key" }).then(setHasGroqKey);
    invoke<boolean>("has_api_key", { keyName: "deepl_api_key" }).then(setHasDeeplKey);
  }, []);

  const cues = useMemo(() => (result ? parseSrt(result.srt_content) : []), [result]);
  const translatedCues = useMemo(
    () => (translation ? parseSrt(translation.srt_content) : []),
    [translation],
  );

  const transcribeStages = useMemo(
    () =>
      settings?.stt_engine === "cloud"
        ? TRANSCRIBE_STAGES.filter((s) => s.key !== "download_model")
        : TRANSCRIBE_STAGES,
    [settings],
  );
  const stageIndex = progress ? transcribeStages.findIndex((s) => s.key === progress.stage) : -1;
  const isTranslateStage = progress?.stage === "translate" || progress?.stage === "download_translation_model";

  async function pickFile() {
    const path = await invoke<string | null>("pick_file");
    if (path) {
      setInputPath(path);
      setResult(null);
      setError(null);
      setTranslation(null);
      setTranslationError(null);
    }
  }

  async function generate() {
    if (!inputPath) return;
    setRunning(true);
    setError(null);
    setResult(null);
    setTranslation(null);
    setTranslationError(null);
    setProgress(null);
    try {
      const output = await invoke<PipelineOutput>("run_pipeline", { inputPath });
      setResult(output);
      if (output.language) {
        setSourceLang(output.language.slice(0, 2).toLowerCase());
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setRunning(false);
    }
  }

  async function saveAs() {
    if (!result) return;
    const dest = await save({
      defaultPath: result.srt_path,
      filters: [{ name: "SRT subtitles", extensions: ["srt"] }],
    });
    if (dest) {
      await invoke("save_subtitle", { destPath: dest, content: result.srt_content });
    }
  }

  async function saveTranslationAs() {
    if (!translation) return;
    const dest = await save({
      defaultPath: translation.srt_path,
      filters: [{ name: "SRT subtitles", extensions: ["srt"] }],
    });
    if (dest) {
      await invoke("save_subtitle", { destPath: dest, content: translation.srt_content });
    }
  }

  async function runTranslation() {
    if (!result) return;
    setTranslating(true);
    setTranslationError(null);
    setTranslation(null);
    setProgress(null);
    try {
      const output = await invoke<TranslationOutput>("translate_subtitles", {
        srtPath: result.srt_path,
        srtContent: result.srt_content,
        sourceLang: sourceLang === "auto" ? null : sourceLang,
        targetLang,
      });
      setTranslation(output);
    } catch (e) {
      setTranslationError(String(e));
    } finally {
      setTranslating(false);
    }
  }

  async function updateSettings(patch: Partial<Settings>) {
    if (!settings) return;
    const next = { ...settings, ...patch };
    setSettings(next);
    await invoke("set_settings", { settings: next });
  }

  async function saveGroqKey() {
    if (!groqKeyInput.trim()) return;
    await invoke("set_api_key", { keyName: "groq_api_key", value: groqKeyInput.trim() });
    setHasGroqKey(true);
    setGroqKeyInput("");
  }

  async function saveDeeplKey() {
    if (!deeplKeyInput.trim()) return;
    await invoke("set_api_key", { keyName: "deepl_api_key", value: deeplKeyInput.trim() });
    setHasDeeplKey(true);
    setDeeplKeyInput("");
  }

  const status: "idle" | "running" | "done" | "error" = error
    ? "error"
    : running
      ? "running"
      : result
        ? "done"
        : "idle";

  const statusLabel: Record<typeof status, string> = {
    idle: "Idle",
    running: "Running",
    done: "Done",
    error: "Error",
  };

  return (
    <main className="app">
      <header className="app-header">
        <span className={`status-dot status-dot--${status}`} aria-hidden="true" />
        <span className="app-mark">Subtitles</span>
        <span className="app-status">{statusLabel[status]}</span>
        <button
          className="btn-icon"
          onClick={() => setShowSettings((v) => !v)}
          type="button"
          aria-label="Settings"
        >
          <svg viewBox="0 0 24 24" fill="none" aria-hidden="true">
            <path
              d="M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6Z"
              stroke="currentColor"
              strokeWidth="1.6"
            />
            <path
              d="M19.4 13a7.4 7.4 0 0 0 0-2l1.9-1.5-2-3.4-2.2.9a7.6 7.6 0 0 0-1.7-1L15 3.6h-6l-.4 2.4a7.6 7.6 0 0 0-1.7 1l-2.2-.9-2 3.4L4.6 11a7.4 7.4 0 0 0 0 2l-1.9 1.5 2 3.4 2.2-.9a7.6 7.6 0 0 0 1.7 1l.4 2.4h6l.4-2.4a7.6 7.6 0 0 0 1.7-1l2.2.9 2-3.4L19.4 13Z"
              stroke="currentColor"
              strokeWidth="1.4"
              strokeLinejoin="round"
            />
          </svg>
        </button>
      </header>

      {showSettings && settings && (
        <div className="panel settings-panel">
          <div className="settings-row">
            <span className="settings-label">Transcription engine</span>
            <div className="segmented">
              <button
                className={settings.stt_engine === "local" ? "segmented-active" : ""}
                onClick={() => updateSettings({ stt_engine: "local" })}
                type="button"
              >
                Local (offline)
              </button>
              <button
                className={settings.stt_engine === "cloud" ? "segmented-active" : ""}
                onClick={() => updateSettings({ stt_engine: "cloud" })}
                type="button"
              >
                Cloud (Groq)
              </button>
            </div>
          </div>
          {settings.stt_engine === "cloud" && (
            <div className="settings-row">
              <span className="settings-label">Groq API key {hasGroqKey && "(saved)"}</span>
              <div className="cmd-row">
                <input
                  className="settings-input"
                  type="password"
                  placeholder="gsk_..."
                  value={groqKeyInput}
                  onChange={(e) => setGroqKeyInput(e.currentTarget.value)}
                />
                <button className="btn btn-ghost" onClick={saveGroqKey} type="button">
                  Save
                </button>
              </div>
            </div>
          )}

          <div className="settings-row">
            <span className="settings-label">Translation</span>
            <div className="segmented">
              <button
                className={settings.translation_engine === "none" ? "segmented-active" : ""}
                onClick={() => updateSettings({ translation_engine: "none" })}
                type="button"
              >
                Off
              </button>
              <button
                className={settings.translation_engine === "local" ? "segmented-active" : ""}
                onClick={() => updateSettings({ translation_engine: "local" })}
                type="button"
              >
                Local (offline)
              </button>
              <button
                className={settings.translation_engine === "cloud" ? "segmented-active" : ""}
                onClick={() => updateSettings({ translation_engine: "cloud" })}
                type="button"
              >
                Cloud (DeepL)
              </button>
            </div>
          </div>
          {settings.translation_engine === "cloud" && (
            <div className="settings-row">
              <span className="settings-label">DeepL API key {hasDeeplKey && "(saved)"}</span>
              <div className="cmd-row">
                <input
                  className="settings-input"
                  type="password"
                  placeholder="xxxxxxxx-xxxx-...:fx"
                  value={deeplKeyInput}
                  onChange={(e) => setDeeplKeyInput(e.currentTarget.value)}
                />
                <button className="btn btn-ghost" onClick={saveDeeplKey} type="button">
                  Save
                </button>
              </div>
            </div>
          )}
        </div>
      )}

      <div className="app-body">
        {!inputPath && (
          <button className="dropzone" onClick={pickFile} type="button">
            <svg className="dropzone-icon" viewBox="0 0 48 48" fill="none" aria-hidden="true">
              <path
                d="M24 6v22m0 0-8-8m8 8 8-8M10 34v4a4 4 0 0 0 4 4h20a4 4 0 0 0 4-4v-4"
                stroke="currentColor"
                strokeWidth="2.4"
                strokeLinecap="round"
                strokeLinejoin="round"
              />
            </svg>
            <span className="dropzone-title">Choose a video or audio file</span>
            <span className="dropzone-hint">mp4 · mov · mkv · mp3 · wav · m4a · flac</span>
          </button>
        )}

        {inputPath && (
          <div className="file-chip">
            <svg className="file-chip-icon" viewBox="0 0 24 24" fill="none" aria-hidden="true">
              <path
                d="M4 4.5A1.5 1.5 0 0 1 5.5 3H13l5 5v12.5A1.5 1.5 0 0 1 16.5 22h-11A1.5 1.5 0 0 1 4 20.5v-16Z"
                stroke="currentColor"
                strokeWidth="1.6"
                strokeLinejoin="round"
              />
              <path d="M13 3v5h5" stroke="currentColor" strokeWidth="1.6" strokeLinejoin="round" />
            </svg>
            <span className="file-chip-name" title={inputPath}>
              {fileName(inputPath)}
            </span>
            <button className="btn-link" onClick={pickFile} disabled={running} type="button">
              Change
            </button>
          </div>
        )}

        {inputPath && !result && (
          <button className="btn btn-primary" onClick={generate} disabled={running} type="button">
            {running ? "Generating…" : "Generate subtitles"}
          </button>
        )}

        {running && !isTranslateStage && (
          <div className="panel progress-panel">
            <ol className="stepper">
              {transcribeStages.map((stage, i) => (
                <li
                  key={stage.key}
                  className={
                    "step" +
                    (i < stageIndex ? " step--done" : i === stageIndex ? " step--active" : "")
                  }
                >
                  <span className="step-marker">{i < stageIndex ? "✓" : i + 1}</span>
                  <span className="step-label">{stage.label}</span>
                </li>
              ))}
            </ol>
            <div className="progress-track">
              <div
                className="progress-fill"
                style={{ width: `${Math.round((progress?.fraction ?? 0) * 100)}%` }}
              />
            </div>
          </div>
        )}

        {error && (
          <div className="panel error-panel">
            <strong>Generation failed.</strong>
            <span>{error}</span>
          </div>
        )}

        {result && (
          <div className="panel result-panel">
            <div className="result-summary">
              <span className="pill">{result.language.toUpperCase()}</span>
              <span className="result-meta">
                {cues.length} line{cues.length > 1 ? "s" : ""}
              </span>
              <span className="result-path" title={result.srt_path}>
                {result.srt_path}
              </span>
              <button className="btn btn-ghost" onClick={saveAs} type="button">
                Save as…
              </button>
            </div>

            <CueList cues={cues} />
          </div>
        )}

        {result && settings?.translation_engine !== "none" && (
          <div className="panel translate-panel">
            <div className="translate-controls">
              <select
                className="settings-select"
                value={sourceLang}
                onChange={(e) => setSourceLang(e.currentTarget.value)}
              >
                <option value="auto">
                  Auto{settings?.translation_engine === "cloud" ? " (detect)" : " (use detected)"}
                </option>
                {languages.map((l) => (
                  <option key={l.code} value={l.code}>
                    {l.name}
                  </option>
                ))}
              </select>
              <span className="translate-arrow">→</span>
              <select
                className="settings-select"
                value={targetLang}
                onChange={(e) => setTargetLang(e.currentTarget.value)}
              >
                {languages.map((l) => (
                  <option key={l.code} value={l.code}>
                    {l.name}
                  </option>
                ))}
              </select>
              <button
                className="btn btn-primary translate-btn"
                onClick={runTranslation}
                disabled={translating}
                type="button"
              >
                {translating ? "Translating…" : "Translate"}
              </button>
            </div>

            {translating && isTranslateStage && (
              <div className="progress-track">
                <div
                  className="progress-fill"
                  style={{ width: `${Math.round((progress?.fraction ?? 0) * 100)}%` }}
                />
              </div>
            )}

            {translationError && (
              <p className="error-inline">Translation failed: {translationError}</p>
            )}

            {translation && (
              <>
                <div className="result-summary">
                  <span className="pill">{targetLang.toUpperCase()}</span>
                  <span className="result-path" title={translation.srt_path}>
                    {translation.srt_path}
                  </span>
                  <button className="btn btn-ghost" onClick={saveTranslationAs} type="button">
                    Save as…
                  </button>
                </div>
                <CueList cues={translatedCues} />
              </>
            )}
          </div>
        )}
      </div>
    </main>
  );
}

export default App;
