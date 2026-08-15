// Embedded local inference: spawns the bundled llama-server (llama.cpp) as a
// child process at app startup, loads the bundled Qwen 2.5 1.5B GGUF, and
// exposes an OpenAI-compatible /v1/chat/completions endpoint on a random
// localhost port.
//
// This makes FunButton genuinely zero-install + zero-key on first launch: no
// Groq account, no Ollama install, no internet required for cleanup.

use anyhow::{anyhow, Context as _, Result};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

const SERVER_BIN: &str = "llama-server";
const STARTUP_TIMEOUT: Duration = Duration::from_secs(90);

/// Process-wide registry of live llama-server child PIDs, recorded the instant
/// each child is spawned — BEFORE the /health poll finishes and before the
/// `EmbeddedServer` handle is stored in `AppState`. Quit teardown drains this
/// to reap any child whose startup was still in flight (its handle never made
/// it into `AppState`, so the normal handle-kill would miss it and orphan it).
static LLAMA_PIDS: Lazy<Mutex<Vec<u32>>> = Lazy::new(|| Mutex::new(Vec::new()));

/// Serializes "am I shutting down? if not, spawn + register the child" against
/// "set shutting-down + reap". Held only around the cheap spawn/register (never
/// across the /health poll), so it can't stall startup. This is what makes
/// orphaning impossible: a spawn either registers its PID before the reap sees
/// it, or observes `SHUTTING_DOWN` and never spawns at all.
static SPAWN_GATE: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));
static SHUTTING_DOWN: AtomicBool = AtomicBool::new(false);

/// Quit-teardown entry point: latch "no new servers", then SIGKILL every
/// llama-server child still registered as live. Under `SPAWN_GATE`, so a spawn
/// racing this either already registered its PID (reaped here) or will see the
/// latch and skip. Idempotent.
pub fn begin_shutdown_and_reap() {
    let _gate = SPAWN_GATE.lock();
    SHUTTING_DOWN.store(true, Ordering::SeqCst);
    let pids: Vec<u32> = LLAMA_PIDS.lock().drain(..).collect();
    for pid in pids {
        log::warn!("reaping orphaned llama-server pid {pid}");
        let _ = Command::new("kill").arg("-9").arg(pid.to_string()).status();
    }
}

/// Locates the bundled llama-server binary (still shipped inside the .app — it's
/// small, signed runtime code). The GGUF is NO LONGER bundled; it's downloaded
/// into Application Support and passed to `spawn` by the caller.
///
/// In development (`cargo run` / `tauri dev`) it resolves to
/// `src-tauri/vendor/llama/`. In a bundled app it resolves to
/// `Contents/Resources/vendor/llama/` via Tauri's resource API.
fn locate_server_bin(app: &tauri::AppHandle) -> Result<PathBuf> {
    use tauri::Manager as _;
    // Bundled app: Tauri copies declared resources under Contents/Resources/.
    if let Ok(resource_dir) = app.path().resource_dir() {
        let bundled = resource_dir.join("vendor").join("llama").join(SERVER_BIN);
        if bundled.exists() {
            return Ok(bundled);
        }
    }
    // Dev: walk up from CARGO_MANIFEST_DIR to find src-tauri/vendor/llama.
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    if !manifest.is_empty() {
        let dev = PathBuf::from(manifest)
            .join("vendor")
            .join("llama")
            .join(SERVER_BIN);
        if dev.exists() {
            return Ok(dev);
        }
    }
    // Last resort: try CWD-relative.
    let cwd = std::env::current_dir()
        .unwrap_or_default()
        .join("vendor")
        .join("llama")
        .join(SERVER_BIN);
    if cwd.exists() {
        return Ok(cwd);
    }
    Err(anyhow!(
        "could not locate the bundled llama-server binary (vendor/llama/{SERVER_BIN})"
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
    /// The child's PID, also registered in `LLAMA_PIDS` so quit teardown can
    /// reap it even if this handle never reached `AppState`.
    pid: u32,
}

impl EmbeddedServer {
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Spawn llama-server pointing at `gguf` (the active cleanup model in
    /// Application Support, resolved by the caller). Returns once /health
    /// responds 200 or after STARTUP_TIMEOUT.
    pub async fn spawn(app: &tauri::AppHandle, gguf: PathBuf) -> Result<Self> {
        let bin = locate_server_bin(app)?;
        if !gguf.exists() {
            return Err(anyhow!(
                "cleanup model not present at {} — download it in Settings → Models",
                gguf.display()
            ));
        }
        let port = pick_free_port()?;
        let base_url = format!("http://127.0.0.1:{port}");
        log::info!(
            "spawning llama-server on {base_url} with model {}",
            gguf.display()
        );

        // Spawn + PID-register under SPAWN_GATE, checking the shutdown latch
        // inside the lock. If teardown already began, don't spawn at all — this
        // is what closes the "child created after the reap ran" race. The gate
        // is released before the /health poll, so startup latency is unchanged.
        let (child, pid) = {
            let _gate = SPAWN_GATE.lock();
            if SHUTTING_DOWN.load(Ordering::SeqCst) {
                return Err(anyhow!("app is shutting down — not starting llama-server"));
            }
            // Flags: short ctx (cleanup is bounded), no warmup spam,
            // OpenAI-compatible chat at /v1/chat/completions enabled by default.
            let child = Command::new(&bin)
                .arg("--host")
                .arg("127.0.0.1")
                .arg("--port")
                .arg(port.to_string())
                .arg("--model")
                .arg(&gguf)
                .arg("--ctx-size")
                .arg("4096")
                .arg("--threads")
                .arg(num_threads().to_string())
                .arg("--no-webui") // skip the bundled chat UI
                .arg("--log-disable") // we capture via stdout/stderr inheritance instead
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .stdin(Stdio::null())
                .spawn()
                .with_context(|| format!("spawn llama-server at {}", bin.display()))?;
            let pid = child.id();
            LLAMA_PIDS.lock().push(pid);
            (child, pid)
        };

        let server = EmbeddedServer {
            base_url: base_url.clone(),
            child: Mutex::new(Some(child)),
            pid,
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
                return Err(anyhow!(
                    "llama-server did not become ready within {}s",
                    STARTUP_TIMEOUT.as_secs()
                ));
            }
            if let Ok(r) = client.get(format!("{base_url}/health")).send().await {
                if r.status().is_success() {
                    log::info!(
                        "llama-server ready at {base_url} ({}ms)",
                        started.elapsed().as_millis()
                    );
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
                ChatMessage {
                    role: "system",
                    content: system.to_string(),
                },
                ChatMessage {
                    role: "user",
                    content: user.to_string(),
                },
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
        // Deregister so quit teardown's orphan sweep doesn't try to re-kill a
        // PID we've already reaped (which could by then belong to something else).
        LLAMA_PIDS.lock().retain(|&p| p != self.pid);
    }
}

impl Drop for EmbeddedServer {
    fn drop(&mut self) {
        self.kill();
    }
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
