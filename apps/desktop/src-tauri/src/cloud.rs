// Cloud client — talks to the FunButton Worker at `api.funbutton.ai`.
// Activated by the user pasting a license JWT into settings. When set,
// transcribe + cleanup route through the Worker (premium models, metered
// usage, cap enforcement). Otherwise the existing BYOK Groq path is used.

use anyhow::{anyhow, Context as _, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct CloudClient {
    pub base_url: String,
    pub jwt: String,
}

impl CloudClient {
    pub fn new(base_url: String, jwt: String) -> Self {
        Self { base_url, jwt }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url.trim_end_matches('/'), path)
    }

    pub async fn transcribe(&self, wav: Vec<u8>) -> Result<String> {
        // The Worker's /v1/transcribe accepts raw audio in the body.
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()?;
        let res = client
            .post(self.url("/v1/transcribe"))
            .bearer_auth(&self.jwt)
            .header("Content-Type", "audio/wav")
            .body(wav)
            .send()
            .await
            .context("transcribe request failed")?;
        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            return Err(anyhow!("transcribe {}: {}", status, body));
        }
        let parsed: TranscribeResponse = res.json().await?;
        Ok(parsed.text)
    }

    pub async fn cleanup(
        &self,
        model: &str,
        transcript: &str,
        mode: &str,
        dictionary: &[String],
    ) -> Result<CleanupOutcome> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()?;
        let body = CleanupRequest {
            model: model.to_string(),
            transcript: transcript.to_string(),
            mode: mode.to_string(),
            dictionary: dictionary.to_vec(),
        };
        let res = client
            .post(self.url("/v1/cleanup"))
            .bearer_auth(&self.jwt)
            .json(&body)
            .send()
            .await
            .context("cleanup request failed")?;
        let status = res.status();
        if status.as_u16() == 402 {
            // Cap exceeded — caller should silently fall back to fast tier.
            return Ok(CleanupOutcome::CapExceeded);
        }
        if !status.is_success() {
            let txt = res.text().await.unwrap_or_default();
            return Err(anyhow!("cleanup {}: {}", status, txt));
        }
        let parsed: CleanupResponse = res.json().await?;
        Ok(CleanupOutcome::Ok {
            text: parsed.text,
            cost_cents: parsed.cost_cents,
        })
    }

    pub async fn verify_license(&self) -> Result<LicenseVerifyResponse> {
        let client = reqwest::Client::new();
        let res = client
            .post(self.url("/v1/license/verify"))
            .bearer_auth(&self.jwt)
            .send()
            .await?;
        if !res.status().is_success() {
            let s = res.status();
            let body = res.text().await.unwrap_or_default();
            return Err(anyhow!("verify {}: {}", s, body));
        }
        Ok(res.json().await?)
    }
}

#[derive(Debug, Deserialize)]
struct TranscribeResponse {
    text: String,
    #[allow(dead_code)]
    duration_ms: u64,
    #[allow(dead_code)]
    words: u64,
}

#[derive(Debug, Serialize)]
struct CleanupRequest {
    model: String,
    transcript: String,
    mode: String,
    dictionary: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CleanupResponse {
    text: String,
    #[serde(default)]
    cost_cents: u64,
}

#[derive(Debug)]
pub enum CleanupOutcome {
    Ok {
        text: String,
        #[allow(dead_code)]
        cost_cents: u64,
    },
    CapExceeded,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LicenseVerifyResponse {
    pub valid: bool,
    pub tier: String,
    pub expires_at: u64,
    pub included_premium_words: u64,
    pub words_used_this_month: u64,
    pub cap_cents: u64,
}
