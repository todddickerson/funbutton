use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Serialize)]
struct GenerateRequest<'a> {
    model: &'a str,
    prompt: String,
    stream: bool,
    options: Options,
}

#[derive(Debug, Serialize)]
struct Options {
    temperature: f32,
    num_predict: u32,
}

#[derive(Debug, Deserialize)]
struct GenerateResponse {
    response: String,
}

pub async fn is_available(base_url: &str) -> bool {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_millis(800))
        .build()
    {
        Ok(c) => c,
        Err(_) => return false,
    };
    client
        .get(format!("{}/api/tags", base_url.trim_end_matches('/')))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

pub async fn generate(
    base_url: &str,
    model: &str,
    system: &str,
    user: &str,
) -> Result<String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(90))
        .build()?;
    let prompt = format!("<|system|>\n{system}\n<|user|>\n{user}\n<|assistant|>\n");
    let req = GenerateRequest {
        model,
        prompt,
        stream: false,
        options: Options { temperature: 0.2, num_predict: 1024 },
    };
    let resp = client
        .post(format!("{}/api/generate", base_url.trim_end_matches('/')))
        .json(&req)
        .send()
        .await
        .context("ollama generate failed")?;
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(anyhow!("ollama {}: {}", status, body));
    }
    let parsed: GenerateResponse =
        serde_json::from_str(&body).context("parsing ollama response")?;
    Ok(parsed.response)
}
