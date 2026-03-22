use std::collections::HashMap;
use std::error::Error as _;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
#[cfg(not(target_os = "windows"))]
use std::time::UNIX_EPOCH;

use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tauri::{AppHandle, Emitter, LogicalPosition, Manager, Position, State, WebviewUrl, WebviewWindowBuilder};

mod archive;
mod disk_image;
mod core;
mod context_menu;
mod dir_size;
mod icons;
mod indexer;
mod pinned_favorites;
mod sidebar;
mod storage;
mod thumbnails;
mod transfer;
use core::{read_dir, walk_dir, CancelFlag, FileCoreState, ReadOptions};
use indexer::IndexerState;
use context_menu::{show_file_context_menu_cmd, ContextMenuState};
use dir_size::{cancel_dir_size_cmd, compute_dir_size_cmd, DirSizeState};
use icons::get_file_icon_png_base64;
use sidebar::build_sidebar;
use storage::{bootstrap_storage, StoragePaths};
use thumbnails::ThumbnailState;
use tauri::menu::{AboutMetadata, CheckMenuItem, IsMenuItem, Menu, MenuBuilder, MenuItem,
                  PredefinedMenuItem, SubmenuBuilder};

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

struct NewItemMenuState {
  menu: Mutex<Option<Menu<tauri::Wry>>>,
}

impl NewItemMenuState {
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

struct EditorState {
  pending: Mutex<HashMap<String, String>>,
  counter: AtomicU64,
  watchers: Mutex<HashMap<String, notify::RecommendedWatcher>>,
}

impl EditorState {
  fn new() -> Self {
    Self {
      pending: Mutex::new(HashMap::new()),
      counter: AtomicU64::new(0),
      watchers: Mutex::new(HashMap::new()),
    }
  }
}

// ── Archive dialog state ──────────────────────────────────────────────────────

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct ArchiveDialogParams {
  kind:           String,   // "extract" | "create"
  paths:          Vec<String>,
  dest_dir:       String,
  selection_kind: String,   // "file"|"files"|"dir"|"dirs"|"mixed" (create only)
}

struct ArchiveState {
  pending: Mutex<HashMap<String, ArchiveDialogParams>>,
  counter: AtomicU64,
  cancels: Mutex<HashMap<String, Arc<std::sync::atomic::AtomicBool>>>,
}

impl ArchiveState {
  fn new() -> Self {
    Self {
      pending: Mutex::new(HashMap::new()),
      counter: AtomicU64::new(0),
      cancels: Mutex::new(HashMap::new()),
    }
  }
}

// ── Quick Look state ──────────────────────────────────────────────────────────
struct QuickLookState {
  path: Mutex<String>,
}

impl QuickLookState {
  fn new() -> Self {
    Self { path: Mutex::new(String::new()) }
  }
}

// ── Search ────────────────────────────────────────────────────────────────────
#[derive(serde::Serialize, Clone)]
struct SearchResult {
  path:        String,
  name:        String,
  ext:         Option<String>,
  is_dir:      bool,
  size:        Option<u64>,
  modified_ms: Option<u64>,
}

#[derive(serde::Serialize)]
struct IndexStats {
  file_count:   u64,
  last_indexed: Option<u64>,
  is_running:   bool,
}

// ── Quick Look: rich file metadata ────────────────────────────────────────────
#[derive(serde::Serialize)]
struct FileMetadata {
  path:         String,
  name:         String,
  ext:          Option<String>,
  size:         Option<u64>,
  modified_ms:  Option<u128>,
  created_ms:   Option<u128>,
  is_dir:       bool,
  image_width:  Option<u32>,
  image_height: Option<u32>,
  line_count:   Option<u64>,
  mime_type:    String,
}

fn ext_to_mime(ext: &str) -> &'static str {
  match ext {
    "png"                         => "image/png",
    "jpg" | "jpeg"                => "image/jpeg",
    "gif"                         => "image/gif",
    "webp"                        => "image/webp",
    "bmp"                         => "image/bmp",
    "avif"                        => "image/avif",
    "tiff" | "tif"                => "image/tiff",
    "ico"                         => "image/x-icon",
    "svg"                         => "image/svg+xml",
    "qoi"                         => "image/qoi",
    "pdf"                         => "application/pdf",
    "txt" | "md" | "markdown"     => "text/plain",
    "rs"                          => "text/x-rust",
    "js" | "mjs" | "cjs"         => "text/javascript",
    "ts"                          => "text/typescript",
    "jsx" | "tsx"                 => "text/jsx",
    "vue"                         => "text/x-vue",
    "html" | "htm"                => "text/html",
    "css" | "scss" | "sass"       => "text/css",
    "json"                        => "application/json",
    "toml"                        => "text/x-toml",
    "yaml" | "yml"                => "text/yaml",
    "xml"                         => "text/xml",
    "sh" | "bash" | "zsh"         => "text/x-shellscript",
    "py"                          => "text/x-python",
    "rb"                          => "text/x-ruby",
    "go"                          => "text/x-go",
    "java"                        => "text/x-java",
    "c" | "h"                     => "text/x-c",
    "cpp" | "cc" | "cxx" | "hpp"  => "text/x-c++",
    "swift"                       => "text/x-swift",
    "kt" | "kts"                  => "text/x-kotlin",
    "cs"                          => "text/x-csharp",
    "php"                         => "text/x-php",
    "mp4" | "m4v"                 => "video/mp4",
    "mov"                         => "video/quicktime",
    "webm"                        => "video/webm",
    "mp3"                         => "audio/mpeg",
    "m4a" | "aac"                 => "audio/mp4",
    "wav"                         => "audio/wav",
    "ogg"                         => "audio/ogg",
    "flac"                        => "audio/flac",
    "opus"                        => "audio/ogg; codecs=opus",
    _                             => "application/octet-stream",
  }
}

const IMAGE_EXTS: &[&str] = &[
  "jpg", "jpeg", "png", "gif", "webp", "bmp", "avif", "tiff", "tif", "ico", "qoi",
];

const TEXT_EXTS: &[&str] = &[
  "txt", "md", "markdown", "rs", "js", "mjs", "cjs", "ts", "jsx", "tsx", "vue",
  "html", "htm", "css", "scss", "sass", "json", "toml", "yaml", "yml", "xml",
  "sh", "bash", "zsh", "py", "rb", "go", "java", "c", "h", "cpp", "cc", "cxx",
  "hpp", "swift", "kt", "kts", "cs", "php", "lua", "r", "sql", "gitignore",
  "env", "dockerfile", "makefile", "cmake",
];

