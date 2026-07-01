import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { save } from "@tauri-apps/plugin-dialog";
import "./App.css";

type Stage = "download_model" | "extract_audio" | "transcribe" | "write_subtitles";

type PipelineProgress = {
  stage: Stage;
  fraction: number;
};

type PipelineOutput = {
  srt_path: string;
  srt_content: string;
  language: string;
};

type Cue = {
  index: number;
  start: string;
  end: string;
  text: string;
};

const STAGES: { key: Stage; label: string }[] = [
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

function App() {
  const [inputPath, setInputPath] = useState<string | null>(null);
  const [progress, setProgress] = useState<PipelineProgress | null>(null);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<PipelineOutput | null>(null);

  useEffect(() => {
    const unlisten = listen<PipelineProgress>("pipeline-progress", (event) => {
      setProgress(event.payload);
    });
    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  const cues = useMemo(() => (result ? parseSrt(result.srt_content) : []), [result]);
  const stageIndex = progress ? STAGES.findIndex((s) => s.key === progress.stage) : -1;

  async function pickFile() {
    const path = await invoke<string | null>("pick_file");
    if (path) {
      setInputPath(path);
      setResult(null);
      setError(null);
    }
  }

  async function generate() {
    if (!inputPath) return;
    setRunning(true);
    setError(null);
    setResult(null);
    setProgress(null);
    try {
      const output = await invoke<PipelineOutput>("run_pipeline", { inputPath });
      setResult(output);
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
      </header>

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

        {running && (
          <div className="panel progress-panel">
            <ol className="stepper">
              {STAGES.map((stage, i) => (
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
          </div>
        )}
      </div>
    </main>
  );
}

export default App;
