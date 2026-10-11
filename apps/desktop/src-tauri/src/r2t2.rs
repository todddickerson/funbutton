// Experimental on-device STT: Confucius4-R2T2 (Qwen3-ASR architecture, MLX
// 4-bit) via a python sidecar running mlx-audio. Opt-in from Settings → Models;
// whisper-base.en stays the default STT engine.
//
// Why a sidecar: no production no-python runtime loads this architecture with
// hotword biasing today (mlx-audio-swift supports Qwen3-ASR but has no hotword
// input; Rust MLX bindings would need an architecture port). Hotwords are the
// entire point of this engine — it spells the user's dictionary and on-screen
// terms correctly — so we run the proven python path, managed exactly like the
// bundled llama-server child (PID registry + spawn gate + quit reap).
//
// Self-containment rules honored here:
// - NOTHING goes inside the .app bundle: the runtime (uv + a uv-managed
//   CPython + a pinned venv) lives in Application Support next to the model
//   store. Deleting the app leaves no hidden ~/.local or ~/.cache debris
//   (UV_PYTHON_INSTALL_DIR/UV_CACHE_DIR are pinned under the runtime dir).
// - Every byte is pinned: the uv tarball by SHA-256, python by uv's
//   `only-managed` 3.12 channel, and every wheel by `==` version.
// - The server script is baked into this binary (include_str!) and rewritten
//   at every spawn, so script and app can never drift.
//
// PRIVACY: hotwords (user dictionary + focused-window title tokens) are sent
// only to 127.0.0.1 and never logged or persisted — not here, not in the
// python script (request logging is disabled there).

use anyhow::{anyhow, Context as _, Result};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde::Deserialize;
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::Emitter;

/// Pinned uv release (the only bootstrap download; everything after is pinned
/// by uv itself). SHA-256 verified before the tarball is ever extracted.
const UV_VERSION: &str = "0.13.0";
const UV_URL: &str =
    "https://github.com/astral-sh/uv/releases/download/0.13.0/uv-aarch64-apple-darwin.tar.gz";
const UV_TAR_SHA256: &str = "a9c1b29002cf3c83f07fa9cd8a887a3be0107d90e23189721221e7257db8e3d6";

/// Python channel for the uv-managed interpreter. `only-managed` keeps the
/// venv off Homebrew/system pythons (a `brew upgrade` removing python@3.12
/// would break a symlinked venv).
const PYTHON_SERIES: &str = "3.12";

/// The exact environment the R2T2 spike proved on this hardware
/// (2026-09-21, re-proven from scratch 2026-10-11): mlx-audio 0.5.5 pins.
const REQUIREMENTS: &str = "\
annotated-doc==0.0.5
anyio==4.15.1
certifi==2026.7.22
cffi==2.1.1
click==8.5.0
filelock==4.0.1
fsspec==2026.9.0
h11==0.16.0
hf-xet==1.6.0
httpcore==1.0.9
httpx==0.28.1
huggingface-hub==1.32.0
idna==3.20
markdown-it-py==4.2.0
mdurl==0.1.2
miniaudio==1.71
mlx==0.32.2
mlx-audio==0.5.5
mlx-metal==0.32.2
numpy==2.5.3
packaging==26.3
pycparser==3.0
pygments==2.21.0
pyyaml==6.0.3
regex==2026.9.10
rich==15.0.0
safetensors==0.8.0
scipy==1.18.1
shellingham==1.5.4
sounddevice==0.5.6
soundfile==0.14.0
tokenizers==0.23.2
tqdm==4.70.1
transformers==5.17.0
typer==0.27.2
typing-extensions==4.16.0
";

/// The sidecar HTTP server, baked into the binary and written out at spawn.
const SERVER_PY: &str = include_str!("r2t2_server.py");

/// First health-OK includes python import + weight load (~2 s warm, ~20 s on a
/// cold page cache); generous ceiling, and a load *failure* surfaces long
/// before this via /health 500.
const STARTUP_TIMEOUT: Duration = Duration::from_secs(240);

// Same orphan-proof lifecycle as embedded_llm: PID registry + a gate that
// serializes "check shutting-down + spawn + register" against "latch + reap".
static R2T2_PIDS: Lazy<Mutex<Vec<u32>>> = Lazy::new(|| Mutex::new(Vec::new()));
static SPAWN_GATE: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));
static SHUTTING_DOWN: AtomicBool = AtomicBool::new(false);

