import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export async function showNativeFileContextMenu({ x, y, kind, paths, isPinned, isAppBundle = false }) {
  return invoke('show_file_context_menu_cmd', { x, y, kind, paths, isPinned, isAppBundle });
}

export async function listenFileContextMenu(handler) {
  return listen('fm://file-context-menu', (event) => {
    handler?.(event.payload || {});
  });
}
