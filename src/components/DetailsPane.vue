<template>
  <Transition name="props-slide">
    <aside v-if="entries.length" class="properties-pane" @click.stop>
      <div class="props-header">
        <span class="props-title">Properties</span>
        <button class="icon-btn" title="Close" @click="$emit('close')">
          <X :size="14" />
        </button>
      </div>

      <!-- Single item: show its icon -->
      <div v-if="isSingle" class="props-icon-area">
        <FileIcon :path="entries[0].path" :is-dir="entries[0].is_dir" :size="48" />
      </div>

      <!-- Multi: show count badge -->
      <div v-else class="props-multi-badge">{{ entries.length }} items</div>

      <div class="props-name" :title="isSingle ? entries[0].name : ''">
        {{ isSingle ? entries[0].name : multiLabel }}
      </div>

      <div class="props-body">
        <div class="prop-row">
          <span class="prop-label">Type</span>
          <span class="prop-value">{{ typeLabel }}</span>
        </div>

        <div class="prop-row">
          <span class="prop-label">Size</span>
          <span class="prop-value">{{ sizeLabel }}</span>
        </div>

        <div class="prop-row">
          <span class="prop-label">Modified</span>
          <span class="prop-value">{{ modifiedLabel }}</span>
        </div>

        <div v-if="commonParent" class="prop-row prop-row--path">
          <span class="prop-label">{{ isSingle ? 'Path' : 'Location' }}</span>
          <div class="prop-value-row">
            <span class="prop-value prop-value--path" :title="isSingle ? entries[0].path : commonParent">
              {{ isSingle ? entries[0].path : commonParent }}
            </span>
            <button class="prop-copy-btn" :class="{ copied: justCopied }" :title="justCopied ? 'Copied!' : 'Copy'" @click="copyPath">
              <Check v-if="justCopied" :size="11" />
              <Copy v-else :size="11" />
            </button>
          </div>
        </div>
      </div>
    </aside>
  </Transition>
</template>

<script setup>
import { computed, onUnmounted, ref, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { Check, Copy, X } from 'lucide-vue-next';
import FileIcon from './FileIcon.vue';

const props = defineProps({
  entries: { type: Array, default: () => [] }
});
defineEmits(['close']);

// ── Folder size (progressive) ─────────────────────────────────────────────────
const sizeBytes = ref(null);
const sizeDone  = ref(false);
let unlistenSize = null;

async function startSizeCompute(paths) {
  sizeBytes.value = null;
  sizeDone.value  = false;

  if (unlistenSize) { unlistenSize(); unlistenSize = null; }

  unlistenSize = await listen('fm://dir-size', (ev) => {
    sizeBytes.value = ev.payload.bytes;
    if (ev.payload.done) sizeDone.value = true;
  });

  await invoke('compute_dir_size_cmd', { paths });
}

function stopSizeCompute() {
  if (unlistenSize) { unlistenSize(); unlistenSize = null; }
  invoke('cancel_dir_size_cmd').catch(() => {});
}

// Trigger computation whenever entries that contain a folder change
watch(
  () => props.entries,
  (next) => {
    stopSizeCompute();
    // Need computation if any entry is a dir, or for files we still sum via Rust
    // (lets Rust handle all paths uniformly)
    if (next.length > 0) {
      void startSizeCompute(next.map((e) => e.path));
    }
  },
  { immediate: true }
);

onUnmounted(stopSizeCompute);

// ── Derived display values ────────────────────────────────────────────────────
const isSingle = computed(() => props.entries.length === 1);

const allDirs  = computed(() => props.entries.every((e) => e.is_dir));
const allFiles = computed(() => props.entries.every((e) => !e.is_dir));

const multiLabel = computed(() => {
  if (allDirs.value) return `${props.entries.length} Folders`;
  if (allFiles.value) return `${props.entries.length} Files`;
  const dirs  = props.entries.filter((e) => e.is_dir).length;
  const files = props.entries.length - dirs;
  return `${files} file${files !== 1 ? 's' : ''}, ${dirs} folder${dirs !== 1 ? 's' : ''}`;
});

const typeLabel = computed(() => {
  if (isSingle.value) {
    const e = props.entries[0];
    return e.is_dir ? 'Folder' : (e.ext ? e.ext.toUpperCase() + ' File' : 'File');
  }
  if (allDirs.value) return 'Folders';
  if (allFiles.value) {
    const exts = new Set(props.entries.map((e) => (e.ext || '').toUpperCase()).filter(Boolean));
    if (exts.size === 1) return `${[...exts][0]} Files`;
    return 'Files';
  }
  return 'Mixed';
});

function formatBytes(n) {
  if (n === null || n === undefined) return '…';
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${Math.round(n / 1024)} KB`;
  if (n < 1024 * 1024 * 1024) return `${(n / (1024 * 1024)).toFixed(1)} MB`;
  return `${(n / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

const sizeLabel = computed(() => {
  // All files and no dirs: Rust still computes but it's fast; show result cleanly
  if (sizeBytes.value === null) return 'Computing…';
  const label = formatBytes(sizeBytes.value);
  return sizeDone.value ? label : `${label}…`;
});

const modifiedLabel = computed(() => {
  const times = props.entries
    .map((e) => Number(e.modified_ms))
    .filter((t) => t && !Number.isNaN(t));
  if (times.length === 0) return '—';
  const min = Math.min(...times);
  const max = Math.max(...times);
  const fmt = (t) => new Date(t).toLocaleString();
  return min === max ? fmt(min) : `${new Date(min).toLocaleDateString()} – ${new Date(max).toLocaleDateString()}`;
});

const justCopied = ref(false);
let copyTimer = 0;

function copyPath() {
  const text = isSingle.value ? props.entries[0].path : commonParent.value;
  if (!text) return;
  writeText(text).then(() => {
    justCopied.value = true;
    if (copyTimer) clearTimeout(copyTimer);
    copyTimer = setTimeout(() => { justCopied.value = false; }, 1500);
  });
}

function parentDir(path) {
  const sep = path.includes('\\') ? '\\' : '/';
  const idx = path.lastIndexOf(sep);
  return idx > 0 ? path.slice(0, idx) : null;
}

const commonParent = computed(() => {
  if (props.entries.length === 0) return null;
  const parents = props.entries.map((e) => parentDir(e.path));
  const first = parents[0];
  return parents.every((p) => p === first) ? first : null;
});
</script>