/// Quit-teardown entry point — see `embedded_llm::begin_shutdown_and_reap`.
pub fn begin_shutdown_and_reap() {
    let _gate = SPAWN_GATE.lock();
    SHUTTING_DOWN.store(true, Ordering::SeqCst);
    let pids: Vec<u32> = R2T2_PIDS.lock().drain(..).collect();
    for pid in pids {
        log::warn!("reaping orphaned r2t2 sidecar pid {pid}");
        let _ = Command::new("kill").arg("-9").arg(pid.to_string()).status();
    }
}

/// `…/Application Support/ai.funbutton.desktop/r2t2-runtime` — a sibling of
/// the model store (so the `FUNBUTTON_MODELS_DIR` test override relocates both
/// together). Never inside the .app.
pub fn runtime_dir() -> PathBuf {
    let models = crate::models::models_dir();
    models
        .parent()
        .map(|p| p.join("r2t2-runtime"))
        .unwrap_or_else(|| models.join("r2t2-runtime"))
}

pub fn venv_python() -> PathBuf {
    runtime_dir().join("venv").join("bin").join("python")
}

fn stamp_path() -> PathBuf {
    runtime_dir().join("provisioned.json")
}

fn stamp_value() -> String {
    // Any change to the pins invalidates the stamp → reprovision on next use.
    let req_hash = {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(REQUIREMENTS.as_bytes());
        hex::encode(h.finalize())
    };
    format!("uv={UV_VERSION};py={PYTHON_SERIES};reqs={req_hash}")
}

/// True when the venv exists and was provisioned with the exact current pins.
pub fn runtime_ready() -> bool {
    venv_python().exists()
        && std::fs::read_to_string(stamp_path())
            .map(|s| s == stamp_value())
            .unwrap_or(false)
}

fn emit_runtime_stage(app: &tauri::AppHandle, stage: &str) {
    log::info!("r2t2 runtime: {stage}");
    let _ = app.emit("funbutton:r2t2-runtime", stage.to_string());
}

