<template>
  <div class="ql-shell" :data-platform="isMac ? 'macos' : 'other'">

    <!-- Titlebar -->
    <header class="ql-titlebar" data-tauri-drag-region @mousedown="onTitlebarMousedown">
      <div class="ql-title" data-tauri-drag-region>
        <span class="ql-filename">{{ metadata?.name ?? '' }}</span>
      </div>
      <div class="ql-title-actions">
        <WinControls v-if="!isMac" />
      </div>
    </header>

    <!-- Loading / error -->
    <div v-if="loading" class="ql-loading">Loading…</div>
    <div v-else-if="loadError" class="ql-error">{{ loadError }}</div>

    <!-- Content -->
    <template v-else-if="metadata">
      <!-- Preview area -->
      <div class="ql-preview" :class="previewClass">

        <!-- Image -->
        <div v-if="previewType === 'image'" class="ql-image-wrap">
          <img
            v-if="previewSrc"
            :src="previewSrc"
            class="ql-image"
            alt=""
          />
          <div v-else class="ql-image-placeholder">
            <FileIcon :path="metadata.path" :is-dir="metadata.is_dir" :size="64" />
            <span class="ql-img-loading">Loading preview…</span>
          </div>
        </div>

        <!-- Rich / rendered (Markdown, HTML) -->
        <iframe
          v-else-if="previewType === 'rich'"
          class="ql-rich-frame"
          sandbox="allow-same-origin"
          :srcdoc="renderedHtml"
        />

        <!-- Text / code (truncated) -->
        <div v-else-if="previewType === 'text'" class="ql-editor-wrap">
          <div ref="editorContainer" class="ql-editor-pane" />
          <div v-if="previewTruncated" class="ql-preview-note">
            Showing first {{ PREVIEW_LINE_LIMIT.toLocaleString() }} of {{ metadata.line_count?.toLocaleString() }} lines
          </div>
        </div>

        <!-- Generic icon fallback -->
        <div v-else class="ql-icon-wrap">
          <FileIcon :path="metadata.path" :is-dir="metadata.is_dir" :size="72" />
        </div>
      </div>

      <!-- Metadata -->
      <div class="ql-meta">
        <div class="ql-meta-grid">
          <span class="ql-label">Name</span>
          <span class="ql-value">{{ metadata.name }}</span>

          <span class="ql-label">Kind</span>
          <span class="ql-value">{{ kindLabel }}</span>

          <span class="ql-label">Size</span>
          <span class="ql-value">{{ sizeLabel }}</span>

          <span class="ql-label">Modified</span>
          <span class="ql-value">{{ modifiedLabel }}</span>

          <span class="ql-label">Created</span>
          <span class="ql-value">{{ createdLabel }}</span>

          <span class="ql-label">Location</span>
          <span class="ql-value ql-path" :title="parentPath">{{ parentPath }}</span>

          <template v-if="metadata.image_width">
            <span class="ql-label">Dimensions</span>
            <span class="ql-value">{{ metadata.image_width }} × {{ metadata.image_height }} px</span>
          </template>

          <template v-if="metadata.line_count != null">
            <span class="ql-label">Lines</span>
            <span class="ql-value">{{ metadata.line_count.toLocaleString() }}</span>
          </template>
        </div>
      </div>
    </template>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onBeforeUnmount, nextTick } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { listen } from '@tauri-apps/api/event';
import { EditorView } from '@codemirror/view';
import { EditorState } from '@codemirror/state';
import { detectLanguage, languageCompartment, readonlyExtensions } from '@/lib/editorSetup.js';
import { marked } from 'marked';
import FileIcon from './components/FileIcon.vue';
import WinControls from './components/WinControls.vue';

const isMac = navigator.platform.toUpperCase().includes('MAC');
const PREVIEW_LINE_LIMIT = 300;

const editorContainer  = ref(null);
const metadata         = ref(null);
const loading          = ref(true);
const loadError        = ref('');
const previewSrc       = ref('');
const previewType      = ref('none'); // 'image' | 'rich' | 'text' | 'none'
const renderedHtml     = ref('');
const previewTruncated = ref(false);

