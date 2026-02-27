use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::UNIX_EPOCH;

use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tauri::{AppHandle, Emitter, LogicalPosition, Manager, Position, State};

mod core;
mod context_menu;
mod icons;
mod sidebar;
mod storage;
use core::{read_dir, walk_dir, CancelFlag, FileCoreState, ReadOptions};
use context_menu::{show_file_context_menu_cmd, FileContextMenuState};
use icons::get_file_icon_png_base64;
use sidebar::build_sidebar;
use storage::{bootstrap_storage, StoragePaths};
use tauri::menu::{Menu, MenuItem};

struct AddressMenuState {
  menu: Mutex<Option<Menu<tauri::Wry>>>,
}

impl AddressMenuState {
  fn new() -> Self {
    Self {
      menu: Mutex::new(None),
    }
  }
}

struct StorageState {
  paths: StoragePaths,
}

impl StorageState {
  fn new(paths: StoragePaths) -> Self {
    Self { paths }
  }
}

struct ActiveDirWatcher {
  watcher: RecommendedWatcher,
  path: String,
}

struct DirWatchState {
  active: Mutex<Option<ActiveDirWatcher>>,
}

impl DirWatchState {
  fn new() -> Self {
    Self {
      active: Mutex::new(None),
    }
  }
}

#[tauri::command]
async fn read_dir_cmd(
  app: AppHandle,
  state: State<'_, Arc<FileCoreState>>,
  path: String,
  opts: ReadOptions,
) -> Result<u64, String> {
  let request_id = state.new_request_id();
  let cancel = CancelFlag::new();
  state.insert_job(request_id, cancel.clone());

  let app_clone = app.clone();
  let state_clone = state.inner().clone();
  let path_buf = PathBuf::from(path);
  tauri::async_runtime::spawn(async move {
    read_dir(app_clone, state_clone, request_id, path_buf, opts, cancel).await;
  });

  Ok(request_id)
}

#[tauri::command]
async fn walk_dir_cmd(
  app: AppHandle,
  state: State<'_, Arc<FileCoreState>>,
  path: String,
  opts: ReadOptions,
) -> Result<u64, String> {
  let request_id = state.new_request_id();
  let cancel = CancelFlag::new();
  state.insert_job(request_id, cancel.clone());

  let app_clone = app.clone();
  let state_clone = state.inner().clone();
  let path_buf = PathBuf::from(path);
  tauri::async_runtime::spawn(async move {
    walk_dir(app_clone, state_clone, request_id, path_buf, opts, cancel).await;
  });

  Ok(request_id)
}

#[tauri::command]
fn cancel_cmd(state: State<'_, Arc<FileCoreState>>, request_id: u64) -> bool {
  state.cancel_job(request_id)
}

#[tauri::command]
fn get_file_icon_cmd(path: String, size: Option<u16>) -> Result<String, String> {
  let px = size.unwrap_or(24);
  get_file_icon_png_base64(&path, px)
}

#[tauri::command]
fn get_sidebar_cmd() -> Vec<sidebar::SidebarSection> {
  build_sidebar()
}

#[tauri::command]
fn get_storage_paths_cmd(state: State<'_, StorageState>) -> StoragePaths {
  state.paths.clone()
}

#[tauri::command]
fn open_path_cmd(path: String) -> Result<(), String> {
  let path_buf = PathBuf::from(&path);
  if !path_buf.exists() {
    return Err("path does not exist".to_string());
  }
  opener::open(path_buf).map_err(|e| e.to_string())
}

#[tauri::command]
fn is_valid_dir_cmd(path: String) -> bool {
  let path_buf = PathBuf::from(path);
  std::fs::metadata(path_buf)
    .map(|m| m.is_dir())
    .unwrap_or(false)
}

#[tauri::command]
fn expand_path_cmd(path: String) -> String {
  #[cfg(unix)]
  {
    shellexpand::full(&path)
      .map(|s| s.into_owned())
      .unwrap_or(path)
  }
  #[cfg(not(unix))]
  {
    path
  }
}

#[tauri::command]
fn rename_path_cmd(path: String, new_name: String) -> Result<String, String> {
  let old_path = PathBuf::from(&path);
  if !old_path.exists() {
    return Err("path does not exist".to_string());
  }

  let trimmed = new_name.trim();
  if trimmed.is_empty() {
    return Err("new name cannot be empty".to_string());
  }
  if trimmed.contains('/') || trimmed.contains('\\') {
    return Err("new name cannot include path separators".to_string());
  }

  let Some(parent) = old_path.parent() else {
    return Err("cannot rename root path".to_string());
  };

  let new_path = parent.join(trimmed);
  if new_path == old_path {
    return Ok(old_path.to_string_lossy().to_string());
  }
  if new_path.exists() {
    return Err("target already exists".to_string());
  }

  std::fs::rename(&old_path, &new_path).map_err(|e| e.to_string())?;
  Ok(new_path.to_string_lossy().to_string())
}

