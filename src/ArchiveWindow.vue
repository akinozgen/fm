<template>
  <div class="aw-shell" :data-platform="isMac ? 'macos' : 'other'">

    <!-- ── Titlebar ──────────────────────────────────────────────────────────── -->
    <header class="aw-titlebar" @mousedown="onTitlebarMousedown" @mousemove="onTitlebarMousemove" @mouseup="onTitlebarMouseup">
      <span v-if="!isMac" class="aw-title">{{ windowTitle }}</span>
      <WinControls v-if="!isMac" />
    </header>

    <!-- ── Loading / error ───────────────────────────────────────────────────── -->
    <div v-if="loadError" class="aw-error">{{ loadError }}</div>

    <template v-else-if="params">

      <!-- ══ EXTRACT VIEW ══════════════════════════════════════════════════════ -->
      <template v-if="params.kind === 'extract'">
        <div class="aw-body">

          <!-- Archive info -->
          <div class="aw-file-info">
            <div class="aw-file-icon">
              <Archive :size="28" />
            </div>
            <div class="aw-file-meta">
              <span class="aw-file-name">{{ archiveFileName }}</span>
              <span class="aw-format-badge">{{ formatLabel }}</span>
            </div>
          </div>

          <div class="aw-divider" />

          <!-- Destination -->
          <div class="aw-field-row">
            <label class="aw-label">Extract to</label>
            <div class="aw-path-row">
              <input v-model="destPath" class="aw-input" spellcheck="false" :disabled="running" />
              <button class="aw-browse-btn" :disabled="running" @click="browseDest">Browse…</button>
            </div>
          </div>

          <!-- Options -->
          <div class="aw-options">
            <label class="aw-check">
              <input v-model="extractSubfolder" type="checkbox" :disabled="running" />
              <span>Extract into subfolder <em>{{ subfolderName }}</em></span>
            </label>
            <label class="aw-check">
              <input v-model="overwrite" type="checkbox" :disabled="running" />
              <span>Overwrite existing files</span>
            </label>
          </div>

          <!-- Progress -->
          <template v-if="running || done">
            <div class="aw-progress-area">
              <div class="aw-progress-bar-wrap">
                <div class="aw-progress-bar" :style="{ width: progressPct + '%' }" />
              </div>
              <span class="aw-progress-label">{{ progressLabel }}</span>
            </div>
          </template>

          <!-- Done state -->
          <div v-if="done && !doneError" class="aw-done-msg">
            <CheckCircle :size="15" />
            <span>Extracted successfully</span>
          </div>
          <div v-if="doneError" class="aw-error-msg">
            <AlertCircle :size="15" />
            <span>{{ doneError }}</span>
          </div>
        </div>

        <!-- Footer -->
        <footer class="aw-footer">
          <button v-if="done" class="aw-btn primary" @click="openOutput">Open Folder</button>
          <button v-if="done" class="aw-btn" @click="closeWindow">Close</button>
          <template v-else>
            <button class="aw-btn primary" :disabled="running || !destPath" @click="runExtract">
              {{ running ? 'Extracting…' : 'Extract' }}
            </button>
            <button class="aw-btn" :disabled="!running" @click="cancelOp">Cancel</button>
            <button class="aw-btn" :disabled="running" @click="closeWindow">Close</button>
          </template>
        </footer>
      </template>

      <!-- ══ CREATE VIEW ═══════════════════════════════════════════════════════ -->
      <template v-else>
        <div class="aw-body">

          <!-- Source files list -->
          <div class="aw-section-label">
            <span>Files to archive ({{ params.paths.length }})</span>
            <button class="aw-toggle-list" @click="showFileList = !showFileList">
              {{ showFileList ? 'Hide' : 'Show' }}
            </button>
          </div>
          <div v-if="showFileList" class="aw-file-list">
            <div v-for="p in params.paths" :key="p" class="aw-file-list-item" :title="p">
              <Folder v-if="isDir(p)" :size="13" />
              <FileIcon2 v-else :size="13" />
              <span>{{ basename(p) }}</span>
            </div>
          </div>

          <div class="aw-divider" />

          <!-- Output filename -->
          <div class="aw-field-row">
            <label class="aw-label">Archive name</label>
            <input v-model="outputName" class="aw-input" spellcheck="false" :disabled="running" />
          </div>

          <!-- Save location -->
          <div class="aw-field-row">
            <label class="aw-label">Save to</label>
            <div class="aw-path-row">
              <input v-model="destPath" class="aw-input" spellcheck="false" :disabled="running" />
              <button class="aw-browse-btn" :disabled="running" @click="browseDest">Browse…</button>
            </div>
          </div>

          <!-- Format -->
          <div class="aw-field-row">
            <label class="aw-label">Format</label>
            <div class="aw-format-select">
              <button
                v-for="f in availableFormats"
                :key="f.value"
                class="aw-format-btn"
                :class="{ active: archiveFormat === f.value }"
                :disabled="running"
                @click="archiveFormat = f.value"
              >{{ f.label }}</button>
            </div>
          </div>

          <!-- Progress -->
          <template v-if="running || done">
            <div class="aw-progress-area">
              <div class="aw-progress-bar-wrap">
                <div class="aw-progress-bar" :style="{ width: progressPct + '%' }" />
              </div>
              <span class="aw-progress-label">{{ progressLabel }}</span>
            </div>
          </template>

          <!-- Done state -->
          <div v-if="done && !doneError" class="aw-done-msg">
            <CheckCircle :size="15" />
            <span>Archive created: <strong>{{ outputName }}</strong></span>
          </div>
          <div v-if="doneError" class="aw-error-msg">
            <AlertCircle :size="15" />
            <span>{{ doneError }}</span>
          </div>
        </div>

        <!-- Footer -->
        <footer class="aw-footer">
          <button v-if="done" class="aw-btn primary" @click="openOutput">Show in Folder</button>
          <button v-if="done" class="aw-btn" @click="closeWindow">Close</button>
          <template v-else>
            <button class="aw-btn primary" :disabled="running || !outputName || !destPath" @click="runCreate">
              {{ running ? 'Archiving…' : 'Archive' }}
            </button>
            <button class="aw-btn" :disabled="!running" @click="cancelOp">Cancel</button>
            <button class="aw-btn" :disabled="running" @click="closeWindow">Close</button>
          </template>
        </footer>
      </template>

    </template>

    <!-- Loading spinner -->
    <div v-else class="aw-loading">Loading…</div>

  </div>
