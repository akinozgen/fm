use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use walkdir::WalkDir;

fn wait_while_paused(cancel: &AtomicBool, paused: &AtomicBool) {
  while paused.load(Relaxed) && !cancel.load(Relaxed) {
    thread::sleep(Duration::from_millis(100));
  }
}

const COPY_CHUNK_SIZE: usize = 256 * 1024; // 256 KB per chunk so cancel is checked often

/// Normalize path for comparison: no trailing separator, lowercase on Windows.
fn norm_for_cmp(p: &Path) -> PathBuf {
  let s = p.to_string_lossy().trim_end_matches(|c| c == '/' || c == '\\').to_string();
  #[cfg(windows)]
  let s = s.to_lowercase();
  PathBuf::from(s)
}

/// True when paste target is the same directory as every source (no-op for move).
fn move_target_is_same_dir(src_paths: &[String], dest_dir: &str) -> bool {
  let dest = norm_for_cmp(Path::new(dest_dir));
  if dest.as_os_str().is_empty() { return false; }
  for raw in src_paths {
    let src = Path::new(raw);
    let parent = match src.parent() {
      Some(p) => p,
      None => return false,
    };
    if norm_for_cmp(parent) != dest { return false; }
  }
  true
}

pub(crate) struct TransferStateInner {
  jobs: Mutex<HashMap<u64, (Arc<AtomicBool>, Arc<AtomicBool>)>>,
  next_id: AtomicU64,
}

impl TransferStateInner {
  fn next_id(&self) -> u64 {
    self.next_id.fetch_add(1, Relaxed)
  }
  fn insert(&self, job_id: u64, cancel: Arc<AtomicBool>, paused: Arc<AtomicBool>) {
    self.jobs.lock().unwrap().insert(job_id, (cancel, paused));
  }
  fn cancel(&self, job_id: u64) {
    if let Some((cancel, _)) = self.jobs.lock().unwrap().get(&job_id) {
      cancel.store(true, Relaxed);
    }
  }
  fn pause(&self, job_id: u64) {
    if let Some((_, paused)) = self.jobs.lock().unwrap().get(&job_id) {
      paused.store(true, Relaxed);
    }
  }
  fn resume(&self, job_id: u64) {
    if let Some((_, paused)) = self.jobs.lock().unwrap().get(&job_id) {
      paused.store(false, Relaxed);
    }
  }
  fn remove(&self, job_id: u64) {
    self.jobs.lock().unwrap().remove(&job_id);
  }
}

pub struct TransferState(pub Arc<TransferStateInner>);

impl TransferState {
  pub fn new() -> Self {
    Self(Arc::new(TransferStateInner {
      jobs: Mutex::new(HashMap::new()),
      next_id: AtomicU64::new(1),
    }))
  }
  fn next_id(&self) -> u64 {
    self.0.next_id()
  }
  fn insert(&self, job_id: u64, cancel: Arc<AtomicBool>, paused: Arc<AtomicBool>) {
    self.0.insert(job_id, cancel, paused);
  }
  pub fn cancel(&self, job_id: u64) {
    self.0.cancel(job_id);
  }
  pub fn pause(&self, job_id: u64) {
    self.0.pause(job_id);
  }
  pub fn resume(&self, job_id: u64) {
    self.0.resume(job_id);
  }
  pub fn remove(&self, job_id: u64) {
    self.0.remove(job_id);
  }
}

#[derive(Clone, serde::Serialize)]
pub struct TransferProgressEvent {
  pub job_id: u64,
  pub op: String,
  pub done: u64,
  pub total: u64,
  pub bytes_done: u64,
  pub bytes_total: u64,
  pub current: String,
}

#[derive(Clone, serde::Serialize)]
pub struct TransferDoneEvent {
  pub job_id: u64,
  pub op: String,
  pub done: u64,
  pub total: u64,
  pub bytes_done: u64,
  pub bytes_total: u64,
  pub errors: Vec<String>,
  pub cancelled: bool,
}

/// Total byte size of a file or recursive directory contents.
fn total_size(path: &Path) -> u64 {
  let meta = match std::fs::metadata(path) {
    Ok(m) => m,
    Err(_) => return 0,
  };
  if meta.is_file() {
    return meta.len();
  }
  let mut sum = 0u64;
  if let Ok(entries) = std::fs::read_dir(path) {
    for entry in entries.flatten() {
      let p = entry.path();
      sum += total_size(&p);
    }
  }
  sum
}