let editorView  = null;
let unlistenNav = null;

// ── Extension sets ─────────────────────────────────────────────────────────────
const IMAGE_EXTS = new Set(['jpg','jpeg','png','gif','webp','bmp','avif','tiff','tif','ico','qoi']);
const RICH_EXTS  = new Set(['md','markdown','html','htm']);
const TEXT_EXTS  = new Set(['txt','rs','js','mjs','cjs','ts','jsx','tsx','vue',
  'css','scss','sass','json','toml','yaml','yml','xml','sh','bash','zsh',
  'py','rb','go','java','c','h','cpp','cc','cxx','hpp','swift','kt','kts','cs','php',
  'lua','r','sql','gitignore','env','dockerfile','makefile','cmake']);

function getPreviewType(ext) {
  if (!ext) return 'none';
  const e = ext.toLowerCase();
  if (IMAGE_EXTS.has(e)) return 'image';
  if (RICH_EXTS.has(e))  return 'rich';
  if (TEXT_EXTS.has(e))  return 'text';
  return 'none';
}

const previewClass = computed(() => ({
  'ql-preview--image': previewType.value === 'image',
  'ql-preview--rich':  previewType.value === 'rich',
  'ql-preview--text':  previewType.value === 'text',
  'ql-preview--icon':  previewType.value === 'none',
}));

const kindLabel = computed(() => {
  if (!metadata.value) return '';
  if (metadata.value.is_dir) return 'Folder';
  const ext = metadata.value.ext;
  if (!ext) return 'File';
  const mime = metadata.value.mime_type;
  if (mime.startsWith('image/')) return `${ext.toUpperCase()} Image`;
  if (mime.startsWith('text/'))  return `${ext.toUpperCase()} File`;
  return `${ext.toUpperCase()} File`;
});

const sizeLabel = computed(() => {
  const s = metadata.value?.size;
  if (s == null) return '—';
  if (s < 1024)             return `${s} B`;
  if (s < 1024 * 1024)      return `${(s / 1024).toFixed(1)} KB`;
  if (s < 1024 * 1024 * 1024) return `${(s / 1024 / 1024).toFixed(1)} MB`;
  return `${(s / 1024 / 1024 / 1024).toFixed(2)} GB`;
});

function formatMs(ms) {
  if (ms == null) return '—';
  return new Date(Number(ms)).toLocaleString(undefined, {
    year: 'numeric', month: 'short', day: 'numeric',
    hour: '2-digit', minute: '2-digit',
  });
}
const modifiedLabel = computed(() => formatMs(metadata.value?.modified_ms));
const createdLabel  = computed(() => formatMs(metadata.value?.created_ms));

const parentPath = computed(() => {
  const p = metadata.value?.path ?? '';
  const sep = p.includes('/') ? '/' : '\\';
  const parts = p.split(sep);
  parts.pop();
  return parts.join(sep) || sep;
});

// ── Rich preview helpers ───────────────────────────────────────────────────────
const RICH_STYLES = `
  * { box-sizing: border-box; }
  html, body { margin: 0; padding: 0; }
  body {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
    font-size: 13px; color: #1c1c1e; line-height: 1.65;
    padding: 16px 20px; word-break: break-word; overflow-x: hidden;
  }
  h1,h2,h3,h4,h5,h6 { margin: 1.1em 0 0.35em; font-weight: 600; line-height: 1.3; }
  h1 { font-size: 1.5em; border-bottom: 1px solid #d2d2d7; padding-bottom: 0.25em; }
  h2 { font-size: 1.25em; border-bottom: 1px solid #d2d2d7; padding-bottom: 0.15em; }
  p { margin: 0.5em 0; }
  a { color: #0a84ff; }
  code { background: #f2f2f4; padding: 2px 5px; border-radius: 3px;
         font-family: 'SFMono-Regular', Menlo, monospace; font-size: 11.5px; }
  pre { background: #f2f2f4; padding: 12px 14px; border-radius: 6px;
        overflow-x: auto; margin: 0.7em 0; }
  pre code { background: none; padding: 0; }
  blockquote { border-left: 3px solid #d2d2d7; margin: 0.7em 0;
               padding: 3px 0 3px 14px; color: #6e6e73; }
  table { border-collapse: collapse; width: 100%; margin: 0.7em 0; }
  th, td { border: 1px solid #d2d2d7; padding: 5px 10px; text-align: left; font-size: 12px; }
  th { background: #f2f2f4; font-weight: 600; }
  img { max-width: 100%; }
  hr { border: none; border-top: 1px solid #d2d2d7; margin: 1em 0; }
  ul, ol { padding-left: 1.5em; margin: 0.4em 0; }
  li { margin: 0.15em 0; }
`;