</template>

<script setup>
import { ref, computed, watch, onMounted, onBeforeUnmount } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { listen } from '@tauri-apps/api/event';
import { open as dialogOpen } from '@tauri-apps/plugin-dialog';
import { Archive, CheckCircle, AlertCircle, Folder, File as FileIcon2 } from 'lucide-vue-next';
import WinControls from './components/WinControls.vue';

const isMac = navigator.platform.toUpperCase().includes('MAC');

// ── State ─────────────────────────────────────────────────────────────────────
const params    = ref(null);
const loadError = ref('');
const destPath  = ref('');
const running   = ref(false);
const done      = ref(false);
const doneError = ref('');
const doneOutputPath = ref('');

// Extract options
const extractSubfolder = ref(true);
const overwrite        = ref(false);

// Create options
const outputName   = ref('');
const archiveFormat = ref('zip');
const showFileList = ref(false);

// Progress
const progressDone  = ref(0);
const progressTotal = ref(0);
const progressCurrent = ref('');

const ALL_FORMATS = [
  { value: 'zip',     label: 'ZIP' },
  { value: 'tar.gz',  label: 'TAR.GZ' },
  { value: 'tar.bz2', label: 'TAR.BZ2' },
  { value: 'tar.xz',  label: 'TAR.XZ' },
  { value: 'gz',      label: 'GZ' },
  { value: 'bz2',     label: 'BZ2' },
  { value: 'xz',      label: 'XZ' },
];

