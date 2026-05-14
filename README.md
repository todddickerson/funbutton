# FunButton.ai

> Talk fast. Stay local. Pay less.

The fun, dev-grade voice dictation tool. Press the button, talk, watch your cleaned-up text land at the cursor. No cloud lock-in. No 800 MB Electron tax. **Zero API key, zero install — cleanup runs on a bundled local model out of the box.**

**Status:** Sprint 1 building (May 8-11, 2026). MVP target: Monday morning.

## What's different

| | Wispr Flow | FunButton |
|---|---|---|
| Bundle | 800 MB Electron, 8% CPU idle | ~1 GB Tauri 2 (incl. bundled LLM), ~150 MB RAM cleanup-only |
| Cloud | required | optional (bundled local model is the default) |
| Code mode | not really | first-class — spoken symbols + casing taxonomy |
| API key | required | not required — cleanup runs on bundled Qwen 2.5 1.5B |
| Linux | none | first-class (post-MVP) |
| Source | closed | GPLv3 desktop core |
| Price | $144/yr, no lifetime | $99 lifetime founder pricing (planned) |

## Architecture

- **Tauri 2** shell — single Rust binary + WebKit webview, ~10 MB.
- **cpal** for audio capture (Core Audio on macOS, WASAPI on Windows, ALSA on Linux).
- **rdev** for global Right Option push-to-talk (modifier-only key, requires Accessibility permission).
- **Groq Whisper Large v3 Turbo** (~300 ms TTFT) for transcription.
- **Cleanup is pluggable**:
  - **Embedded** (default) — bundled `llama.cpp` server + `Qwen 2.5 1.5B Instruct Q4_K_M` GGUF, spawned as a child process on app launch. Zero install, zero key, runs offline.
  - **Groq Llama 3.3 70B** — fastest, cloud, requires `GROQ_API_KEY` or a FunButton license.
  - **Ollama at `localhost:11434`** — external Ollama if you've already got it (power-user path).
  - **Auto** — tries embedded → ollama-external → groq, picks the first that's available.
- **Code mode** — auto-detected from the frontmost app (Cursor / VS Code / JetBrains / Vim / Terminal / Xcode). System prompt handles spoken symbols (`open paren` → `(`, `arrow` → `->`, `equals` → `=`) and casing (`camelCase X Y Z` → `xYZ`, `snake_case X Y Z` → `x_y_z`, etc.).
- **Text injection** — clipboard write + Cmd+V via `enigo`, with prior clipboard restored ~1 s later.

## Install

(Pre-release — until v0.1.0 .dmg exists, build from source.)

```bash
git clone https://github.com/todddickerson/funbutton.git
cd funbutton/apps/desktop/src-tauri
bash scripts/fetch-vendor-deps.sh   # one-time: pulls llama-server + Qwen 2.5 1.5B GGUF (~1 GB)
cd ..
npm install
npm run tauri build -- --target aarch64-apple-darwin
open src-tauri/target/aarch64-apple-darwin/release/bundle/macos/FunButton.app
```

If macOS Gatekeeper blocks the unsigned build: `sudo xattr -cr /Applications/FunButton.app`.

**First run:** macOS will ask for **Microphone** and **Accessibility** permissions. Grant both. The bundled LLM warms up in the background and you'll see a toast when it's ready (~1 s warm cache, ~10 s cold).

**Note on transcription:** Whisper transcription is *not* bundled yet. Free-tier users still need a Groq key OR a FunButton license for the speech-to-text step. The cleanup model is fully local. Bundled Whisper is on the V1.2 roadmap.

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