function sanitizeAndWrap(html) {
  const parser = new DOMParser();
  const doc = parser.parseFromString(html, 'text/html');
  doc.querySelectorAll('script, style').forEach(el => el.remove());
  doc.querySelectorAll('*').forEach(el => {
    [...el.attributes].forEach(attr => {
      if (attr.name.startsWith('on')) el.removeAttribute(attr.name);
    });
    if (el.tagName === 'A') {
      const href = el.getAttribute('href') ?? '';
      if (href.trimStart().toLowerCase().startsWith('javascript:')) el.removeAttribute('href');
      el.setAttribute('target', '_blank');
    }
  });
  return `<!doctype html><html><head><meta charset="utf-8"><style>${RICH_STYLES}</style></head><body>${doc.body.innerHTML}</body></html>`;
}

// ── Load preview ───────────────────────────────────────────────────────────────
async function loadPreview(path) {
  // Reset for new file
  loading.value = true;
  loadError.value = '';
  previewSrc.value = '';
  renderedHtml.value = '';
  previewTruncated.value = false;
  previewType.value = 'none';
  metadata.value = null;
  if (editorView) { editorView.destroy(); editorView = null; }

  // 1. Fetch metadata — show it as soon as possible
  try {
    const meta = await invoke('get_file_metadata_cmd', { path });
    metadata.value = meta;
    previewType.value = getPreviewType(meta.ext);
  } catch (err) {
    loadError.value = String(err);
    loading.value = false;
    return;
  }

  // 2. Reveal metadata + preview skeleton immediately
  loading.value = false;
  await nextTick(); // let DOM render (editorContainer / iframe now in DOM)

  // 3. Load preview content — doesn't block metadata display
  if (previewType.value === 'image') {
    try {
      const results = await invoke('get_thumbnails_batch_cmd', { paths: [path], size: 1200 });
      if (results[0]) previewSrc.value = `data:image/jpeg;base64,${results[0]}`;
    } catch { /* stay on icon fallback */ }

  } else if (previewType.value === 'rich') {
    let content = '';
    try { content = await invoke('read_text_file_cmd', { path }); } catch { /* empty */ }
    const ext = metadata.value?.ext?.toLowerCase() ?? '';
    const html = (ext === 'md' || ext === 'markdown')
      ? marked.parse(content)
      : content;
    renderedHtml.value = sanitizeAndWrap(html);

  } else if (previewType.value === 'text') {
    let content = '';
    try { content = await invoke('read_text_file_cmd', { path }); } catch { content = '(Could not read file)'; }
    const lines = content.split('\n');
    if (lines.length > PREVIEW_LINE_LIMIT) {
      previewTruncated.value = true;
      content = lines.slice(0, PREVIEW_LINE_LIMIT).join('\n');
    }
    initOrUpdateEditor(path, content);
  }
}

function initOrUpdateEditor(path, content) {
  // Called after loading=false + nextTick, so editorContainer.value is guaranteed present
  if (editorView) {
    editorView.dispatch({
      changes: { from: 0, to: editorView.state.doc.length, insert: content },
    });
    detectLanguage(path).then(({ extension }) => {
      editorView.dispatch({ effects: languageCompartment.reconfigure(extension) });
    });
    return;
  }
  if (!editorContainer.value) return;
  const state = EditorState.create({
    doc: content,
    extensions: [
      ...readonlyExtensions(),
      languageCompartment.of([]),
      EditorView.lineWrapping,
    ],
  });
  editorView = new EditorView({ state, parent: editorContainer.value });
  detectLanguage(path).then(({ extension }) => {
    editorView.dispatch({ effects: languageCompartment.reconfigure(extension) });
  });
}

