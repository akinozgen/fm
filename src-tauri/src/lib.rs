use std::error::Error as _;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
#[cfg(not(target_os = "windows"))]
use std::time::UNIX_EPOCH;

use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tauri::{AppHandle, Emitter, LogicalPosition, Manager, Position, State};

mod core;
mod context_menu;
mod dir_size;
mod icons;
mod sidebar;
mod storage;
mod thumbnails;
mod transfer;
use core::{read_dir, walk_dir, CancelFlag, FileCoreState, ReadOptions};
use context_menu::{show_file_context_menu_cmd, ContextMenuState};
use dir_size::{cancel_dir_size_cmd, compute_dir_size_cmd, DirSizeState};
use icons::get_file_icon_png_base64;
use sidebar::build_sidebar;
use storage::{bootstrap_storage, StoragePaths};
use thumbnails::ThumbnailState;
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

  let path_trimmed = path.trim();
  #[cfg(windows)]
  if is_unc_path(path_trimmed) {
    return Err("Network paths are not supported".to_string());
  }
  let app_clone = app.clone();
  let state_clone = state.inner().clone();
  #[cfg(windows)]
  let path_buf = resolve_windows_unix_style_path(path_trimmed).unwrap_or_else(|| PathBuf::from(path));
  #[cfg(not(windows))]
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

  let path_trimmed = path.trim();
  #[cfg(windows)]
  if is_unc_path(path_trimmed) {
    return Err("Network paths are not supported".to_string());
  }
  let app_clone = app.clone();
  let state_clone = state.inner().clone();
  #[cfg(windows)]
  let path_buf = resolve_windows_unix_style_path(path_trimmed).unwrap_or_else(|| PathBuf::from(path));
  #[cfg(not(windows))]
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

/// On Windows, resolve Unix-style paths that frontend may send ("/", "/Users") to real paths.
#[cfg(windows)]
fn resolve_windows_unix_style_path(trimmed: &str) -> Option<PathBuf> {
  let normalized = trimmed.trim().replace('/', std::path::MAIN_SEPARATOR_STR);
  if normalized.is_empty() || normalized == "\\" {
    return std::env::current_dir().ok().map(|p| p.components().take(2).collect::<PathBuf>());
  }
  if normalized.eq_ignore_ascii_case("\\Users") || normalized.eq_ignore_ascii_case("\\Users\\") {
    return std::env::current_dir()
      .ok()
      .map(|p| p.components().take(2).collect::<PathBuf>().join("Users"));
  }
  None
}

#[tauri::command]
fn get_file_icon_cmd(path: String, size: Option<u16>) -> Result<String, String> {
  let trimmed = path.trim();
  if trimmed.is_empty() || trimmed.starts_with("fm://") {
    return Err("icon unavailable for virtual path".to_string());
  }

  let path_buf = PathBuf::from(trimmed);
  #[cfg(windows)]
  let path_buf = resolve_windows_unix_style_path(trimmed).unwrap_or(path_buf);
  #[cfg(windows)]
  if !path_buf.is_absolute() {
    return Err("icon path must be absolute".to_string());
  }
  if !path_buf.exists() {
    return Err("icon path does not exist".to_string());
  }

  let px = size.unwrap_or(24);
  let resolved = std::fs::canonicalize(&path_buf).unwrap_or(path_buf);
  get_file_icon_png_base64(&resolved.to_string_lossy(), px)
}

#[tauri::command]
async fn get_thumbnails_batch_cmd(
  storage: State<'_, StorageState>,
  thumb_state: State<'_, ThumbnailState>,
  paths: Vec<String>,
  size: u32,
) -> Result<Vec<Option<String>>, String> {
  let thumb_dir = PathBuf::from(&storage.paths.config_dir).join("thumbnails");
  // Reset cancel flag so a fresh batch runs to completion unless interrupted.
  thumb_state.cancel.store(false, std::sync::atomic::Ordering::SeqCst);
  let cancel = thumb_state.cancel.clone();
  let pool = thumb_state.pool.clone();
  tokio::task::spawn_blocking(move || {
    thumbnails::get_thumbnails_parallel(&paths, size, &thumb_dir, &cancel, &pool)
  })
  .await
  .map_err(|e| e.to_string())
}

