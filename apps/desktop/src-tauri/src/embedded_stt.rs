// Embedded on-device speech-to-text: whisper base.en (GGUF, Q8_0) via
// transcribe-cpp — the same whisper.cpp/ggml wrapper Handy uses, statically
// linked with the Metal backend on macOS. Loaded once at startup from the
// bundled vendor dir; transcription is fully offline, zero API key.
//
// Groq Whisper remains available as an optional faster/cloud path via the
// `stt_backend` setting — but this module is the default.

use anyhow::{anyhow, Context as _, Result};
use parking_lot::Mutex;
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tauri::Emitter;

pub const BUNDLED_STT_MODEL_FILE: &str = "whisper-base.en-Q8_0.gguf";
const WHISPER_SAMPLE_RATE: u32 = 16_000;
/// whisper.cpp misbehaves on sub-second inputs; pad the tail with silence.
const MIN_SAMPLES: usize = (WHISPER_SAMPLE_RATE as usize * 12) / 10; // 1.2 s

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SttStatus {
    Starting,
    Ready,
    Failed(String),
}

impl SttStatus {
    pub fn label(&self) -> &'static str {
        match self {
            SttStatus::Starting => "starting",
            SttStatus::Ready => "ready",
            SttStatus::Failed(_) => "failed",
        }
    }
}

pub struct EmbeddedStt {
    session: Mutex<Option<transcribe_cpp::Session>>,
    status: Mutex<SttStatus>,
}

pub type EmbeddedSttHandle = Arc<EmbeddedStt>;

/// Locate the vendored GGUF: bundled resource dir first, then the dev tree.
fn locate_model(app: &tauri::AppHandle) -> Result<PathBuf> {
    use tauri::Manager as _;
    if let Ok(resource_dir) = app.path().resource_dir() {
        let p = resource_dir
            .join("vendor")
            .join("whisper")
            .join(BUNDLED_STT_MODEL_FILE);
        if p.exists() {
            return Ok(p);
        }
    }
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    if !manifest.is_empty() {
        let p = PathBuf::from(manifest)
            .join("vendor")
            .join("whisper")
            .join(BUNDLED_STT_MODEL_FILE);
        if p.exists() {
            return Ok(p);
        }
    }
    let cwd = std::env::current_dir()
        .unwrap_or_default()
        .join("vendor")
        .join("whisper")
        .join(BUNDLED_STT_MODEL_FILE);
    if cwd.exists() {
        return Ok(cwd);
    }
    Err(anyhow!(
        "could not locate vendor/whisper/{BUNDLED_STT_MODEL_FILE} — run scripts/fetch-vendor-deps.sh"
    ))
}

impl EmbeddedStt {
    pub fn new() -> EmbeddedSttHandle {
        Arc::new(EmbeddedStt {
            session: Mutex::new(None),
            status: Mutex::new(SttStatus::Starting),
        })
    }

    pub fn status(&self) -> SttStatus {
        self.status.lock().clone()
    }

    /// Spawn the model load on a dedicated thread. Emits
    /// `funbutton:stt-ready` / `funbutton:stt-failed` when settled.
    pub fn init(self: &Arc<Self>, app: &tauri::AppHandle) {
        ensure_backends();
        let this = Arc::clone(self);
        let app = app.clone();
        std::thread::spawn(move || {
            let started = Instant::now();
            let result = (|| -> Result<transcribe_cpp::Session> {
                let path = locate_model(&app)?;
                let backend = if transcribe_cpp::backend_available(transcribe_cpp::Backend::Metal) {
                    transcribe_cpp::Backend::Metal
                } else {
                    transcribe_cpp::Backend::Auto
                };
                let options = transcribe_cpp::ModelOptions {
                    backend,
                    gpu_device: 0,
                };
                let model = transcribe_cpp::Model::load_with(&path, &options)
                    .with_context(|| format!("load whisper model at {}", path.display()))?;
                log::info!(
                    "embedded STT model loaded ({:?} backend, {}ms)",
                    model.backend(),
                    started.elapsed().as_millis()
                );
                model.session().context("create whisper session")
            })();
            match result {
                Ok(session) => {
                    *this.session.lock() = Some(session);
                    *this.status.lock() = SttStatus::Ready;
                    let _ = app.emit("funbutton:stt-ready", ());
                }
                Err(e) => {
                    log::warn!("embedded STT failed to load: {e:#}");
                    *this.status.lock() = SttStatus::Failed(e.to_string());
                    let _ = app.emit("funbutton:stt-failed", e.to_string());
                }
            }
        });
    }

