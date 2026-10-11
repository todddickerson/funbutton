//! Model manager: manifest, on-disk model store, resumable downloads, and
//! migration from the old bundled layout.
//!
//! FunButton no longer ships the .gguf files inside the .app (that was ~1.2 GB
//! for a 17 MB app, and — critically — writing into the bundle later would
//! break code signatures). Models now live in Application Support
//! (`~/Library/Application Support/ai.funbutton.desktop/models/`), downloaded
//! on first run and verified against a pinned SHA-256 manifest.
//!
//! Design constraints honored here:
//! - Nothing is ever written inside the .app bundle. `models_dir()` is always
//!   under Application Support (or the `FUNBUTTON_MODELS_DIR` test override).
//! - A download is only accepted after its full-file SHA-256 matches the
//!   manifest — a truncated/corrupt gguf fails loudly, never half-works.
//! - Downloads resume via HTTP Range (`.part` file) and are cancellable; the
//!   quit teardown cancels any in flight (see `crate::shutdown`).

use anyhow::{anyhow, Context as _, Result};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

/// Bundle identifier — must match `tauri.conf.json` `identifier`. Used to build
/// the Application Support path without needing an `AppHandle` (so the offline
/// test and any non-Tauri context resolve the same directory the app uses).
pub const APP_IDENTIFIER: &str = "ai.funbutton.desktop";

/// Live manifest, fetched at startup so new models can be added without an app
/// update. Falls back to the baked-in copy if the network fetch fails.
const MANIFEST_URL: &str =
    "https://raw.githubusercontent.com/todddickerson/funbutton/main/apps/desktop/src-tauri/models_manifest.json";

/// Baked-in fallback — compiled into the binary.
const BAKED_MANIFEST: &str = include_str!("../models_manifest.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Stt,
    Cleanup,
}

impl Role {
    pub fn label(self) -> &'static str {
        match self {
            Role::Stt => "stt",
            Role::Cleanup => "cleanup",
        }
    }
}

/// Which engine executes a model. `Builtin` = the compiled-in engines
/// (whisper.cpp for STT, llama-server for cleanup); `MlxR2t2` = the
/// experimental python MLX sidecar (see `crate::r2t2`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ModelRuntime {
    #[default]
    Builtin,
    MlxR2t2,
}

/// One file of a multi-file model (schema v2). Lands in the store under
/// `<models_dir>/<entry.filename>/<file.filename>`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelFile {
    pub filename: String,
    pub url: String,
    pub sha256: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelEntry {
    pub id: String,
    pub role: Role,
    pub name: String,
    /// Single-file model: the file name in the store. Multi-file model
    /// (`files` non-empty): the store *subdirectory* name.
    pub filename: String,
    pub url: String,
    /// Single-file models only; empty when `files` carries per-file hashes.
    pub sha256: String,
    /// Single-file: the file's size. Multi-file: the sum of `files` sizes
    /// (pinned by a manifest test).
    pub size_bytes: u64,
    #[serde(default)]
    pub blurb: String,
    #[serde(default)]
    pub license: String,
    #[serde(default)]
    pub default: bool,
    #[serde(default)]
    pub recommended: bool,
    #[serde(default = "default_min_app")]
    pub min_app_version: String,
    #[serde(default)]
    pub runtime: ModelRuntime,
    /// Schema v2: multi-file models (e.g. an MLX safetensors directory).
    /// Empty for classic single-gguf entries.
    #[serde(default)]
    pub files: Vec<ModelFile>,
}

fn default_min_app() -> String {
    "0.0.0".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub schema_version: u32,
    pub models: Vec<ModelEntry>,
}

impl Manifest {
    /// Parse the baked-in manifest. Panics only if the compiled-in JSON is
    /// malformed — a build-time invariant, caught by `cargo test`.
    pub fn baked() -> Manifest {
        serde_json::from_str(BAKED_MANIFEST).expect("baked-in models_manifest.json is valid")
    }

    pub fn entry(&self, id: &str) -> Option<&ModelEntry> {
        self.models.iter().find(|m| m.id == id)
    }

    /// Active model for a role: the id the user picked, else the manifest
    /// default, else the first of that role. Never returns None if the manifest
    /// has any model for the role.
    pub fn active_entry(&self, role: Role, chosen_id: &str) -> Option<&ModelEntry> {
        self.models
            .iter()
            .find(|m| m.role == role && m.id == chosen_id)
            .or_else(|| self.models.iter().find(|m| m.role == role && m.default))
            .or_else(|| self.models.iter().find(|m| m.role == role))
    }

    #[allow(dead_code)] // exercised by tests; handy manifest API
    pub fn default_id(&self, role: Role) -> Option<String> {
        self.models
            .iter()
            .find(|m| m.role == role && m.default)
            .or_else(|| self.models.iter().find(|m| m.role == role))
            .map(|m| m.id.clone())
    }
}

