// Embedded local inference: spawns the bundled llama-server (llama.cpp) as a
// child process at app startup, loads the bundled Qwen 2.5 1.5B GGUF, and
// exposes an OpenAI-compatible /v1/chat/completions endpoint on a random
// localhost port.
//
// This makes FunButton genuinely zero-install + zero-key on first launch: no
// Groq account, no Ollama install, no internet required for cleanup.

use anyhow::{anyhow, Context as _, Result};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub const BUNDLED_MODEL_FILE: &str = "qwen2.5-1.5b-instruct-q4_k_m.gguf";
const SERVER_BIN: &str = "llama-server";
const STARTUP_TIMEOUT: Duration = Duration::from_secs(90);

/// Locates the vendored llama-server binary + GGUF. Returns (server_path, gguf_path).
///
/// In development (`cargo run` / `tauri dev`) the path resolves to
/// `src-tauri/vendor/llama/`. In a bundled app it resolves to
/// `Contents/Resources/_up_/vendor/llama/` via Tauri's resource API.
fn locate_vendor(app: &tauri::AppHandle) -> Result<(PathBuf, PathBuf)> {
    use tauri::Manager as _;
    // Bundled app: Tauri copies declared resources under Contents/Resources/.
    if let Ok(resource_dir) = app.path().resource_dir() {
        let bundled = resource_dir.join("vendor").join("llama");
        if bundled.join(SERVER_BIN).exists() {
            return Ok((bundled.join(SERVER_BIN), bundled.join(BUNDLED_MODEL_FILE)));
        }
    }
    // Dev: walk up from CARGO_MANIFEST_DIR to find src-tauri/vendor/llama.
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    if !manifest.is_empty() {
        let dev = PathBuf::from(manifest).join("vendor").join("llama");
        if dev.join(SERVER_BIN).exists() {
            return Ok((dev.join(SERVER_BIN), dev.join(BUNDLED_MODEL_FILE)));
        }
    }
    // Last resort: try CWD-relative.
    let cwd = std::env::current_dir().unwrap_or_default().join("vendor").join("llama");
    if cwd.join(SERVER_BIN).exists() {
        return Ok((cwd.join(SERVER_BIN), cwd.join(BUNDLED_MODEL_FILE)));
    }
    Err(anyhow!(
        "could not locate vendor/llama — run scripts/fetch-vendor-deps.sh"
    ))
}

fn pick_free_port() -> Result<u16> {
    let l = TcpListener::bind("127.0.0.1:0").context("bind ephemeral port")?;
    Ok(l.local_addr()?.port())
}

/// Handle to a running llama-server. Drop kills the child.
pub struct EmbeddedServer {
    base_url: String,
    child: Mutex<Option<Child>>,
}

impl EmbeddedServer {
    pub fn base_url(&self) -> &str { &self.base_url }

    /// Spawn llama-server pointing at the bundled GGUF. Returns once /health
    /// responds 200 or after STARTUP_TIMEOUT.
    pub async fn spawn(app: &tauri::AppHandle) -> Result<Self> {
        let (bin, gguf) = locate_vendor(app)?;
        if !gguf.exists() {
            return Err(anyhow!(
                "bundled GGUF not found at {} — run scripts/fetch-vendor-deps.sh",
                gguf.display()
            ));
        }
        let port = pick_free_port()?;
        let base_url = format!("http://127.0.0.1:{port}");
        log::info!("spawning llama-server on {base_url} with model {}", gguf.display());

        // Flags: short ctx (cleanup is bounded), no warmup spam, OpenAI-compatible
        // chat at /v1/chat/completions enabled by default.
        let child = Command::new(&bin)
            .arg("--host").arg("127.0.0.1")
            .arg("--port").arg(port.to_string())
            .arg("--model").arg(&gguf)
            .arg("--ctx-size").arg("4096")
            .arg("--threads").arg(num_threads().to_string())
            .arg("--no-webui")     // skip the bundled chat UI
            .arg("--log-disable")  // we capture via stdout/stderr inheritance instead
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .spawn()
            .with_context(|| format!("spawn llama-server at {}", bin.display()))?;

        let server = EmbeddedServer {
            base_url: base_url.clone(),
            child: Mutex::new(Some(child)),
        };

        // Poll /health until ready (or timeout). Server takes a few seconds on
        // first launch — Metal compile + GGUF mmap.
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(800))
            .build()?;
        let started = Instant::now();
        loop {
            if started.elapsed() > STARTUP_TIMEOUT {
                server.kill();
                return Err(anyhow!("llama-server did not become ready within {}s", STARTUP_TIMEOUT.as_secs()));
            }
            if let Ok(r) = client.get(format!("{base_url}/health")).send().await {
                if r.status().is_success() {
                    log::info!("llama-server ready at {base_url} ({}ms)", started.elapsed().as_millis());
                    break;
                }
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        Ok(server)
    }

    /// Quick liveness probe.
    #[allow(dead_code)]
    pub async fn is_alive(&self) -> bool {
        let client = match reqwest::Client::builder()
            .timeout(Duration::from_millis(800))
            .build()
        {
            Ok(c) => c,
            Err(_) => return false,
        };
        client
            .get(format!("{}/health", self.base_url))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    /// Run a cleanup roundtrip. Same OpenAI chat shape we use for Groq.
    pub async fn generate(&self, system: &str, user: &str) -> Result<String> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()?;
        let body = ChatRequest {
            messages: vec![
                ChatMessage { role: "system", content: system.to_string() },
                ChatMessage { role: "user", content: user.to_string() },
            ],
            temperature: 0.2,
            max_tokens: 1024,
            stream: false,
        };
        let resp = client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .json(&body)
            .send()
            .await
            .context("embedded llama chat request failed")?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(anyhow!("embedded llama {}: {}", status, text));
        }
        let parsed: ChatResponse =
            serde_json::from_str(&text).context("parse embedded llama chat response")?;
        let content = parsed
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .unwrap_or_default()
            .trim()
            .to_string();
        Ok(content)
    }

    pub fn kill(&self) {
        if let Some(mut c) = self.child.lock().take() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

impl Drop for EmbeddedServer {
    fn drop(&mut self) { self.kill(); }
}

#[derive(Serialize)]
struct ChatRequest {
    messages: Vec<ChatMessage>,
    temperature: f32,
    max_tokens: u32,
    stream: bool,
}

#[derive(Serialize)]
struct ChatMessage {
    role: &'static str,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatChoiceMessage,
}

#[derive(Deserialize)]
struct ChatChoiceMessage {
    content: String,
}

/// Use ~half the cores so we don't starve audio + the rest of the OS.
fn num_threads() -> usize {
    let n = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    std::cmp::max(2, n / 2)
}

/// Shared handle that can live in AppState across async tasks.
pub type EmbeddedServerHandle = Arc<EmbeddedServer>;