// ── Computed ──────────────────────────────────────────────────────────────────
const windowTitle = computed(() => {
  if (!params.value) return 'Archive';
  return params.value.kind === 'extract' ? 'Extract Archive' : 'Create Archive';
});

const archiveFileName = computed(() => {
  if (!params.value?.paths[0]) return '';
  return basename(params.value.paths[0]);
});

const formatLabel = computed(() => {
  if (!params.value?.paths[0]) return '';
  const name = params.value.paths[0].toLowerCase();
  if (name.endsWith('.tar.gz') || name.endsWith('.tgz'))  return 'TAR.GZ';
  if (name.endsWith('.tar.bz2') || name.endsWith('.tbz2')) return 'TAR.BZ2';
  if (name.endsWith('.tar.xz') || name.endsWith('.txz'))  return 'TAR.XZ';
  if (name.endsWith('.tar'))  return 'TAR';
  if (name.endsWith('.zip'))  return 'ZIP';
  return 'Archive';
});

const subfolderName = computed(() => {
  if (!params.value?.paths[0]) return '';
  const name = basename(params.value.paths[0]).toLowerCase();
  for (const ext of ['.tar.gz', '.tar.bz2', '.tar.xz', '.tgz', '.tbz2', '.txz', '.tar', '.zip']) {
    if (name.endsWith(ext)) return name.slice(0, -ext.length);
  }
  return name;
});

const progressPct = computed(() => {
  if (progressTotal.value === 0) return running.value ? 100 : 0; // indeterminate → full pulse
  return Math.round((progressDone.value / progressTotal.value) * 100);
});

const progressLabel = computed(() => {
  if (done.value) return '';
  if (progressCurrent.value) return progressCurrent.value;
  return running.value ? 'Working…' : '';
});

const outputPath = computed(() => {
  if (!destPath.value || !outputName.value) return '';
  const sep = destPath.value.includes('\\') ? '\\' : '/';
  return destPath.value.replace(/[/\\]$/, '') + sep + outputName.value;
});

// Formats available depend on selection_kind. Single file gets raw gz/bz2/xz too.
const availableFormats = computed(() => {
  const sk = params.value?.selection_kind ?? 'files';
  if (sk === 'file') return ALL_FORMATS;
  return ALL_FORMATS.slice(0, 4); // zip, tar.gz, tar.bz2, tar.xz only
});

// When format changes, update outputName extension
watch(archiveFormat, (newFmt) => {
  if (!outputName.value) return;
  const extMap = {
    'zip': '.zip', 'tar.gz': '.tar.gz', 'tar.bz2': '.tar.bz2',
    'tar.xz': '.tar.xz', 'tar': '.tar', 'gz': '.gz', 'bz2': '.bz2', 'xz': '.xz',
  };
  const newExt = extMap[newFmt] ?? '.zip';
  // Strip current known extension then append new one
  const stripped = outputName.value.replace(/\.(tar\.gz|tar\.bz2|tar\.xz|tar|zip|gz|bz2|xz)$/i, '');
  outputName.value = stripped + newExt;
});

// ── Helpers ───────────────────────────────────────────────────────────────────
function basename(p) {
  return p.replace(/\\/g, '/').split('/').filter(Boolean).pop() || p;
}

function isDir(p) {
  // Heuristic: no extension in last segment (not perfect, but good enough for display)
  const base = basename(p);
  return !base.includes('.');
}

function formatExt(fmt) {
  return { 'zip': '.zip', 'tar.gz': '.tar.gz', 'tar.bz2': '.tar.bz2',
           'tar.xz': '.tar.xz', 'tar': '.tar', 'gz': '.gz', 'bz2': '.bz2', 'xz': '.xz' }[fmt] ?? '.zip';
}

function defaultOutputName(paths, format) {
  const ext = formatExt(format);
  const first = basename(paths[0] || 'archive');
  const stripped = first.replace(/\.(tar\.gz|tar\.bz2|tar\.xz|tgz|tbz2|txz|tar|zip|gz|bz2|xz)$/i, '');
  return (paths.length > 1 ? 'archive' : stripped) + ext;
}

