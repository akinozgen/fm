use serde::Serialize;
use std::collections::HashMap;
#[cfg(windows)]
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::{
  atomic::{AtomicBool, AtomicU64, Ordering},
  Arc, Mutex,
};
use std::time::UNIX_EPOCH;
use tauri::AppHandle;
use tauri::Emitter;
use walkdir::WalkDir;

#[derive(Clone)]
pub struct CancelFlag(Arc<AtomicBool>);

impl CancelFlag {
  pub fn new() -> Self {
    Self(Arc::new(AtomicBool::new(false)))
  }

  pub fn cancel(&self) {
    self.0.store(true, Ordering::Relaxed);
  }

  pub fn is_cancelled(&self) -> bool {
    self.0.load(Ordering::Relaxed)
  }
}

pub struct FileCoreState {
  next_id: AtomicU64,
  jobs: Mutex<HashMap<u64, CancelFlag>>,
}

impl FileCoreState {
  pub fn new() -> Self {
    Self {
      next_id: AtomicU64::new(1),
      jobs: Mutex::new(HashMap::new()),
    }
  }

  pub fn new_request_id(&self) -> u64 {
    self.next_id.fetch_add(1, Ordering::Relaxed)
  }

  pub fn insert_job(&self, id: u64, flag: CancelFlag) {
    let mut jobs = self.jobs.lock().unwrap();
    jobs.insert(id, flag);
  }

  pub fn cancel_job(&self, id: u64) -> bool {
    let jobs = self.jobs.lock().unwrap();
    if let Some(flag) = jobs.get(&id) {
      flag.cancel();
      true
    } else {
      false
    }
  }

  pub fn remove_job(&self, id: u64) {
    let mut jobs = self.jobs.lock().unwrap();
    jobs.remove(&id);
  }
}

#[derive(Debug, Serialize, Clone)]
pub struct DirEntryInfo {
  pub path: String,
  pub name: String,
  pub is_dir: bool,
  pub size: Option<u64>,
  pub modified_ms: Option<u128>,
  pub ext: Option<String>,
  pub hidden: bool,
}

#[derive(Debug, Serialize, Clone)]
pub struct ReadChunk {
  pub request_id: u64,
  pub root_path: String,
  pub seq: u64,
  pub done: bool,
  pub error: Option<String>,
  pub entries: Vec<DirEntryInfo>,
}

#[derive(Debug, serde::Deserialize, Clone)]
pub struct ReadOptions {
  pub recursive: Option<bool>,
  pub include_hidden: Option<bool>,
  pub max_depth: Option<u32>,
  pub chunk_size: Option<usize>,
}

fn is_hidden_entry(name: &str, metadata: &std::fs::Metadata) -> bool {
  if name.starts_with('.') {
    return true;
  }
  #[cfg(windows)]
  {
    const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
    return metadata.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0;
  }
  #[cfg(not(windows))]
  {
    let _ = metadata;
    false
  }
}

fn to_entry_info(path: &Path, name: String, metadata: std::fs::Metadata) -> DirEntryInfo {
  let is_dir = metadata.is_dir();
  let size = if metadata.is_file() { Some(metadata.len()) } else { None };
  let modified_ms = metadata
    .modified()
    .ok()
    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
    .map(|d| d.as_millis());
  let ext = path.extension().and_then(|s| s.to_str()).map(|s| s.to_string());
  let hidden = is_hidden_entry(&name, &metadata);

  DirEntryInfo {
    path: path.to_string_lossy().to_string(),
    name,
    is_dir,
    size,
    modified_ms,
    ext,
    hidden,
  }
}

fn emit_chunk(app: &AppHandle, chunk: ReadChunk) {
  let _ = app.emit("fm://dir-chunk", chunk);
}