#[tauri::command]
fn create_empty_file_cmd(dir_path: String, file_name: String) -> Result<String, String> {
  let dir = PathBuf::from(&dir_path);
  if !dir.exists() {
    return Err("directory does not exist".to_string());
  }
  if !dir.is_dir() {
    return Err("target is not a directory".to_string());
  }

  let trimmed = file_name.trim();
  if trimmed.is_empty() {
    return Err("file name cannot be empty".to_string());
  }
  if trimmed.contains('/') || trimmed.contains('\\') {
    return Err("file name cannot include path separators".to_string());
  }

  let file_path = dir.join(trimmed);
  if file_path.exists() {
    return Err("target already exists".to_string());
  }

  std::fs::OpenOptions::new()
    .create_new(true)
    .write(true)
    .open(&file_path)
    .map_err(|e| e.to_string())?;

  Ok(file_path.to_string_lossy().to_string())
}

#[tauri::command]
fn create_directory_cmd(dir_path: String, dir_name: String) -> Result<String, String> {
  let dir = PathBuf::from(&dir_path);
  if !dir.exists() {
    return Err("directory does not exist".to_string());
  }
  if !dir.is_dir() {
    return Err("target is not a directory".to_string());
  }

  let trimmed = dir_name.trim();
  if trimmed.is_empty() {
    return Err("folder name cannot be empty".to_string());
  }
  if trimmed.contains('/') || trimmed.contains('\\') {
    return Err("folder name cannot include path separators".to_string());
  }

  let target_dir = dir.join(trimmed);
  if target_dir.exists() {
    return Err("target already exists".to_string());
  }

  std::fs::create_dir(&target_dir).map_err(|e| e.to_string())?;
  Ok(target_dir.to_string_lossy().to_string())
}

#[tauri::command]
fn delete_paths_cmd(paths: Vec<String>, permanent: Option<bool>) -> Result<u32, String> {
  if paths.is_empty() {
    return Ok(0);
  }
  let permanent = permanent.unwrap_or(false);

  let mut deleted = 0u32;
  let mut errors: Vec<String> = Vec::new();

  for raw in paths {
    let path = PathBuf::from(&raw);
    if !path.is_absolute() {
      errors.push(format!("{raw}: path must be absolute"));
      continue;
    }
    if path.parent().is_none() {
      errors.push(format!("{raw}: refusing to delete filesystem root"));
      continue;
    }
    let meta = match std::fs::symlink_metadata(&path) {
      Ok(m) => m,
      Err(e) => {
        errors.push(format!("{raw}: {e}"));
        continue;
      }
    };

    let result = if permanent {
      if meta.is_dir() {
        std::fs::remove_dir_all(&path)
      } else {
        std::fs::remove_file(&path)
      }
    } else {
      trash::delete(&path).map_err(std::io::Error::other)
    };

    match result {
      Ok(_) => deleted += 1,
      Err(e) => errors.push(format!("{raw}: {e}")),
    }
  }

  if errors.is_empty() {
    Ok(deleted)
  } else {
    Err(errors.join("\n"))
  }
}

fn trash_root_dir() -> Result<PathBuf, String> {
  let Some(base) = directories::BaseDirs::new() else {
    return Err("unable to resolve home directory".to_string());
  };

  #[cfg(target_os = "linux")]
  {
    return Ok(base.home_dir().join(".local/share/Trash/files"));
  }
  #[cfg(target_os = "macos")]
  {
    return Ok(base.home_dir().join(".Trash"));
  }
  #[cfg(target_os = "windows")]
  {
    Err("trash listing is not supported on Windows yet".to_string())
  }
}

fn to_dir_entry_info(path: &std::path::Path, metadata: std::fs::Metadata) -> core::DirEntryInfo {
  let name = path
    .file_name()
    .and_then(|n| n.to_str())
    .unwrap_or_default()
    .to_string();
  let is_dir = metadata.is_dir();
  let size = if metadata.is_file() { Some(metadata.len()) } else { None };
  let modified_ms = metadata
    .modified()
    .ok()
    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
    .map(|d| d.as_millis());
  let ext = path.extension().and_then(|s| s.to_str()).map(|s| s.to_string());

  core::DirEntryInfo {
    path: path.to_string_lossy().to_string(),
    name,
    is_dir,
    size,
    modified_ms,
    ext,
    hidden: false,
  }
}

#[tauri::command]
fn list_trash_entries_cmd() -> Result<Vec<core::DirEntryInfo>, String> {
  let root = trash_root_dir()?;
  if !root.exists() {
    return Ok(Vec::new());
  }
  if !root.is_dir() {
    return Err("trash location is not a directory".to_string());
  }

  let mut out = Vec::new();
  let entries = std::fs::read_dir(&root).map_err(|e| e.to_string())?;
  for entry in entries {
    let Ok(entry) = entry else {
      continue;
    };
    let path = entry.path();
    let Ok(metadata) = entry.metadata() else {
      continue;
    };
    out.push(to_dir_entry_info(&path, metadata));
  }
  Ok(out)
}