#[tauri::command]
fn get_file_metadata_cmd(path: String) -> Result<FileMetadata, String> {
  let p = std::path::Path::new(&path);
  let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
  let ext  = p.extension().and_then(|e| e.to_str()).map(|s| s.to_lowercase());
  let ext_str = ext.as_deref().unwrap_or("");
  let mime_type = ext_to_mime(ext_str).to_string();

  let meta = std::fs::metadata(&path).map_err(|e| e.to_string())?;
  let is_dir = meta.is_dir();
  let size   = if is_dir { None } else { Some(meta.len()) };

  let modified_ms = meta.modified().ok()
    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
    .map(|d| d.as_millis());
  let created_ms = meta.created().ok()
    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
    .map(|d| d.as_millis());

  // Image dimensions — fast header-only read
  let (image_width, image_height) = if IMAGE_EXTS.contains(&ext_str) {
    match image::ImageReader::open(&path)
      .ok()
      .and_then(|r| r.with_guessed_format().ok())
      .and_then(|r| r.into_dimensions().ok())
    {
      Some((w, h)) => (Some(w), Some(h)),
      None => (None, None),
    }
  } else {
    (None, None)
  };

  // Line count — read up to 1 MiB, count newlines
  let line_count = if TEXT_EXTS.contains(&ext_str) || TEXT_EXTS.iter().any(|e| *e == ext_str) {
    use std::io::Read;
    std::fs::File::open(&path).ok().map(|mut f| {
      let mut buf = vec![0u8; 1 << 20]; // 1 MiB
      let n = f.read(&mut buf).unwrap_or(0);
      buf[..n].iter().filter(|&&b| b == b'\n').count() as u64 + if n > 0 { 1 } else { 0 }
    })
  } else {
    None
  };

  Ok(FileMetadata { path, name, ext, size, modified_ms, created_ms, is_dir,
                    image_width, image_height, line_count, mime_type })
}

#[tauri::command]
fn open_devtools_cmd(app: AppHandle) {
  if let Some(w) = app.get_webview_window("main") {
    #[cfg(debug_assertions)]
    w.open_devtools();
  }
}

#[tauri::command]
fn open_quicklook_cmd(
  app: AppHandle,
  state: State<'_, QuickLookState>,
  path: String,
) -> Result<(), String> {
  *state.path.lock().unwrap() = path.clone();

  // If window already exists, update it and bring it to front
  if let Some(win) = app.get_webview_window("quicklook") {
    let _ = win.emit("fm://quicklook-navigate", &path);
    let _ = win.set_focus();
    return Ok(());
  }

  let builder = WebviewWindowBuilder::new(&app, "quicklook", WebviewUrl::App("quicklook.html".into()))
    .title("")
    .inner_size(480.0, 640.0)
    .min_inner_size(360.0, 480.0)
    .resizable(true);

  #[cfg(target_os = "macos")]
  let builder = builder.title_bar_style(tauri::TitleBarStyle::Overlay).title("");

  #[cfg(not(target_os = "macos"))]
  let builder = builder.decorations(false);

  builder.build().map_err(|e| e.to_string())?;
  Ok(())
}

#[tauri::command]
fn get_quicklook_path_cmd(state: State<'_, QuickLookState>) -> Result<String, String> {
  Ok(state.path.lock().unwrap().clone())
}

// ── Audio metadata (album art, tags, duration) ────────────────────────────────
#[derive(serde::Serialize)]
struct AudioMetadata {
  title:          Option<String>,
  artist:         Option<String>,
  album:          Option<String>,
  duration_secs:  Option<f64>,
  artwork_base64: Option<String>,
  artwork_mime:   Option<String>,
}

#[tauri::command]
fn get_audio_metadata_cmd(path: String) -> Result<AudioMetadata, String> {
  use lofty::prelude::{Accessor, AudioFile, TaggedFileExt};
  use lofty::probe::Probe;

  let tagged = Probe::open(&path)
    .map_err(|e| e.to_string())?
    .guess_file_type()
    .map_err(|e| e.to_string())?
    .read()
    .map_err(|e| e.to_string())?;

  let duration_secs = Some(tagged.properties().duration().as_secs_f64());

  let tag = tagged.primary_tag();
  let (title, artist, album, artwork_base64, artwork_mime) = if let Some(tag) = tag {
    let title  = tag.title().map(|s| s.to_string());
    let artist = tag.artist().map(|s| s.to_string());
    let album  = tag.album().map(|s| s.to_string());

    let (art_b64, art_mime) = tag.pictures().first().map(|pic| {
      use base64::Engine;
      let b64 = base64::engine::general_purpose::STANDARD.encode(pic.data());
      let mime = match pic.mime_type() {
        Some(lofty::picture::MimeType::Png)  => "image/png".to_string(),
        _                                     => "image/jpeg".to_string(),
      };
      (b64, mime)
    }).map(|(b, m)| (Some(b), Some(m))).unwrap_or((None, None));

    (title, artist, album, art_b64, art_mime)
  } else {
    (None, None, None, None, None)
  };

  Ok(AudioMetadata { title, artist, album, duration_secs, artwork_base64, artwork_mime })
}

// ── App menu dynamic path store ───────────────────────────────────────────────
struct MenuPathStore {
  vol_paths:     Mutex<Vec<String>>,
  home_paths:    Mutex<Vec<String>>,
  history_paths: Mutex<Vec<String>>,
}

impl MenuPathStore {
  fn new() -> Self {
    Self {
      vol_paths:     Mutex::new(Vec::new()),
      home_paths:    Mutex::new(Vec::new()),
      history_paths: Mutex::new(Vec::new()),
    }
  }
}

// ── App menu params (from JS) ─────────────────────────────────────────────────
#[derive(serde::Deserialize, Clone)]
struct MenuVolume {
  label: String,
  path:  String,
}

#[derive(serde::Deserialize)]
struct AppMenuParams {
  view_mode:       String,
  show_hidden:     bool,
  show_extensions: bool,
  show_checkboxes: bool,
  sort_by:         String,
  sort_dir:        String,
  can_go_back:     bool,
  can_go_forward:  bool,
  can_go_up:       bool,
  history_paths:   Vec<String>,
  volumes:         Vec<MenuVolume>,
  home_dirs:       Vec<MenuVolume>,
  has_selection:   bool,
  has_clipboard:   bool,
}

