use anyhow::{Context, Result};
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: i64,
    pub ts: i64,
    pub raw_transcript: String,
    pub cleaned_text: String,
    pub mode_used: String,
    pub frontmost_app: Option<String>,
    pub paste_succeeded: bool,
    pub audio_duration_ms: Option<i64>,
    pub model_used: String,
}

pub struct History {
    conn: Mutex<Connection>,
}

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS history (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    ts                INTEGER NOT NULL,
    raw_transcript    TEXT NOT NULL,
    cleaned_text      TEXT NOT NULL,
    mode_used         TEXT NOT NULL,
    frontmost_app     TEXT,
    paste_succeeded   INTEGER NOT NULL DEFAULT 0,
    audio_duration_ms INTEGER,
    model_used        TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_history_ts ON history(ts DESC);
"#;

impl History {
    pub fn open(path: PathBuf) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let conn = Connection::open(&path)
            .with_context(|| format!("opening sqlite at {:?}", path))?;
        conn.execute_batch(SCHEMA).context("apply schema")?;
        Ok(History { conn: Mutex::new(conn) })
    }

    /// Insert a row immediately after cleanup completes, before paste.
    /// `paste_succeeded` starts as false and is updated by `mark_paste_result`.
    pub fn insert_pre_paste(
        &self,
        raw: &str,
        cleaned: &str,
        mode: &str,
        frontmost_app: Option<&str>,
        audio_duration_ms: Option<i64>,
        model_used: &str,
    ) -> Result<i64> {
        let ts = now_secs();
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO history (ts, raw_transcript, cleaned_text, mode_used, frontmost_app, paste_succeeded, audio_duration_ms, model_used) VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6, ?7)",
            params![ts, raw, cleaned, mode, frontmost_app, audio_duration_ms, model_used],
        )
        .context("insert history row")?;
        Ok(conn.last_insert_rowid())
    }

    pub fn mark_paste_result(&self, id: i64, success: bool) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE history SET paste_succeeded = ?1 WHERE id = ?2",
            params![if success { 1 } else { 0 }, id],
        )
        .context("update paste result")?;
        Ok(())
    }

    pub fn list(
        &self,
        limit: i64,
        search: Option<&str>,
        mode_filter: Option<&str>,
    ) -> Result<Vec<HistoryEntry>> {
        let conn = self.conn.lock();
        let mut sql = String::from(
            "SELECT id, ts, raw_transcript, cleaned_text, mode_used, frontmost_app, paste_succeeded, audio_duration_ms, model_used FROM history WHERE 1=1",
        );
        let mut args: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        if let Some(q) = search.filter(|s| !s.trim().is_empty()) {
            sql.push_str(" AND (cleaned_text LIKE ?1 OR raw_transcript LIKE ?1)");
            args.push(Box::new(format!("%{}%", q)));
        }
        if let Some(mode) = mode_filter.filter(|s| !s.is_empty() && *s != "all") {
            let placeholder = format!("?{}", args.len() + 1);
            sql.push_str(&format!(" AND mode_used = {placeholder}"));
            args.push(Box::new(mode.to_string()));
        }
        let limit_placeholder = format!("?{}", args.len() + 1);
        sql.push_str(&format!(" ORDER BY ts DESC LIMIT {limit_placeholder}"));
        args.push(Box::new(limit));

        let arg_refs: Vec<&dyn rusqlite::ToSql> = args.iter().map(|b| &**b as &dyn rusqlite::ToSql).collect();
        let mut stmt = conn.prepare(&sql).context("prepare list query")?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(arg_refs), |r| {
                Ok(HistoryEntry {
                    id: r.get(0)?,
                    ts: r.get(1)?,
                    raw_transcript: r.get(2)?,
                    cleaned_text: r.get(3)?,
                    mode_used: r.get(4)?,
                    frontmost_app: r.get(5)?,
                    paste_succeeded: r.get::<_, i64>(6)? != 0,
                    audio_duration_ms: r.get(7)?,
                    model_used: r.get(8)?,
                })
            })
            .context("execute list query")?;
        let mut out = Vec::new();
        for entry in rows {
            out.push(entry?);
        }
        Ok(out)
    }

    pub fn last_failed(&self) -> Result<Option<HistoryEntry>> {
        let mut rows = self.list(1, None, None)?;
        if let Some(top) = rows.pop() {
            if !top.paste_succeeded {
                return Ok(Some(top));
            }
        }
        Ok(None)
    }

    pub fn purge_older_than(&self, retention_days: u32) -> Result<u64> {
        if retention_days == 0 {
            return Ok(0); // 0 = retain forever
        }
        let cutoff = now_secs() - (retention_days as i64) * 86_400;
        let conn = self.conn.lock();
        let n = conn
            .execute("DELETE FROM history WHERE ts < ?1", params![cutoff])
            .context("purge old history")?;
        Ok(n as u64)
    }
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