    /// Transcribe an in-memory WAV (mono PCM-16, any sample rate — the shape
    /// audio.rs produces). `initial_prompt` biases whisper toward dictionary
    /// terms (user dictionary + dev vocabulary). Blocking — call from
    /// spawn_blocking.
    pub fn transcribe_wav(&self, wav: &[u8], initial_prompt: Option<String>) -> Result<String> {
        let samples = decode_wav_to_16k_mono(wav)?;
        if samples.is_empty() {
            return Ok(String::new());
        }

        let Some(mut session) = self.session.lock().take() else {
            return Err(anyhow!(
                "embedded STT not ready ({:?})",
                self.status().label()
            ));
        };

        let family = initial_prompt.filter(|p| !p.is_empty()).map(|p| {
            transcribe_cpp::RunExtension::Whisper(transcribe_cpp::WhisperRunOptions {
                initial_prompt: Some(p),
                ..Default::default()
            })
        });
        let opts = transcribe_cpp::RunOptions {
            task: transcribe_cpp::Task::Transcribe,
            language: Some("en".to_string()),
            family,
            ..Default::default()
        };

        let started = Instant::now();
        // ggml can abort deep inside C++ on malformed state; catch panics at
        // the Rust boundary so one bad run degrades to the fallback chain
        // instead of taking the app down. On panic the session is gone —
        // mark the engine failed rather than reusing poisoned state.
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            let r = session.run(&samples, &opts);
            (session, r)
        }));
        match outcome {
            Ok((session, run_result)) => {
                *self.session.lock() = Some(session);
                let text = run_result.map_err(|e| anyhow!("whisper run: {e}"))?.text;
                log::info!(
                    "embedded STT transcribed in {}ms",
                    started.elapsed().as_millis()
                );
                Ok(strip_whisper_artifacts(&text))
            }
            Err(_) => {
                *self.status.lock() = SttStatus::Failed("engine panicked".into());
                Err(anyhow!("embedded STT engine panicked — falling back"))
            }
        }
    }

    /// Drop the model before process exit — ggml-metal's device teardown
    /// SIGABRTs if Metal resources are still alive at static-destructor time.
    pub fn unload(&self) {
        *self.session.lock() = None;
    }

    /// Synchronous load from an explicit path — test harness entry point.
    #[cfg(test)]
    fn load_from_path(&self, path: &std::path::Path) -> Result<()> {
        ensure_backends();
        let options = transcribe_cpp::ModelOptions {
            backend: transcribe_cpp::Backend::Auto,
            gpu_device: 0,
        };
        let model = transcribe_cpp::Model::load_with(path, &options)?;
        *self.session.lock() = Some(model.session()?);
        *self.status.lock() = SttStatus::Ready;
        Ok(())
    }
}

/// Backend registration must happen once per process, before any load. On
/// macOS with the static Metal build this is effectively a no-op, but
/// transcribe-cpp still requires the call.
fn ensure_backends() {
    static BACKEND_INIT: std::sync::Once = std::sync::Once::new();
    BACKEND_INIT.call_once(|| {
        transcribe_cpp::init_logging();
        if let Err(e) = transcribe_cpp::init_backends_default() {
            log::warn!("transcribe-cpp backend init: {e}");
        }
    });
}

/// WAV bytes → f32 mono @ 16 kHz. Accepts what audio.rs writes (mono PCM-16
/// at device-native rate); tolerates multi-channel input by averaging.
fn decode_wav_to_16k_mono(wav: &[u8]) -> Result<Vec<f32>> {
    let mut reader = hound::WavReader::new(Cursor::new(wav)).context("parse WAV")?;
    let spec = reader.spec();
    if spec.sample_format != hound::SampleFormat::Int || spec.bits_per_sample != 16 {
        return Err(anyhow!(
            "unexpected WAV format: {:?} {}-bit",
            spec.sample_format,
            spec.bits_per_sample
        ));
    }
    let channels = spec.channels.max(1) as usize;
    let mut mono: Vec<f32> = Vec::with_capacity(reader.len() as usize / channels);
    if channels == 1 {
        for s in reader.samples::<i16>() {
            mono.push(s? as f32 / 32768.0);
        }
    } else {
        let all: Result<Vec<i16>, _> = reader.samples::<i16>().collect();
        for frame in all?.chunks_exact(channels) {
            let mix: f32 = frame.iter().map(|&v| v as f32 / 32768.0).sum::<f32>() / channels as f32;
            mono.push(mix);
        }
    }
    let mut out = resample_to_16k(mono, spec.sample_rate)?;
    if !out.is_empty() && out.len() < MIN_SAMPLES {
        out.resize(MIN_SAMPLES, 0.0);
    }
    Ok(out)
}