/// Provision the self-contained runtime: pinned uv → uv-managed CPython →
/// pinned venv. Idempotent (stamped); ~500 MB of downloads on first run, all
/// under the runtime dir. Blocking work rides spawn_blocking.
pub async fn provision(app: &tauri::AppHandle) -> Result<()> {
    if runtime_ready() {
        return Ok(());
    }
    let dir = runtime_dir();
    std::fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;

    // 1. Fetch + verify + extract uv (idempotent: skip when already unpacked
    //    at the pinned version).
    let uv_bin = dir.join("uv").join("uv");
    let uv_ok = Command::new(&uv_bin)
        .arg("--version")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains(UV_VERSION))
        .unwrap_or(false);
    if !uv_ok {
        emit_runtime_stage(app, "downloading uv");
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(300))
            .build()?;
        let bytes = client
            .get(UV_URL)
            .send()
            .await
            .context("download uv")?
            .error_for_status()?
            .bytes()
            .await?;
        {
            use sha2::{Digest, Sha256};
            let got = hex::encode(Sha256::digest(&bytes));
            if !got.eq_ignore_ascii_case(UV_TAR_SHA256) {
                return Err(anyhow!("uv tarball sha256 mismatch: got {got}"));
            }
        }
        let tar = dir.join("uv.tar.gz");
        std::fs::write(&tar, &bytes)?;
        let uv_dir = dir.join("uv");
        std::fs::create_dir_all(&uv_dir)?;
        let out = tokio::task::spawn_blocking({
            let tar = tar.clone();
            let uv_dir = uv_dir.clone();
            move || {
                Command::new("/usr/bin/tar")
                    .arg("-xzf")
                    .arg(&tar)
                    .arg("-C")
                    .arg(&uv_dir)
                    .arg("--strip-components=1")
                    .output()
            }
        })
        .await??;
        let _ = std::fs::remove_file(&tar);
        if !out.status.success() {
            return Err(anyhow!(
                "uv extract failed: {}",
                String::from_utf8_lossy(&out.stderr)
            ));
        }
    }

    // 2. venv with a uv-managed CPython, then the pinned wheel set. All uv
    //    state (interpreters, wheel cache) stays under the runtime dir.
    let reqs = dir.join("requirements.txt");
    std::fs::write(&reqs, REQUIREMENTS)?;
    let venv = dir.join("venv");
    let run_uv = {
        let uv_bin = uv_bin.clone();
        let dir = dir.clone();
        move |args: Vec<String>| {
            let uv_bin = uv_bin.clone();
            let dir = dir.clone();
            tokio::task::spawn_blocking(move || {
                Command::new(&uv_bin)
                    .args(&args)
                    .env("UV_PYTHON_INSTALL_DIR", dir.join("python"))
                    .env("UV_CACHE_DIR", dir.join("cache"))
                    .env("UV_NO_CONFIG", "1")
                    .env("UV_PYTHON_PREFERENCE", "only-managed")
                    .stdin(Stdio::null())
                    .output()
            })
        }
    };
    emit_runtime_stage(app, "installing python 3.12 (uv-managed)");
    let out = run_uv(vec![
        "venv".into(),
        "--python".into(),
        PYTHON_SERIES.into(),
        venv.to_string_lossy().into_owned(),
    ])
    .await??;
    if !out.status.success() {
        return Err(anyhow!(
            "uv venv failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    emit_runtime_stage(app, "installing pinned wheels (mlx-audio 0.5.5)");
    let out = run_uv(vec![
        "pip".into(),
        "install".into(),
        "--python".into(),
        venv.join("bin")
            .join("python")
            .to_string_lossy()
            .into_owned(),
        "-r".into(),
        reqs.to_string_lossy().into_owned(),
    ])
    .await??;
    if !out.status.success() {
        return Err(anyhow!(
            "uv pip install failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    // The wheel cache (~350 MB) only speeds up a RE-provision (pin bump);
    // drop it so the one-time setup costs ~480 MB on disk, not ~850 MB.
    let _ = run_uv(vec!["cache".into(), "clean".into()]).await;

    std::fs::write(stamp_path(), stamp_value())?;
    emit_runtime_stage(app, "runtime ready");
    Ok(())
}

fn pick_free_port() -> Result<u16> {
    let l = TcpListener::bind("127.0.0.1:0").context("bind ephemeral port")?;
    Ok(l.local_addr()?.port())
}

/// Handle to a running R2T2 sidecar. Drop kills the child.
pub struct R2t2Server {
    base_url: String,
    child: Mutex<Option<Child>>,
    pid: u32,
}

pub type R2t2ServerHandle = Arc<R2t2Server>;

#[derive(Deserialize)]
struct TranscribeResponse {
    text: String,
}

#[derive(Deserialize)]
struct HealthResponse {
    #[allow(dead_code)] // wire field; only `error` is consumed
    status: String,
    #[serde(default)]
    error: Option<String>,
}

impl R2t2Server {
    #[allow(dead_code)] // symmetry with EmbeddedServer; used by diagnostics
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Spawn the sidecar on the provisioned venv, pointing at `model_dir`
    /// (the multi-file R2T2 model in the store). Returns once /health is 200.
    /// Fails fast if the model load errors (/health 500).
    pub async fn spawn(model_dir: PathBuf) -> Result<Self> {
        let python = venv_python();
        if !python.exists() {
            return Err(anyhow!("r2t2 runtime not provisioned"));
        }
        if !model_dir.join("model.safetensors").exists() {
            return Err(anyhow!(
                "R2T2 model not present at {} — download it in Settings → Models",
                model_dir.display()
            ));
        }
        // Keep script-on-disk in lockstep with the binary.
        let script = runtime_dir().join("r2t2_server.py");
        std::fs::write(&script, SERVER_PY).context("write r2t2_server.py")?;

        let port = pick_free_port()?;
        let base_url = format!("http://127.0.0.1:{port}");
        log::info!(
            "spawning r2t2 sidecar on {base_url} with model {}",
            model_dir.display()
        );

        let (child, pid) = {
            let _gate = SPAWN_GATE.lock();
            if SHUTTING_DOWN.load(Ordering::SeqCst) {
                return Err(anyhow!("app is shutting down — not starting r2t2 sidecar"));
            }
            let child = Command::new(&python)
                .arg(&script)
                .arg(port.to_string())
                .arg(&model_dir)
                // The model is fully local — never let huggingface_hub phone
                // home for metadata.
                .env("HF_HUB_OFFLINE", "1")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .stdin(Stdio::null())
                .spawn()
                .with_context(|| format!("spawn r2t2 sidecar via {}", python.display()))?;
            let pid = child.id();
            R2T2_PIDS.lock().push(pid);
            (child, pid)
        };

        let server = R2t2Server {
            base_url: base_url.clone(),
            child: Mutex::new(Some(child)),
            pid,
        };

        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(800))
            .build()?;
        let started = Instant::now();
        loop {
            if started.elapsed() > STARTUP_TIMEOUT {
                server.kill();
                return Err(anyhow!(
                    "r2t2 sidecar did not become ready within {}s",
                    STARTUP_TIMEOUT.as_secs()
                ));
            }
            if let Ok(r) = client.get(format!("{base_url}/health")).send().await {
                let code = r.status();
                if code.is_success() {
                    log::info!(
                        "r2t2 sidecar ready at {base_url} ({}ms)",
                        started.elapsed().as_millis()
                    );
                    break;
                }
                // 500 = the model load failed — no point waiting the timeout.
                if code.as_u16() == 500 {
                    let detail = r
                        .json::<HealthResponse>()
                        .await
                        .ok()
                        .and_then(|h| h.error)
                        .unwrap_or_else(|| "model load failed".into());
                    server.kill();
                    return Err(anyhow!("r2t2 model load failed: {detail}"));
                }
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        Ok(server)
    }

    /// Transcribe an in-memory WAV. `hotwords` bias proper-noun/identifier
    /// spelling; they ride a percent-encoded localhost header and are never
    /// logged.
    pub async fn transcribe(&self, wav: Vec<u8>, hotwords: &[String]) -> Result<String> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()?;
        let mut req = client
            .post(format!("{}/transcribe", self.base_url))
            .header("Content-Type", "audio/wav");
        if !hotwords.is_empty() {
            let joined = hotwords.join(",");
            req = req.header(
                "X-Funbutton-Hotwords",
                crate::cloud::encode_header_value(&joined),
            );
        }
        let resp = req
            .body(wav)
            .send()
            .await
            .context("r2t2 transcribe request")?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            // Truncate the body: a python exception string could embed prompt
            // content (hotwords), and this error reaches the app log.
            let brief: String = text.chars().take(160).collect();
            return Err(anyhow!("r2t2 transcribe {status}: {brief}"));
        }
        let parsed: TranscribeResponse =
            serde_json::from_str(&text).context("parse r2t2 transcribe response")?;
        Ok(parsed.text)
    }

    pub fn kill(&self) {
        if let Some(mut c) = self.child.lock().take() {
            let _ = c.kill();
            let _ = c.wait();
        }
        R2T2_PIDS.lock().retain(|&p| p != self.pid);
    }
}

impl Drop for R2t2Server {
    fn drop(&mut self) {
        self.kill();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_dir_is_store_sibling_never_in_bundle() {
        let d = runtime_dir();
        assert!(d.ends_with("r2t2-runtime"));
        let exe = std::env::current_exe().unwrap();
        assert!(!d.starts_with(exe.parent().unwrap()));
    }

    #[test]
    fn stamp_changes_when_pins_change() {
        // The stamp must encode uv + python + the full requirement set, so any
        // pin bump reprovisions instead of reusing a stale venv.
        let v = stamp_value();
        assert!(v.contains(UV_VERSION));
        assert!(v.contains(PYTHON_SERIES));
        assert!(v.contains("reqs="));
    }

    /// Live end-to-end sidecar proof (ignored: needs the provisioned runtime +
    /// the downloaded model). Spawns the real server, transcribes a fixture
    /// with and without hotwords, asserts the hotword effect, kills the child.
    /// `cargo test --release --lib r2t2_sidecar_live -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn r2t2_sidecar_live() {
        let model_dir = std::env::var("FUNBUTTON_R2T2_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| crate::models::models_dir().join("confucius4-r2t2-mlx-4bit"));
        let wav = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/stt/code.wav");
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let srv = R2t2Server::spawn(model_dir).await.expect("spawn sidecar");
            let bytes = std::fs::read(&wav).expect("fixture wav");
            let plain = srv.transcribe(bytes.clone(), &[]).await.expect("plain");
            // Decoding is greedy (temperature 0) but the hotword list is
            // folded into the system prompt, so ORDER changes the output —
            // this is the spike's proven list/order (bench-stt.py's default).
            let hot = srv
                .transcribe(
                    bytes,
                    &[
                        "ClickFunnels".into(),
                        "auth_middleware.rs".into(),
                        "kubectl".into(),
                    ],
                )
                .await
                .expect("hotwords");
            eprintln!("PLAIN: {plain}\nHOT:   {hot}");
            assert_ne!(plain, hot, "hotwords must bias the output");
            assert!(
                hot.to_lowercase().contains("kubectl"),
                "hotword biasing should produce 'kubectl', got: {hot}"
            );
            srv.kill();
        });
    }
}