fn resolve_dest(dir: &Path, name: &str) -> std::path::PathBuf {
  let candidate = dir.join(name);
  if !candidate.exists() { return candidate; }
  let stem = Path::new(name).file_stem()
    .unwrap_or_default().to_string_lossy().to_string();
  let ext = Path::new(name).extension()
    .map(|e| format!(".{}", e.to_string_lossy()))
    .unwrap_or_default();
  let mut i = 1u32;
  loop {
    let c = dir.join(format!("{stem} ({i}){ext}"));
    if !c.exists() { return c; }
    i += 1;
  }
}

fn same_volume(a: &Path, b: &Path) -> bool {
  a.components().next() == b.components().next()
}

/// Copy a single file in chunks, checking cancel and paused between each chunk.
/// Calls `on_progress(n)` after each chunk with the number of bytes just written.
fn copy_file_with_cancel<F: FnMut(u64)>(
  src: &Path,
  dest: &Path,
  cancel: &AtomicBool,
  paused: &AtomicBool,
  on_progress: &mut Option<&mut F>,
) -> Result<(), String> {
  if cancel.load(Relaxed) {
    return Err("cancelled".into());
  }
  wait_while_paused(cancel, paused);
  let mut reader = fs::File::open(src).map_err(|e| e.to_string())?;
  let mut writer = fs::File::create(dest).map_err(|e| e.to_string())?;
  let mut buf = vec![0u8; COPY_CHUNK_SIZE];
  loop {
    if cancel.load(Relaxed) {
      let _ = fs::remove_file(dest);
      return Err("cancelled".into());
    }
    wait_while_paused(cancel, paused);
    let n = reader.read(&mut buf).map_err(|e| e.to_string())?;
    if n == 0 {
      break;
    }
    writer.write_all(&buf[..n]).map_err(|e| e.to_string())?;
    if let Some(ref mut f) = on_progress {
      (*f)(n as u64);
    }
  }
  writer.sync_all().map_err(|e| e.to_string())?;
  Ok(())
}

fn copy_recursive<F: FnMut(u64)>(
  src: &Path,
  dest: &Path,
  cancel: &AtomicBool,
  paused: &AtomicBool,
  on_progress: &mut Option<&mut F>,
) -> Result<(), String> {
  if src.is_dir() {
    fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    for entry in WalkDir::new(src).min_depth(1) {
      if cancel.load(Relaxed) {
        return Err("cancelled".into());
      }
      wait_while_paused(cancel, paused);
      let entry = entry.map_err(|e| e.to_string())?;
      let rel = entry.path().strip_prefix(src).unwrap();
      let target = dest.join(rel);
      if entry.file_type().is_dir() {
        fs::create_dir_all(&target).map_err(|e| e.to_string())?;
      } else {
        copy_file_with_cancel(entry.path(), &target, cancel, paused, on_progress)
          .map_err(|e| format!("{}: {e}", entry.path().display()))?;
      }
    }
  } else {
    copy_file_with_cancel(src, dest, cancel, paused, on_progress)?;
  }
  Ok(())
}

fn move_item<F: FnMut(u64)>(
  src: &Path,
  dest: &Path,
  cancel: &AtomicBool,
  paused: &AtomicBool,
  on_progress: &mut Option<&mut F>,
) -> Result<(), String> {
  if same_volume(src, dest) {
    fs::rename(src, dest).map_err(|e| e.to_string())
  } else {
    copy_recursive(src, dest, cancel, paused, on_progress)?;
    if src.is_dir() {
      fs::remove_dir_all(src).map_err(|e| e.to_string())?;
    } else {
      fs::remove_file(src).map_err(|e| e.to_string())?;
    }
    Ok(())
  }
}

