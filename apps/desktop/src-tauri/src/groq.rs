use anyhow::{anyhow, Context, Result};
use reqwest::multipart;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const GROQ_BASE: &str = "https://api.groq.com/openai/v1";
const WHISPER_MODEL: &str = "whisper-large-v3-turbo";
pub const LLAMA_MODEL: &str = "llama-3.3-70b-versatile";

#[derive(Debug, Deserialize)]
struct WhisperResponse {
    text: String,
}

pub async fn transcribe(api_key: &str, wav: Vec<u8>) -> Result<String> {
    if api_key.trim().is_empty() {
        return Err(anyhow!("missing GROQ_API_KEY"));
    }
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()?;
    let file_part = multipart::Part::bytes(wav)
        .file_name("audio.wav")
        .mime_str("audio/wav")?;
    let form = multipart::Form::new()
        .text("model", WHISPER_MODEL)
        .text("response_format", "json")
        .text("temperature", "0")
        .part("file", file_part);
    let resp = client
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
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;
    let body = ChatRequest {
        model: LLAMA_MODEL,
        messages: vec![
            ChatMessage { role: "system", content: system },
            ChatMessage { role: "user", content: user },
        ],
        temperature: 0.2,
        max_tokens: Some(1024),
    };
    let resp = client
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