fn resample_to_16k(samples: Vec<f32>, in_rate: u32) -> Result<Vec<f32>> {
    if in_rate == WHISPER_SAMPLE_RATE || samples.is_empty() {
        return Ok(samples);
    }
    use rubato::Resampler;
    const CHUNK: usize = 1024;
    let mut rs =
        rubato::FftFixedIn::<f32>::new(in_rate as usize, WHISPER_SAMPLE_RATE as usize, CHUNK, 1, 1)
            .context("create resampler")?;
    let mut padded = samples;
    let rem = padded.len() % CHUNK;
    if rem != 0 {
        padded.resize(padded.len() + (CHUNK - rem), 0.0);
    }
    let mut out =
        Vec::with_capacity(padded.len() * WHISPER_SAMPLE_RATE as usize / in_rate as usize + CHUNK);
    for block in padded.chunks_exact(CHUNK) {
        let processed = rs.process(&[block], None).context("resample block")?;
        out.extend_from_slice(&processed[0]);
    }
    Ok(out)
}

/// whisper.cpp emits bracketed non-speech tokens on silence/noise
/// ("[BLANK_AUDIO]", "(music)", …). Strip them; collapse leftover whitespace.
fn strip_whisper_artifacts(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '[' || c == '(' {
            let close = if c == '[' { ']' } else { ')' };
            let mut token = String::new();
            let mut closed = false;
            for t in chars.by_ref() {
                if t == close {
                    closed = true;
                    break;
                }
                token.push(t);
            }
            // Non-speech annotations are short and contain no lowercase
            // sentence content: [BLANK_AUDIO], [MUSIC], (laughs), (silence).
            let is_artifact = closed
                && token.len() <= 24
                && !token.contains([',', '.'])
                && (token
                    .chars()
                    .all(|t| !t.is_alphabetic() || t.is_uppercase() || t == '_')
                    || matches!(
                        token.to_lowercase().as_str(),
                        "laughs"
                            | "laughter"
                            | "music"
                            | "silence"
                            | "noise"
                            | "applause"
                            | "inaudible"
                            | "blank_audio"
                            | "sound"
                    ));
            if !is_artifact {
                out.push(c);
                out.push_str(&token);
                if closed {
                    out.push(close);
                }
            }
        } else {
            out.push(c);
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_blank_audio() {
        assert_eq!(strip_whisper_artifacts(" [BLANK_AUDIO] "), "");
        assert_eq!(
            strip_whisper_artifacts("hello [MUSIC] world"),
            "hello world"
        );
        assert_eq!(strip_whisper_artifacts("(laughs) ok"), "ok");
        // Real content in brackets survives.
        assert_eq!(
            strip_whisper_artifacts("array[index] and fn(x, y)"),
            "array[index] and fn(x, y)"
        );
    }

    /// End-to-end offline proof: bundled GGUF + real speech WAV → text.
    /// Needs the vendor model and a WAV at $FUNBUTTON_TEST_WAV; run with:
    /// `cargo test --release transcribes_real_wav_offline -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn transcribes_real_wav_offline() {
        let model = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("vendor/whisper")
            .join(BUNDLED_STT_MODEL_FILE);
        assert!(
            model.exists(),
            "vendor model missing — run scripts/fetch-vendor-deps.sh"
        );
        let wav_path = std::env::var("FUNBUTTON_TEST_WAV").expect("set FUNBUTTON_TEST_WAV");
        let bytes = std::fs::read(&wav_path).expect("read test wav");
        let stt = EmbeddedStt::new();
        stt.load_from_path(&model).expect("load model");
        let text = stt
            .transcribe_wav(&bytes, Some("git, npm, FunButton".into()))
            .expect("transcribe");
        eprintln!("TRANSCRIPT: {text:?}");
        assert!(!text.trim().is_empty(), "expected non-empty transcript");
        stt.unload();
    }

    #[test]
    fn resample_48k_halves_ish() {
        let input = vec![0.5f32; 48_000];
        let out = resample_to_16k(input, 48_000).unwrap();
        // 1 s of 48 kHz → ~1 s of 16 kHz (± resampler latency)
        assert!(
            (out.len() as i64 - 16_000).abs() < 2_000,
            "got {}",
            out.len()
        );
    }
}