#[tauri::command]
fn cancel_thumbnails_cmd(thumb_state: State<'_, ThumbnailState>) {
  thumb_state.cancel.store(true, std::sync::atomic::Ordering::SeqCst);
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

#[cfg(windows)]
fn is_unc_path(s: &str) -> bool {
  let t = s.trim_start();
  t.starts_with("\\\\") || (t.starts_with("//") && t.chars().nth(2).map_or(false, |c| c != '/'))
}

/// Open a path if it exists, otherwise pass the input to the OS (URL, command name, etc.).
/// Used by the address bar: cmd, calc, http://google.com, or existing file paths.
/// Windows UNC paths (\\server\share) are rejected; network shares are not supported.
#[tauri::command]
fn open_or_run_cmd(input: String) -> Result<(), String> {
  let trimmed = input.trim();
  if trimmed.is_empty() {
    return Err("input is empty".to_string());
  }
  #[cfg(windows)]
  if is_unc_path(trimmed) {
    return Err("Network paths are not supported".to_string());
  }
  let path_buf = PathBuf::from(trimmed);
  if path_buf.exists() {
    opener::open(path_buf).map_err(|e| e.to_string())
  } else {
    opener::open(trimmed).map_err(|e| e.to_string())
  }
}

#[tauri::command]
fn is_valid_dir_cmd(path: String) -> bool {
  #[cfg(windows)]
  if is_unc_path(&path) {
    return false;
  }
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
      // On Windows, items listed from the Recycle Bin have paths inside
      // $Recycle.Bin. Use purge_all so both the $R data file and the $I
      // metadata file are removed and the shell is notified.
      #[cfg(target_os = "windows")]
      {
        let in_recycle_bin = path
          .to_string_lossy()
          .to_lowercase()
          .contains("\\$recycle.bin\\");
        if in_recycle_bin {
          match trash::os_limited::list() {
            Ok(items) => {
              let matching: Vec<_> =
                items.into_iter().filter(|i| PathBuf::from(&i.id) == path).collect();
              if !matching.is_empty() {
                trash::os_limited::purge_all(matching).map_err(std::io::Error::other)
              } else {
                // Not found in trash list — delete directly as fallback
                if meta.is_dir() { std::fs::remove_dir_all(&path) } else { std::fs::remove_file(&path) }
              }
            }
            Err(e) => Err(std::io::Error::other(e)),
          }
        } else {
          if meta.is_dir() { std::fs::remove_dir_all(&path) } else { std::fs::remove_file(&path) }
        }
      }
      #[cfg(not(target_os = "windows"))]
      {
        if meta.is_dir() { std::fs::remove_dir_all(&path) } else { std::fs::remove_file(&path) }
      }
    } else {
      let mut result = trash::delete(&path).map_err(std::io::Error::other);
      if result.is_err() {
        let err_str = result.as_ref().unwrap_err().to_string();
        if err_str.contains("aborted") || err_str.contains("in use") || err_str.contains("access") {
          std::thread::sleep(std::time::Duration::from_millis(400));
          result = trash::delete(&path).map_err(std::io::Error::other);
        }
      }
      result
    };

    match result {
      Ok(_) => deleted += 1,
      Err(e) => {
        let msg = e.to_string();
        let friendly = if msg.contains("aborted") || msg.contains("Some operations were aborted") {
          "File may be in use (e.g. after cancelling a copy). Try again in a moment or use permanent delete."
        } else {
          msg.as_str()
        };
        errors.push(format!("{raw}: {friendly}"));
      }
    }
  }

  if errors.is_empty() {
    Ok(deleted)
  } else {
    Err(errors.join("\n"))
  }
}

#[cfg(not(target_os = "windows"))]
fn trash_root_dir() -> Result<PathBuf, String> {
  let Some(_base) = directories::BaseDirs::new() else {
    return Err("unable to resolve home directory".to_string());
  };

  #[cfg(target_os = "linux")]
  {
    return Ok(_base.home_dir().join(".local/share/Trash/files"));
  }
  #[cfg(target_os = "macos")]
  {
    return Ok(_base.home_dir().join(".Trash"));
  }
  #[cfg(target_os = "windows")]
  {
    Err("trash listing is not supported on Windows yet".to_string())
  }
}

#[cfg(not(target_os = "windows"))]
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
  #[cfg(target_os = "windows")]
  {
    let items = trash::os_limited::list().map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for item in &items {
      // item.id is the full path of the $R* file inside $Recycle.Bin
      let rb_path = PathBuf::from(&item.id);
      let is_dir = rb_path.is_dir();
      let size = if !is_dir { std::fs::metadata(&rb_path).ok().map(|m| m.len()) } else { None };
      // time_deleted is a Unix timestamp in seconds
      let modified_ms = if item.time_deleted > 0 {
        Some((item.time_deleted as u128).saturating_mul(1000))
      } else {
        None
      };
      let name_str = item.name.to_string_lossy().to_string();
      let ext = std::path::Path::new(&item.name)
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string());
      out.push(core::DirEntryInfo {
        path: rb_path.to_string_lossy().to_string(),
        name: name_str,
        is_dir,
        size,
        modified_ms,
        ext,
        hidden: false,
      });
    }
    return Ok(out);
  }

  #[cfg(not(target_os = "windows"))]
  {
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
      let Ok(entry) = entry else { continue };
      let path = entry.path();
      let Ok(metadata) = entry.metadata() else { continue };
      out.push(to_dir_entry_info(&path, metadata));
    }
    Ok(out)
  }
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
          let is_not_found = err
            .source()
            .and_then(|source| source.downcast_ref::<std::io::Error>())
            .map(|source| source.kind() == std::io::ErrorKind::NotFound)
            .unwrap_or(false);
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
    .manage(DirWatchState::new())
    .manage(StorageState::new(storage_paths))
    .manage(ThumbnailState::new())
    .manage(DirSizeState::new())
    .manage(ContextMenuState::new())
    .manage(transfer::TransferState::new())
    .invoke_handler(tauri::generate_handler![
      read_dir_cmd,
      walk_dir_cmd,
      cancel_cmd,
      get_file_icon_cmd,
      get_thumbnails_batch_cmd,
      cancel_thumbnails_cmd,
      get_sidebar_cmd,
      get_storage_paths_cmd,
      open_path_cmd,
      open_or_run_cmd,
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
      show_file_context_menu_cmd,
      compute_dir_size_cmd,
      cancel_dir_size_cmd,
      transfer::paste_cmd,
      transfer::cancel_transfer_cmd,
      transfer::pause_transfer_cmd,
      transfer::resume_transfer_cmd
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
            .level_for("file_icon_provider", log::LevelFilter::Error)
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
      } else if event.id().as_ref().starts_with("context.") {
        context_menu::handle_menu_event(app, event.id().as_ref());
      }
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