// ----------------------------------------------------------------------------
// On-disk store
// ----------------------------------------------------------------------------

/// `~/Library/Application Support/ai.funbutton.desktop/models` — or the
/// `FUNBUTTON_MODELS_DIR` override (used by the offline test to point at a
/// throwaway dir). Never inside the .app bundle.
pub fn models_dir() -> PathBuf {
    if let Ok(over) = std::env::var("FUNBUTTON_MODELS_DIR") {
        if !over.is_empty() {
            return PathBuf::from(over);
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join(APP_IDENTIFIER)
        .join("models")
}

pub fn ensure_models_dir() -> Result<PathBuf> {
    let d = models_dir();
    std::fs::create_dir_all(&d).with_context(|| format!("create models dir {}", d.display()))?;
    Ok(d)
}

pub fn model_path(entry: &ModelEntry) -> PathBuf {
    models_dir().join(&entry.filename)
}

/// Cheap installed check: the file exists with the exact expected byte length
/// (every file, for multi-file models). Full SHA verification happens at
/// download/migration time (a finalized file is only ever produced after its
/// hash matched), so a length match here is a trustworthy, hash-free proxy
/// for the model manager list.
pub fn is_installed(entry: &ModelEntry) -> bool {
    if entry.files.is_empty() {
        return std::fs::metadata(model_path(entry))
            .map(|m| m.len() == entry.size_bytes)
            .unwrap_or(false);
    }
    let dir = model_path(entry);
    entry.files.iter().all(|f| {
        std::fs::metadata(dir.join(&f.filename))
            .map(|m| m.len() == f.size_bytes)
            .unwrap_or(false)
    })
}

/// Total bytes used by installed manifest models under the models dir.
pub fn disk_used(manifest: &Manifest) -> u64 {
    manifest
        .models
        .iter()
        .filter(|m| is_installed(m))
        .map(|m| m.size_bytes)
        .sum()
}

/// SHA-256 of a file, lowercase hex. Streams in 1 MiB blocks so a multi-GB gguf
/// never lands fully in memory.
pub fn sha256_file(path: &std::path::Path) -> Result<String> {
    let mut f = std::fs::File::open(path).with_context(|| format!("open {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1024 * 1024];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

// ----------------------------------------------------------------------------
// Manifest fetch (baked-in fallback)
// ----------------------------------------------------------------------------

/// Fetch the live manifest, falling back to the baked-in copy on any failure.
/// Only accepts a fetched manifest whose `schema_version` we understand and
/// that actually contains models, so a garbled response can never blank the
/// model list.
pub async fn fetch_manifest() -> Manifest {
    let baked = Manifest::baked();
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
    {
        Ok(c) => c,
        Err(_) => return baked,
    };
    match client.get(MANIFEST_URL).send().await {
        Ok(resp) if resp.status().is_success() => match resp.text().await {
            Ok(body) => match serde_json::from_str::<Manifest>(&body) {
                Ok(m) if m.schema_version == baked.schema_version && !m.models.is_empty() => {
                    log::info!("fetched live model manifest ({} models)", m.models.len());
                    m
                }
                Ok(m) => {
                    log::warn!(
                        "live manifest schema/shape mismatch (v{}, {} models) — using baked-in",
                        m.schema_version,
                        m.models.len()
                    );
                    baked
                }
                Err(e) => {
                    log::warn!("live manifest parse failed: {e} — using baked-in");
                    baked
                }
            },
            Err(e) => {
                log::warn!("live manifest read failed: {e} — using baked-in");
                baked
            }
        },
        Ok(resp) => {
            log::warn!("live manifest HTTP {} — using baked-in", resp.status());
            baked
        }
        Err(e) => {
            log::info!("live manifest fetch failed ({e}) — using baked-in (offline is fine)");
            baked
        }
    }
}

// ----------------------------------------------------------------------------
// Download manager
// ----------------------------------------------------------------------------

/// Tracks in-flight downloads so they can be cancelled (from Settings or on
/// quit) and so the UI can tell "downloading" from "not started".
#[derive(Default)]
pub struct DownloadManager {
    /// model id → cancel flag. Presence means "download in flight".
    active: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

pub type DownloadManagerHandle = Arc<DownloadManager>;

impl DownloadManager {
    pub fn new() -> DownloadManagerHandle {
        Arc::new(DownloadManager::default())
    }

    pub fn is_downloading(&self, id: &str) -> bool {
        self.active.lock().contains_key(id)
    }

    pub fn cancel(&self, id: &str) {
        if let Some(flag) = self.active.lock().get(id) {
            flag.store(true, Ordering::SeqCst);
            log::info!("model download cancel requested: {id}");
        }
    }

    /// Cancel every in-flight download — called from quit teardown so a hung
    /// download can't keep the process alive or leave a half-written file the
    /// next launch treats as progress (the `.part` is fine to resume).
    pub fn cancel_all(&self) {
        for (id, flag) in self.active.lock().iter() {
            flag.store(true, Ordering::SeqCst);
            log::info!("model download cancel-all: {id}");
        }
    }

    fn register(&self, id: &str) -> Arc<AtomicBool> {
        let flag = Arc::new(AtomicBool::new(false));
        self.active.lock().insert(id.to_string(), flag.clone());
        flag
    }

    fn deregister(&self, id: &str) {
        self.active.lock().remove(id);
    }
}

#[derive(Serialize, Clone)]
struct ProgressEvent {
    id: String,
    role: &'static str,
    status: &'static str, // "downloading" | "verifying" | "done" | "error" | "cancelled"
    downloaded: u64,
    total: u64,
    speed_bps: u64,
    eta_secs: u64,
    error: Option<String>,
}

fn emit_progress(app: Option<&AppHandle>, ev: ProgressEvent) {
    if let Some(app) = app {
        let _ = app.emit("funbutton:model-progress", ev);
    }
}

/// Download (or resume) a model into the models dir, verify its SHA-256, and
/// atomically move it into place. Emits `funbutton:model-progress` throughout.
///
/// - Resumable: appends to `<filename>.part` via an HTTP Range request.
/// - Cancellable: honors the manager's per-id flag between chunks.
/// - Retries transient network failures with capped exponential backoff,
///   resuming from whatever bytes already landed.
/// - Never accepts a file whose full-file hash doesn't match the manifest.
pub async fn download(
    app: AppHandle,
    manager: DownloadManagerHandle,
    entry: ModelEntry,
) -> Result<()> {
    let cancel = manager.register(&entry.id);
    let result = download_inner(Some(&app), &entry, &cancel).await;
    manager.deregister(&entry.id);

    match &result {
        Ok(()) => emit_progress(
            Some(&app),
            ProgressEvent {
                id: entry.id.clone(),
                role: entry.role.label(),
                status: "done",
                downloaded: entry.size_bytes,
                total: entry.size_bytes,
                speed_bps: 0,
                eta_secs: 0,
                error: None,
            },
        ),
        Err(e) => {
            let cancelled = cancel.load(Ordering::SeqCst);
            emit_progress(
                Some(&app),
                ProgressEvent {
                    id: entry.id.clone(),
                    role: entry.role.label(),
                    status: if cancelled { "cancelled" } else { "error" },
                    downloaded: bytes_on_disk(&entry),
                    total: entry.size_bytes,
                    speed_bps: 0,
                    eta_secs: 0,
                    error: Some(format!("{e:#}")),
                },
            );
        }
    }
    result
}

/// One concrete file to fetch: either the whole single-file model, or one
/// member of a multi-file model. `base` is how many bytes of the entry were
/// already completed by earlier files — progress events always speak in
/// whole-entry terms so the UI shows one coherent bar.
struct FileSpec {
    url: String,
    sha256: String,
    size_bytes: u64,
    dest: PathBuf,
    base: u64,
}

impl FileSpec {
    fn part(&self) -> PathBuf {
        self.dest.with_file_name(format!(
            "{}.part",
            self.dest.file_name().unwrap_or_default().to_string_lossy()
        ))
    }
}

/// Bytes of this entry currently on disk (finals + parts) — for honest
/// progress in the error/cancel event.
fn bytes_on_disk(entry: &ModelEntry) -> u64 {
    let len = |p: &std::path::Path| std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);
    if entry.files.is_empty() {
        let f = model_path(entry);
        let part = models_dir().join(format!("{}.part", entry.filename));
        return len(&f).max(len(&part));
    }
    let dir = model_path(entry);
    entry
        .files
        .iter()
        .map(|f| {
            let fin = dir.join(&f.filename);
            let part = dir.join(format!("{}.part", f.filename));
            len(&fin).max(len(&part))
        })
        .sum()
}

async fn download_inner(
    app: Option<&AppHandle>,
    entry: &ModelEntry,
    cancel: &AtomicBool,
) -> Result<()> {
    ensure_models_dir()?;
    if entry.files.is_empty() {
        let spec = FileSpec {
            url: entry.url.clone(),
            sha256: entry.sha256.clone(),
            size_bytes: entry.size_bytes,
            dest: model_path(entry),
            base: 0,
        };
        return download_file(app, entry, &spec, cancel).await;
    }
    // Multi-file model: everything lands in the entry's store subdirectory,
    // sequentially, each file resumable + SHA-verified on its own.
    let dir = model_path(entry);
    std::fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
    let mut base = 0u64;
    for f in &entry.files {
        let spec = FileSpec {
            url: f.url.clone(),
            sha256: f.sha256.clone(),
            size_bytes: f.size_bytes,
            dest: dir.join(&f.filename),
            base,
        };
        download_file(app, entry, &spec, cancel).await?;
        base += f.size_bytes;
    }
    write_derivative_notice(entry, &dir);
    Ok(())
}

/// Required attribution for derivative-licensed models — written alongside the
/// weights so redistribution terms travel with them (the license text itself
/// is a pinned manifest file, LICENSE-R2T2.txt).
const R2T2_NOTICE: &str = "\
NOTICE — Confucius4-R2T2 (MLX 4-bit)

This directory contains a Derivative Work of NetEase Youdao's Confucius4-R2T2
(itself a fine-tune of Qwen/Qwen3-ASR-1.7B), quantized and converted to MLX
format (selcukkubur/Confucius4-R2T2-mlx-4bit on Hugging Face).

Notice required by the NetEase Youdao Model Use License Agreement §4.1(a):

  Any modifications made to the original model in this Derivative Work are
  not endorsed, warranted, or guaranteed by the original right-holder of the
  original model, and the original right-holder disclaims all liability
  related to this Derivative Work.

The full license text is in LICENSE-R2T2.txt in this directory.
";

fn write_derivative_notice(entry: &ModelEntry, dir: &std::path::Path) {
    if entry.runtime == ModelRuntime::MlxR2t2 {
        if let Err(e) = std::fs::write(dir.join("NOTICE-R2T2.txt"), R2T2_NOTICE) {
            log::warn!("could not write NOTICE-R2T2.txt: {e}");
        }
    }
}

async fn download_file(
    app: Option<&AppHandle>,
    entry: &ModelEntry,
    spec: &FileSpec,
    cancel: &AtomicBool,
) -> Result<()> {
    let fname = spec.dest.file_name().unwrap_or_default().to_string_lossy();
    // Already there and the right size? Verify hash and short-circuit.
    if std::fs::metadata(&spec.dest)
        .map(|m| m.len() == spec.size_bytes)
        .unwrap_or(false)
    {
        emit_progress(
            app,
            ProgressEvent {
                id: entry.id.clone(),
                role: entry.role.label(),
                status: "verifying",
                downloaded: spec.base + spec.size_bytes,
                total: entry.size_bytes,
                speed_bps: 0,
                eta_secs: 0,
                error: None,
            },
        );
        let got = sha256_file(&spec.dest)?;
        if got.eq_ignore_ascii_case(&spec.sha256) {
            return Ok(());
        }
        log::warn!("existing {fname} failed hash — re-downloading");
        let _ = std::fs::remove_file(&spec.dest);
    }

    const MAX_ATTEMPTS: u32 = 5;
    let mut attempt = 0u32;
    loop {
        attempt += 1;
        match fetch_to_part(app, entry, spec, cancel).await {
            Ok(()) => break,
            Err(e) => {
                if cancel.load(Ordering::SeqCst) {
                    return Err(anyhow!("cancelled"));
                }
                if attempt >= MAX_ATTEMPTS {
                    return Err(e.context(format!("gave up after {attempt} attempts")));
                }
                // Capped exponential backoff: 1s, 2s, 4s, 8s (max 8s). The
                // `.part` file persists, so the next attempt resumes.
                let backoff = Duration::from_secs(1u64 << (attempt - 1).min(3));
                log::warn!(
                    "download {fname} attempt {attempt} failed ({e:#}); retrying in {}s",
                    backoff.as_secs()
                );
                sleep_cancellable(backoff, cancel).await;
                if cancel.load(Ordering::SeqCst) {
                    return Err(anyhow!("cancelled"));
                }
            }
        }
    }

    // Full file is on disk as `.part`. Verify before promoting.
    let pp = spec.part();
    let len = std::fs::metadata(&pp).map(|m| m.len()).unwrap_or(0);
    if len != spec.size_bytes {
        let _ = std::fs::remove_file(&pp);
        return Err(anyhow!(
            "size mismatch after download: got {len}, expected {}",
            spec.size_bytes
        ));
    }
    emit_progress(
        app,
        ProgressEvent {
            id: entry.id.clone(),
            role: entry.role.label(),
            status: "verifying",
            downloaded: spec.base + len,
            total: entry.size_bytes,
            speed_bps: 0,
            eta_secs: 0,
            error: None,
        },
    );
    let got = sha256_file(&pp)?;
    if !got.eq_ignore_ascii_case(&spec.sha256) {
        let _ = std::fs::remove_file(&pp);
        return Err(anyhow!(
            "sha256 mismatch for {fname}: got {got}, expected {} — deleted, retry",
            spec.sha256
        ));
    }
    std::fs::rename(&pp, &spec.dest).with_context(|| format!("promote {fname} into place"))?;
    log::info!(
        "model file {fname} downloaded + verified ({} bytes)",
        spec.size_bytes
    );
    Ok(())
}

/// One download attempt: opens (or resumes) the file's `.part` and streams the
/// body in, honoring the cancel flag between chunks and emitting throttled
/// progress (in whole-entry terms via `spec.base`). Returns Ok once the
/// `.part` reaches the expected size.
async fn fetch_to_part(
    app: Option<&AppHandle>,
    entry: &ModelEntry,
    spec: &FileSpec,
    cancel: &AtomicBool,
) -> Result<()> {
    use std::io::Write as _;

    let pp = spec.part();
    let mut have = std::fs::metadata(&pp).map(|m| m.len()).unwrap_or(0);
    if have > spec.size_bytes {
        // Corrupt/oversized leftover — start clean.
        let _ = std::fs::remove_file(&pp);
        have = 0;
    }
    if have == spec.size_bytes {
        return Ok(());
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(600))
        .connect_timeout(Duration::from_secs(20))
        .build()?;
    let mut req = client.get(&spec.url);
    if have > 0 {
        req = req.header(reqwest::header::RANGE, format!("bytes={have}-"));
    }
    let resp = req.send().await.context("send download request")?;
    let status = resp.status();
    if !(status.is_success() || status == reqwest::StatusCode::PARTIAL_CONTENT) {
        return Err(anyhow!("HTTP {status} fetching {}", entry.url));
    }
    // Server ignored our Range (200 not 206 while resuming) → restart from 0.
    let mut file = if have > 0 && status == reqwest::StatusCode::PARTIAL_CONTENT {
        std::fs::OpenOptions::new().append(true).open(&pp)?
    } else {
        have = 0;
        std::fs::File::create(&pp)?
    };

    let mut resp = resp;
    let mut downloaded = have;
    let start = Instant::now();
    let mut window_bytes = 0u64;
    let mut window_start = Instant::now();
    let mut last_emit = Instant::now();
    // Prime the UI immediately.
    emit_progress(
        app,
        ProgressEvent {
            id: entry.id.clone(),
            role: entry.role.label(),
            status: "downloading",
            downloaded: spec.base + downloaded,
            total: entry.size_bytes,
            speed_bps: 0,
            eta_secs: 0,
            error: None,
        },
    );

    while let Some(chunk) = resp.chunk().await.context("read chunk")? {
        if cancel.load(Ordering::SeqCst) {
            let _ = file.flush();
            return Err(anyhow!("cancelled"));
        }
        file.write_all(&chunk).context("write chunk")?;
        downloaded += chunk.len() as u64;
        window_bytes += chunk.len() as u64;

        if last_emit.elapsed() >= Duration::from_millis(250) {
            let win = window_start.elapsed().as_secs_f64().max(0.001);
            let speed = (window_bytes as f64 / win) as u64;
            // ETA over the whole entry, so a multi-file bar reads coherently.
            let remaining = entry.size_bytes.saturating_sub(spec.base + downloaded);
            let eta = remaining.checked_div(speed).unwrap_or(0);
            emit_progress(
                app,
                ProgressEvent {
                    id: entry.id.clone(),
                    role: entry.role.label(),
                    status: "downloading",
                    downloaded: spec.base + downloaded,
                    total: entry.size_bytes,
                    speed_bps: speed,
                    eta_secs: eta,
                    error: None,
                },
            );
            last_emit = Instant::now();
            window_bytes = 0;
            window_start = Instant::now();
        }
    }
    file.flush().ok();
    log::info!(
        "download attempt for {} finished at {downloaded}/{} ({:.1}s)",
        spec.dest.file_name().unwrap_or_default().to_string_lossy(),
        spec.size_bytes,
        start.elapsed().as_secs_f64()
    );

    if downloaded < spec.size_bytes {
        return Err(anyhow!(
            "stream ended early at {downloaded}/{}",
            spec.size_bytes
        ));
    }
    Ok(())
}

async fn sleep_cancellable(dur: Duration, cancel: &AtomicBool) {
    let start = Instant::now();
    while start.elapsed() < dur {
        if cancel.load(Ordering::SeqCst) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// Delete an installed model (and any stray `.part`). Returns the bytes freed.
pub fn delete(entry: &ModelEntry) -> Result<u64> {
    let freed = bytes_on_disk(entry);
    let p = model_path(entry);
    if !entry.files.is_empty() {
        // Multi-file model: the whole store subdirectory (weights, parts,
        // license + notice) goes.
        if p.exists() {
            std::fs::remove_dir_all(&p).with_context(|| format!("delete {}", p.display()))?;
        }
        return Ok(freed);
    }
    if p.exists() {
        std::fs::remove_file(&p).with_context(|| format!("delete {}", p.display()))?;
    }
    let pp = models_dir().join(format!("{}.part", entry.filename));
    if pp.exists() {
        let _ = std::fs::remove_file(&pp);
    }
    Ok(freed)
}

// ----------------------------------------------------------------------------
// Migration from the old bundled layout
// ----------------------------------------------------------------------------

/// Move/copy models from a previous bundled install into the models dir so an
/// upgrader doesn't re-download ~1.1 GB. Scans candidate bundle roots for the
/// old `vendor/whisper/<file>` and `vendor/llama/<file>` layout; a match is
/// only accepted after its SHA-256 matches the manifest.
///
/// Returns the ids that were migrated.
///
/// Reality note: a plain DMG install *replaces* /Applications/FunButton.app, so
/// the old bundle's models are usually already gone by first run of the new
/// build — in that case nothing is found and we fall through to the download
/// flow. This recovers models when a prior copy is still reachable: a dev build
/// with the vendor tree present, a side-by-side install, or an old .app not yet
/// deleted.
pub fn migrate_from_bundle(app: &AppHandle, manifest: &Manifest) -> Vec<String> {
    use tauri::Manager as _;
    let mut migrated = Vec::new();
    let dest_dir = match ensure_models_dir() {
        Ok(d) => d,
        Err(e) => {
            log::warn!("migration: cannot create models dir: {e:#}");
            return migrated;
        }
    };

    // Candidate directories that could hold an old `vendor/` tree.
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Ok(res) = app.path().resource_dir() {
        roots.push(res);
    }
    if let Ok(exe) = std::env::current_exe() {
        // .../FunButton.app/Contents/MacOS/FunButton → Contents/Resources
        if let Some(macos) = exe.parent() {
            if let Some(contents) = macos.parent() {
                roots.push(contents.join("Resources"));
            }
        }
    }
    // Dev tree.
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    if !manifest_dir.is_empty() {
        roots.push(PathBuf::from(manifest_dir));
    }

    for entry in &manifest.models {
        if is_installed(entry) {
            continue;
        }
        // Multi-file (schema v2) models never shipped inside any old bundle.
        if !entry.files.is_empty() {
            continue;
        }
        let subdir = match entry.role {
            Role::Stt => "whisper",
            Role::Cleanup => "llama",
        };
        let mut found: Option<PathBuf> = None;
        for root in &roots {
            // Tauri bundles declared resources under Resources/ and, for paths
            // that climb out of src-tauri, under a `_up_/` prefix — check both.
            for candidate in [
                root.join("vendor").join(subdir).join(&entry.filename),
                root.join("_up_")
                    .join("vendor")
                    .join(subdir)
                    .join(&entry.filename),
            ] {
                if candidate
                    .metadata()
                    .map(|m| m.len() == entry.size_bytes)
                    .unwrap_or(false)
                {
                    found = Some(candidate);
                    break;
                }
            }
            if found.is_some() {
                break;
            }
        }
        let Some(src) = found else { continue };
        // Don't migrate a file that is really the same on-disk file we're about
        // to write (dev symlink edge case): compare canonical paths.
        let dest = dest_dir.join(&entry.filename);
        if src == dest
            || src
                .canonicalize()
                .ok()
                .zip(dest.canonicalize().ok())
                .map(|(a, b)| a == b)
                .unwrap_or(false)
        {
            continue;
        }
        // Verify hash of the source before trusting it.
        match sha256_file(&src) {
            Ok(got) if got.eq_ignore_ascii_case(&entry.sha256) => {}
            Ok(got) => {
                log::warn!(
                    "migration: {} at {} failed hash (got {got}); skipping",
                    entry.id,
                    src.display()
                );
                continue;
            }
            Err(e) => {
                log::warn!("migration: hashing {} failed: {e:#}", src.display());
                continue;
            }
        }
        // Prefer a cheap rename (same volume); fall back to copy. Never delete
        // the source on copy — it belongs to the old bundle, not us.
        let ok = std::fs::rename(&src, &dest).is_ok()
            || std::fs::copy(&src, &dest).map(|_| true).unwrap_or(false);
        if ok {
            log::info!(
                "migration: relocated {} from {} into models dir",
                entry.id,
                src.display()
            );
            migrated.push(entry.id.clone());
        } else {
            log::warn!("migration: failed to place {} into models dir", entry.id);
        }
    }
    migrated
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Serializes tests that mutate the process-global FUNBUTTON_MODELS_DIR
    /// override (cargo runs tests on parallel threads). parking_lot — no
    /// poisoning on a failed assert.
    static ENV_LOCK: Lazy<parking_lot::Mutex<()>> = Lazy::new(|| parking_lot::Mutex::new(()));
    use once_cell::sync::Lazy;

    #[test]
    fn baked_manifest_is_valid_and_has_defaults() {
        let m = Manifest::baked();
        assert!(m.schema_version >= 1);
        assert!(!m.models.is_empty());
        // Exactly one default per role, and every url/sha/size is populated.
        let stt_defaults = m
            .models
            .iter()
            .filter(|x| x.role == Role::Stt && x.default)
            .count();
        let cleanup_defaults = m
            .models
            .iter()
            .filter(|x| x.role == Role::Cleanup && x.default)
            .count();
        assert_eq!(stt_defaults, 1, "exactly one default STT model");
        assert_eq!(cleanup_defaults, 1, "exactly one default cleanup model");
        for e in &m.models {
            assert!(!e.id.is_empty());
            assert!(e.url.starts_with("https://"), "{} url", e.id);
            assert!(e.size_bytes > 0, "{} size", e.id);
            if e.files.is_empty() {
                assert_eq!(e.sha256.len(), 64, "{} sha256 len", e.id);
            } else {
                // Multi-file (schema v2): every member pinned individually,
                // sizes summing to the entry total the UI displays.
                for f in &e.files {
                    assert!(
                        !f.filename.contains('/'),
                        "{} {} nested path",
                        e.id,
                        f.filename
                    );
                    assert!(f.url.starts_with("https://"), "{} {} url", e.id, f.filename);
                    assert_eq!(f.sha256.len(), 64, "{} {} sha256", e.id, f.filename);
                    assert!(f.size_bytes > 0, "{} {} size", e.id, f.filename);
                }
                assert_eq!(
                    e.files.iter().map(|f| f.size_bytes).sum::<u64>(),
                    e.size_bytes,
                    "{} size_bytes must equal the sum of its files",
                    e.id
                );
            }
        }
        assert!(m.default_id(Role::Stt).is_some());
        assert!(m.default_id(Role::Cleanup).is_some());
        // Default engines stay builtin — the experimental sidecar runtime is
        // strictly opt-in.
        for role in [Role::Stt, Role::Cleanup] {
            let d = m
                .models
                .iter()
                .find(|x| x.role == role && x.default)
                .unwrap();
            assert_eq!(d.runtime, ModelRuntime::Builtin, "{} default runtime", d.id);
        }
    }

    #[test]
    fn multi_file_entry_install_and_delete_semantics() {
        // FUNBUTTON_MODELS_DIR is process-global; ENV_LOCK serializes the two
        // tests that touch it (this one and the dmg-upgrade proof).
        let _env = ENV_LOCK.lock();
        let tmp = std::env::temp_dir().join(format!("fb-multifile-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let saved = std::env::var("FUNBUTTON_MODELS_DIR").ok();
        std::env::set_var("FUNBUTTON_MODELS_DIR", &tmp);

        let entry = ModelEntry {
            id: "mf".into(),
            role: Role::Stt,
            name: "mf".into(),
            filename: "mf-dir".into(),
            url: "https://example.invalid/repo".into(),
            sha256: String::new(),
            size_bytes: 10,
            blurb: String::new(),
            license: String::new(),
            default: false,
            recommended: false,
            min_app_version: "0.0.0".into(),
            runtime: ModelRuntime::MlxR2t2,
            files: vec![
                ModelFile {
                    filename: "a.bin".into(),
                    url: "https://example.invalid/a".into(),
                    sha256: "0".repeat(64),
                    size_bytes: 6,
                },
                ModelFile {
                    filename: "b.json".into(),
                    url: "https://example.invalid/b".into(),
                    sha256: "0".repeat(64),
                    size_bytes: 4,
                },
            ],
        };
        let dir = models_dir().join("mf-dir");
        std::fs::create_dir_all(&dir).unwrap();
        assert!(!is_installed(&entry), "no files yet");
        std::fs::write(dir.join("a.bin"), b"123456").unwrap();
        assert!(!is_installed(&entry), "one of two files");
        std::fs::write(dir.join("b.json"), b"1234").unwrap();
        assert!(is_installed(&entry), "all files at exact size");
        // Wrong size on any member → not installed.
        std::fs::write(dir.join("b.json"), b"12345").unwrap();
        assert!(!is_installed(&entry));
        std::fs::write(dir.join("b.json"), b"1234").unwrap();

        // model_path is the subdirectory; delete removes it wholesale.
        assert_eq!(model_path(&entry), dir);
        let freed = delete(&entry).unwrap();
        assert_eq!(freed, 10);
        assert!(!dir.exists());

        match saved {
            Some(v) => std::env::set_var("FUNBUTTON_MODELS_DIR", v),
            None => std::env::remove_var("FUNBUTTON_MODELS_DIR"),
        }
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn active_entry_falls_back_to_default() {
        let m = Manifest::baked();
        // Unknown chosen id → default.
        let e = m.active_entry(Role::Stt, "does-not-exist").unwrap();
        assert!(e.default);
        // Known id → that one.
        let base = m.active_entry(Role::Stt, "whisper-base.en").unwrap();
        assert_eq!(base.id, "whisper-base.en");
    }

    /// LIVE multi-file download proof: fetches the real R2T2 entry (1.6 GB
    /// from HF + the NetEase license from GitHub raw) through the production
    /// download path — per-file resume/verify/promote, NOTICE written — into
    /// FUNBUTTON_MODELS_DIR (set it! this test intentionally honors the env
    /// so QA can populate the real store) and asserts installed state.
    /// `FUNBUTTON_MODELS_DIR=/path cargo test --release --lib \
    ///    multifile_download_live -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn multifile_download_live() {
        let entry = Manifest::baked()
            .entry("confucius4-r2t2")
            .expect("r2t2 entry in baked manifest")
            .clone();
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let cancel = AtomicBool::new(false);
        rt.block_on(async {
            download_inner(None, &entry, &cancel)
                .await
                .expect("download");
        });
        assert!(is_installed(&entry), "all files at pinned sizes");
        let dir = model_path(&entry);
        // License + required derivative notice travel with the weights.
        let lic = std::fs::read(dir.join("LICENSE-R2T2.txt")).expect("license present");
        assert_eq!(lic.len(), 11071);
        let notice = std::fs::read_to_string(dir.join("NOTICE-R2T2.txt")).expect("notice");
        assert!(notice.contains("not endorsed, warranted, or guaranteed"));
        eprintln!(
            "INSTALLED at {} ({} bytes)",
            dir.display(),
            entry.size_bytes
        );
    }

    /// The models store must survive a DMG upgrade (dragging a new .app over
    /// the old one). That holds only while the store path is derived from
    /// HOME + APP_IDENTIFIER and never from the executable/bundle location —
    /// this pins it. Also pins that APP_IDENTIFIER matches tauri.conf.json:
    /// renaming the bundle identifier would silently orphan every user's
    /// downloaded models (a fresh store path ⇒ full re-download).
    #[test]
    fn store_is_bundle_independent_so_dmg_upgrades_keep_models() {
        // APP_IDENTIFIER == tauri.conf.json "identifier".
        let conf: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        assert_eq!(
            conf["identifier"].as_str().unwrap(),
            APP_IDENTIFIER,
            "bundle identifier drifted from models::APP_IDENTIFIER — this would orphan installed models"
        );

        // Default store path: HOME-scoped, bundle/exe-independent.
        // FUNBUTTON_MODELS_DIR is process-global — ENV_LOCK serializes the
        // tests that touch it.
        let _env = ENV_LOCK.lock();
        let saved = std::env::var("FUNBUTTON_MODELS_DIR").ok();
        std::env::remove_var("FUNBUTTON_MODELS_DIR");
        let dir = models_dir();
        let home = std::env::var("HOME").unwrap();
        assert_eq!(
            dir,
            PathBuf::from(&home)
                .join("Library")
                .join("Application Support")
                .join(APP_IDENTIFIER)
                .join("models"),
            "models_dir must be exactly the HOME-scoped App Support store"
        );
        let exe = std::env::current_exe().unwrap();
        assert!(
            !dir.starts_with(exe.parent().unwrap()),
            "models_dir must never live under the executable/bundle"
        );

        // A model counts as installed purely by store contents (exists + exact
        // size — the cheap startup check), regardless of where the .app is.
        // So replacing the .app cannot invalidate an installed model.
        let tmp = std::env::temp_dir().join(format!("fb-dmg-proof-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        std::env::set_var("FUNBUTTON_MODELS_DIR", &tmp);
        let entry = ModelEntry {
            id: "proof".into(),
            role: Role::Cleanup,
            name: "proof".into(),
            filename: "proof.gguf".into(),
            url: "https://example.invalid/proof.gguf".into(),
            sha256: "0".repeat(64),
            size_bytes: 8,
            blurb: String::new(),
            license: String::new(),
            default: false,
            recommended: false,
            min_app_version: "0.0.0".into(),
            runtime: ModelRuntime::Builtin,
            files: Vec::new(),
        };
        assert!(!is_installed(&entry), "empty store → not installed");
        std::fs::write(tmp.join("proof.gguf"), b"12345678").unwrap();
        assert!(
            is_installed(&entry),
            "exact-size file in the store → installed, no bundle involved"
        );
        // Wrong size (truncated/corrupt) must read as NOT installed, so the
        // download flow takes over rather than loading a bad file.
        std::fs::write(tmp.join("proof.gguf"), b"1234").unwrap();
        assert!(!is_installed(&entry), "size mismatch → not installed");

        // Restore process env + scratch dir.
        match saved {
            Some(v) => std::env::set_var("FUNBUTTON_MODELS_DIR", v),
            None => std::env::remove_var("FUNBUTTON_MODELS_DIR"),
        }
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
