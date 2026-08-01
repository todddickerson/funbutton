use crate::app_detect::FrontApp;
use crate::cleanup::{self, Mode};
use crate::cloud::{CleanupOutcome, CloudClient};
use crate::groq;
use crate::ollama;
use crate::state::{AppStateHandle, Backend, ModeOverride, Status, SttBackend};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct PipelineResult {
    pub raw: String,
    pub cleaned: String,
    pub mode: &'static str,
    pub backend_used: &'static str,
    /// Frontmost app at dictation time — detected once here, reused for the
    /// history row so we don't shell out to osascript twice.
    pub frontmost: String,
}

pub async fn run(state: AppStateHandle, wav: Vec<u8>) -> anyhow::Result<PipelineResult> {
    let (
        api_key,
        backend,
        stt_backend,
        ollama_url,
        ollama_model,
        mode_override,
        dictionary,
        license_jwt,
        cloud_api_base,
        premium_model,
    ) = {
        let s = state.settings.lock();
        (
            s.groq_api_key.clone(),
            s.backend,
            s.stt_backend,
            s.ollama_url.clone(),
            s.ollama_model.clone(),
            s.mode_override,
            s.dictionary.clone(),
            s.license_jwt.clone(),
            s.cloud_api_base.clone(),
            s.premium_model.clone(),
        )
    };

    let cloud = (!license_jwt.is_empty())
        .then(|| CloudClient::new(cloud_api_base.clone(), license_jwt.clone()));

    // Mode is resolved BEFORE transcription so the on-device whisper can be
    // biased with mode-appropriate vocabulary via its initial prompt.
    let front = FrontApp::detect();
    let frontmost = front.label();
    let mode = match mode_override {
        ModeOverride::Auto => Mode::from_front_app(&front),
        ModeOverride::Code => Mode::Code,
        ModeOverride::Email => Mode::Email,
        ModeOverride::Slack => Mode::Slack,
        ModeOverride::Raw => Mode::Raw,
    };
    let mode_label = match mode {
        Mode::Auto => "auto",
        Mode::Code => "code",
        Mode::Email => "email",
        Mode::Slack => "slack",
        Mode::Raw => "raw",
    };

    *state.status.lock() = Status::Transcribing;

    // Transcription chain. On-device whisper is the default (zero key,
    // offline); Groq/cloud is the optional faster path. Whatever the user
    // picked, the other engines remain silent fallbacks so dictation
    // degrades instead of hard-failing.
    let stt_prompt = build_stt_prompt(&dictionary, mode);
    #[derive(Clone, Copy, PartialEq)]
    enum SttSource {
        Embedded,
        Cloud,
        GroqByok,
    }
    let order: [SttSource; 3] = match stt_backend {
        SttBackend::Local => [SttSource::Embedded, SttSource::Cloud, SttSource::GroqByok],
        SttBackend::Groq => [SttSource::Cloud, SttSource::GroqByok, SttSource::Embedded],
    };
    let mut raw: Option<String> = None;
    let mut stt_errors: Vec<String> = Vec::new();
    for src in order {
        let (name, attempt): (&str, anyhow::Result<String>) = match src {
            SttSource::Embedded => {
                let stt = Arc::clone(&state.stt);
                let wav_c = wav.clone();
                let prompt = stt_prompt.clone();
                let joined = tokio::task::spawn_blocking(move || {
                    stt.transcribe_wav(&wav_c, prompt)
                })
                .await;
                (
                    "embedded-whisper",
                    joined.unwrap_or_else(|e| Err(anyhow::anyhow!("stt task join: {e}"))),
                )
            }
            SttSource::Cloud => match &cloud {
                Some(cli) => ("cloud", cli.transcribe(wav.clone()).await),
                None => continue,
            },
            SttSource::GroqByok => {
                if api_key.is_empty() {
                    continue;
                }
                ("groq-byok", groq::transcribe(&api_key, wav.clone()).await)
            }
        };
        match attempt {
            Ok(t) => {
                log::info!("transcribed via {name}");
                raw = Some(t);
                break;
            }
            Err(e) => {
                log::warn!("stt {name} failed: {e:#}");
                stt_errors.push(format!("{name}: {e}"));
            }
        }
    }
    let raw = raw.ok_or_else(|| {
        anyhow::anyhow!(
            "no transcription backend available ({})",
            if stt_errors.is_empty() {
                "on-device model not ready and no Groq key set".to_string()
            } else {
                stt_errors.join(" | ")
            }
        )
    })?;
    if raw.trim().is_empty() {
        return Err(anyhow::anyhow!("no speech detected"));
    }
    *state.last_transcript.lock() = raw.clone();
    let base_prompt = cleanup::system_prompt(mode);
    let mut prompt = base_prompt.to_string();
    if matches!(mode, Mode::Code) {
        prompt.push_str(
            "\n\nDEV VOCABULARY (normalize to these exact spellings and casings when the user says them): ",
        );
        prompt.push_str(&cleanup::DEV_DICTIONARY.join(", "));
    }
    if !dictionary.is_empty() {
        let dict_lines: Vec<String> = dictionary
            .iter()
            .filter(|s| !s.trim().is_empty())
            .map(|s| format!("- {s}"))
            .collect();
        prompt.push_str(&format!(
            "\n\nUSER DICTIONARY (preserve these names and spellings exactly when they appear, even if the transcriber heard them slightly differently — these outrank the dev vocabulary):\n{}",
            dict_lines.join("\n")
        ));
    }

    *state.status.lock() = Status::Cleaning;

    // Cloud cleanup path: tries preferred premium model, silently falls back
    // to fast tier on HTTP 402 (cap exceeded). Worker enforces caps + rate limit.
    if let Some(cli) = &cloud {
        let cloud_mode = mode_label;
        match cli
            .cleanup(&premium_model, &raw, cloud_mode, &dictionary)
            .await
        {
            Ok(CleanupOutcome::Ok { text, .. }) => {
                let cleaned = post_process(text);
                return Ok(PipelineResult {
                    raw,
                    cleaned,
                    mode: mode_label,
                    backend_used: "cloud",
                    frontmost,
                });
            }
            Ok(CleanupOutcome::CapExceeded) => {
                log::info!("cloud cap exceeded — falling back to fast tier on cloud");
                if let Ok(CleanupOutcome::Ok { text, .. }) =
                    cli.cleanup("fast", &raw, cloud_mode, &dictionary).await
                {
                    let cleaned = post_process(text);
                    return Ok(PipelineResult {
                        raw,
                        cleaned,
                        mode: mode_label,
                        backend_used: "cloud-fallback",
                        frontmost,
                    });
                }
                log::warn!("cloud fast fallback failed; trying BYOK groq next");
            }
            Err(e) => {
                log::warn!("cloud cleanup failed, falling back to BYOK groq: {e:#}");
            }
        }
    }

    // BYOK / local path. Fallback chain depends on user preference:
    //   Embedded → try only the bundled llama.cpp.
    //   Local    → try only user-installed Ollama.
    //   Groq     → try only the cloud (BYOK key required).
    //   Auto     → embedded (always available) → ollama-external → groq.
    let embedded = state.embedded.lock().clone();

    let try_embedded = matches!(backend, Backend::Embedded | Backend::Auto)
        && embedded.is_some();
    let try_ollama = matches!(backend, Backend::Local | Backend::Auto)
        && ollama::is_available(&ollama_url).await;
    let try_groq = matches!(backend, Backend::Groq | Backend::Auto)
        && !api_key.is_empty();

    let (cleaned, used) = 'cleanup: {
        if try_embedded {
            if let Some(srv) = &embedded {
                match srv.generate(&prompt, &raw).await {
                    Ok(t) => break 'cleanup (t, "embedded"),
                    Err(e) => log::warn!("embedded cleanup failed: {e:#}"),
                }
            }
        }
        if try_ollama {
            match ollama::generate(&ollama_url, &ollama_model, &prompt, &raw).await {
                Ok(t) => break 'cleanup (t, "ollama"),
                Err(e) => log::warn!("ollama cleanup failed: {e:#}"),
            }
        }
        if try_groq {
            match groq::chat_complete(&api_key, &prompt, &raw).await {
                Ok(t) => break 'cleanup (t, "groq"),
                Err(e) => log::warn!("groq cleanup failed: {e:#}"),
            }
        }
        // Last resort: surface raw transcript verbatim. Better than a hard
        // failure for a free user with no backends available.
        log::warn!("all cleanup backends unavailable — returning raw transcript");
        (raw.clone(), "raw-passthrough")
    };

    let cleaned = post_process(cleaned);
    Ok(PipelineResult { raw, cleaned, mode: mode_label, backend_used: used, frontmost })
}

