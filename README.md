# FunButton.ai

> Talk fast. Stay local. Pay less.

The fun, dev-grade voice dictation tool. Press the button, talk, watch your cleaned-up text land at the cursor. No cloud lock-in. No 800 MB Electron tax. No API key required (local mode).

**Status:** Sprint 1 building (May 8-11, 2026). MVP target: Monday morning.

## What's different

| | Wispr Flow | FunButton |
|---|---|---|
| Bundle | 800 MB Electron, 8% CPU idle | ~10 MB Tauri 2, ~50 MB RAM |
| Cloud | required | optional (local Ollama default) |
| Code mode | not really | first-class — spoken symbols + casing taxonomy |
| API key | required | not required (local mode) |
| Linux | none | first-class (post-MVP) |
| Source | closed | GPLv3 desktop core |
| Price | $144/yr, no lifetime | $99 lifetime founder pricing (planned) |

## Architecture

- **Tauri 2** shell — single Rust binary + WebKit webview, ~10 MB.
- **cpal** for audio capture (Core Audio on macOS, WASAPI on Windows, ALSA on Linux).
- **rdev** for global Right Option push-to-talk (modifier-only key, requires Accessibility permission).
- **Groq Whisper Large v3 Turbo** (~300 ms TTFT) for transcription.
- **Cleanup is pluggable**:
  - **Groq Llama 3.3 70B** — fastest, cloud, requires `GROQ_API_KEY`.
  - **Ollama at `localhost:11434`** — local, default model `qwen2.5:1.5b` (~1 GB GGUF), zero cloud dependency.
  - **Auto** — uses local if running, falls back to Groq.
- **Code mode** — auto-detected from the frontmost app (Cursor / VS Code / JetBrains / Vim / Terminal / Xcode). System prompt handles spoken symbols (`open paren` → `(`, `arrow` → `->`, `equals` → `=`) and casing (`camelCase X Y Z` → `xYZ`, `snake_case X Y Z` → `x_y_z`, etc.).
- **Text injection** — clipboard write + Cmd+V via `enigo`, with prior clipboard restored ~1 s later.

## Install

(Pre-release — until v0.1.0 .dmg exists, build from source.)

```bash
git clone https://github.com/todddickerson/funbutton.git
cd funbutton/apps/desktop
npm install
npm run tauri build -- --target aarch64-apple-darwin
open src-tauri/target/aarch64-apple-darwin/release/bundle/macos/FunButton.app
```

If macOS Gatekeeper blocks the unsigned build: `sudo xattr -cr /Applications/FunButton.app`.

**First run:** macOS will ask for **Microphone** and **Accessibility** permissions. Grant both.

**Local cleanup mode:** install [Ollama](https://ollama.ai) and `ollama pull qwen2.5:1.5b`.

## Use

1. Hold **Right Option**.
2. Talk.
3. Release.
4. Cleaned text appears at your cursor.

Tray menu: **Settings** opens the configuration window. **Quit** exits the app.

## Develop

```bash
cd apps/desktop
npm install
GROQ_API_KEY=... npm run tauri dev
```

## Verify the pipeline without installing

You can sanity-check that the Groq Whisper + Llama cleanup chain works on your network and key, without installing the .app:

```bash
brew install jq ffmpeg  # one time
GROQ_API_KEY=gsk_... ./scripts/test_pipeline.sh
```

It synthesizes a WAV via macOS `say`, runs the full pipeline, and prints raw vs cleaned. Pass your own phrase as the first arg.

## License

GPL-3.0-or-later (desktop core). Cloud / sync / team features (when they exist) will be source-available under a separate license.

## Status

See [`PROGRESS.md`](PROGRESS.md) for the build heartbeat. See [`PRD.md`](PRD.md) for scope. See [`RESEARCH.md`](RESEARCH.md) for the competitive analysis. See `~/clawd/projects/funbutton/COMPETITIVE-LANDSCAPE.md` for the wedge sharpening.