// ── Titlebar drag ──────────────────────────────────────────────────────────────
function onTitlebarMousedown(e) {
  if (e.target.closest('button')) return;
  if (e.detail > 1) return; // ignore double-click
  getCurrentWindow().startDragging();
}

// ── Keyboard ──────────────────────────────────────────────────────────────────
function onKeyDown(e) {
  if (e.key === 'Escape') getCurrentWindow().close();
}

// ── Lifecycle ─────────────────────────────────────────────────────────────────
onMounted(async () => {
  unlistenNav = await listen('fm://quicklook-navigate', ({ payload }) => {
    loadPreview(payload);
  });

  window.addEventListener('keydown', onKeyDown);

  const path = await invoke('get_quicklook_path_cmd');
  if (path) await loadPreview(path);
  else loadError.value = 'No file path provided.';
});

onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKeyDown);
  unlistenNav?.();
  editorView?.destroy();
});
</script>

<style scoped>
.ql-shell {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background: var(--bg);
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
  font-size: 13px;
  color: var(--ink);
  overflow: hidden;
}

/* ── Titlebar ── */
.ql-titlebar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 36px;
  padding: 0 10px 0 14px;
  flex-shrink: 0;
  user-select: none;
  -webkit-user-select: none;
  background: var(--bg);
  border-bottom: 1px solid var(--line);
}

[data-platform="macos"] .ql-titlebar {
  padding-left: 80px;
}

.ql-title {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
}

.ql-filename {
  font-size: 13px;
  font-weight: 500;
  color: var(--ink);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* ── Preview area ── */
.ql-preview {
  flex: 1 1 0;
  min-height: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  background: var(--panel-muted, #f2f2f4);
  border-bottom: 1px solid var(--line);
}

.ql-preview--text,
.ql-preview--rich {
  align-items: stretch;
}

.ql-image-wrap {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 100%;
  height: 100%;
  padding: 16px;
}

.ql-image {
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
  border-radius: 4px;
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.12);
}

.ql-image-placeholder,
.ql-icon-wrap {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 10px;
  opacity: 0.5;
}

.ql-img-loading {
  font-size: 11px;
  color: var(--muted);
}

.ql-rich-frame {
  width: 100%;
  height: 100%;
  border: none;
  background: #fff;
  display: block;
}

.ql-editor-wrap {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.ql-editor-pane {
  flex: 1 1 0;
  min-height: 0;
  overflow: auto;
}

.ql-preview-note {
  flex-shrink: 0;
  font-size: 11px;
  color: var(--muted);
  text-align: center;
  padding: 4px 8px;
  background: var(--panel-muted, #f2f2f4);
  border-top: 1px solid var(--line);
}

/* ── Metadata ── */
.ql-meta {
  flex-shrink: 0;
  padding: 12px 16px;
  overflow-y: auto;
  max-height: 260px;
}

.ql-meta-grid {
  display: grid;
  grid-template-columns: 80px 1fr;
  row-gap: 5px;
  column-gap: 8px;
}

.ql-label {
  color: var(--muted);
  font-size: 11.5px;
  font-weight: 500;
  padding-top: 1px;
  white-space: nowrap;
}

.ql-value {
  color: var(--ink);
  font-size: 11.5px;
  word-break: break-word;
}

.ql-path {
  color: var(--muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* ── States ── */
.ql-loading,
.ql-error {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--muted);
  font-size: 13px;
  padding: 24px;
  text-align: center;
}

.ql-error {
  color: #ff3b30;
}
</style>

<style>
:root {
  --bg: #f5f5f7; --panel: #ffffff; --ink: #1c1c1e;
  --muted: #6e6e73; --accent: #0a84ff; --line: #d2d2d7;
}
* { box-sizing: border-box; }
html, body { margin: 0; padding: 0; background: var(--bg); overflow: hidden; }
</style>