/// Vocabulary bias for the on-device whisper: the user's dictionary, plus
/// (in code mode) the built-in dev vocabulary. whisper's initial prompt is
/// capped (~224 tokens), so trim to a safe budget.
fn build_stt_prompt(dictionary: &[String], mode: Mode) -> Option<String> {
    let mut terms: Vec<&str> = dictionary
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    if matches!(mode, Mode::Code) {
        terms.extend(cleanup::DEV_DICTIONARY.iter().copied());
    }
    if terms.is_empty() {
        return None;
    }
    let mut prompt = String::with_capacity(600);
    for t in terms {
        if prompt.len() + t.len() + 2 > 600 {
            break;
        }
        if !prompt.is_empty() {
            prompt.push_str(", ");
        }
        prompt.push_str(t);
    }
    Some(prompt)
}

fn post_process(s: String) -> String {
    let trimmed = s.trim();
    let stripped = if (trimmed.starts_with('"') && trimmed.ends_with('"'))
        || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
    {
        let inner = &trimmed[1..trimmed.len().saturating_sub(1)];
        inner.trim().to_string()
    } else {
        trimmed.to_string()
    };
    let lower = stripped.to_lowercase();
    for prefix in ["cleaned text:", "output:", "cleaned:", "result:"] {
        if lower.starts_with(prefix) {
            return stripped[prefix.len()..].trim().to_string();
        }
    }
    stripped
}