// ── macOS menu builder ────────────────────────────────────────────────────────
#[cfg(target_os = "macos")]
fn build_and_set_macos_menu(app: &AppHandle, p: AppMenuParams) -> Result<(), String> {
  let e = |e: tauri::Error| e.to_string();

  // Update path store so on_menu_event can resolve IDs → paths
  if let Some(store) = app.try_state::<MenuPathStore>() {
    *store.vol_paths.lock().unwrap()     = p.volumes.iter().map(|v| v.path.clone()).collect();
    *store.home_paths.lock().unwrap()    = p.home_dirs.iter().map(|v| v.path.clone()).collect();
    *store.history_paths.lock().unwrap() = p.history_paths.clone();
  }

  // ── FM submenu ──
  let make_about = || AboutMetadata {
    name:          Some("FM".to_string()),
    version:       Some(env!("CARGO_PKG_VERSION").to_string()),
    short_version: None,
    authors:       Some(vec!["Akın Özgen".to_string()]),
    comments:      Some("A cross-platform file manager that bridges modern web technologies with native OS capabilities.".to_string()),
    copyright:     Some("© 2026 Akın Özgen".to_string()),
    license:       Some("MIT".to_string()),
    website:       Some("https://akinozgen.com".to_string()),
    website_label: Some("akinozgen.com".to_string()),
    credits:       Some(concat!(
      "Website\nhttps://akinozgen.com/projects/fm\n\n",
      "Source Code\nhttps://github.com/akinozgen/fm\n\n",
      "License\nMIT — Permission is hereby granted, free of charge, to any\n",
      "person obtaining a copy of this software to use, copy, modify,\n",
      "merge, publish, distribute, sublicense, and/or sell copies of\n",
      "the Software, subject to the above copyright notice.",
    ).to_string()),
    icon:          None,
  };
  let fm_submenu = SubmenuBuilder::new(app, "FM")
    .item(&PredefinedMenuItem::about(app, Some("About FM"), Some(make_about())).map_err(e)?)
    .separator()
    .item(&MenuItem::with_id(app, "menu.fm.website", "Website", true, None::<&str>).map_err(e)?)
    .separator()
    .services()
    .separator()
    .hide()
    .hide_others()
    .show_all()
    .separator()
    .quit()
    .build()
    .map_err(e)?;

  // ── Edit submenu ──
  let edit_submenu = SubmenuBuilder::new(app, "Edit")
    .item(&MenuItem::with_id(app, "menu.edit.cut",        "Cut",              p.has_selection, None::<&str>).map_err(e)?)
    .item(&MenuItem::with_id(app, "menu.edit.copy",       "Copy",             p.has_selection, None::<&str>).map_err(e)?)
    .item(&MenuItem::with_id(app, "menu.edit.paste",      "Paste",            p.has_clipboard, None::<&str>).map_err(e)?)
    .separator()
    .item(&MenuItem::with_id(app, "menu.edit.select_all", "Select All",       true,            None::<&str>).map_err(e)?)
    .item(&MenuItem::with_id(app, "menu.edit.invert",     "Invert Selection", true,            None::<&str>).map_err(e)?)
    .item(&MenuItem::with_id(app, "menu.edit.clear",      "Clear Selection",  true,            None::<&str>).map_err(e)?)
    .build()
    .map_err(e)?;

  // ── View submenu ──
  let view_submenu = SubmenuBuilder::new(app, "View")
    .item(&CheckMenuItem::with_id(app, "menu.view.list",       "List View",        true, p.view_mode == "list",     None::<&str>).map_err(e)?)
    .item(&CheckMenuItem::with_id(app, "menu.view.grid",       "Grid View",        true, p.view_mode == "grid",     None::<&str>).map_err(e)?)
    .separator()
    .item(&CheckMenuItem::with_id(app, "menu.view.hidden",     "Show Hidden Files",  true, p.show_hidden,     None::<&str>).map_err(e)?)
    .item(&CheckMenuItem::with_id(app, "menu.view.extensions", "Show Extensions",    true, p.show_extensions, None::<&str>).map_err(e)?)
    .item(&CheckMenuItem::with_id(app, "menu.view.checkboxes", "Show Checkboxes",    true, p.show_checkboxes, None::<&str>).map_err(e)?)
    .separator()
    .item(&CheckMenuItem::with_id(app, "menu.view.sort_name",     "Sort by Name",  true, p.sort_by == "name",     None::<&str>).map_err(e)?)
    .item(&CheckMenuItem::with_id(app, "menu.view.sort_type",     "Sort by Type",  true, p.sort_by == "type",     None::<&str>).map_err(e)?)
    .item(&CheckMenuItem::with_id(app, "menu.view.sort_size",     "Sort by Size",  true, p.sort_by == "size",     None::<&str>).map_err(e)?)
    .item(&CheckMenuItem::with_id(app, "menu.view.sort_modified", "Sort by Date",  true, p.sort_by == "modified", None::<&str>).map_err(e)?)
    .separator()
    .item(&CheckMenuItem::with_id(app, "menu.view.sort_asc",  "Ascending",  true, p.sort_dir == "asc",  None::<&str>).map_err(e)?)
    .item(&CheckMenuItem::with_id(app, "menu.view.sort_desc", "Descending", true, p.sort_dir == "desc", None::<&str>).map_err(e)?)
    .build()
    .map_err(e)?;

  // ── History submenu ──
  let back_item = MenuItem::with_id(app, "menu.history.back",    "Previous",      p.can_go_back,    None::<&str>).map_err(e)?;
  let fwd_item  = MenuItem::with_id(app, "menu.history.forward", "Next",          p.can_go_forward, None::<&str>).map_err(e)?;
  let up_item   = MenuItem::with_id(app, "menu.history.up",      "Directory Up",  p.can_go_up,      None::<&str>).map_err(e)?;
  let hist_sep  = PredefinedMenuItem::separator(app).map_err(e)?;
  let recent_items: Vec<MenuItem<tauri::Wry>> = p.history_paths.iter().enumerate()
    .map(|(i, path)| {
      let name = std::path::Path::new(path)
        .file_name().and_then(|n| n.to_str()).unwrap_or(path.as_str()).to_string();
      MenuItem::with_id(app, format!("menu.history.recent.{i}"), name, true, None::<&str>)
    })
    .collect::<Result<Vec<_>, _>>()
    .map_err(e)?;
  let mut hist_dyn: Vec<&dyn IsMenuItem<tauri::Wry>> = vec![&back_item, &fwd_item, &up_item, &hist_sep];
  for item in &recent_items { hist_dyn.push(item); }
  let history_submenu = SubmenuBuilder::new(app, "History").items(&hist_dyn).build().map_err(e)?;

  // ── Go submenu ──
  let addr_item = MenuItem::with_id(app, "menu.go.address", "Go to Directory\u{2026}", true, None::<&str>).map_err(e)?;
  let go_sep1   = PredefinedMenuItem::separator(app).map_err(e)?;
  let go_sep2   = PredefinedMenuItem::separator(app).map_err(e)?;
  let vol_items: Vec<MenuItem<tauri::Wry>> = p.volumes.iter().enumerate()
    .map(|(i, v)| MenuItem::with_id(app, format!("menu.go.vol.{i}"), &v.label, true, None::<&str>))
    .collect::<Result<Vec<_>, _>>()
    .map_err(e)?;
  let home_items: Vec<MenuItem<tauri::Wry>> = p.home_dirs.iter().enumerate()
    .map(|(i, v)| MenuItem::with_id(app, format!("menu.go.home.{i}"), &v.label, true, None::<&str>))
    .collect::<Result<Vec<_>, _>>()
    .map_err(e)?;
  let mut go_dyn: Vec<&dyn IsMenuItem<tauri::Wry>> = vec![&addr_item, &go_sep1];
  for item in &vol_items  { go_dyn.push(item); }
  go_dyn.push(&go_sep2);
  for item in &home_items { go_dyn.push(item); }
  let go_submenu = SubmenuBuilder::new(app, "Go").items(&go_dyn).build().map_err(e)?;

  // ── Help submenu ──
  let help_about = PredefinedMenuItem::about(app, Some("About FM"), Some(make_about())).map_err(e)?;
  let help_submenu = SubmenuBuilder::new(app, "Help")
    .item(&help_about)
    .separator()
    .item(&MenuItem::with_id(app, "menu.help.website", "Website",      true, None::<&str>).map_err(e)?)
    .item(&MenuItem::with_id(app, "menu.help.source",  "Source Code",  true, None::<&str>).map_err(e)?)
    .item(&MenuItem::with_id(app, "menu.help.issue",   "Open an Issue",true, None::<&str>).map_err(e)?)
    .separator()
    .item(&MenuItem::with_id(app, "menu.help.email",   "Write Me",     true, None::<&str>).map_err(e)?)
    .build()
    .map_err(e)?;

  let menu = MenuBuilder::new(app)
    .item(&fm_submenu)
    .item(&edit_submenu)
    .item(&view_submenu)
    .item(&history_submenu)
    .item(&go_submenu)
    .item(&help_submenu)
    .build()
    .map_err(e)?;
  app.set_menu(menu).map(|_| ()).map_err(e)
}

