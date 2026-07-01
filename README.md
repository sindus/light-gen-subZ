# light-gen-subZ

Turn a video or audio file into a subtitle track. Pick a file, generate, get
an `.srt` with accurate timestamps — fully offline, powered by a local
[whisper.cpp](https://github.com/ggerganov/whisper.cpp) model.

- Local speech-to-text, no account, no upload of your media anywhere.
- Multilingual, auto-detected.
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
  directory on macOS.

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
     → whisper.cpp (local transcription, language auto-detect)
     → segmentation (split overly long cues)
     → .srt writer
```

See `src-tauri/src/pipeline/` for the implementation.

## License

MIT
