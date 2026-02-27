use std::sync::Mutex;

use tauri::menu::{Menu, MenuItem};
use tauri::{AppHandle, Emitter, LogicalPosition, Manager, Position, State};

#[derive(Debug, Clone, serde::Serialize)]
pub struct FileContextClick {
  pub kind: String,
  pub path: Option<String>,
}

pub struct FileContextMenuState {
  pub last_click: Mutex<Option<FileContextClick>>,
}

impl FileContextMenuState {
  pub fn new() -> Self {
    Self {
      last_click: Mutex::new(None),
    }
  }
}

#[tauri::command]
pub fn show_file_context_menu_cmd(
  window: tauri::Window,
  app: AppHandle,
  state: State<'_, FileContextMenuState>,
  x: f64,
  y: f64,
  kind: String,
  path: Option<String>,
) -> Result<(), String> {
  let kind = match kind.as_str() {
    "empty" | "file" | "dir" => kind,
    _ => return Err("unsupported context kind".to_string()),
  };

  {
    let mut guard = state.last_click.lock().unwrap();
    *guard = Some(FileContextClick {
      kind: kind.clone(),
      path,
    });
  }

  let label = match kind.as_str() {
    "file" => "Context: File",
    "dir" => "Context: Folder",
    _ => "Context: Empty Space",
  };
  let info_item =
    MenuItem::with_id(&app, "context.info", label, true, None::<&str>).map_err(|e| e.to_string())?;
  let menu = Menu::with_items(&app, &[&info_item]).map_err(|e| e.to_string())?;

  window
    .popup_menu_at(&menu, Position::Logical(LogicalPosition::new(x, y)))
    .map_err(|e| e.to_string())
}

pub fn emit_last_click(app: &AppHandle) {
  if let Some(state) = app.try_state::<FileContextMenuState>() {
    let payload = state.last_click.lock().unwrap().clone();
    if let Some(click) = payload {
      let _ = app.emit("fm://file-context-menu", click);
    }
  }
}
