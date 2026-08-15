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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelEntry {
    pub id: String,
    pub role: Role,
    pub name: String,
    pub filename: String,
    pub url: String,
    pub sha256: String,
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

/// Cheap installed check: the file exists with the exact expected byte length.
/// Full SHA verification happens at download/migration time (a finalized file
/// is only ever produced after its hash matched), so a length match here is a
/// trustworthy, hash-free proxy for the model manager list.
pub fn is_installed(entry: &ModelEntry) -> bool {
    std::fs::metadata(model_path(entry))
        .map(|m| m.len() == entry.size_bytes)
        .unwrap_or(false)
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

fn emit_progress(app: &AppHandle, ev: ProgressEvent) {
    let _ = app.emit("funbutton:model-progress", ev);
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
    let result = download_inner(&app, &entry, &cancel).await;
    manager.deregister(&entry.id);

    match &result {
        Ok(()) => emit_progress(
            &app,
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
                &app,
                ProgressEvent {
                    id: entry.id.clone(),
                    role: entry.role.label(),
                    status: if cancelled { "cancelled" } else { "error" },
                    downloaded: current_part_len(&entry),
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

fn part_path(entry: &ModelEntry) -> PathBuf {
    models_dir().join(format!("{}.part", entry.filename))
}

fn current_part_len(entry: &ModelEntry) -> u64 {
    std::fs::metadata(part_path(entry))
        .map(|m| m.len())
        .unwrap_or(0)
}

async fn download_inner(app: &AppHandle, entry: &ModelEntry, cancel: &AtomicBool) -> Result<()> {
    ensure_models_dir()?;
    let final_path = model_path(entry);
    // Already there and the right size? Verify hash and short-circuit.
    if std::fs::metadata(&final_path)
        .map(|m| m.len() == entry.size_bytes)
        .unwrap_or(false)
    {
        emit_progress(
            app,
            ProgressEvent {
                id: entry.id.clone(),
                role: entry.role.label(),
                status: "verifying",
                downloaded: entry.size_bytes,
                total: entry.size_bytes,
                speed_bps: 0,
                eta_secs: 0,
                error: None,
            },
        );
        let got = sha256_file(&final_path)?;
        if got.eq_ignore_ascii_case(&entry.sha256) {
            return Ok(());
        }
        log::warn!("existing {} failed hash — re-downloading", entry.filename);
        let _ = std::fs::remove_file(&final_path);
    }

    const MAX_ATTEMPTS: u32 = 5;
    let mut attempt = 0u32;
    loop {
        attempt += 1;
        match fetch_to_part(app, entry, cancel).await {
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
                    "download {} attempt {attempt} failed ({e:#}); retrying in {}s",
                    entry.filename,
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
    let pp = part_path(entry);
    let len = std::fs::metadata(&pp).map(|m| m.len()).unwrap_or(0);
    if len != entry.size_bytes {
        let _ = std::fs::remove_file(&pp);
        return Err(anyhow!(
            "size mismatch after download: got {len}, expected {}",
            entry.size_bytes
        ));
    }
    emit_progress(
        app,
        ProgressEvent {
            id: entry.id.clone(),
            role: entry.role.label(),
            status: "verifying",
            downloaded: len,
            total: entry.size_bytes,
            speed_bps: 0,
            eta_secs: 0,
            error: None,
        },
    );
    let got = sha256_file(&pp)?;
    if !got.eq_ignore_ascii_case(&entry.sha256) {
        let _ = std::fs::remove_file(&pp);
        return Err(anyhow!(
            "sha256 mismatch for {}: got {got}, expected {} — deleted, retry",
            entry.filename,
            entry.sha256
        ));
    }
    std::fs::rename(&pp, &final_path)
        .with_context(|| format!("promote {} into place", entry.filename))?;
    log::info!(
        "model {} downloaded + verified ({} bytes)",
        entry.filename,
        entry.size_bytes
    );
    Ok(())
}

/// One download attempt: opens (or resumes) `<filename>.part` and streams the
/// body in, honoring the cancel flag between chunks and emitting throttled
/// progress. Returns Ok once the `.part` reaches the expected size.
async fn fetch_to_part(app: &AppHandle, entry: &ModelEntry, cancel: &AtomicBool) -> Result<()> {
    use std::io::Write as _;

    let pp = part_path(entry);
    let mut have = std::fs::metadata(&pp).map(|m| m.len()).unwrap_or(0);
    if have > entry.size_bytes {
        // Corrupt/oversized leftover — start clean.
        let _ = std::fs::remove_file(&pp);
        have = 0;
    }
    if have == entry.size_bytes {
        return Ok(());
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(600))
        .connect_timeout(Duration::from_secs(20))
        .build()?;
    let mut req = client.get(&entry.url);
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
            downloaded,
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
            let remaining = entry.size_bytes.saturating_sub(downloaded);
            let eta = remaining.checked_div(speed).unwrap_or(0);
            emit_progress(
                app,
                ProgressEvent {
                    id: entry.id.clone(),
                    role: entry.role.label(),
                    status: "downloading",
                    downloaded,
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
        entry.filename,
        entry.size_bytes,
        start.elapsed().as_secs_f64()
    );

    if downloaded < entry.size_bytes {
        return Err(anyhow!(
            "stream ended early at {downloaded}/{}",
            entry.size_bytes
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
    let p = model_path(entry);
    let freed = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
    if p.exists() {
        std::fs::remove_file(&p).with_context(|| format!("delete {}", p.display()))?;
    }
    let pp = part_path(entry);
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
            assert_eq!(e.sha256.len(), 64, "{} sha256 len", e.id);
            assert!(e.size_bytes > 0, "{} size", e.id);
        }
        assert!(m.default_id(Role::Stt).is_some());
        assert!(m.default_id(Role::Cleanup).is_some());
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
}