#[tauri::command]
fn rebuild_app_menu_cmd(app: AppHandle, params: AppMenuParams) -> Result<(), String> {
  #[cfg(target_os = "macos")]
  return build_and_set_macos_menu(&app, params);
  #[cfg(not(target_os = "macos"))]
  { let _ = params; Ok(()) }
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

  #[cfg(windows)]
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

  #[cfg(windows)]
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
fn get_pinned_favorites_cmd(state: State<'_, StorageState>) -> Vec<String> {
  pinned_favorites::get_pinned_favorites(&state.paths.config_dir)
}

#[tauri::command]
fn add_pinned_favorite_cmd(state: State<'_, StorageState>, path: String) -> Result<(), String> {
  pinned_favorites::add_pinned_favorite(&state.paths.config_dir, path)
}

#[tauri::command]
fn remove_pinned_favorite_cmd(state: State<'_, StorageState>, path: String) -> Result<(), String> {
  pinned_favorites::remove_pinned_favorite(&state.paths.config_dir, path)
}

#[tauri::command]
fn set_pinned_favorites_cmd(state: State<'_, StorageState>, paths: Vec<String>) -> Result<(), String> {
  pinned_favorites::set_pinned_favorites(&state.paths.config_dir, paths)
}

#[tauri::command]
fn get_sidebar_cmd(pinned_paths: Option<Vec<String>>) -> Vec<sidebar::SidebarSection> {
  build_sidebar(pinned_paths.as_deref())
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
    is_app_bundle: false,
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
        is_app_bundle: false,
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

  #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
  return Err("empty trash is not supported on this platform".to_string());

  #[cfg(target_os = "macos")]
  {
    use std::fs;
    let home = std::env::var("HOME").map_err(|e: std::env::VarError| e.to_string())?;
    let trash_dir = std::path::Path::new(&home).join(".Trash");
    if !trash_dir.exists() {
      return Ok(0);
    }
    let mut count = 0u32;
    let mut errors: Vec<String> = Vec::new();
    for entry in fs::read_dir(&trash_dir).map_err(|e: std::io::Error| e.to_string())? {
      let entry: std::fs::DirEntry = entry.map_err(|e: std::io::Error| e.to_string())?;
      let path = entry.path();
      let result: Result<(), std::io::Error> = if path.is_dir() {
        fs::remove_dir_all(&path)
      } else {
        fs::remove_file(&path)
      };
      match result {
        Ok(_) => count += 1,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => errors.push(format!("{}: {}", path.display(), e)),
      }
    }
    if errors.is_empty() {
      Ok(count)
    } else {
      Err(errors.join("\n"))
    }
  }
}

#[tauri::command]
fn mount_disk_image_cmd(path: String) -> Result<Option<String>, String> {
  disk_image::mount(std::path::Path::new(&path))
}

#[tauri::command]
fn unmount_drive_cmd(path: String) -> Result<(), String> {
  #[cfg(not(any(target_os = "macos", target_os = "linux")))]
  return Err("unmount not supported on this platform".to_string());

  #[cfg(target_os = "macos")]
  {
    let output = std::process::Command::new("diskutil")
      .args(["unmount", &path])
      .output()
      .map_err(|e| e.to_string())?;
    if !output.status.success() {
      let msg = String::from_utf8_lossy(&output.stderr);
      return Err(msg.trim().to_string());
    }
    Ok(())
  }

  #[cfg(target_os = "linux")]
  {
    let output = std::process::Command::new("umount")
      .arg(&path)
      .output()
      .map_err(|e| e.to_string())?;
    if !output.status.success() {
      let msg = String::from_utf8_lossy(&output.stderr);
      return Err(msg.trim().to_string());
    }
    Ok(())
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

#[tauri::command]
fn show_new_item_menu_cmd(
  window: tauri::Window,
  state: State<'_, NewItemMenuState>,
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

#[tauri::command]
fn read_text_file_cmd(path: String) -> Result<String, String> {
  let path_buf = PathBuf::from(&path);
  if !path_buf.is_file() {
    return Err("path is not a file".to_string());
  }
  let size = std::fs::metadata(&path_buf).map(|m| m.len()).unwrap_or(0);
  if size > 2 * 1024 * 1024 {
    return Err(format!("file is too large to edit ({} MB)", size / 1024 / 1024));
  }
  std::fs::read_to_string(&path_buf).map_err(|e| e.to_string())
}

#[tauri::command]
fn write_text_file_cmd(path: String, content: String) -> Result<(), String> {
  let path_buf = PathBuf::from(&path);
  if !path_buf.is_file() {
    return Err("file does not exist".to_string());
  }
  let Some(parent) = path_buf.parent() else {
    return Err("cannot determine parent directory".to_string());
  };
  let tmp_path = parent.join(format!(".fm_write_{}.tmp", std::process::id()));
  std::fs::write(&tmp_path, content.as_bytes()).map_err(|e| e.to_string())?;
  std::fs::rename(&tmp_path, &path_buf).map_err(|e| {
    let _ = std::fs::remove_file(&tmp_path);
    e.to_string()
  })?;
  Ok(())
}

#[tauri::command]
fn open_editor_cmd(
  app: AppHandle,
  state: State<'_, EditorState>,
  path: String,
) -> Result<(), String> {
  let path_buf = PathBuf::from(&path);
  if !path_buf.is_file() {
    return Err("file does not exist".to_string());
  }
  let title = path_buf
    .file_name()
    .and_then(|n| n.to_str())
    .unwrap_or("Edit")
    .to_string();
  let count = state.counter.fetch_add(1, Ordering::SeqCst);
  let label = format!("editor-{}", count);
  {
    let mut map = state.pending.lock().unwrap();
    map.insert(label.clone(), path);
  }
  let builder = WebviewWindowBuilder::new(&app, &label, WebviewUrl::App("editor.html".into()))
    .title(&title)
    .inner_size(800.0, 600.0)
    .min_inner_size(500.0, 400.0);

  #[cfg(target_os = "macos")]
  let builder = builder.title_bar_style(tauri::TitleBarStyle::Overlay).title("");

  #[cfg(not(target_os = "macos"))]
  let builder = builder.decorations(false);

  builder.build().map_err(|e| e.to_string())?;
  Ok(())
}

#[tauri::command]
fn get_editor_path_cmd(state: State<'_, EditorState>, label: String) -> Result<String, String> {
  let mut map = state.pending.lock().unwrap();
  map.remove(&label).ok_or_else(|| "no pending path for this editor window".to_string())
}

/// Watch a specific file for external modifications and notify the editor window.
/// Uses the parent directory with a filename filter to work on all platforms.
#[tauri::command]
fn watch_editor_file_cmd(
  app: AppHandle,
  state: State<'_, EditorState>,
  label: String,
  path: String,
) -> Result<(), String> {
  let path_buf = std::path::PathBuf::from(&path);
  let parent = path_buf.parent().ok_or("cannot determine parent directory")?.to_path_buf();
  let file_name = path_buf.file_name().ok_or("cannot determine filename")?.to_os_string();

  let label_clone = label.clone();
  let path_clone = path.clone();

  let mut watcher = notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
    let Ok(event) = res else { return };
    if matches!(event.kind, EventKind::Access(_)) { return; }
    if !matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_)) { return; }
    if !event.paths.iter().any(|p| p.file_name() == Some(file_name.as_os_str())) { return; }
    if let Some(win) = app.get_webview_window(&label_clone) {
      let _ = win.emit("fm://editor-file-changed", &path_clone);
    }
  })
  .map_err(|e| e.to_string())?;

  watcher.watch(&parent, RecursiveMode::NonRecursive).map_err(|e| e.to_string())?;
  state.watchers.lock().unwrap().insert(label, watcher);
  Ok(())
}

