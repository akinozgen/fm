use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use walkdir::WalkDir;

#[derive(Clone, Serialize)]
pub struct DirSizeEvent {
  pub bytes: u64,
  pub done: bool,
}

pub struct DirSizeState {
  cancel: Mutex<Arc<AtomicBool>>,
}

impl DirSizeState {
  pub fn new() -> Self {
    Self {
      cancel: Mutex::new(Arc::new(AtomicBool::new(false))),
    }
  }

  pub fn reset(&self) -> Arc<AtomicBool> {
    let mut guard = self.cancel.lock().unwrap();
    guard.store(true, Relaxed);
    let fresh = Arc::new(AtomicBool::new(false));
    *guard = fresh.clone();
    fresh
  }

  pub fn cancel(&self) {
    self.cancel.lock().unwrap().store(true, Relaxed);
  }
}

pub fn walk_and_emit(paths: Vec<String>, cancel: Arc<AtomicBool>, app: AppHandle) {
  let mut total: u64 = 0;
  let mut last_emit = Instant::now();

  for path in &paths {
    if cancel.load(Relaxed) {
      return;
    }
    for entry in WalkDir::new(path).follow_links(false).into_iter().filter_map(|e| e.ok()) {
      if cancel.load(Relaxed) {
        return;
      }
      if entry.file_type().is_file() {
        if let Ok(meta) = entry.metadata() {
          total += meta.len();
        }
      }
      if last_emit.elapsed().as_millis() >= 120 {
        let _ = app.emit("fm://dir-size", DirSizeEvent { bytes: total, done: false });
        last_emit = Instant::now();
      }
    }
  }

  if !cancel.load(Relaxed) {
    let _ = app.emit("fm://dir-size", DirSizeEvent { bytes: total, done: true });
  }
}

#[tauri::command]
pub async fn compute_dir_size_cmd(
  app: AppHandle,
  state: tauri::State<'_, DirSizeState>,
  paths: Vec<String>,
) -> Result<(), String> {
  let cancel = state.reset();
  tokio::task::spawn_blocking(move || walk_and_emit(paths, cancel, app))
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn cancel_dir_size_cmd(state: tauri::State<'_, DirSizeState>) {
  state.cancel();
}