fn run_transfer(
  job_id: u64,
  src_paths: Vec<String>,
  dest_dir: String,
  op: String,
  cancel: Arc<AtomicBool>,
  paused: Arc<AtomicBool>,
  state: Arc<TransferStateInner>,
  app: AppHandle,
) {
  let total = src_paths.len() as u64;
  if total == 0 {
    let _ = app.emit("fm://transfer-done", TransferDoneEvent {
      job_id,
      op: op.clone(),
      done: 0,
      total: 0,
      bytes_done: 0,
      bytes_total: 0,
      errors: vec![],
      cancelled: false,
    });
    state.remove(job_id);
    return;
  }

  if op == "move" && move_target_is_same_dir(&src_paths, &dest_dir) {
    let bytes_total: u64 = src_paths.iter().map(|p| total_size(Path::new(p))).sum();
    let _ = app.emit("fm://transfer-done", TransferDoneEvent {
      job_id,
      op,
      done: total,
      total,
      bytes_done: bytes_total,
      bytes_total,
      errors: vec![],
      cancelled: false,
    });
    state.remove(job_id);
    return;
  }

  let items: Vec<(String, String, u64)> = src_paths
    .iter()
    .map(|raw| {
      let src = Path::new(raw);
      let name = src
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| raw.clone());
      let size = total_size(src);
      (raw.clone(), name, size)
    })
    .collect();
  let bytes_total: u64 = items.iter().map(|(_, _, s)| *s).sum();

  let dest = Path::new(&dest_dir);
  let mut done = 0u64;
  let mut bytes_done = 0u64;
  let mut errors: Vec<String> = Vec::new();
  let mut last_emit = Instant::now();

  for (raw, name, item_bytes) in &items {
    if cancel.load(Relaxed) { break; }
    wait_while_paused(&cancel, &paused);

    let src = Path::new(raw);

    if last_emit.elapsed().as_millis() >= 100 || done == 0 {
      let _ = app.emit("fm://transfer-progress", TransferProgressEvent {
        job_id,
        op: op.clone(),
        done,
        total,
        bytes_done,
        bytes_total,
        current: name.clone(),
      });
      last_emit = Instant::now();
    }

    let dest_path = resolve_dest(dest, name);

    let mut on_progress = |delta: u64| {
      bytes_done += delta;
      if last_emit.elapsed().as_millis() >= 100 {
        let _ = app.emit(
          "fm://transfer-progress",
          TransferProgressEvent {
            job_id,
            op: op.clone(),
            done,
            total,
            bytes_done,
            bytes_total,
            current: name.clone(),
          },
        );
        last_emit = Instant::now();
      }
    };

    let mut progress_opt = Some(&mut on_progress);
    let result = if op == "move" {
      move_item(src, &dest_path, &cancel, &paused, &mut progress_opt)
    } else {
      copy_recursive(src, &dest_path, &cancel, &paused, &mut progress_opt)
    };

    match result {
      Ok(()) => {
        done += 1;
        bytes_done += item_bytes;
        if last_emit.elapsed().as_millis() >= 100 {
          let _ = app.emit("fm://transfer-progress", TransferProgressEvent {
            job_id,
            op: op.clone(),
            done,
            total,
            bytes_done,
            bytes_total,
            current: name.clone(),
          });
          last_emit = Instant::now();
        }
      }
      Err(e) if e == "cancelled" => break,
      Err(e) => errors.push(format!("{name}: {e}")),
    }
  }

  let cancelled = cancel.load(Relaxed);
  let _ = app.emit("fm://transfer-done", TransferDoneEvent {
    job_id,
    op,
    done,
    total,
    bytes_done,
    bytes_total,
    errors,
    cancelled,
  });
  state.remove(job_id);
}

#[tauri::command]
pub async fn paste_cmd(
  app: AppHandle,
  state: tauri::State<'_, TransferState>,
  src_paths: Vec<String>,
  dest_dir: String,
  op: String,
) -> Result<u64, String> {
  let job_id = state.next_id();
  let cancel = Arc::new(AtomicBool::new(false));
  let paused = Arc::new(AtomicBool::new(false));
  state.insert(job_id, cancel.clone(), paused.clone());
  let state_inner = state.0.clone();
  tokio::task::spawn_blocking(move || {
    run_transfer(job_id, src_paths, dest_dir, op, cancel, paused, state_inner, app);
  });
  Ok(job_id)
}

#[tauri::command(rename_all = "snake_case")]
pub fn cancel_transfer_cmd(state: tauri::State<'_, TransferState>, job_id: u64) {
  state.cancel(job_id);
}

#[tauri::command(rename_all = "snake_case")]
pub fn pause_transfer_cmd(state: tauri::State<'_, TransferState>, job_id: u64) {
  state.pause(job_id);
}

#[tauri::command(rename_all = "snake_case")]
pub fn resume_transfer_cmd(state: tauri::State<'_, TransferState>, job_id: u64) {
  state.resume(job_id);
}
