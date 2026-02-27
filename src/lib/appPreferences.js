import { getPreferencesStore } from './preferencesStore';

const KEY_GLOBAL_PREFS = 'globalPrefs';
const KEY_MANUAL_HISTORY = 'manualPathHistory';
const KEY_DIR_PREFS = 'directoryPrefs';

const DEFAULT_GLOBAL_PREFS = {
  showHidden: false,
  showExtensions: true,
  showSelectionCheckboxes: false
};

function fnv1aHash(input) {
  let hash = 0x811c9dc5;
  for (let i = 0; i < input.length; i += 1) {
    hash ^= input.charCodeAt(i);
    hash = Math.imul(hash, 0x01000193);
  }
  return (hash >>> 0).toString(16).padStart(8, '0');
}

function normalizePath(path) {
  return (path || '').replace(/\\/g, '/');
}

export async function loadGlobalPrefs() {
  const store = getPreferencesStore();
  if (!store) return { ...DEFAULT_GLOBAL_PREFS };
  const stored = (await store.get(KEY_GLOBAL_PREFS)) || {};
  return { ...DEFAULT_GLOBAL_PREFS, ...stored };
}

export async function saveGlobalPrefs(prefs) {
  const store = getPreferencesStore();
  if (!store) return;
  await store.set(KEY_GLOBAL_PREFS, prefs);
  await store.save();
}

export async function loadManualPathHistory() {
  const store = getPreferencesStore();
  if (!store) return [];
  const stored = await store.get(KEY_MANUAL_HISTORY);
  return Array.isArray(stored) ? stored : [];
}

export async function pushManualPath(path) {
  const store = getPreferencesStore();
  if (!store) return;
  const normalized = normalizePath(path);
  if (!normalized) return;

  const existing = await loadManualPathHistory();
  const next = [normalized, ...existing.filter((p) => normalizePath(p) !== normalized)].slice(0, 40);
  await store.set(KEY_MANUAL_HISTORY, next);
  await store.save();
}

export async function removeManualPath(path) {
  const store = getPreferencesStore();
  if (!store) return;
  const normalized = normalizePath(path);
  const existing = await loadManualPathHistory();
  const next = existing.filter((p) => normalizePath(p) !== normalized);
  await store.set(KEY_MANUAL_HISTORY, next);
  await store.save();
}

export async function getDirectoryPrefs(path) {
  const store = getPreferencesStore();
  if (!store) return null;
  const normalized = normalizePath(path);
  if (!normalized) return null;
  const hash = fnv1aHash(normalized);
  const all = (await store.get(KEY_DIR_PREFS)) || {};
  const item = all[hash];
  if (!item || item.path !== normalized) return null;
  return item.prefs || null;
}

export async function setDirectoryPrefs(path, patch) {
  const store = getPreferencesStore();
  if (!store) return;
  const normalized = normalizePath(path);
  if (!normalized) return;
  const hash = fnv1aHash(normalized);
  const all = (await store.get(KEY_DIR_PREFS)) || {};
  const current = all[hash]?.prefs || {};
  all[hash] = {
    path: normalized,
    prefs: {
      ...current,
      ...patch
    }
  };
  await store.set(KEY_DIR_PREFS, all);
  await store.save();
}