// ── Browse ────────────────────────────────────────────────────────────────────
async function browseDest() {
  const selected = await dialogOpen({ directory: true, multiple: false, defaultPath: destPath.value || undefined });
  if (selected) destPath.value = selected;
}

// ── Run ───────────────────────────────────────────────────────────────────────
async function runExtract() {
  running.value = true;
  doneError.value = '';
  progressDone.value = 0;
  progressTotal.value = 0;
  progressCurrent.value = '';
  const win = getCurrentWindow();
  await invoke('run_archive_cmd', {
    label:       win.label,
    sourcePaths: params.value.paths,
    dest:        destPath.value,
    overwrite:   overwrite.value,
    format:      'extract',
    subfolder:   extractSubfolder.value,
  });
}

async function runCreate() {
  running.value = true;
  doneError.value = '';
  progressDone.value = 0;
  progressTotal.value = 0;
  progressCurrent.value = '';
  const win = getCurrentWindow();
  const sep = destPath.value.includes('\\') ? '\\' : '/';
  const fullOutput = destPath.value.replace(/[/\\]$/, '') + sep + outputName.value;
  await invoke('run_archive_cmd', {
    label:       win.label,
    sourcePaths: params.value.paths,
    dest:        fullOutput,
    overwrite:   false,
    format:      archiveFormat.value,
    subfolder:   false,
  });
}

function cancelOp() {
  const win = getCurrentWindow();
  invoke('cancel_archive_cmd', { label: win.label }).catch(() => {});
}

function openOutput() {
  if (!doneOutputPath.value) return;
  // For create: open the parent folder. For extract: open the dest folder directly.
  const target = params.value.kind === 'create'
    ? doneOutputPath.value.replace(/[/\\][^/\\]+$/, '') || doneOutputPath.value
    : doneOutputPath.value;
  invoke('open_path_cmd', { path: target }).catch(() => {});
}

async function closeWindow() {
  await getCurrentWindow().close();
}

// ── Titlebar drag ──────────────────────────────────────────────────────────────
let pendingDrag = false;
function onTitlebarMousedown(e) {
  if (e.button !== 0) return;
  if (e.target.closest('button, a, input, select, [role="button"]')) return;
  if (e.detail === 2) {
    const win = getCurrentWindow();
    win.isMaximized().then((m) => (m ? win.unmaximize() : win.maximize()));
    return;
  }
  pendingDrag = true;
}
function onTitlebarMousemove() { if (!pendingDrag) return; pendingDrag = false; void getCurrentWindow().startDragging(); }
function onTitlebarMouseup() { pendingDrag = false; }

// ── Lifecycle ──────────────────────────────────────────────────────────────────
let unlistenProgress = null;
let unlistenDone = null;
let unlistenClose = null;

onMounted(async () => {
  const win = getCurrentWindow();

  unlistenClose = await win.onCloseRequested((event) => {
    event.preventDefault();
    if (!running.value) {
      unlistenClose?.(); unlistenClose = null;
      win.close();
    }
  });

  try {
    params.value = await invoke('get_archive_dialog_params_cmd', { label: win.label });
  } catch (e) {
    loadError.value = String(e);
    return;
  }

  destPath.value = params.value.dest_dir;

  if (params.value.kind === 'create') {
    // For non-single-file selections, clamp format to multi-file formats
    const sk = params.value.selection_kind;
    if (sk !== 'file' && ['gz', 'bz2', 'xz'].includes(archiveFormat.value)) {
      archiveFormat.value = 'zip';
    }
    outputName.value = defaultOutputName(params.value.paths, archiveFormat.value);
  }

  unlistenProgress = await listen('fm://archive-progress', (ev) => {
    progressDone.value    = ev.payload.done;
    progressTotal.value   = ev.payload.total;
    progressCurrent.value = ev.payload.current;
  });

  unlistenDone = await listen('fm://archive-done', (ev) => {
    running.value = false;
    done.value    = true;
    if (ev.payload.ok) {
      doneOutputPath.value = ev.payload.output_path ?? '';
    } else if (ev.payload.error !== 'cancelled') {
      doneError.value = ev.payload.error ?? 'Unknown error';
    } else {
      done.value = false; // cancelled — reset to form
      progressCurrent.value = '';
    }
  });
});

onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKeydown);
  unlistenProgress?.();
  unlistenDone?.();
  unlistenClose?.();
});

function onKeydown(e) {
  if ((e.metaKey || e.ctrlKey) && e.key === 'w') {
    e.preventDefault();
    if (!running.value) closeWindow();
  }
  if (e.key === 'Escape' && !running.value) closeWindow();
}

onMounted(() => window.addEventListener('keydown', onKeydown));
</script>

<style>
:root {
  --bg: #f5f5f7; --panel: #ffffff; --ink: #1c1c1e;
  --muted: #6e6e73; --accent: #0a84ff; --line: #d2d2d7; --danger: #ff375f;
  --success: #34c759;
}
* { box-sizing: border-box; }
html, body { background: var(--bg); margin: 0; height: 100vh; overflow: hidden; }
body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; color: var(--ink); font-size: 13px; }
#app { height: 100vh; overflow: hidden; }
</style>

<style scoped>
.aw-shell {
  display: flex; flex-direction: column; height: 100vh;
  background: var(--panel); overflow: hidden;
  user-select: none; -webkit-user-select: none;
}

/* ── Titlebar ─────────────────────────────────────────────────────────────── */
.aw-titlebar {
  display: flex; align-items: center; justify-content: space-between;
  height: 36px; padding: 0 10px 0 14px;
  background: var(--bg); border-bottom: 1px solid var(--line);
  flex-shrink: 0;
}
[data-platform="macos"] .aw-titlebar { padding-left: 80px; justify-content: center; }
.aw-title { font-size: 13px; font-weight: 500; color: var(--ink); }

/* ── Body ────────────────────────────────────────────────────────────────── */
.aw-body {
  flex: 1; overflow-y: auto; padding: 16px 18px;
  display: flex; flex-direction: column; gap: 12px;
}

.aw-loading, .aw-error {
  flex: 1; display: flex; align-items: center; justify-content: center;
  padding: 24px; color: var(--muted); text-align: center;
}
.aw-error { color: var(--danger); }

/* ── File info (extract) ─────────────────────────────────────────────────── */
.aw-file-info {
  display: flex; align-items: center; gap: 12px;
  padding: 10px 12px; background: var(--bg); border-radius: 8px;
  border: 1px solid var(--line);
}
.aw-file-icon { color: var(--accent); flex-shrink: 0; }
.aw-file-meta { display: flex; flex-direction: column; gap: 3px; min-width: 0; }
.aw-file-name {
  font-size: 13px; font-weight: 500; white-space: nowrap;
  overflow: hidden; text-overflow: ellipsis;
}
.aw-format-badge {
  display: inline-flex; align-items: center;
  font-size: 10px; font-weight: 600; letter-spacing: 0.05em;
  padding: 1px 6px; border-radius: 4px;
  background: #0a84ff18; color: var(--accent);
  border: 1px solid #0a84ff33; width: fit-content;
}

.aw-divider { border: none; border-top: 1px solid var(--line); margin: 0; }

