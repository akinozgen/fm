use std::sync::Mutex;

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::{AppHandle, Emitter, LogicalPosition, Manager, Position};

// ── Emitted event payload ─────────────────────────────────────────────────────
#[derive(Debug, Clone, serde::Serialize)]
pub struct FileContextClick {
  pub kind: String,
  pub paths: Vec<String>,
  pub action: String,
}

// ── State: stores the active selection so on_menu_event can emit it ───────────
pub struct ContextMenuState {
  paths: Mutex<Vec<String>>,
  kind: Mutex<String>,
}

impl ContextMenuState {
  pub fn new() -> Self {
    Self {
      paths: Mutex::new(Vec::new()),
      kind: Mutex::new(String::new()),
    }
  }
}

// ── Helpers ───────────────────────────────────────────────────────────────────
fn item(app: &AppHandle, action: &str, label: &str) -> Result<MenuItem<tauri::Wry>, String> {
  MenuItem::with_id(app, format!("context.{action}"), label, true, None::<&str>)
    .map_err(|e| e.to_string())
}

fn sep(app: &AppHandle) -> Result<PredefinedMenuItem<tauri::Wry>, String> {
  PredefinedMenuItem::separator(app).map_err(|e| e.to_string())
}

// ── Menu builders ─────────────────────────────────────────────────────────────
fn build_empty_menu(app: &AppHandle) -> Result<Menu<tauri::Wry>, String> {
  Menu::with_items(app, &[
    &item(app, "new_folder",    "New Folder")?,
    &item(app, "new_file",      "New File")?,
    &sep(app)?,
    &item(app, "paste",         "Paste")?,
    &sep(app)?,
    &item(app, "refresh",       "Refresh")?,
    &sep(app)?,
    &item(app, "open_terminal", "Open in Terminal")?,
  ]).map_err(|e| e.to_string())
}

fn build_file_menu(app: &AppHandle) -> Result<Menu<tauri::Wry>, String> {
  Menu::with_items(app, &[
    &item(app, "open",       "Open")?,
    &sep(app)?,
    &item(app, "rename",     "Rename")?,
    &item(app, "copy",       "Copy")?,
    &item(app, "cut",        "Cut")?,
    &sep(app)?,
    &item(app, "delete",     "Delete")?,
    &sep(app)?,
    &item(app, "properties", "Properties")?,
  ]).map_err(|e| e.to_string())
}

// Multiple files selected — no Open / Rename
fn build_files_menu(app: &AppHandle) -> Result<Menu<tauri::Wry>, String> {
  Menu::with_items(app, &[
    &item(app, "copy",       "Copy")?,
    &item(app, "cut",        "Cut")?,
    &sep(app)?,
    &item(app, "delete",     "Delete")?,
    &sep(app)?,
    &item(app, "properties", "Properties")?,
  ]).map_err(|e| e.to_string())
}

fn build_dir_menu(app: &AppHandle) -> Result<Menu<tauri::Wry>, String> {
  Menu::with_items(app, &[
    &item(app, "open",          "Open")?,
    &sep(app)?,
    &item(app, "rename",        "Rename")?,
    &item(app, "copy",          "Copy")?,
    &item(app, "cut",           "Cut")?,
    &sep(app)?,
    &item(app, "delete",        "Delete")?,
    &sep(app)?,
    &item(app, "open_terminal", "Open in Terminal")?,
    &item(app, "properties",    "Properties")?,
  ]).map_err(|e| e.to_string())
}

// Multiple folders selected — no Open / Rename
fn build_dirs_menu(app: &AppHandle) -> Result<Menu<tauri::Wry>, String> {
  Menu::with_items(app, &[
    &item(app, "copy",       "Copy")?,
    &item(app, "cut",        "Cut")?,
    &sep(app)?,
    &item(app, "delete",     "Delete")?,
    &sep(app)?,
    &item(app, "properties", "Properties")?,
  ]).map_err(|e| e.to_string())
}

// Mixed files + folders — only common actions
fn build_mixed_menu(app: &AppHandle) -> Result<Menu<tauri::Wry>, String> {
  Menu::with_items(app, &[
    &item(app, "delete",     "Delete")?,
    &sep(app)?,
    &item(app, "properties", "Properties")?,
  ]).map_err(|e| e.to_string())
}

// Sidebar folder items — navigate + info only
fn build_sidebar_item_menu(app: &AppHandle) -> Result<Menu<tauri::Wry>, String> {
  Menu::with_items(app, &[
    &item(app, "open",       "Open")?,
    &sep(app)?,
    &item(app, "properties", "Properties")?,
  ]).map_err(|e| e.to_string())
}

// ── Command ───────────────────────────────────────────────────────────────────
#[tauri::command]
pub fn show_file_context_menu_cmd(
  window: tauri::Window,
  app: AppHandle,
  state: tauri::State<'_, ContextMenuState>,
  x: f64,
  y: f64,
  kind: String,
  paths: Vec<String>,
) -> Result<(), String> {
  *state.paths.lock().unwrap() = paths;
  *state.kind.lock().unwrap() = kind.clone();

  let menu = match kind.as_str() {
    "empty" => build_empty_menu(&app)?,
    "file"  => build_file_menu(&app)?,
    "files" => build_files_menu(&app)?,
    "dir"   => build_dir_menu(&app)?,
    "dirs"  => build_dirs_menu(&app)?,
    "mixed"        => build_mixed_menu(&app)?,
    "sidebar_item" => build_sidebar_item_menu(&app)?,
    _              => return Err(format!("unsupported context kind: {kind}")),
  };

  window
    .popup_menu_at(&menu, Position::Logical(LogicalPosition::new(x, y)))
    .map_err(|e| e.to_string())
}

// ── Called from on_menu_event ─────────────────────────────────────────────────
pub fn handle_menu_event(app: &AppHandle, id: &str) {
  let action = match id.strip_prefix("context.") {
    Some(a) => a.to_string(),
    None => return,
  };
  let Some(state) = app.try_state::<ContextMenuState>() else { return };
  let paths = state.paths.lock().unwrap().clone();
  let kind  = state.kind.lock().unwrap().clone();
  let _ = app.emit("fm://file-context-menu", FileContextClick { kind, paths, action });
}
