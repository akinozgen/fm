<template>
  <div class="sv-shell">
    <!-- Header -->
    <div class="sv-header">
      <div class="sv-input-row">
        <Search :size="14" class="sv-search-icon" />
        <input
          ref="inputRef"
          v-model="query"
          class="sv-input"
          type="text"
          placeholder="Search files…"
          autocomplete="off"
          spellcheck="false"
        />
        <button v-if="query" class="sv-clear-btn" title="Clear" @click="query = ''">
          <X :size="12" />
        </button>
      </div>

      <!-- Scope chips — no form controls -->
      <div class="sv-scope-bar">
        <button
          class="sv-scope-chip"
          :class="{ active: scope === 'everywhere' }"
          @click="scope = 'everywhere'"
        >Everywhere</button>

        <button
          class="sv-scope-chip"
          :class="{ active: scope === 'home' }"
          @click="scope = 'home'"
        >Home</button>

        <!-- Current folder — only shows when browsing a real directory -->
        <button
          v-if="hasCurrentPath"
          class="sv-scope-chip sv-scope-chip--path"
          :class="{ active: scope === 'current' }"
          :title="currentPath"
          @click="scope = 'current'"
        >
          <Folder :size="11" />
          <span class="sv-chip-label">{{ folderName(currentPath) }}</span>
        </button>

        <!-- Custom location — shows name when one is selected -->
        <button
          class="sv-scope-chip sv-scope-chip--pick"
          :class="{ active: scope === 'custom', 'has-path': !!customPath }"
          :title="customPath || 'Choose a folder…'"
          @click="pickCustomLocation"
        >
          <FolderOpen :size="11" />
          <span class="sv-chip-label">{{ customPath ? folderName(customPath) : 'Choose…' }}</span>
        </button>
      </div>

      <div v-if="indexStats" class="sv-stats">
        <span v-if="indexStats.is_running" class="sv-stats-text">Indexing in progress…</span>
        <span v-else-if="indexStats.last_indexed" class="sv-stats-text">
          Indexed {{ formatAge(indexStats.last_indexed) }} ago · {{ indexStats.file_count.toLocaleString() }} files
        </span>
        <span v-else class="sv-stats-text">Index not yet built</span>
        <button
          class="sv-reindex-btn"
          :disabled="indexStats?.is_running"
          :title="indexStats?.is_running ? 'Indexing in progress…' : 'Re-index filesystem'"
          @click="reindex"
        >
          <RefreshCw :size="11" :class="{ 'sv-spin': indexStats?.is_running }" />
          {{ indexStats?.is_running ? 'Indexing…' : 'Re-index' }}
        </button>
      </div>
    </div>

    <!-- Results -->
    <div class="sv-results" @click="selectedPaths.clear(); selectedPaths = new Set()" @contextmenu.prevent="onBgContextMenu">
      <div v-if="!query.trim()" class="sv-empty">
        <Search :size="32" class="sv-empty-icon" />
        <span>Start typing to search</span>
      </div>

      <div v-else-if="searching" class="sv-empty">
        <span>Searching…</span>
      </div>

      <div v-else-if="searchError" class="sv-empty sv-error">
        <span>Error: {{ searchError }}</span>
      </div>

      <div v-else-if="results.length === 0" class="sv-empty">
        <span>No results for "{{ query }}"</span>
      </div>

      <template v-else>
        <section class="list sv-results-list">
          <button
            v-for="item in results"
            :key="item.path"
            class="list-row"
            :class="{ folder: item.is_dir, file: !item.is_dir, selected: selectedPaths.has(item.path) }"
            :data-path="item.path"
            @click.stop="handleItemClick(item, $event)"
            @dblclick.stop="openItem(item)"
            @contextmenu.prevent.stop="onItemContextMenu(item, $event)"
          >
            <FileIcon :path="item.path" :is-dir="item.is_dir" :size="16" />
            <span class="name">{{ item.name }}</span>
            <span class="meta">{{ item.is_dir ? 'Folder' : (item.ext ? item.ext.toUpperCase() : 'File') }}</span>
            <span class="meta">{{ item.is_dir ? '—' : fmtSize(item.size) }}</span>
            <span class="meta sv-location-meta">{{ parentDir(item.path) }}</span>
          </button>
        </section>
        <div v-if="results.length >= 200" class="sv-limit-note">
          Showing first 200 results — refine your query
        </div>
      </template>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, watch, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { Search, X, RefreshCw, Folder, FolderOpen } from 'lucide-vue-next';
import FileIcon from './FileIcon.vue';
import { showNativeFileContextMenu } from '../lib/contextMenu';

