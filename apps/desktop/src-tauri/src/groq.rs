use anyhow::{anyhow, Context, Result};
use once_cell::sync::Lazy;
use reqwest::multipart;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const GROQ_BASE: &str = "https://api.groq.com/openai/v1";
const WHISPER_MODEL: &str = "whisper-large-v3-turbo";
pub const LLAMA_MODEL: &str = "llama-3.3-70b-versatile";

/// One process-wide reqwest client. HTTP/2 multiplexing + connection pool keep
/// the TLS session alive between calls — first call still pays the ~150-200 ms
/// handshake; every subsequent call reuses the warm connection. The result is
/// the bulk of the latency win the wargame asked for.
static GROQ_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    // HTTP/2 is negotiated via ALPN with reqwest's rustls + http2 feature.
    // Skip http2_prior_knowledge — that's for cleartext h2c, which Groq does not speak.
    reqwest::Client::builder()
        .http2_keep_alive_interval(Duration::from_secs(30))
        .http2_keep_alive_timeout(Duration::from_secs(60))
        .http2_keep_alive_while_idle(true)
        .pool_idle_timeout(Duration::from_secs(120))
        .pool_max_idle_per_host(4)
        .tcp_keepalive(Duration::from_secs(30))
        .timeout(Duration::from_secs(60))
        .connect_timeout(Duration::from_secs(8))
        .user_agent("funbutton/0.1.0 (https://funbutton.ai)")
        .build()
        .expect("groq client build")
});

/// Hit `/v1/models` with HEAD-equivalent semantics so the TCP+TLS+HTTP/2 connection
/// is open and pooled before the user ever holds the hotkey. Best-effort — failure
/// here is fine; the next real call will just pay the handshake cost.
pub async fn prewarm(api_key: &str) {
    if api_key.trim().is_empty() {
        return;
    }
    let url = format!("{GROQ_BASE}/models");
    match GROQ_CLIENT.get(&url).bearer_auth(api_key).send().await {
        Ok(r) => log::info!("groq prewarm ok ({})", r.status()),
        Err(e) => log::info!("groq prewarm skipped: {e:#}"),
    }
}

#[derive(Debug, Deserialize)]
struct WhisperResponse {
    text: String,
}

pub async fn transcribe(api_key: &str, wav: Vec<u8>) -> Result<String> {
    if api_key.trim().is_empty() {
        return Err(anyhow!("missing GROQ_API_KEY"));
    }
    let file_part = multipart::Part::bytes(wav)
        .file_name("audio.wav")
        .mime_str("audio/wav")?;
    let form = multipart::Form::new()
        .text("model", WHISPER_MODEL)
        .text("response_format", "json")
        .text("temperature", "0")
        .part("file", file_part);
    let resp = GROQ_CLIENT
        .post(format!("{GROQ_BASE}/audio/transcriptions"))
        .bearer_auth(api_key)
        .multipart(form)
        .send()
        .await
        .context("groq whisper request failed")?;
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(anyhow!("groq whisper {}: {}", status, body));
    }
    let parsed: WhisperResponse =
        serde_json::from_str(&body).context("parsing groq whisper response")?;
    Ok(parsed.text)
}

#[derive(Debug, Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatMessage<'a> {
    pub role: &'a str,
    pub content: &'a str,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: OwnedMessage,
}

#[derive(Debug, Deserialize)]
struct OwnedMessage {
    content: String,
}

pub async fn chat_complete(
    api_key: &str,
    system: &str,
    user: &str,
) -> Result<String> {
    if api_key.trim().is_empty() {
        return Err(anyhow!("missing GROQ_API_KEY"));
    }
    let body = ChatRequest {
        model: LLAMA_MODEL,
        messages: vec![
            ChatMessage { role: "system", content: system },
            ChatMessage { role: "user", content: user },
        ],
        temperature: 0.2,
        max_tokens: Some(1024),
    };
    let resp = GROQ_CLIENT
        .post(format!("{GROQ_BASE}/chat/completions"))
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .context("groq chat request failed")?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(anyhow!("groq chat {}: {}", status, text));
    }
    let parsed: ChatResponse = serde_json::from_str(&text).context("parsing groq chat response")?;
    let content = parsed
        .choices
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("groq chat: no choices"))?
        .message
        .content;
    Ok(content)
}
