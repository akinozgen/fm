import { invoke } from '@tauri-apps/api/core';

const iconCache = new Map();

export async function getFileIcon(path, size = 24) {
  const key = `${path}:${size}`;
  if (iconCache.has(key)) return iconCache.get(key);

  try {
    const b64 = await invoke('get_file_icon_cmd', { path, size });
    const dataUrl = `data:image/png;base64,${b64}`;
    iconCache.set(key, dataUrl);
    return dataUrl;
  } catch (err) {
    return null;
  }
}