const props = defineProps({
  currentPath: { type: String, default: '' },
});
const emit = defineEmits(['navigate', 'open-file', 'update:result-count']);

const inputRef   = ref(null);
const query      = ref('');
const scope      = ref('everywhere');  // 'everywhere' | 'home' | 'current' | 'custom'
const customPath = ref('');
const results      = ref([]);
const searching    = ref(false);
const searchError  = ref('');
const indexStats   = ref(null);
let   selectedPaths = ref(new Set());

// Only show "current folder" chip when actually browsing a real directory
const hasCurrentPath = computed(
  () => !!props.currentPath && !props.currentPath.startsWith('fm://')
);

// Keep scope in sync with path availability
watch(hasCurrentPath, (has) => {
  if (!has && scope.value === 'current') scope.value = 'everywhere';
});

let debounceTimer = null;

watch(query, () => {
  clearTimeout(debounceTimer);
  selectedPaths.value = new Set();
  if (!query.value.trim()) { results.value = []; emit('update:result-count', 0); return; }
  debounceTimer = setTimeout(doSearch, 200);
});

watch([scope, customPath], () => {
  if (query.value.trim()) doSearch();
});

async function doSearch() {
  // Custom scope needs a path
  if (scope.value === 'custom' && !customPath.value) return;

  searching.value = true;
  searchError.value = '';
  try {
    const backendScope = scope.value === 'custom' ? 'current' : scope.value;
    const scopePath =
      scope.value === 'current' ? (props.currentPath || null) :
      scope.value === 'custom'  ? customPath.value :
      null;

    const q = query.value.trim();
    const res = await invoke('search_files_cmd', {
      query:       q,
      scope:       backendScope,
      currentPath: scopePath,
      limit:       200,
    });
    results.value = res ?? [];
    emit('update:result-count', results.value.length);
  } catch (err) {
    console.error('[search] error:', err);
    searchError.value = String(err);
    results.value = [];
  } finally {
    searching.value = false;
  }
}

async function pickCustomLocation() {
  const selected = await openDialog({ directory: true, multiple: false }).catch(() => null);
  if (typeof selected === 'string' && selected) {
    customPath.value = selected;
    scope.value = 'custom';
  }
}

async function reindex() {
  await invoke('start_index_cmd').catch(() => {});
  indexStats.value = await invoke('get_index_stats_cmd').catch(() => null);
}

function handleItemClick(item, event) {
  if (event.metaKey || event.ctrlKey) {
    const next = new Set(selectedPaths.value);
    if (next.has(item.path)) next.delete(item.path);
    else next.add(item.path);
    selectedPaths.value = next;
  } else {
    selectedPaths.value = new Set([item.path]);
  }
}

function openItem(item) {
  if (item.is_dir) {
    emit('navigate', item.path);
  } else {
    emit('open-file', item.path);
  }
}

function onItemContextMenu(item, event) {
  if (!selectedPaths.value.has(item.path)) {
    selectedPaths.value = new Set([item.path]);
  }
  const paths = Array.from(selectedPaths.value);
  const allDirs  = paths.every(p => results.value.find(r => r.path === p)?.is_dir);
  const allFiles = paths.every(p => !results.value.find(r => r.path === p)?.is_dir);
  let kind;
  if (paths.length === 1) kind = item.is_dir ? 'dir' : 'file';
  else if (allDirs)  kind = 'dirs';
  else if (allFiles) kind = 'files';
  else               kind = 'mixed';
  void showNativeFileContextMenu({ x: event.clientX, y: event.clientY, kind, paths, isAppBundle: false, isSearch: true });
}

function onBgContextMenu(event) {
  selectedPaths.value = new Set();
  void showNativeFileContextMenu({ x: event.clientX, y: event.clientY, kind: 'empty', paths: [] });
}

function parentDir(path) {
  const sep = path.includes('/') ? '/' : '\\';
  const parts = path.split(sep);
  parts.pop();
  return parts.join(sep) || sep;
}

function folderName(path) {
  if (!path) return '';
  const sep = path.includes('/') ? '/' : '\\';
  return path.split(sep).filter(Boolean).pop() || path;
}

