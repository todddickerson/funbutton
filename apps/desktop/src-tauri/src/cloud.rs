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

    pub async fn transcribe(&self, wav: Vec<u8>, stt_prompt: Option<&str>) -> Result<String> {
        // The Worker's /v1/transcribe accepts raw audio in the body. The whisper
        // vocabulary-bias prompt (built by `build_stt_prompt`) can't ride the
        // body — it goes in a percent-encoded header the Worker decodes and
        // forwards to Groq Whisper's `prompt` field, so the premium path gets the
        // same dev-term accuracy as the free BYOK/on-device paths.
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()?;
        let mut req = client
            .post(self.url("/v1/transcribe"))
            .bearer_auth(&self.jwt)
            .header("Content-Type", "audio/wav");
        if let Some(p) = stt_prompt.map(str::trim).filter(|p| !p.is_empty()) {
            req = req.header("X-Funbutton-Stt-Prompt", encode_header_value(p));
        }
        let res = req
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
        dev_dictionary: &[String],
    ) -> Result<CleanupOutcome> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()?;
        let body = CleanupRequest {
            model: model.to_string(),
            transcript: transcript.to_string(),
            mode: mode.to_string(),
            dictionary: dictionary.to_vec(),
            dev_dictionary: dev_dictionary.to_vec(),
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
    // Built-in dev vocabulary (Rust `DEV_DICTIONARY`), sent only in dev mode so
    // the Worker can inject the same normalize-to-these-spellings block the
    // on-device path uses. Empty in non-dev modes — the Worker omits the block.
    dev_dictionary: Vec<String>,
}

/// Percent-encode a string so it is a valid HTTP header value the Worker can
/// recover with `decodeURIComponent`. Encodes every byte that is not an
/// unreserved ASCII alphanumeric — the result is pure ASCII (letters, digits,
/// and `%XX`), so it round-trips any UTF-8 (e.g. an accented user-dictionary
/// term) without a percent-encoding dependency.
fn encode_header_value(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for &b in s.as_bytes() {
        if b.is_ascii_alphanumeric() {
            out.push(b as char);
        } else {
            out.push('%');
            out.push(
                char::from_digit((b >> 4) as u32, 16)
                    .unwrap()
                    .to_ascii_uppercase(),
            );
            out.push(
                char::from_digit((b & 0x0f) as u32, 16)
                    .unwrap()
                    .to_ascii_uppercase(),
            );
        }
    }
    out
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

#[cfg(test)]
mod tests {
    use super::encode_header_value;

    #[test]
    fn encodes_to_pure_ascii_valid_header_bytes() {
        // The STT prompt is a comma-joined term list (see build_stt_prompt).
        let out = encode_header_value("git, GitHub, Node.js, kubectl");
        // Every output byte must be a valid HTTP header value byte (visible
        // ASCII), so reqwest never rejects it.
        assert!(out.bytes().all(|b| b.is_ascii_graphic()));
        assert!(out.is_ascii());
    }

    #[test]
    fn matches_decode_uri_component_semantics() {
        // Exactly what the Worker's decodeURIComponent must reverse: alnum stays
        // literal, everything else is %XX (uppercase) of the UTF-8 bytes.
        assert_eq!(encode_header_value("git, GitHub"), "git%2C%20GitHub");
        assert_eq!(encode_header_value("Node.js"), "Node%2Ejs");
        assert_eq!(encode_header_value("kebab-case"), "kebab%2Dcase");
        // Non-ASCII round-trips as percent-encoded UTF-8 (é = C3 A9), so an
        // accented user-dictionary term never breaks the header.
        assert_eq!(encode_header_value("café"), "caf%C3%A9");
    }
}