#[tauri::command]
fn empty_trash_cmd() -> Result<u32, String> {
  #[cfg(any(target_os = "linux", target_os = "windows"))]
  {
    let items = trash::os_limited::list().map_err(|e| e.to_string())?;
    if items.is_empty() {
      return Ok(0);
    }

    let mut purged = 0u32;
    let mut errors: Vec<String> = Vec::new();
    for item in items {
      match trash::os_limited::purge_all(vec![item]) {
        Ok(_) => purged += 1,
        Err(err) => {
          // Ignore races where an item was already removed by another process.
          let is_not_found = match &err {
            trash::Error::FileSystem { source, .. } => {
              source.kind() == std::io::ErrorKind::NotFound
            }
            _ => false,
          };
          if !is_not_found {
            errors.push(err.to_string());
          }
        }
      }
    }

    if errors.is_empty() {
      return Ok(purged);
    }
    return Err(errors.join("\n"));
  }

  #[cfg(not(any(target_os = "linux", target_os = "windows")))]
  {
    Err("empty trash is not supported on this platform yet".to_string())
  }
}

#[tauri::command]
fn stop_dir_watch_cmd(state: State<'_, DirWatchState>) -> Result<(), String> {
  let mut guard = state.active.lock().unwrap();
  if let Some(mut active) = guard.take() {
    let _ = active.watcher.unwatch(std::path::Path::new(&active.path));
  }
  Ok(())
}

#[tauri::command]
fn start_dir_watch_cmd(
  app: AppHandle,
  state: State<'_, DirWatchState>,
  path: String,
) -> Result<(), String> {
  let dir = PathBuf::from(&path);
  if !dir.is_dir() {
    return Err("watch target is not a directory".to_string());
  }
  let watch_path = dir.to_string_lossy().to_string();
  let emit_path = watch_path.clone();
  let app_handle = app.clone();

  let mut watcher = notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
    let Ok(event) = res else {
      return;
    };
    if matches!(event.kind, EventKind::Access(_)) {
      return;
    }
    let _ = app_handle.emit("fm://dir-changed", emit_path.clone());
  })
  .map_err(|e| e.to_string())?;

  watcher
    .watch(std::path::Path::new(&watch_path), RecursiveMode::NonRecursive)
    .map_err(|e| e.to_string())?;

  let mut guard = state.active.lock().unwrap();
  if let Some(mut old) = guard.take() {
    let _ = old.watcher.unwatch(std::path::Path::new(&old.path));
  }
  *guard = Some(ActiveDirWatcher {
    watcher,
    path: watch_path,
  });
  Ok(())
}

#[tauri::command]
fn show_address_menu_cmd(
  window: tauri::Window,
  state: State<'_, AddressMenuState>,
  x: f64,
  y: f64,
) -> Result<(), String> {
  let menu_guard = state.menu.lock().unwrap();
  let Some(menu) = menu_guard.as_ref() else {
    return Err("menu not initialized".to_string());
  };

  window
    .popup_menu_at(menu, Position::Logical(LogicalPosition::new(x, y)))
    .map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  let storage_paths = bootstrap_storage().expect("failed to bootstrap storage");

  tauri::Builder::default()
    .manage(Arc::new(FileCoreState::new()))
    .manage(AddressMenuState::new())
    .manage(FileContextMenuState::new())
    .manage(DirWatchState::new())
    .manage(StorageState::new(storage_paths))
    .invoke_handler(tauri::generate_handler![
      read_dir_cmd,
      walk_dir_cmd,
      cancel_cmd,
      get_file_icon_cmd,
      get_sidebar_cmd,
      get_storage_paths_cmd,
      open_path_cmd,
      is_valid_dir_cmd,
      expand_path_cmd,
      rename_path_cmd,
      create_empty_file_cmd,
      create_directory_cmd,
      delete_paths_cmd,
      list_trash_entries_cmd,
      empty_trash_cmd,
      start_dir_watch_cmd,
      stop_dir_watch_cmd,
      show_address_menu_cmd,
      show_file_context_menu_cmd
    ])
    .setup(|app| {
      let handle = app.handle();
      let copy_item = MenuItem::with_id(handle, "address.copy", "Copy Address", true, None::<&str>)?;
      let clear_item = MenuItem::with_id(handle, "address.clear", "Clear History", true, None::<&str>)?;
      let menu = Menu::with_items(handle, &[&copy_item, &clear_item])?;

      if let Some(state) = app.try_state::<AddressMenuState>() {
        let mut guard = state.menu.lock().unwrap();
        *guard = Some(menu);
      }

      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }
      app.handle().plugin(tauri_plugin_clipboard_manager::init())?;
      app.handle().plugin(tauri_plugin_dialog::init())?;
      app.handle().plugin(tauri_plugin_store::Builder::default().build())?;
      Ok(())
    })
    .on_menu_event(|app, event| {
      if event.id() == "address.copy" {
        let _ = app.emit("fm://address-menu", "copy");
      } else if event.id() == "address.clear" {
        let _ = app.emit("fm://address-menu", "clear");
      } else if event.id() == "context.info" {
        context_menu::emit_last_click(app);
      }
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