/* ── Fields ──────────────────────────────────────────────────────────────── */
.aw-field-row { display: flex; flex-direction: column; gap: 5px; }
.aw-label { font-size: 11px; font-weight: 500; color: var(--muted); text-transform: uppercase; letter-spacing: 0.04em; }
.aw-path-row { display: flex; gap: 6px; }
.aw-input {
  flex: 1; border: 1px solid var(--line); border-radius: 5px;
  background: var(--bg); color: var(--ink);
  padding: 5px 8px; font-size: 12px; font-family: inherit; outline: none;
}
.aw-input:focus { border-color: var(--accent); box-shadow: 0 0 0 2px #0a84ff22; }
.aw-input:disabled { opacity: 0.5; }
.aw-browse-btn {
  flex-shrink: 0; padding: 5px 10px; border-radius: 5px;
  border: 1px solid var(--line); background: var(--panel); color: var(--ink);
  font-size: 12px; font-family: inherit; cursor: default;
}
.aw-browse-btn:hover:not(:disabled) { background: #ebebf0; }
.aw-browse-btn:disabled { opacity: 0.4; }

/* ── Options (checkboxes) ────────────────────────────────────────────────── */
.aw-options { display: flex; flex-direction: column; gap: 7px; }
.aw-check {
  display: flex; align-items: flex-start; gap: 7px;
  font-size: 12px; color: var(--ink); cursor: default;
}
.aw-check input { margin-top: 2px; flex-shrink: 0; cursor: default; }
.aw-check em { font-style: normal; font-weight: 500; color: var(--accent); }

/* ── Format selector ─────────────────────────────────────────────────────── */
.aw-format-select { display: flex; gap: 5px; flex-wrap: wrap; }
.aw-format-btn {
  padding: 4px 12px; border-radius: 5px;
  border: 1px solid var(--line); background: var(--panel); color: var(--ink);
  font-size: 12px; font-family: inherit; cursor: default;
  transition: background 0.1s, border-color 0.1s;
}
.aw-format-btn:hover:not(:disabled) { background: #ebebf0; }
.aw-format-btn.active { background: #0a84ff18; color: var(--accent); border-color: #0a84ff55; font-weight: 500; }
.aw-format-btn:disabled { opacity: 0.4; }

/* ── Section header / file list ──────────────────────────────────────────── */
.aw-section-label {
  display: flex; align-items: center; justify-content: space-between;
  font-size: 11px; font-weight: 500; color: var(--muted);
  text-transform: uppercase; letter-spacing: 0.04em;
}
.aw-toggle-list {
  font-size: 11px; color: var(--accent); background: none; border: none;
  cursor: default; padding: 0;
}
.aw-file-list {
  max-height: 110px; overflow-y: auto;
  border: 1px solid var(--line); border-radius: 6px;
  background: var(--bg); padding: 4px 0;
}
.aw-file-list-item {
  display: flex; align-items: center; gap: 7px;
  padding: 3px 10px; font-size: 12px; color: var(--ink);
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
}
.aw-file-list-item svg { flex-shrink: 0; color: var(--muted); }

/* ── Progress ────────────────────────────────────────────────────────────── */
.aw-progress-area { display: flex; flex-direction: column; gap: 5px; }
.aw-progress-bar-wrap {
  height: 5px; background: #d2d2d7; border-radius: 3px; overflow: hidden;
}
.aw-progress-bar {
  height: 100%; background: var(--accent); border-radius: 3px;
  transition: width 0.15s ease;
  min-width: 4px;
}
.aw-progress-label {
  font-size: 11px; color: var(--muted);
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
}

/* ── Done / error messages ───────────────────────────────────────────────── */
.aw-done-msg, .aw-error-msg {
  display: flex; align-items: center; gap: 7px;
  padding: 8px 10px; border-radius: 6px; font-size: 12px;
}
.aw-done-msg  { background: #34c75918; color: #1a6632; border: 1px solid #34c75940; }
.aw-error-msg { background: #ff375f18; color: #9b0020; border: 1px solid #ff375f40; }

/* ── Footer ──────────────────────────────────────────────────────────────── */
.aw-footer {
  display: flex; align-items: center; justify-content: flex-end; gap: 8px;
  padding: 10px 18px; border-top: 1px solid var(--line);
  background: var(--bg); flex-shrink: 0;
}
.aw-btn {
  display: inline-flex; align-items: center; gap: 5px;
  font-size: 12px; font-family: inherit; padding: 5px 14px;
  border-radius: 6px; border: 1px solid var(--line);
  background: var(--panel); color: var(--ink); cursor: default;
  transition: background 0.1s;
}
.aw-btn:hover:not(:disabled) { background: #ebebf0; }
.aw-btn:disabled { opacity: 0.4; }
.aw-btn.primary { background: var(--accent); color: #fff; border-color: var(--accent); }
.aw-btn.primary:hover:not(:disabled) { background: #0070e0; }
</style>