function fmtSize(bytes) {
  if (bytes < 1024)              return `${bytes} B`;
  if (bytes < 1024 * 1024)       return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

function formatModified(ms) {
  if (!ms) return '—';
  return new Date(ms).toLocaleDateString();
}

function formatAge(lastIndexedSecs) {
  const diff = Math.floor(Date.now() / 1000) - lastIndexedSecs;
  if (diff < 60)    return `${diff}s`;
  if (diff < 3600)  return `${Math.floor(diff / 60)}m`;
  if (diff < 86400) return `${Math.floor(diff / 3600)}h`;
  return `${Math.floor(diff / 86400)}d`;
}

onMounted(async () => {
  if (hasCurrentPath.value) scope.value = 'current';
  inputRef.value?.focus();
  indexStats.value = await invoke('get_index_stats_cmd').catch(() => null);
});
</script>

<style scoped>
.sv-shell {
  display: flex;
  flex-direction: column;
  min-height: 0;
}

/* ── Header ── */
.sv-header {
  flex-shrink: 0;
  padding: 12px 16px 10px;
  border-bottom: 1px solid var(--line);
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.sv-input-row {
  display: flex;
  align-items: center;
  gap: 6px;
  background: var(--panel, #fff);
  border: 1px solid var(--line);
  border-radius: 7px;
  padding: 0 8px;
  height: 30px;
}

.sv-search-icon {
  flex-shrink: 0;
  color: var(--muted);
}

.sv-input {
  flex: 1;
  border: none;
  outline: none;
  background: transparent;
  font-size: 13px;
  color: var(--ink);
  min-width: 0;
}

.sv-input::placeholder {
  color: var(--muted);
}

.sv-clear-btn {
  flex-shrink: 0;
  background: none;
  border: none;
  cursor: pointer;
  padding: 2px;
  color: var(--muted);
  display: flex;
  align-items: center;
  border-radius: 3px;
}

.sv-clear-btn:hover {
  color: var(--ink);
}

.sv-spin {
  animation: sv-spin 1s linear infinite;
}

@keyframes sv-spin {
  to { transform: rotate(360deg); }
}

/* ── Scope chips ── */
.sv-scope-bar {
  display: flex;
  flex-wrap: wrap;
  gap: 5px;
}

.sv-scope-chip {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  height: 22px;
  padding: 0 10px;
  border-radius: 11px;
  border: 1px solid var(--line);
  background: transparent;
  font-size: 11.5px;
  font-family: inherit;
  color: var(--muted);
  cursor: pointer;
  white-space: nowrap;
  max-width: 180px;
  transition: background 0.1s, color 0.1s, border-color 0.1s;
  user-select: none;
  -webkit-user-select: none;
}

.sv-scope-chip:hover {
  background: var(--panel-muted, #f2f2f4);
  color: var(--ink);
}

.sv-scope-chip.active {
  background: var(--accent);
  border-color: var(--accent);
  color: #fff;
}

/* Path chips: folder icon + truncated name */
.sv-scope-chip--path,
.sv-scope-chip--pick {
  padding: 0 8px 0 7px;
}

.sv-chip-label {
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 120px;
}

/* "Choose…" chip has a dashed border until a path is selected */
.sv-scope-chip--pick:not(.has-path) {
  border-style: dashed;
}

/* ── Stats ── */
.sv-stats {
  display: flex;
  align-items: center;
  gap: 8px;
}

.sv-stats-text {
  flex: 1;
  font-size: 11px;
  color: var(--muted);
  display: flex;
  align-items: center;
  gap: 3px;
}

.sv-reindex-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  height: 22px;
  padding: 0 8px;
  border-radius: 5px;
  border: 1px solid var(--line);
  background: transparent;
  font-size: 11px;
  font-family: inherit;
  color: var(--muted);
  cursor: pointer;
  white-space: nowrap;
  transition: background 0.1s, color 0.1s;
  flex-shrink: 0;
}

.sv-reindex-btn:hover:not(:disabled) {
  background: var(--panel-muted, #f2f2f4);
  color: var(--ink);
}

.sv-reindex-btn:disabled {
  opacity: 0.5;
  cursor: default;
}

/* ── Results ── */
.sv-results {
  flex: 1 1 0;
  min-height: 0;
  overflow-y: auto;
}

.sv-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  gap: 10px;
  color: var(--muted);
  font-size: 13px;
  opacity: 0.6;
}

.sv-empty-icon {
  opacity: 0.4;
}

.sv-error {
  color: var(--danger, #c0392b);
  opacity: 1;
  font-size: 12px;
  padding: 0 16px;
  text-align: center;
}

/* Widen the last column to show parent path instead of fixed date width */
.sv-results-list .list-row {
  grid-template-columns: 16px 1fr 80px 70px 1fr;
}

.sv-results-list {
  padding: 0 4px;
}

.sv-location-meta {
  text-align: left !important;
  direction: rtl;
  unicode-bidi: plaintext;
}

.sv-limit-note {
  font-size: 11px;
  color: var(--muted);
  text-align: center;
  padding: 8px;
  border-top: 1px solid var(--line);
}
</style>
