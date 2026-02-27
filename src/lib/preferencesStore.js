import { invoke } from '@tauri-apps/api/core';
import { load } from '@tauri-apps/plugin-store';

let preferencesStore = null;
let storagePaths = null;

export async function bootstrapPreferencesStore() {
  if (preferencesStore) {
    return { store: preferencesStore, paths: storagePaths };
  }

  storagePaths = await invoke('get_storage_paths_cmd');
  preferencesStore = await load(storagePaths.prefs_path, { autoSave: false });
  return { store: preferencesStore, paths: storagePaths };
}

export function getPreferencesStore() {
  return preferencesStore;
}