/// Stop watching a file when its editor window closes.
#[tauri::command]
fn unwatch_editor_file_cmd(state: State<'_, EditorState>, label: String) -> Result<(), String> {
  state.watchers.lock().unwrap().remove(&label);
  Ok(())
}

// ── Archive dialog commands ───────────────────────────────────────────────────

#[tauri::command]
fn open_extract_dialog_cmd(
  app: AppHandle,
  state: State<'_, ArchiveState>,
  path: String,
) -> Result<(), String> {
  let path_buf = std::path::PathBuf::from(&path);
  if !path_buf.is_file() {
    return Err("file does not exist".to_string());
  }
  let filename = path_buf
    .file_name()
    .and_then(|n| n.to_str())
    .unwrap_or("archive")
    .to_string();
  let dest_dir = path_buf
    .parent()
    .map(|p| p.to_string_lossy().into_owned())
    .unwrap_or_default();

  let count = state.counter.fetch_add(1, Ordering::SeqCst);
  let label = format!("archive-{count}");

  state.pending.lock().unwrap().insert(
    label.clone(),
    ArchiveDialogParams {
      kind:           "extract".to_string(),
      paths:          vec![path],
      dest_dir,
      selection_kind: "file".to_string(),
    },
  );

  let builder = WebviewWindowBuilder::new(
    &app,
    &label,
    WebviewUrl::App("archive.html".into()),
  )
  .title(format!("Extract \u{2013} {filename}"))
  .inner_size(520.0, 360.0)
  .min_inner_size(460.0, 300.0)
  .resizable(true);

  #[cfg(target_os = "macos")]
  let builder = builder.title_bar_style(tauri::TitleBarStyle::Overlay).title("");
  #[cfg(not(target_os = "macos"))]
  let builder = builder.decorations(false);

  builder.build().map_err(|e| e.to_string())?;
  Ok(())
}

