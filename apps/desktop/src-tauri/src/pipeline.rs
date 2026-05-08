use crate::app_detect::FrontApp;
use crate::cleanup::{self, Mode};
use crate::groq;
use crate::ollama;
use crate::state::{AppStateHandle, Backend, Status};

#[derive(Debug, Clone)]
pub struct PipelineResult {
    pub raw: String,
    pub cleaned: String,
    pub mode: &'static str,
    pub backend_used: &'static str,
}

pub async fn run(state: AppStateHandle, wav: Vec<u8>) -> anyhow::Result<PipelineResult> {
    let (api_key, backend, ollama_url, ollama_model) = {
        let s = state.settings.lock();
        (s.groq_api_key.clone(), s.backend, s.ollama_url.clone(), s.ollama_model.clone())
    };

    *state.status.lock() = Status::Transcribing;

    let raw = groq::transcribe(&api_key, wav).await?;
    *state.last_transcript.lock() = raw.clone();

    let front = FrontApp::detect();
    let mode = Mode::from_front_app(&front);
    let mode_label = match mode {
        Mode::Auto => "auto",
        Mode::Code => "code",
        Mode::Email => "email",
        Mode::Slack => "slack",
        Mode::Raw => "raw",
    };
    let prompt = cleanup::system_prompt(mode);

    *state.status.lock() = Status::Cleaning;

    // Determine cleanup backend
    let local_available = match backend {
        Backend::Local => true, // we'll try and surface error
        Backend::Auto => ollama::is_available(&ollama_url).await,
        Backend::Groq => false,
    };

    let (cleaned, used) = if local_available {
        match ollama::generate(&ollama_url, &ollama_model, prompt, &raw).await {
            Ok(t) => (t, "local"),
            Err(e) => {
                log::warn!("local cleanup failed, falling back to groq: {e:#}");
                let cleaned = groq::chat_complete(&api_key, prompt, &raw).await?;
                (cleaned, "groq")
            }
        }
    } else {
        let cleaned = groq::chat_complete(&api_key, prompt, &raw).await?;
        (cleaned, "groq")
    };

    let cleaned = post_process(cleaned);
    Ok(PipelineResult { raw, cleaned, mode: mode_label, backend_used: used })
}

fn post_process(s: String) -> String {
    let trimmed = s.trim();
    // Strip surrounding quotes the model sometimes adds.
    let stripped = if (trimmed.starts_with('"') && trimmed.ends_with('"'))
        || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
    {
        let inner = &trimmed[1..trimmed.len().saturating_sub(1)];
        inner.trim().to_string()
    } else {
        trimmed.to_string()
    };
    // Remove leading "Cleaned text:" labels.
    let lower = stripped.to_lowercase();
    for prefix in ["cleaned text:", "output:", "cleaned:", "result:"] {
        if lower.starts_with(prefix) {
            return stripped[prefix.len()..].trim().to_string();
        }
    }
    stripped
}
