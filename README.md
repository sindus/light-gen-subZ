# light-gen-subZ

Turn a video or audio file into a subtitle track. Pick a file, generate, get
an `.srt` with accurate timestamps — fully offline, powered by a local
[whisper.cpp](https://github.com/ggerganov/whisper.cpp) model.

- Local speech-to-text, no account, no upload of your media anywhere.
- Optional cloud transcription (Groq's Whisper API) for speed, if you'd rather not run locally.
- Multilingual, auto-detected.
- Optional subtitle translation, either fully offline (local NLLB-200 model)
  or via the DeepL API.
- Standard `.srt` output, ready to drop into any video editor.
- Native desktop app (Tauri + Rust), small and fast.

## Install

### Linux (.deb / AppImage)

```sh
curl -fsSL https://raw.githubusercontent.com/sindus/light-gen-subZ/main/install.sh | sh
```

Detects your architecture and package manager, and installs the latest
release automatically (`.deb` on Debian/Ubuntu, `.AppImage` otherwise).

### macOS (Homebrew)

```sh
brew tap sindus/light-gen-subz
brew install --cask light-gen-subz
```

> Apple Silicon only for now.

Prebuilt binaries are also available on the
[releases page](https://github.com/sindus/light-gen-subZ/releases).

## Requirements

- [`ffmpeg`](https://ffmpeg.org) must be installed and on your `PATH` (used to
  extract audio from video files).
- The first run downloads a whisper model (~190 MB) to
  `~/.local/share/light-gen-subZ/models/` (Linux) or the equivalent app data
  directory on macOS. Enabling local translation downloads an additional
  NLLB-200 model (~900 MB).
- **Linux:** requires glibc ≥ 2.38 (Ubuntu 24.04+, Debian 13+, Fedora 39+),
  needed by the bundled ONNX Runtime used for local translation.
- Cloud transcription/translation (Groq, DeepL) require an API key, entered
  in the app's Settings panel and stored in your OS keychain.

## Usage

1. Open the app, choose a video or audio file.
2. Click **Generate subtitles**.
3. Once done, the `.srt` is written next to your source file and previewed
   in the app. Use **Save as…** to save it elsewhere.

## Build from source

Requires Rust, Node.js, `ffmpeg`, and (for the local whisper engine) `cmake`
and `clang`.

```sh
git clone https://github.com/sindus/light-gen-subZ.git
cd light-gen-subZ
npm install
npm run tauri dev    # development
npm run tauri build   # release bundles (.deb / .AppImage / .app / .dmg)
```

## How it works

```
file → ffmpeg (extract to 16kHz mono WAV)
     → whisper.cpp or Groq API (transcription, language auto-detect)
     → segmentation (split overly long cues)
     → .srt writer
     → (optional) NLLB-200 (ONNX) or DeepL API → translated .srt
```

See `src-tauri/src/pipeline/` for the implementation.

## License

MIT
