use rusqlite::Connection;
use serde::Serialize;
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize)]
pub struct StoragePaths {
  pub config_dir: String,
  pub prefs_path: String,
  pub db_path: String,
}

pub fn bootstrap_storage() -> Result<StoragePaths, String> {
  let config_dir = resolve_config_dir()?;
  fs::create_dir_all(&config_dir).map_err(|e| format!("failed to create config dir: {e}"))?;

  let prefs_path = config_dir.join("preferences.json");
  if !prefs_path.exists() {
    fs::write(&prefs_path, b"{}").map_err(|e| format!("failed to create preferences file: {e}"))?;
  }

  let db_path = config_dir.join("fm.db");
  bootstrap_db(&db_path)?;

  Ok(StoragePaths {
    config_dir: config_dir.to_string_lossy().to_string(),
    prefs_path: prefs_path.to_string_lossy().to_string(),
    db_path: db_path.to_string_lossy().to_string(),
  })
}

fn resolve_config_dir() -> Result<PathBuf, String> {
  #[cfg(windows)]
  {
    let base = env::var("LOCALAPPDATA").map_err(|_| "LOCALAPPDATA is not set".to_string())?;
    return Ok(PathBuf::from(base).join("fm"));
  }

  #[cfg(not(windows))]
  {
    let home = env::var("HOME").map_err(|_| "HOME is not set".to_string())?;
    Ok(PathBuf::from(home).join(".config").join("fm"))
  }
}

fn bootstrap_db(db_path: &PathBuf) -> Result<(), String> {
  let connection = Connection::open(db_path).map_err(|e| format!("failed to open db: {e}"))?;
  connection
    .execute_batch(
      r#"
      PRAGMA journal_mode=WAL;
      PRAGMA foreign_keys=ON;

      CREATE TABLE IF NOT EXISTS navigation_history (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        path TEXT NOT NULL,
        visited_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
      );

      CREATE INDEX IF NOT EXISTS idx_navigation_history_visited_at
      ON navigation_history(visited_at DESC);
      "#,
    )
    .map_err(|e| format!("failed to initialize db schema: {e}"))?;
  Ok(())
}