pub async fn read_dir(
  app: AppHandle,
  state: Arc<FileCoreState>,
  request_id: u64,
  path: PathBuf,
  opts: ReadOptions,
  cancel: CancelFlag,
) {
  let root_path = path.to_string_lossy().to_string();
  let root_path_for_task = root_path.clone();
  let include_hidden = opts.include_hidden.unwrap_or(false);
  let chunk_size = opts.chunk_size.unwrap_or(200).max(1);

  let mut seq = 0u64;
  let mut buffer: Vec<DirEntryInfo> = Vec::with_capacity(chunk_size);
  let app_for_task = app.clone();

  let result = tokio::task::spawn_blocking(move || -> std::io::Result<()> {
    let entries = std::fs::read_dir(&path)?;
    for entry in entries {
      if cancel.is_cancelled() {
        return Ok(());
      }

      let entry = entry?;
      let file_name = entry
        .file_name()
        .to_string_lossy()
        .to_string();
      let metadata = match entry.metadata() {
        Ok(m) => m,
        Err(_) => continue,
      };
      if !include_hidden && is_hidden_entry(&file_name, &metadata) {
        continue;
      }
      buffer.push(to_entry_info(&entry.path(), file_name, metadata));

      if buffer.len() >= chunk_size {
        seq += 1;
        emit_chunk(
          &app_for_task,
          ReadChunk {
            request_id,
            root_path: root_path_for_task.clone(),
            seq,
            done: false,
            error: None,
            entries: std::mem::take(&mut buffer),
          },
        );
      }
    }

    if !buffer.is_empty() {
      seq += 1;
      emit_chunk(
        &app_for_task,
        ReadChunk {
          request_id,
          root_path: root_path_for_task.clone(),
          seq,
          done: true,
          error: None,
          entries: std::mem::take(&mut buffer),
        },
      );
    } else {
      seq += 1;
      emit_chunk(
        &app_for_task,
        ReadChunk {
          request_id,
          root_path: root_path_for_task.clone(),
          seq,
          done: true,
          error: None,
          entries: Vec::new(),
        },
      );
    }

    Ok(())
  })
  .await;

  if let Ok(Err(err)) = result {
    emit_chunk(
      &app,
      ReadChunk {
        request_id,
        root_path: root_path.clone(),
        seq: 0,
        done: true,
        error: Some(err.to_string()),
        entries: Vec::new(),
      },
    );
  } else if let Err(join_err) = result {
    emit_chunk(
      &app,
      ReadChunk {
        request_id,
        root_path,
        seq: 0,
        done: true,
        error: Some(join_err.to_string()),
        entries: Vec::new(),
      },
    );
  }
  state.remove_job(request_id);
}

pub async fn walk_dir(
  app: AppHandle,
  state: Arc<FileCoreState>,
  request_id: u64,
  path: PathBuf,
  opts: ReadOptions,
  cancel: CancelFlag,
) {
  let root_path = path.to_string_lossy().to_string();
  let root_path_for_task = root_path.clone();
  let include_hidden = opts.include_hidden.unwrap_or(false);
  let chunk_size = opts.chunk_size.unwrap_or(200).max(1);
  let max_depth = opts.max_depth;

  let mut seq = 0u64;
  let mut buffer: Vec<DirEntryInfo> = Vec::with_capacity(chunk_size);
  let app_for_task = app.clone();

  let result = tokio::task::spawn_blocking(move || -> std::io::Result<()> {
    let mut walker = WalkDir::new(&path);
    if let Some(d) = max_depth {
      walker = walker.max_depth(d as usize);
    }
    let mut it = walker.into_iter();

    while let Some(entry) = it.next() {
      if cancel.is_cancelled() {
        return Ok(());
      }

      let entry = match entry {
        Ok(e) => e,
        Err(_) => continue,
      };

      if entry.path() == path.as_path() {
        continue;
      }

      let name = entry.file_name().to_string_lossy().to_string();
      let metadata = match entry.metadata() {
        Ok(m) => m,
        Err(_) => continue,
      };
      let hidden = is_hidden_entry(&name, &metadata);
      if hidden && !include_hidden {
        if entry.file_type().is_dir() {
          it.skip_current_dir();
        }
        continue;
      }

      buffer.push(to_entry_info(entry.path(), name, metadata));

      if buffer.len() >= chunk_size {
        seq += 1;
        emit_chunk(
          &app_for_task,
          ReadChunk {
            request_id,
            root_path: root_path_for_task.clone(),
            seq,
            done: false,
            error: None,
            entries: std::mem::take(&mut buffer),
          },
        );
      }
    }

    if !buffer.is_empty() {
      seq += 1;
      emit_chunk(
        &app_for_task,
        ReadChunk {
          request_id,
          root_path: root_path_for_task.clone(),
          seq,
          done: true,
          error: None,
          entries: std::mem::take(&mut buffer),
        },
      );
    } else {
      seq += 1;
      emit_chunk(
        &app_for_task,
        ReadChunk {
          request_id,
          root_path: root_path_for_task.clone(),
          seq,
          done: true,
          error: None,
          entries: Vec::new(),
        },
      );
    }

    Ok(())
  })
  .await;

  if let Ok(Err(err)) = result {
    emit_chunk(
      &app,
      ReadChunk {
        request_id,
        root_path: root_path.clone(),
        seq: 0,
        done: true,
        error: Some(err.to_string()),
        entries: Vec::new(),
      },
    );
  } else if let Err(join_err) = result {
    emit_chunk(
      &app,
      ReadChunk {
        request_id,
        root_path,
        seq: 0,
        done: true,
        error: Some(join_err.to_string()),
        entries: Vec::new(),
      },
    );
  }
  state.remove_job(request_id);
}
