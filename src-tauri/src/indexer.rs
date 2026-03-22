use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::{
  atomic::{AtomicBool, Ordering},
  Arc,
};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};
use walkdir::WalkDir;

pub struct IndexerState {
  pub cancel:     Arc<AtomicBool>,
  pub is_running: Arc<AtomicBool>,
}

impl IndexerState {
  pub fn new() -> Self {
    Self {
      cancel:     Arc::new(AtomicBool::new(false)),
      is_running: Arc::new(AtomicBool::new(false)),
    }
  }

  /// Reset cancel flag, mark running. Returns a clone of the cancel flag for the worker.
  pub fn start(&self) -> Arc<AtomicBool> {
    self.cancel.store(false, Ordering::Relaxed);
    self.is_running.store(true, Ordering::Relaxed);
    self.cancel.clone()
  }

  pub fn finish(&self) {
    self.is_running.store(false, Ordering::Relaxed);
  }
}

fn index_roots() -> Vec<PathBuf> {
  let mut roots = Vec::new();

  #[cfg(target_os = "macos")]
  {
    if let Ok(home) = std::env::var("HOME") {
      roots.push(PathBuf::from(home));
    }
    roots.push(PathBuf::from("/Volumes"));
  }

  #[cfg(target_os = "linux")]
  {
    if let Ok(home) = std::env::var("HOME") {
      roots.push(PathBuf::from(home));
    }
    roots.push(PathBuf::from("/mnt"));
    roots.push(PathBuf::from("/media"));
  }

  #[cfg(target_os = "windows")]
  {
    for letter in b'A'..=b'Z' {
      let drive = format!("{}:\\", letter as char);
      if PathBuf::from(&drive).exists() {
        roots.push(PathBuf::from(drive));
      }
    }
  }

  roots
}

fn should_skip(path: &Path) -> bool {
  if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
    if matches!(name, ".git" | "node_modules") {
      return true;
    }
  }
  let s = path.to_string_lossy();
  for prefix in &["/proc/", "/sys/", "/dev/", "/run/"] {
    if s.starts_with(prefix) || s.as_ref() == &prefix[..prefix.len() - 1] {
      return true;
    }
  }
  false
}

fn flush_batch(
  conn: &Connection,
  batch: &mut Vec<(String, String, i64, Option<i64>, Option<i64>)>,
) {
  if batch.is_empty() {
    return;
  }
  match conn.unchecked_transaction() {
    Ok(tx) => {
      for (name, path, is_dir, size, modified_ms) in batch.drain(..) {
        let _ = tx.execute(
          "INSERT INTO file_index(name, path, is_dir, size, modified_ms) VALUES (?1,?2,?3,?4,?5)",
          rusqlite::params![name, path, is_dir, size, modified_ms],
        );
      }
      if let Err(e) = tx.commit() {
        log::error!("indexer: commit error: {e}");
      }
    }
    Err(e) => {
      log::error!("indexer: transaction error: {e}");
      batch.clear();
    }
  }
}

pub fn run_index(db_path: String, cancel: Arc<AtomicBool>, app: AppHandle) {
  let conn = match Connection::open(&db_path) {
    Ok(c) => c,
    Err(e) => {
      log::error!("indexer: failed to open db: {e}");
      return;
    }
  };

  if let Err(e) = conn.execute_batch("PRAGMA journal_mode=WAL;") {
    log::error!("indexer: failed to set WAL: {e}");
    return;
  }

  if let Err(e) = conn.execute("DELETE FROM file_index", []) {
    log::error!("indexer: failed to clear index: {e}");
    return;
  }
  let _ = conn.execute("DELETE FROM index_meta", []);

  let roots = index_roots();
  let mut total: u64 = 0;
  let mut batch: Vec<(String, String, i64, Option<i64>, Option<i64>)> = Vec::with_capacity(500);

  'outer: for root in &roots {
    if !root.exists() {
      continue;
    }

    let walker = WalkDir::new(root)
      .follow_links(false)
      .into_iter()
      .filter_entry(|e| e.depth() == 0 || !should_skip(e.path()));

    for entry in walker {
      if cancel.load(Ordering::Relaxed) {
        break 'outer;
      }

      let entry = match entry {
        Ok(e) => e,
        Err(_) => continue,
      };

      if entry.depth() == 0 {
        continue;
      }

      let path = entry.path();
      let name = match path.file_name().and_then(|n| n.to_str()) {
        Some(n) => n.to_string(),
        None => continue,
      };
      let path_str = path.to_string_lossy().to_string();
      let is_dir: i64 = entry.file_type().is_dir() as i64;

      let meta = entry.metadata().ok();
      let size: Option<i64> = meta.as_ref().and_then(|m| {
        if is_dir == 0 { Some(m.len() as i64) } else { None }
      });
      let modified_ms: Option<i64> = meta.as_ref().and_then(|m| {
        m.modified().ok().and_then(|t| {
          t.duration_since(UNIX_EPOCH).ok().map(|d| d.as_millis() as i64)
        })
      });

      batch.push((name, path_str, is_dir, size, modified_ms));
      total += 1;

      if batch.len() >= 500 {
        flush_batch(&conn, &mut batch);
        let _ = app.emit("fm://index-progress", serde_json::json!({ "done": total }));
      }
    }
  }

  flush_batch(&conn, &mut batch);

  let now = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .map(|d| d.as_secs())
    .unwrap_or(0);
  let _ = conn.execute(
    "INSERT OR REPLACE INTO index_meta(key, value) VALUES ('last_indexed', ?1)",
    rusqlite::params![now.to_string()],
  );
  let _ = conn.execute(
    "INSERT OR REPLACE INTO index_meta(key, value) VALUES ('file_count', ?1)",
    rusqlite::params![total.to_string()],
  );

  let _ = app.emit("fm://index-done", serde_json::json!({ "count": total }));
  log::info!("indexer: complete — {total} entries indexed");
}
