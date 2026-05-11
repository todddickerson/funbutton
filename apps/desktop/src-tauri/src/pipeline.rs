use crate::app_detect::FrontApp;
use crate::cleanup::{self, Mode};
use crate::cloud::{CleanupOutcome, CloudClient};
use crate::groq;
use crate::ollama;
use crate::state::{AppStateHandle, Backend, ModeOverride, Status};

#[derive(Debug, Clone)]
pub struct PipelineResult {
    pub raw: String,
    pub cleaned: String,
    pub mode: &'static str,
    pub backend_used: &'static str,
}

pub async fn run(state: AppStateHandle, wav: Vec<u8>) -> anyhow::Result<PipelineResult> {
    let (
        api_key,
        backend,
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

    *state.status.lock() = Status::Transcribing;

    // Transcribe — cloud first when license present, otherwise BYOK Groq direct.
    let raw = if let Some(cli) = &cloud {
        match cli.transcribe(wav.clone()).await {
            Ok(t) => t,
            Err(e) => {
                log::warn!("cloud transcribe failed, falling back to BYOK groq: {e:#}");
                if api_key.is_empty() {
                    return Err(e);
                }
                groq::transcribe(&api_key, wav).await?
            }
        }
    } else {
        groq::transcribe(&api_key, wav).await?
    };
    *state.last_transcript.lock() = raw.clone();

    let mode = match mode_override {
        ModeOverride::Auto => {
            let front = FrontApp::detect();
            Mode::from_front_app(&front)
        }
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
    let base_prompt = cleanup::system_prompt(mode);
    let prompt = if dictionary.is_empty() {
        base_prompt.to_string()
    } else {
        let dict_lines: Vec<String> = dictionary
            .iter()
            .filter(|s| !s.trim().is_empty())
            .map(|s| format!("- {s}"))
            .collect();
        format!(
            "{base_prompt}\n\nUSER DICTIONARY (preserve these names and spellings exactly when they appear, even if Whisper transcribed them slightly differently):\n{dict}",
            dict = dict_lines.join("\n")
        )
    };

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
                    });
                }
                log::warn!("cloud fast fallback failed; trying BYOK groq next");
            }
            Err(e) => {
                log::warn!("cloud cleanup failed, falling back to BYOK groq: {e:#}");
            }
        }
    }

    // BYOK path (or final fallback) — unchanged behavior.
    let local_available = match backend {
        Backend::Local => true,
        Backend::Auto => ollama::is_available(&ollama_url).await,
        Backend::Groq => false,
    };

    let (cleaned, used) = if local_available {
        match ollama::generate(&ollama_url, &ollama_model, &prompt, &raw).await {
            Ok(t) => (t, "local"),
            Err(e) => {
                log::warn!("local cleanup failed, falling back to groq: {e:#}");
                let cleaned = groq::chat_complete(&api_key, &prompt, &raw).await?;
                (cleaned, "groq")
            }
        }
    } else {
        let cleaned = groq::chat_complete(&api_key, &prompt, &raw).await?;
        (cleaned, "groq")
    };

    let cleaned = post_process(cleaned);
    Ok(PipelineResult { raw, cleaned, mode: mode_label, backend_used: used })
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
