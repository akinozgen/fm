import { invoke } from '@tauri-apps/api/core';

const iconCache = new Map();
const thumbnailKeys = new Set();
const ICON_MISS = Symbol('icon-miss');
const CANCELLED = Symbol('cancelled');

const IMAGE_EXTS = new Set([
  'jpg', 'jpeg', 'png', 'gif', 'webp', 'bmp',
  'avif', 'tiff', 'tif', 'ico'
]);

function getExt(path) {
  const dot = path.lastIndexOf('.');
  return dot !== -1 ? path.slice(dot + 1).toLowerCase() : '';
}

export function isThumbnailKey(path, size) {
  return thumbnailKeys.has(`${path}:${size}`);
}

// ── Thumbnail batch queue ─────────────────────────────────────────────────────
// IntersectionObserver callbacks for all items visible in a single frame fire
// within the same browser task. We collect every `enqueueThumbnail` call into
// `pendingBatch` and flush once — as a single Tauri IPC call — via a Promise
// microtask that fires after all observer callbacks in the frame have returned.
//
// On the Rust side, `get_thumbnails_batch_cmd` hands the whole batch to rayon
// which spreads the decode work across all available CPU cores in parallel.

let pendingBatch = []; // { path, size, resolve, reject }
let batchScheduled = false;

function enqueueThumbnail(path, size) {
  return new Promise((resolve, reject) => {
    pendingBatch.push({ path, size, resolve, reject });
    if (!batchScheduled) {
      batchScheduled = true;
      // Microtask fires after all synchronous observer callbacks in this task
      Promise.resolve().then(flushBatch);
    }
  });
}

async function flushBatch() {
  batchScheduled = false;
  if (pendingBatch.length === 0) return;

  const batch = pendingBatch.splice(0);
  const paths = batch.map(item => item.path);
  const size = batch[0].size; // all items in a grid view share the same size

  try {
    // Vec<Option<String>>: null entry = Rust could not decode that file
    const results = await invoke('get_thumbnails_batch_cmd', { paths, size });
    batch.forEach((item, i) => item.resolve(results[i] ?? null));
  } catch (err) {
    const reason = err === CANCELLED ? CANCELLED : err;
    batch.forEach(item => item.reject(reason));
  }
}

// Called by App.vue on navigation — drops items not yet dispatched to Rust.
// In-flight batch invokes complete normally; stale results are ignored because
// the components that requested them will have unmounted.
export function clearThumbnailQueue() {
  const dropped = pendingBatch.splice(0);
  for (const { reject } of dropped) reject(CANCELLED);
}
// ─────────────────────────────────────────────────────────────────────────────

export async function getFileIcon(path, size = 24) {
  const key = `${path}:${size}`;
  if (iconCache.has(key)) {
    const cached = iconCache.get(key);
    return cached === ICON_MISS ? null : cached;
  }

  if (!path || path.startsWith('fm://')) {
    iconCache.set(key, ICON_MISS);
    return null;
  }

  const ext = getExt(path);

  // Thumbnails: raster image files in grid view (size >= 48).
  // Rust decodes, cover-crops, JPEG-encodes, and caches to disk.
  if (size >= 48 && IMAGE_EXTS.has(ext)) {
    try {
      const b64 = await enqueueThumbnail(path, size);
      if (b64) {
        const dataUrl = `data:image/jpeg;base64,${b64}`;
        iconCache.set(key, dataUrl);
        thumbnailKeys.add(key);
        return dataUrl;
      }
      // null: Rust couldn't decode this file — fall through to OS icon
    } catch (err) {
      if (err === CANCELLED) return null; // navigated away; don't cache
      // Rust error — fall through to OS icon
    }
  }

  // OS shell icon via Rust (all non-image files, list-view images, decode failures)
  try {
    const b64 = await invoke('get_file_icon_cmd', { path, size });
    const dataUrl = `data:image/png;base64,${b64}`;
    iconCache.set(key, dataUrl);
    return dataUrl;
  } catch {
    iconCache.set(key, ICON_MISS);
    return null;
  }
}