#[tauri::command]
fn open_archive_dialog_cmd(
  app: AppHandle,
  state: State<'_, ArchiveState>,
  paths: Vec<String>,
  dest_dir: String,
  selection_kind: String,
) -> Result<(), String> {
  if paths.is_empty() {
    return Err("no paths provided".to_string());
  }
  let count = state.counter.fetch_add(1, Ordering::SeqCst);
  let label = format!("archive-{count}");

  state.pending.lock().unwrap().insert(
    label.clone(),
    ArchiveDialogParams {
      kind:           "create".to_string(),
      paths,
      dest_dir,
      selection_kind,
    },
  );

  let builder = WebviewWindowBuilder::new(
    &app,
    &label,
    WebviewUrl::App("archive.html".into()),
  )
  .title("Create Archive")
  .inner_size(560.0, 460.0)
  .min_inner_size(480.0, 360.0)
  .resizable(true);

  #[cfg(target_os = "macos")]
  let builder = builder.title_bar_style(tauri::TitleBarStyle::Overlay).title("");
  #[cfg(not(target_os = "macos"))]
  let builder = builder.decorations(false);

  builder.build().map_err(|e| e.to_string())?;
  Ok(())
}

#[tauri::command]
fn get_archive_dialog_params_cmd(
  state: State<'_, ArchiveState>,
  label: String,
) -> Result<ArchiveDialogParams, String> {
  state
    .pending
    .lock()
    .unwrap()
    .remove(&label)
    .ok_or_else(|| "no pending params for this archive window".to_string())
}

#[derive(Clone, serde::Serialize)]
struct ArchiveProgressEvent {
  done:    u64,
  total:   u64,
  current: String,
}

#[derive(Clone, serde::Serialize)]
struct ArchiveDoneEvent {
  ok:          bool,
  error:       Option<String>,
  output_path: Option<String>,
  count:       u64,
}

/// Archive runner — receives all parameters from the frontend.
#[tauri::command]
async fn run_archive_cmd(
  app: AppHandle,
  state: State<'_, ArchiveState>,
  label: String,
  source_paths: Vec<String>,
  dest: String,
  overwrite: bool,
  format: String,
  subfolder: bool,
) -> Result<(), String> {
  let cancel = Arc::new(std::sync::atomic::AtomicBool::new(false));
  state.cancels.lock().unwrap().insert(label.clone(), cancel.clone());

  let win = app.get_webview_window(&label).ok_or("window not found")?;
  let win2 = win.clone();

  let is_extract = format == "extract";

  tokio::task::spawn_blocking(move || {
    let emit_progress = {
      let win = win.clone();
      move |done: u64, total: u64, current: &str| {
        let _ = win.emit("fm://archive-progress", ArchiveProgressEvent {
          done, total, current: current.to_string(),
        });
      }
    };

    let result: Result<(u64, Option<String>), String> = if is_extract {
      let src = std::path::PathBuf::from(&source_paths[0]);
      let base_dest = std::path::PathBuf::from(&dest);
      let final_dest = if subfolder {
        // Strip compound extensions for subfolder name
        let stem = archive_stem(&src);
        base_dest.join(stem)
      } else {
        base_dest
      };
      archive::extract(&src, &final_dest, overwrite, cancel, emit_progress)
        .map(|_| (0u64, Some(final_dest.to_string_lossy().into_owned())))
    } else {
      let src_paths: Vec<std::path::PathBuf> = source_paths
        .iter()
        .map(|p| std::path::PathBuf::from(p))
        .collect();
      let output = std::path::PathBuf::from(&dest);
      let count = src_paths.len() as u64;
      archive::create(&src_paths, &output, &format, cancel, emit_progress)
        .map(|_| (count, Some(output.to_string_lossy().into_owned())))
    };

    match result {
      Ok((count, output_path)) => {
        let _ = win2.emit("fm://archive-done", ArchiveDoneEvent {
          ok: true,
          error: None,
          output_path,
          count,
        });
      }
      Err(e) if e == "cancelled" => {
        let _ = win2.emit("fm://archive-done", ArchiveDoneEvent {
          ok: false,
          error: Some("cancelled".to_string()),
          output_path: None,
          count: 0,
        });
      }
      Err(e) => {
        let _ = win2.emit("fm://archive-done", ArchiveDoneEvent {
          ok: false,
          error: Some(e),
          output_path: None,
          count: 0,
        });
      }
    }
  });
  Ok(())
}

#[tauri::command]
fn cancel_archive_cmd(state: State<'_, ArchiveState>, label: String) -> Result<(), String> {
  if let Some(cancel) = state.cancels.lock().unwrap().get(&label) {
    cancel.store(true, Ordering::Relaxed);
  }
  Ok(())
}

fn archive_stem(path: &std::path::Path) -> String {
  let name = path
    .file_name()
    .and_then(|n| n.to_str())
    .unwrap_or("archive")
    .to_lowercase();
  for ext in &[".tar.gz", ".tar.bz2", ".tar.xz", ".tgz", ".tbz2", ".txz", ".tar", ".zip"] {
    if let Some(stem) = name.strip_suffix(*ext) {
      return stem.to_string();
    }
  }
  name
}

// ── Search commands ───────────────────────────────────────────────────────────
#[tauri::command]
async fn start_index_cmd(
  app: AppHandle,
  state: State<'_, IndexerState>,
  storage: State<'_, StorageState>,
) -> Result<(), String> {
  if state.is_running.load(Ordering::Relaxed) {
    return Ok(());
  }
  let cancel = state.start();
  let db_path = storage.paths.db_path.clone();
  let is_running = state.is_running.clone();
  tauri::async_runtime::spawn(async move {
    tokio::task::spawn_blocking(move || {
      indexer::run_index(db_path, cancel, app);
      is_running.store(false, Ordering::Relaxed);
    })
    .await
    .ok();
  });
  Ok(())
}

#[tauri::command]
fn cancel_index_cmd(state: State<'_, IndexerState>) {
  state.cancel.store(true, Ordering::Relaxed);
}

#[tauri::command]
fn search_files_cmd(
  storage: State<'_, StorageState>,
  query: String,
  scope: String,
  current_path: Option<String>,
  limit: Option<usize>,
) -> Result<Vec<SearchResult>, String> {
  let q = query.trim();
  // FTS5 trigram tokenizer requires >= 3 characters
  if q.len() < 3 {
    return Ok(vec![]);
  }
  let limit = limit.unwrap_or(200) as i64;

  // Wrap in double quotes for a literal phrase query so FTS5 operators
  // ('-', '*', '(', etc.) inside the user's text are treated as plain characters.
  let fts_query = format!("\"{}\"", q.replace('"', "\"\""));

  let conn = rusqlite::Connection::open(&storage.paths.db_path)
    .map_err(|e| format!("db open error: {e}"))?;

  // Build a path prefix for Rust-side filtering (avoids FTS5 + LIKE interaction quirks)
  let path_prefix: Option<String> = match scope.as_str() {
    "home"    => std::env::var("HOME").ok(),
    "current" => current_path,
    _         => None,
  };

  fn map_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SearchResult> {
    let name: String             = row.get(0)?;
    let path: String             = row.get(1)?;
    let is_dir: i64              = row.get(2)?;
    let size: Option<i64>        = row.get(3)?;
    let modified_ms: Option<i64> = row.get(4)?;
    let ext = std::path::Path::new(&name)
      .extension()
      .and_then(|e| e.to_str())
      .map(|e| e.to_lowercase());
    Ok(SearchResult {
      name, path, ext,
      is_dir: is_dir != 0,
      size: size.map(|s| s as u64),
      modified_ms: modified_ms.map(|m| m as u64),
    })
  }

  // Fetch a large batch from FTS5 (no path filter in SQL — FTS5 applies LIMIT before
  // post-filtering which causes scoped queries to return nothing), then filter in Rust.
  let fetch_limit = if path_prefix.is_some() { limit * 20 } else { limit };
  let mut stmt = conn
    .prepare(
      "SELECT name, path, is_dir, size, modified_ms
       FROM file_index
       WHERE name MATCH ?1
       ORDER BY rank LIMIT ?2",
    )
    .map_err(|e| format!("prepare error: {e}"))?;
  let rows = stmt
    .query_map(rusqlite::params![fts_query, fetch_limit], map_row)
    .map_err(|e| format!("query error: {e}"))?;

  let results: Vec<SearchResult> = rows
    .flatten()
    .filter(|r| {
      match &path_prefix {
        Some(prefix) => r.path.starts_with(prefix.as_str()),
        None => true,
      }
    })
    .take(limit as usize)
    .collect();

  Ok(results)
}

#[tauri::command]
fn get_index_stats_cmd(
  storage: State<'_, StorageState>,
  indexer: State<'_, IndexerState>,
) -> Result<IndexStats, String> {
  let conn = rusqlite::Connection::open(&storage.paths.db_path)
    .map_err(|e| format!("db open error: {e}"))?;

  let mut file_count: u64 = 0;
  let mut last_indexed: Option<u64> = None;

  let mut s = conn
    .prepare("SELECT key, value FROM index_meta")
    .map_err(|e| format!("prepare error: {e}"))?;
  let rows = s
    .query_map([], |row| {
      let key: String   = row.get(0)?;
      let value: String = row.get(1)?;
      Ok((key, value))
    })
    .map_err(|e| format!("query error: {e}"))?;

  for row in rows.flatten() {
    match row.0.as_str() {
      "file_count"   => file_count   = row.1.parse().unwrap_or(0),
      "last_indexed" => last_indexed = row.1.parse().ok(),
      _ => {}
    }
  }

  Ok(IndexStats {
    file_count,
    last_indexed,
    is_running: indexer.is_running.load(Ordering::Relaxed),
  })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  let storage_paths = bootstrap_storage().expect("failed to bootstrap storage");

  tauri::Builder::default()
    .manage(Arc::new(FileCoreState::new()))
    .manage(AddressMenuState::new())
    .manage(NewItemMenuState::new())
    .manage(DirWatchState::new())
    .manage(StorageState::new(storage_paths))
    .manage(ThumbnailState::new())
    .manage(DirSizeState::new())
    .manage(ContextMenuState::new())
    .manage(transfer::TransferState::new())
    .manage(EditorState::new())
    .manage(ArchiveState::new())
    .manage(QuickLookState::new())
    .manage(MenuPathStore::new())
    .manage(IndexerState::new())
    .invoke_handler(tauri::generate_handler![
      read_dir_cmd,
      walk_dir_cmd,
      cancel_cmd,
      get_file_icon_cmd,
      get_thumbnails_batch_cmd,
      cancel_thumbnails_cmd,
      get_pinned_favorites_cmd,
      add_pinned_favorite_cmd,
      remove_pinned_favorite_cmd,
      set_pinned_favorites_cmd,
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
      mount_disk_image_cmd,
      unmount_drive_cmd,
      start_dir_watch_cmd,
      stop_dir_watch_cmd,
      show_address_menu_cmd,
      show_new_item_menu_cmd,
      show_file_context_menu_cmd,
      compute_dir_size_cmd,
      cancel_dir_size_cmd,
      transfer::paste_cmd,
      transfer::cancel_transfer_cmd,
      transfer::pause_transfer_cmd,
      transfer::resume_transfer_cmd,
      read_text_file_cmd,
      write_text_file_cmd,
      open_editor_cmd,
      get_editor_path_cmd,
      watch_editor_file_cmd,
      unwatch_editor_file_cmd,
      open_extract_dialog_cmd,
      open_archive_dialog_cmd,
      get_archive_dialog_params_cmd,
      run_archive_cmd,
      cancel_archive_cmd,
      rebuild_app_menu_cmd,
      get_file_metadata_cmd,
      open_quicklook_cmd,
      get_quicklook_path_cmd,
      get_audio_metadata_cmd,
      start_index_cmd,
      cancel_index_cmd,
      search_files_cmd,
      get_index_stats_cmd,
      open_devtools_cmd,
    ])
    .setup(|app| {
      // Create main window programmatically so we can apply platform-specific titlebar settings.
      // (Config-file windows are created before setup runs and can't be reconfigured after the fact.)
      {
        let url = WebviewUrl::App("/".into());

        let builder = WebviewWindowBuilder::new(app.handle(), "main", url)
          .title("FM")
          .inner_size(800.0, 600.0)
          .min_inner_size(760.0, 480.0)
          .resizable(true);

        #[cfg(target_os = "macos")]
        let builder = builder.title_bar_style(tauri::TitleBarStyle::Overlay).title("");

        #[cfg(not(target_os = "macos"))]
        let builder = builder.decorations(false);

        builder.build()?;
      }

      let handle = app.handle();
      let copy_item = MenuItem::with_id(handle, "address.copy", "Copy Address", true, None::<&str>)?;
      let clear_item = MenuItem::with_id(handle, "address.clear", "Clear History", true, None::<&str>)?;
      let menu = Menu::with_items(handle, &[&copy_item, &clear_item])?;

      if let Some(state) = app.try_state::<AddressMenuState>() {
        let mut guard = state.menu.lock().unwrap();
        *guard = Some(menu);
      }

      let new_file_item   = MenuItem::with_id(handle, "new.file",   "New File",   true, None::<&str>)?;
      let new_folder_item = MenuItem::with_id(handle, "new.folder", "New Folder", true, None::<&str>)?;
      let new_menu = Menu::with_items(handle, &[&new_file_item, &new_folder_item])?;
      if let Some(state) = app.try_state::<NewItemMenuState>() {
        *state.menu.lock().unwrap() = Some(new_menu);
      }

      // macOS app menu — initial build with defaults; JS will rebuild with real state on mount
      #[cfg(target_os = "macos")]
      build_and_set_macos_menu(app.handle(), AppMenuParams {
        view_mode:       "list".to_string(),
        show_hidden:     false,
        show_extensions: true,
        show_checkboxes: false,
        sort_by:         "name".to_string(),
        sort_dir:        "asc".to_string(),
        can_go_back:     false,
        can_go_forward:  false,
        can_go_up:       false,
        history_paths:   Vec::new(),
        volumes:         Vec::new(),
        home_dirs:       Vec::new(),
        has_selection:   false,
        has_clipboard:   false,
      }).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

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

      // Watch for OS-level disk mount/unmount events by polling every 2 seconds.
      {
        let handle = app.handle().clone();
        tauri::async_runtime::spawn(async move {
          fn current_mounts() -> Vec<String> {
            let disks = sysinfo::Disks::new_with_refreshed_list();
            let mut mounts: Vec<String> = disks
              .list()
              .iter()
              .map(|d| d.mount_point().to_string_lossy().into_owned())
              .collect();
            mounts.sort();
            mounts
          }
          let mut prev = current_mounts();
          loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            let now = current_mounts();
            if now != prev {
              prev = now;
              if let Some(win) = handle.get_webview_window("main") {
                let _ = win.emit("fm://disks-changed", ());
              }
            }
          }
        });
      }

      Ok(())
    })
    .on_menu_event(|app, event| {
      let id = event.id().0.as_str();
      if id == "address.copy" {
        let _ = app.emit("fm://address-menu", "copy");
      } else if id == "address.clear" {
        let _ = app.emit("fm://address-menu", "clear");
      } else if id == "new.file" {
        let _ = app.emit("fm://new-item-menu", "file");
      } else if id == "new.folder" {
        let _ = app.emit("fm://new-item-menu", "folder");
      } else if id.starts_with("context.") {
        context_menu::handle_menu_event(app, id);
      } else if id == "menu.fm.website" {
        let _ = opener::open("https://github.com/akinozgen/fm");
      } else if id == "menu.edit.cut"        { let _ = app.emit("fm://menu-action", "cut"); }
        else if id == "menu.edit.copy"       { let _ = app.emit("fm://menu-action", "copy"); }
        else if id == "menu.edit.paste"      { let _ = app.emit("fm://menu-action", "paste"); }
        else if id == "menu.edit.select_all" { let _ = app.emit("fm://menu-action", "select_all"); }
        else if id == "menu.edit.invert"     { let _ = app.emit("fm://menu-action", "invert_selection"); }
        else if id == "menu.edit.clear"      { let _ = app.emit("fm://menu-action", "clear_selection"); }
        else if id == "menu.view.list"       { let _ = app.emit("fm://menu-action", "view_mode:list"); }
        else if id == "menu.view.grid"       { let _ = app.emit("fm://menu-action", "view_mode:grid"); }
        else if id == "menu.view.hidden"     { let _ = app.emit("fm://menu-action", "toggle_hidden"); }
        else if id == "menu.view.extensions" { let _ = app.emit("fm://menu-action", "toggle_extensions"); }
        else if id == "menu.view.checkboxes" { let _ = app.emit("fm://menu-action", "toggle_checkboxes"); }
        else if id == "menu.view.sort_name"     { let _ = app.emit("fm://menu-action", "sort_by:name"); }
        else if id == "menu.view.sort_type"     { let _ = app.emit("fm://menu-action", "sort_by:type"); }
        else if id == "menu.view.sort_size"     { let _ = app.emit("fm://menu-action", "sort_by:size"); }
        else if id == "menu.view.sort_modified" { let _ = app.emit("fm://menu-action", "sort_by:modified"); }
        else if id == "menu.view.sort_asc"   { let _ = app.emit("fm://menu-action", "sort_dir:asc"); }
        else if id == "menu.view.sort_desc"  { let _ = app.emit("fm://menu-action", "sort_dir:desc"); }
        else if id == "menu.history.back"    { let _ = app.emit("fm://menu-action", "history_back"); }
        else if id == "menu.history.forward" { let _ = app.emit("fm://menu-action", "history_forward"); }
        else if id == "menu.history.up"      { let _ = app.emit("fm://menu-action", "history_up"); }
        else if id == "menu.go.address"      { let _ = app.emit("fm://menu-action", "focus_address"); }
        else if id == "menu.help.website"    { let _ = opener::open("https://akinozgen.com/projects/fm"); }
        else if id == "menu.help.source"     { let _ = opener::open("https://github.com/akinozgen/fm"); }
        else if id == "menu.help.issue"      { let _ = opener::open("https://github.com/akinozgen/fm/issues/new"); }
        else if id == "menu.help.email"      { let _ = opener::open("mailto:akinozgen@protonmail.com"); }
        else if let Some(idx_str) = id.strip_prefix("menu.history.recent.") {
          if let Ok(idx) = idx_str.parse::<usize>() {
            if let Some(store) = app.try_state::<MenuPathStore>() {
              let paths = store.history_paths.lock().unwrap();
              if let Some(path) = paths.get(idx) {
                let _ = app.emit("fm://menu-action", format!("navigate:{path}"));
              }
            }
          }
        } else if let Some(idx_str) = id.strip_prefix("menu.go.vol.") {
          if let Ok(idx) = idx_str.parse::<usize>() {
            if let Some(store) = app.try_state::<MenuPathStore>() {
              let paths = store.vol_paths.lock().unwrap();
              if let Some(path) = paths.get(idx) {
                let _ = app.emit("fm://menu-action", format!("navigate:{path}"));
              }
            }
          }
        } else if let Some(idx_str) = id.strip_prefix("menu.go.home.") {
          if let Ok(idx) = idx_str.parse::<usize>() {
            if let Some(store) = app.try_state::<MenuPathStore>() {
              let paths = store.home_paths.lock().unwrap();
              if let Some(path) = paths.get(idx) {
                let _ = app.emit("fm://menu-action", format!("navigate:{path}"));
              }
            }
          }
        }
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
