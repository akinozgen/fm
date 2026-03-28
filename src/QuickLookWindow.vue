<template>
  <div class="ql-shell" :data-platform="isMac ? 'macos' : 'other'">

    <!-- Titlebar -->
    <header class="ql-titlebar" data-tauri-drag-region @mousedown="onTitlebarMousedown">
      <div class="ql-title" data-tauri-drag-region>
        <FileIcon v-if="metadata" :path="metadata.path" :is-dir="metadata.is_dir" :size="14" class="ql-title-icon" />
        <span class="ql-filename">{{ metadata?.name ?? '' }}</span>
      </div>
      <div class="ql-title-actions">
        <span v-if="metadata" class="ql-kind-badge">{{ kindBadge }}</span>
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

        <!-- JSON tree / raw -->
        <div v-else-if="previewType === 'json'" class="ql-json-wrap">
          <!-- Tab bar -->
          <div class="ql-json-bar">
            <div class="ql-json-tabs">
              <button :class="['ql-json-tab', { 'ql-json-tab--active': jsonTab === 'tree' }]" @click="jsonTab = 'tree'">Tree</button>
              <button :class="['ql-json-tab', { 'ql-json-tab--active': jsonTab === 'raw' }]" @click="jsonTab = 'raw'">Raw</button>
            </div>
            <template v-if="jsonTab === 'tree' && !jsonParseError">
              <input
                v-model="jsonSearch"
                class="ql-json-search"
                type="search"
                placeholder="Search…"
                spellcheck="false"
              />
              <button class="ql-json-action" @click="expandAll">Expand all</button>
              <button class="ql-json-action" @click="collapseAll">Collapse all</button>
            </template>
          </div>
          <!-- Tree view -->
          <div v-if="jsonTab === 'tree'" class="ql-json-tree-scroll">
            <div v-if="jsonParseError" class="ql-json-parse-err">
              <span>Could not parse JSON</span>
              <code>{{ jsonParseError }}</code>
            </div>
            <JsonTreeNode
              v-else-if="jsonData !== null"
              :node-key="null"
              :value="jsonData"
              :depth="0"
              :search-query="jsonSearch"
              :expand-seq="jsonExpandSeq"
              :expand-dir="jsonExpandDir"
            />
          </div>
          <!-- Raw view -->
          <div v-else class="ql-json-raw-wrap">
            <div ref="jsonEditorContainer" class="ql-editor-pane" />
          </div>
          <div v-if="jsonTruncated" class="ql-preview-note">
            File exceeds preview limit — showing partial content
          </div>
        </div>

        <!-- PDF -->
        <iframe
          v-else-if="previewType === 'pdf'"
          class="ql-pdf-frame"
          :src="pdfSrc"
        />

        <!-- Font preview -->
        <div v-else-if="previewType === 'font'" class="ql-font-wrap">
          <div class="ql-font-preview">
            <p class="ql-font-showcase">Aa Bb Cc Dd Ee Ff</p>
            <p class="ql-font-pangram">The quick brown fox jumps over the lazy dog</p>
            <p class="ql-font-upper">ABCDEFGHIJKLMNOPQRSTUVWXYZ</p>
            <p class="ql-font-lower">abcdefghijklmnopqrstuvwxyz</p>
            <p class="ql-font-nums">0 1 2 3 4 5 6 7 8 9 &nbsp; ! ? &amp; @ # $ %</p>
          </div>
        </div>

        <!-- ODF text (ODT / ODP) -->
        <div v-else-if="previewType === 'odf-text'" class="ql-editor-wrap">
          <div ref="odfEditorContainer" class="ql-editor-pane" />
        </div>

        <!-- ODF spreadsheet (ODS) — reuses CSV table layout -->
        <div v-else-if="previewType === 'odf-sheet'" class="ql-csv-wrap">
          <div class="ql-csv-scroll">
            <table class="ql-csv-table">
              <thead v-if="odfRows.length">
                <tr>
                  <th class="ql-csv-rn">#</th>
                  <th v-for="(cell, ci) in odfRows[0]" :key="ci">{{ cell }}</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="(row, ri) in odfRows.slice(1)" :key="ri">
                  <td class="ql-csv-rn">{{ ri + 1 }}</td>
                  <td v-for="(cell, ci) in row" :key="ci" :title="cell">{{ cell }}</td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>

        <!-- Archive listing (ZIP / TAR variants) -->
        <div v-else-if="previewType === 'archive'" class="ql-archive-wrap">
          <div class="ql-archive-bar">
            <span>{{ archiveTotalCount.toLocaleString() }} {{ archiveTotalCount === 1 ? 'entry' : 'entries' }}</span>
            <span v-if="archiveTruncated" class="ql-archive-bar-note">first 500 shown</span>
          </div>
          <div class="ql-archive-scroll">
            <div v-for="(e, i) in archiveEntries" :key="i" class="ql-ae-row">
              <component :is="e.is_dir ? LucideFolder : LucideFile" :size="12" class="ql-ae-icon" :class="{ 'ql-ae-icon--dir': e.is_dir }" />
              <span class="ql-ae-name" :title="e.name">{{ e.name }}</span>
              <span class="ql-ae-size">{{ e.is_dir ? '' : formatBytes(e.size) }}</span>
            </div>
          </div>
        </div>

        <!-- 3D model viewer -->
        <div v-else-if="previewType === 'model'" class="ql-model-wrap">
          <ModelViewer :path="metadata.path" />
        </div>

        <!-- CSV / TSV table -->
        <div v-else-if="previewType === 'csv'" class="ql-csv-wrap">
          <div class="ql-csv-scroll">
            <table class="ql-csv-table">
              <thead v-if="csvRows.length">
                <tr>
                  <th class="ql-csv-rn">#</th>
                  <th v-for="(cell, ci) in csvRows[0]" :key="ci">{{ cell }}</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="(row, ri) in csvRows.slice(1)" :key="ri">
                  <td class="ql-csv-rn">{{ ri + 1 }}</td>
                  <td v-for="(cell, ci) in row" :key="ci" :title="cell">{{ cell }}</td>
                </tr>
              </tbody>
            </table>
          </div>
          <div v-if="csvTruncated" class="ql-preview-note">
            Showing first {{ CSV_ROW_LIMIT.toLocaleString() }} of {{ csvTotalRows.toLocaleString() }} rows
          </div>
        </div>

        <!-- Video -->
        <div v-else-if="previewType === 'video'" class="ql-media-wrap">
          <video
            ref="videoEl"
            class="ql-video"
            :src="mediaUrl"
            preload="metadata"
            autoplay
            muted
            playsinline
            @timeupdate="onTimeUpdate"
            @play="onMediaPlay"
            @pause="onMediaPause"
          />
          <div class="ql-media-controls">
            <button class="ql-play-btn" @click="togglePlay(videoEl)">
              {{ mediaEnded ? '↺' : mediaPaused ? '▶' : '⏸' }}
            </button>
            <div class="ql-progress-track" @click="seekClick($event, videoEl)">
              <div class="ql-progress-fill" :style="{ width: mediaProgress + '%' }" />
            </div>
            <span class="ql-time">{{ fmtTime(mediaTime) }} / 0:30</span>
            <span v-if="mediaEnded" class="ql-preview-badge">preview</span>
          </div>
        </div>

        <!-- Audio -->
        <div v-else-if="previewType === 'audio'" class="ql-audio-wrap">
          <div class="ql-artwork-area">
            <img
              v-if="audioMeta?.artwork_base64"
              class="ql-artwork"
              :src="`data:${audioMeta.artwork_mime};base64,${audioMeta.artwork_base64}`"
              alt=""
            />
            <div v-else class="ql-artwork-placeholder">
              <FileIcon :path="metadata.path" :is-dir="false" :size="72" />
            </div>
            <div v-if="audioMeta?.title" class="ql-track-info">
              <span class="ql-track-title">{{ audioMeta.title }}</span>
              <span v-if="audioMeta?.artist" class="ql-track-artist">{{ audioMeta.artist }}</span>
            </div>
          </div>
          <audio
            ref="audioEl"
            :src="mediaUrl"
            preload="metadata"
            @timeupdate="onTimeUpdate"
            @play="onMediaPlay"
            @pause="onMediaPause"
          />
          <div class="ql-media-controls">
            <button class="ql-play-btn" @click="togglePlay(audioEl)">
              {{ mediaEnded ? '↺' : mediaPaused ? '▶' : '⏸' }}
            </button>
            <div class="ql-progress-track" @click="seekClick($event, audioEl)">
              <div class="ql-progress-fill" :style="{ width: mediaProgress + '%' }" />
            </div>
            <span class="ql-time">{{ fmtTime(mediaTime) }} / 0:30</span>
            <span v-if="mediaEnded" class="ql-preview-badge">preview</span>
          </div>
        </div>

        <!-- Generic icon fallback -->
        <div v-else class="ql-icon-wrap">
          <FileIcon :path="metadata.path" :is-dir="metadata.is_dir" :size="72" />
        </div>
      </div>

      <!-- Metadata -->
      <div class="ql-meta">
        <!-- Badge strip: most important facts at a glance -->
        <div class="ql-meta-badges">
          <span class="ql-meta-badge">{{ kindLabel }}</span>
          <span class="ql-meta-badge">{{ sizeLabel }}</span>
          <span v-if="metadata.line_count != null" class="ql-meta-badge">{{ metadata.line_count.toLocaleString() }} lines</span>
          <span v-if="metadata.image_width" class="ql-meta-badge">{{ metadata.image_width }}×{{ metadata.image_height }}</span>
          <span v-if="audioMeta?.duration_secs != null" class="ql-meta-badge">{{ fmtTime(audioMeta.duration_secs) }}</span>
        </div>
        <!-- Compact grid: secondary info only -->
        <div class="ql-meta-grid">
          <span class="ql-label">Modified</span>
          <span class="ql-value" contenteditable="true" spellcheck="false" @beforeinput.prevent>{{ modifiedLabel }}</span>

          <span class="ql-label">Location</span>
          <span class="ql-value ql-path" :title="parentPath" contenteditable="true" spellcheck="false" @beforeinput.prevent>{{ parentPath }}</span>
        </div>
      </div>
    </template>
  </div>
</template>

<script setup>
import { ref, computed, watch, onMounted, onBeforeUnmount, nextTick } from 'vue';
import { invoke, convertFileSrc } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { listen } from '@tauri-apps/api/event';
import { EditorView } from '@codemirror/view';
import { EditorState } from '@codemirror/state';
import { detectLanguage, languageCompartment, readonlyExtensions } from '@/lib/editorSetup.js';
import { marked } from 'marked';
import { File as LucideFile, Folder as LucideFolder } from 'lucide-vue-next';
import FileIcon from './components/FileIcon.vue';
import WinControls from './components/WinControls.vue';
import JsonTreeNode from './components/JsonTreeNode.vue';
import ModelViewer from './components/ModelViewer.vue';

const isMac = navigator.platform.toUpperCase().includes('MAC');
const PREVIEW_LINE_LIMIT = 300;
const JSON_SIZE_LIMIT    = 10 * 1024; // 10 KB
const CSV_ROW_LIMIT      = 500;

const editorContainer  = ref(null);
const videoEl          = ref(null);
const audioEl          = ref(null);
const metadata         = ref(null);
const loading          = ref(true);
const loadError        = ref('');
const previewSrc       = ref('');
const previewType      = ref('none'); // 'image'|'rich'|'json'|'pdf'|'font'|'odf-text'|'odf-sheet'|'archive'|'model'|'text'|'csv'|'video'|'audio'|'none'
const renderedHtml     = ref('');
const previewTruncated = ref(false);
const mediaUrl         = ref('');
const mediaPaused      = ref(true);
const mediaEnded       = ref(false);
const mediaProgress    = ref(0);
const mediaTime        = ref(0);
const audioMeta        = ref(null);
const csvRows          = ref([]);   // parsed rows; row[0] = headers
const csvTruncated     = ref(false);
const csvTotalRows     = ref(0);
const jsonTab          = ref('tree');  // 'tree' | 'raw'
const jsonSearch       = ref('');
const jsonData         = ref(null);
const jsonParseError   = ref('');
const jsonTruncated    = ref(false);
const jsonRawContent   = ref('');
const jsonExpandSeq    = ref(0);
const jsonExpandDir    = ref(true);
const jsonEditorContainer = ref(null);
const pdfSrc           = ref('');
const fontSrc          = ref('');
const odfText          = ref('');
const odfRows          = ref([]);
const archiveEntries   = ref([]);
const archiveTruncated = ref(false);
const archiveTotalCount = ref(0);
const odfEditorContainer = ref(null);
const dirSizeBytes     = ref(null);
const dirSizeDone      = ref(false);

let editorView    = null;
let jsonEditorView = null;
let odfEditorView  = null;
let unlistenNav   = null;
let unlistenDirSz = null;

async function startDirSize(path) {
  dirSizeBytes.value = null;
  dirSizeDone.value  = false;
  if (unlistenDirSz) { unlistenDirSz(); unlistenDirSz = null; }
  unlistenDirSz = await listen('fm://dir-size', (ev) => {
    dirSizeBytes.value = ev.payload.bytes;
    if (ev.payload.done) dirSizeDone.value = true;
  });
  invoke('compute_dir_size_cmd', { paths: [path] });
}

function stopDirSize() {
  if (unlistenDirSz) { unlistenDirSz(); unlistenDirSz = null; }
  invoke('cancel_dir_size_cmd').catch(() => {});
}

// ── Extension sets ─────────────────────────────────────────────────────────────
const IMAGE_EXTS   = new Set(['jpg','jpeg','png','gif','webp','bmp','avif','tiff','tif','ico','qoi','svg']);
const RICH_EXTS  = new Set(['md','markdown','html','htm']);
const JSON_EXTS      = new Set(['json']);
const PDF_EXTS       = new Set(['pdf']);
const FONT_EXTS      = new Set(['ttf','otf','woff','woff2']);
const ODF_TEXT_EXTS  = new Set(['odt','odp']);
const ODF_SHEET_EXTS = new Set(['ods']);
const ARCHIVE_EXTS   = new Set(['zip','tar','tgz','tbz2','txz']);
const MODEL_EXTS     = new Set(['glb','gltf','obj']);
const CSV_EXTS       = new Set(['csv','tsv']);
const TEXT_EXTS  = new Set(['txt','rs','js','mjs','cjs','ts','jsx','tsx','vue',
  'css','scss','sass','toml','yaml','yml','xml','sh','bash','zsh',
  'py','rb','go','java','c','h','cpp','cc','cxx','hpp','swift','kt','kts','cs','php',
  'lua','r','sql','gitignore','env','dockerfile','makefile','cmake']);
const VIDEO_EXTS = new Set(['mp4','m4v','mov','webm']);
const AUDIO_EXTS = new Set(['mp3','m4a','aac','wav','ogg','flac','opus']);

function isArchiveName(name) {
  return ['.tar.gz','.tar.bz2','.tar.xz'].some(s => name.toLowerCase().endsWith(s));
}

function getPreviewType(ext, name = '') {
  if (!ext) return 'none';
  const e = ext.toLowerCase();
  if (IMAGE_EXTS.has(e))      return 'image';
  if (VIDEO_EXTS.has(e))      return 'video';
  if (AUDIO_EXTS.has(e))      return 'audio';
  if (RICH_EXTS.has(e))       return 'rich';
  if (JSON_EXTS.has(e))       return 'json';
  if (PDF_EXTS.has(e))        return 'pdf';
  if (FONT_EXTS.has(e))       return 'font';
  if (ODF_TEXT_EXTS.has(e))   return 'odf-text';
  if (ODF_SHEET_EXTS.has(e))  return 'odf-sheet';
  if (MODEL_EXTS.has(e))      return 'model';
  if (ARCHIVE_EXTS.has(e) || isArchiveName(name)) return 'archive';
  if (CSV_EXTS.has(e))        return 'csv';
  if (TEXT_EXTS.has(e))       return 'text';
  return 'none';
}

const previewClass = computed(() => ({
  'ql-preview--image':     previewType.value === 'image',
  'ql-preview--rich':      previewType.value === 'rich',
  'ql-preview--json':      previewType.value === 'json',
  'ql-preview--pdf':       previewType.value === 'pdf',
  'ql-preview--font':      previewType.value === 'font',
  'ql-preview--odf-text':  previewType.value === 'odf-text',
  'ql-preview--odf-sheet': previewType.value === 'odf-sheet',
  'ql-preview--archive':   previewType.value === 'archive',
  'ql-preview--model':     previewType.value === 'model',
  'ql-preview--csv':       previewType.value === 'csv',
  'ql-preview--text':      previewType.value === 'text',
  'ql-preview--video':     previewType.value === 'video',
  'ql-preview--audio':     previewType.value === 'audio',
  'ql-preview--icon':      previewType.value === 'none',
}));

const kindBadge = computed(() => metadata.value?.ext?.toUpperCase() || (metadata.value?.is_dir ? 'Folder' : 'File'));

const kindLabel = computed(() => {
  if (!metadata.value) return '';
  if (metadata.value.is_dir) return 'Folder';
  const ext = metadata.value.ext?.toLowerCase();
  if (!ext) return 'File';
  if (VIDEO_EXTS.has(ext)) return `${ext.toUpperCase()} Video`;
  if (AUDIO_EXTS.has(ext)) return `${ext.toUpperCase()} Audio`;
  const mime = metadata.value.mime_type;
  if (mime.startsWith('image/')) return `${ext.toUpperCase()} Image`;
  if (mime.startsWith('text/'))  return `${ext.toUpperCase()} File`;
  return `${ext.toUpperCase()} File`;
});

function formatBytes(n) {
  if (n == null) return '—';
  if (n < 1024)               return `${n} B`;
  if (n < 1024 * 1024)        return `${(n / 1024).toFixed(1)} KB`;
  if (n < 1024 * 1024 * 1024) return `${(n / 1024 / 1024).toFixed(1)} MB`;
  return `${(n / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

const sizeLabel = computed(() => {
  if (!metadata.value) return '—';
  if (metadata.value.is_dir) {
    if (dirSizeBytes.value === null) return 'Computing…';
    const label = formatBytes(dirSizeBytes.value);
    return dirSizeDone.value ? label : `${label}…`;
  }
  return formatBytes(metadata.value.size);
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
    font-size: 13px; color: #1c1c1e; line-height: 1.7;
    padding: 20px 24px 24px; word-break: break-word; overflow-x: hidden;
    max-width: 720px;
  }
  h1,h2,h3,h4,h5,h6 { margin: 1.2em 0 0.3em; font-weight: 600; line-height: 1.25; color: #111; }
  h1 { font-size: 1.45em; border-bottom: 1px solid #e5e5ea; padding-bottom: 0.3em; margin-top: 0; }
  h2 { font-size: 1.2em; border-bottom: 1px solid #e5e5ea; padding-bottom: 0.15em; }
  h3 { font-size: 1.05em; }
  p { margin: 0.55em 0; }
  a { color: #0a84ff; text-decoration: none; }
  a:hover { text-decoration: underline; }
  code {
    background: #f2f2f4; padding: 1px 5px; border-radius: 3px;
    font-family: 'SFMono-Regular', Menlo, monospace; font-size: 11.5px;
    border: 1px solid rgba(0,0,0,0.06);
  }
  pre {
    background: #f7f7f9; padding: 12px 16px; border-radius: 7px;
    overflow-x: auto; margin: 0.8em 0; border: 1px solid #e8e8ed;
  }
  pre code { background: none; padding: 0; border: none; font-size: 12px; }
  blockquote {
    border-left: 3px solid #d2d2d7; margin: 0.8em 0;
    padding: 2px 0 2px 14px; color: #6e6e73;
    font-style: italic;
  }
  table { border-collapse: collapse; width: 100%; margin: 0.8em 0; font-size: 12px; }
  th, td { border: 1px solid #d2d2d7; padding: 5px 10px; text-align: left; }
  th { background: #f2f2f4; font-weight: 600; }
  img { max-width: 100%; border-radius: 4px; }
  hr { border: none; border-top: 1px solid #e5e5ea; margin: 1.2em 0; }
  ul, ol { padding-left: 1.6em; margin: 0.4em 0; }
  li { margin: 0.2em 0; }
`;

// ── CSV parser (RFC 4180 basics) ───────────────────────────────────────────────
function parseCsvLine(line, sep) {
  const fields = [];
  let field = '';
  let inQuote = false;
  for (let i = 0; i < line.length; i++) {
    const ch = line[i];
    if (ch === '"') {
      if (inQuote && line[i + 1] === '"') { field += '"'; i++; }
      else inQuote = !inQuote;
    } else if (ch === sep && !inQuote) {
      fields.push(field.trim());
      field = '';
    } else {
      field += ch;
    }
  }
  fields.push(field.trim());
  return fields;
}

function parseCsv(text, ext) {
  const sep = ext === 'tsv' ? '\t' : ',';
  return text
    .split(/\r?\n/)
    .filter(l => l.trim())
    .map(l => parseCsvLine(l, sep));
}

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
  mediaUrl.value = '';
  mediaPaused.value = true;
  mediaEnded.value = false;
  mediaProgress.value = 0;
  mediaTime.value = 0;
  audioMeta.value = null;
  csvRows.value = [];
  csvTruncated.value = false;
  csvTotalRows.value = 0;
  jsonTab.value = 'tree';
  jsonSearch.value = '';
  jsonData.value = null;
  jsonParseError.value = '';
  jsonTruncated.value = false;
  jsonRawContent.value = '';
  jsonExpandSeq.value = 0;
  videoEl.value?.pause();
  audioEl.value?.pause();
  pdfSrc.value = '';
  fontSrc.value = '';
  odfText.value = '';
  odfRows.value = [];
  archiveEntries.value = [];
  archiveTruncated.value = false;
  archiveTotalCount.value = 0;
  document.getElementById('ql-font-face')?.remove();
  if (editorView)    { editorView.destroy();    editorView    = null; }
  if (jsonEditorView){ jsonEditorView.destroy(); jsonEditorView = null; }
  if (odfEditorView) { odfEditorView.destroy();  odfEditorView  = null; }

  stopDirSize();

  // 1. Fetch metadata — show it as soon as possible
  try {
    const meta = await invoke('get_file_metadata_cmd', { path });
    metadata.value = meta;
    previewType.value = getPreviewType(meta.ext, meta.name);
    if (meta.is_dir) startDirSize(path);
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
    const ext = metadata.value?.ext?.toLowerCase() ?? '';
    if (ext === 'svg') {
      // Serve SVG directly to preserve vector quality (no JPEG recompression)
      previewSrc.value = convertFileSrc(path);
    } else {
      try {
        const results = await invoke('get_thumbnails_batch_cmd', { paths: [path], size: 1200 });
        if (results[0]) previewSrc.value = `data:image/jpeg;base64,${results[0]}`;
      } catch { /* stay on icon fallback */ }
    }

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

  } else if (previewType.value === 'json') {
    let content = '';
    try { content = await invoke('read_text_file_cmd', { path }); } catch { content = ''; }
    const lines = content.split('\n');
    if (content.length > JSON_SIZE_LIMIT || lines.length > PREVIEW_LINE_LIMIT) {
      jsonTruncated.value = true;
      const truncated = lines.slice(0, PREVIEW_LINE_LIMIT).join('\n');
      content = truncated.length > JSON_SIZE_LIMIT ? truncated.substring(0, JSON_SIZE_LIMIT) : truncated;
    }
    jsonRawContent.value = content;
    try {
      jsonData.value = JSON.parse(content);
      jsonParseError.value = '';
    } catch (e) {
      jsonData.value = null;
      jsonParseError.value = e.message ?? String(e);
    }

  } else if (previewType.value === 'csv') {
    let content = '';
    try { content = await invoke('read_text_file_cmd', { path }); } catch { /* empty */ }
    const ext = metadata.value?.ext?.toLowerCase() ?? 'csv';
    const all = parseCsv(content, ext);
    csvTotalRows.value = Math.max(0, all.length - 1); // exclude header
    if (all.length > CSV_ROW_LIMIT + 1) {
      csvTruncated.value = true;
      csvRows.value = all.slice(0, CSV_ROW_LIMIT + 1);
    } else {
      csvRows.value = all;
    }

  } else if (previewType.value === 'pdf') {
    pdfSrc.value = convertFileSrc(path);

  } else if (previewType.value === 'font') {
    fontSrc.value = convertFileSrc(path);
    injectFontFace(fontSrc.value);

  } else if (previewType.value === 'odf-text') {
    let text = '';
    try { text = await invoke('extract_odt_text_cmd', { path }); } catch (e) { text = `(Could not read file: ${e})`; }
    odfText.value = text;
    await nextTick();
    initOdfEditor(text);

  } else if (previewType.value === 'odf-sheet') {
    let rows = [];
    try { rows = await invoke('read_ods_cmd', { path }); } catch { rows = []; }
    odfRows.value = rows;

  } else if (previewType.value === 'archive') {
    try {
      const result = await invoke('list_archive_cmd', { path });
      archiveEntries.value   = result.entries;
      archiveTruncated.value = result.truncated;
      archiveTotalCount.value = result.total_count;
    } catch (e) {
      archiveEntries.value = [];
    }

  } else if (previewType.value === 'model') {
    // ModelViewer component handles loading independently

  } else if (previewType.value === 'video') {
    mediaUrl.value = convertFileSrc(path);

  } else if (previewType.value === 'audio') {
    mediaUrl.value = convertFileSrc(path);
    try {
      audioMeta.value = await invoke('get_audio_metadata_cmd', { path });
    } catch { audioMeta.value = null; }
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

// ── JSON tree actions ─────────────────────────────────────────────────────────
function expandAll()   { jsonExpandDir.value = true;  jsonExpandSeq.value++; }
function collapseAll() { jsonExpandDir.value = false; jsonExpandSeq.value++; }

watch(jsonTab, async (tab) => {
  if (tab !== 'raw' || !jsonRawContent.value) return;
  await nextTick();
  if (!jsonEditorContainer.value) return;
  if (jsonEditorView) { jsonEditorView.destroy(); jsonEditorView = null; }
  const state = EditorState.create({
    doc: jsonRawContent.value,
    extensions: [...readonlyExtensions(), languageCompartment.of([]), EditorView.lineWrapping],
  });
  jsonEditorView = new EditorView({ state, parent: jsonEditorContainer.value });
  const path = metadata.value?.path ?? '';
  detectLanguage(path).then(({ extension }) => {
    jsonEditorView?.dispatch({ effects: languageCompartment.reconfigure(extension) });
  });
});

// ── Font preview ──────────────────────────────────────────────────────────────
function injectFontFace(src) {
  document.getElementById('ql-font-face')?.remove();
  const s = document.createElement('style');
  s.id = 'ql-font-face';
  s.textContent = `@font-face { font-family: "__QLPreview"; src: url("${src}"); }`;
  document.head.appendChild(s);
}

// ── ODF text editor ───────────────────────────────────────────────────────────
function initOdfEditor(content) {
  if (!odfEditorContainer.value) return;
  if (odfEditorView) { odfEditorView.destroy(); odfEditorView = null; }
  const state = EditorState.create({
    doc: content,
    extensions: [...readonlyExtensions(), EditorView.lineWrapping],
  });
  odfEditorView = new EditorView({ state, parent: odfEditorContainer.value });
}

// ── Media player ──────────────────────────────────────────────────────────────
function fmtTime(s) {
  const m = Math.floor(s / 60);
  const sec = Math.floor(s % 60).toString().padStart(2, '0');
  return `${m}:${sec}`;
}

function onTimeUpdate(e) {
  const t = e.target.currentTime;
  mediaTime.value = t;
  mediaProgress.value = Math.min((t / 30) * 100, 100);
  if (t >= 30) {
    e.target.pause();
    mediaEnded.value = true;
    mediaPaused.value = true;
  }
}

function onMediaPlay()  { mediaPaused.value = false; }
function onMediaPause() { mediaPaused.value = true; }

function togglePlay(el) {
  if (!el) return;
  if (mediaEnded.value) {
    el.currentTime = 0;
    mediaEnded.value = false;
    mediaProgress.value = 0;
  }
  mediaPaused.value ? el.play() : el.pause();
}

function seekClick(e, el) {
  if (!el) return;
  const rect = e.currentTarget.getBoundingClientRect();
  const pct = (e.clientX - rect.left) / rect.width;
  el.currentTime = Math.min(pct * 30, 30);
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
  jsonEditorView?.destroy();
  odfEditorView?.destroy();
  document.getElementById('ql-font-face')?.remove();
  videoEl.value?.pause();
  audioEl.value?.pause();
  stopDirSize();
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
  height: 38px;
  padding: 0 12px 0 14px;
  flex-shrink: 0;
  user-select: none;
  -webkit-user-select: none;
  background: var(--bg);
  border-bottom: 1px solid var(--line);
  gap: 8px;
}

[data-platform="macos"] .ql-titlebar {
  padding-left: 80px;
}

.ql-title {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 7px;
}

.ql-title-icon {
  flex-shrink: 0;
  opacity: 0.9;
}

.ql-filename {
  font-size: 13px;
  font-weight: 500;
  color: var(--ink);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.ql-title-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.ql-kind-badge {
  font-size: 10.5px;
  font-weight: 500;
  color: var(--muted);
  background: rgba(0, 0, 0, 0.06);
  border-radius: 4px;
  padding: 2px 6px;
  white-space: nowrap;
  letter-spacing: 0.02em;
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

/* White-background types */
.ql-preview--text,
.ql-preview--rich,
.ql-preview--json,
.ql-preview--font,
.ql-preview--odf-text,
.ql-preview--odf-sheet,
.ql-preview--archive {
  background: #fff;
}

/* Stretch-fill types */
.ql-preview--text,
.ql-preview--rich,
.ql-preview--json,
.ql-preview--pdf,
.ql-preview--font,
.ql-preview--odf-text,
.ql-preview--odf-sheet,
.ql-preview--archive,
.ql-preview--model,
.ql-preview--video,
.ql-preview--audio {
  align-items: stretch;
  flex-direction: column;
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

/* ── Media player ── */
.ql-media-wrap {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.ql-video {
  flex: 1;
  width: 100%;
  min-height: 0;
  object-fit: contain;
  background: #000;
  display: block;
}

.ql-audio-wrap {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.ql-artwork-area {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 10px;
  padding: 16px;
  min-height: 0;
}

.ql-artwork {
  max-height: 160px;
  max-width: 160px;
  border-radius: 8px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.18);
  object-fit: cover;
}

.ql-artwork-placeholder {
  opacity: 0.4;
}

.ql-track-info {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
}

.ql-track-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--ink);
  text-align: center;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 240px;
}

.ql-track-artist {
  font-size: 11.5px;
  color: var(--muted);
  text-align: center;
}

.ql-media-controls {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  border-top: 1px solid var(--line);
  background: var(--bg);
}

.ql-play-btn {
  flex-shrink: 0;
  width: 28px;
  height: 28px;
  border: none;
  border-radius: 50%;
  background: var(--ink);
  color: var(--bg);
  font-size: 11px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
}

.ql-progress-track {
  flex: 1;
  height: 4px;
  background: var(--line);
  border-radius: 2px;
  cursor: pointer;
  position: relative;
}

.ql-progress-fill {
  height: 100%;
  background: var(--accent);
  border-radius: 2px;
  transition: width 0.1s linear;
}

.ql-time {
  font-size: 11px;
  color: var(--muted);
  white-space: nowrap;
  flex-shrink: 0;
}

.ql-preview-badge {
  font-size: 10px;
  color: var(--muted);
  background: var(--panel-muted, #f2f2f4);
  border: 1px solid var(--line);
  border-radius: 4px;
  padding: 1px 5px;
  flex-shrink: 0;
}

/* ── Metadata ── */
.ql-meta {
  flex-shrink: 0;
  padding: 10px 14px 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

/* Badge strip: kind · size · lines · dimensions */
.ql-meta-badges {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

.ql-meta-badge {
  display: inline-flex;
  align-items: center;
  font-size: 11px;
  font-weight: 500;
  color: var(--muted);
  background: rgba(0, 0, 0, 0.05);
  border-radius: 4px;
  padding: 2px 7px;
  white-space: nowrap;
}

/* Compact grid: only Modified + Location (+ Duration for audio) */
.ql-meta-grid {
  display: grid;
  grid-template-columns: 66px 1fr;
  row-gap: 4px;
  column-gap: 10px;
}

.ql-label {
  color: var(--muted);
  font-size: 11px;
  font-weight: 500;
  padding-top: 1px;
  white-space: nowrap;
  user-select: none;
  -webkit-user-select: none;
}

.ql-value {
  color: var(--ink);
  font-size: 11px;
  word-break: break-word;
}

.ql-value[contenteditable] {
  outline: none;
  border: none;
  background: transparent;
  caret-color: transparent;
  cursor: text;
}

.ql-value[contenteditable]:focus,
.ql-value[contenteditable]:hover,
.ql-value[contenteditable]:active {
  outline: none;
  border: none;
  background: transparent;
  box-shadow: none;
}

.ql-path {
  color: var(--muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* ── CSV table ── */
.ql-preview--csv {
  background: #fff;
  align-items: stretch;
  flex-direction: column;
}

.ql-csv-wrap {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.ql-csv-scroll {
  flex: 1;
  overflow: auto;
  min-height: 0;
}

.ql-csv-table {
  border-collapse: collapse;
  font-size: 12px;
  min-width: 100%;
}

.ql-csv-table thead {
  position: sticky;
  top: 0;
  z-index: 1;
}

.ql-csv-table th {
  background: #f7f7f9;
  border-bottom: 2px solid var(--line);
  border-right: 1px solid var(--line);
  padding: 6px 12px;
  text-align: left;
  font-weight: 600;
  font-size: 11.5px;
  color: var(--ink);
  white-space: nowrap;
}

.ql-csv-table td {
  border-bottom: 1px solid var(--line);
  border-right: 1px solid var(--line);
  padding: 4px 12px;
  color: var(--ink);
  white-space: nowrap;
  max-width: 260px;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* Row number column */
.ql-csv-rn {
  color: var(--muted) !important;
  font-size: 10.5px !important;
  font-weight: 400 !important;
  text-align: right !important;
  padding-right: 10px !important;
  padding-left: 8px !important;
  min-width: 36px;
  width: 36px;
  background: #f7f7f9;
  border-right: 1px solid var(--line) !important;
  user-select: none;
}

.ql-csv-table thead .ql-csv-rn {
  border-right: 1px solid #d0d0d8 !important;
}

.ql-csv-table tbody tr:hover td {
  background: rgba(0, 0, 0, 0.025);
}

/* ── PDF ── */
.ql-pdf-frame {
  width: 100%;
  height: 100%;
  border: none;
  display: block;
}

/* ── Font preview ── */
.ql-font-wrap {
  flex: 1;
  overflow-y: auto;
  padding: 28px 32px 32px;
  display: flex;
  align-items: flex-start;
  justify-content: center;
  user-select: none;
  -webkit-user-select: none;
}

.ql-font-preview {
  width: 100%;
  max-width: 640px;
}

.ql-font-preview p {
  margin: 0 0 12px;
  font-family: '__QLPreview', serif;
  line-height: 1.3;
}

.ql-font-showcase {
  font-size: 52px !important;
  color: #111;
  letter-spacing: -0.01em;
  margin-bottom: 16px !important;
}

.ql-font-pangram {
  font-size: 16px !important;
  color: #333;
  line-height: 1.6 !important;
}

.ql-font-upper,
.ql-font-lower {
  font-size: 13px !important;
  color: #555;
  letter-spacing: 0.06em;
}

.ql-font-nums {
  font-size: 13px !important;
  color: #888;
  letter-spacing: 0.04em;
}

/* ── Archive listing ── */
.ql-archive-wrap {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  user-select: none;
  -webkit-user-select: none;
}

.ql-archive-bar {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 5px 12px;
  background: #f7f7f9;
  border-bottom: 1px solid var(--line);
  font-size: 11px;
  color: var(--muted);
}

.ql-archive-bar-note {
  font-size: 10.5px;
  color: var(--muted);
  opacity: 0.7;
}

.ql-archive-scroll {
  flex: 1;
  overflow-y: auto;
  min-height: 0;
}

.ql-ae-row {
  display: grid;
  grid-template-columns: 16px 1fr 68px;
  align-items: center;
  gap: 6px;
  padding: 2px 12px;
  min-height: 22px;
}

.ql-ae-row:hover {
  background: rgba(0, 0, 0, 0.03);
}

.ql-ae-icon {
  color: var(--muted);
  flex-shrink: 0;
}

.ql-ae-icon--dir {
  color: var(--accent);
}

.ql-ae-name {
  font-size: 11.5px;
  font-family: 'SFMono-Regular', Menlo, Consolas, monospace;
  color: var(--ink);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.ql-ae-size {
  font-size: 11px;
  color: var(--muted);
  text-align: right;
  white-space: nowrap;
}

/* ── 3D model ── */
.ql-model-wrap {
  flex: 1;
  min-height: 0;
  overflow: hidden;
  background: #161616;
}

/* ── JSON preview ── */
.ql-json-wrap {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.ql-json-bar {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 5px 10px;
  border-bottom: 1px solid var(--line);
  background: #f7f7f9;
  flex-shrink: 0;
}

.ql-json-tabs {
  display: flex;
  background: rgba(0, 0, 0, 0.07);
  border-radius: 6px;
  padding: 2px;
  gap: 1px;
  flex-shrink: 0;
}

.ql-json-tab {
  height: 22px;
  padding: 0 9px;
  border: none;
  border-radius: 4px;
  font-size: 11.5px;
  font-weight: 500;
  cursor: pointer;
  background: transparent;
  color: var(--muted);
  transition: background 0.1s, box-shadow 0.1s;
}

.ql-json-tab--active {
  background: #fff;
  color: var(--ink);
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
}

.ql-json-search {
  flex: 1;
  max-width: 180px;
  height: 24px;
  padding: 0 8px;
  border: 1px solid var(--line);
  border-radius: 5px;
  font-size: 11.5px;
  background: #fff;
  color: var(--ink);
  outline: none;
}

.ql-json-search:focus {
  border-color: var(--accent);
  box-shadow: 0 0 0 2px rgba(10, 132, 255, 0.15);
}

.ql-json-action {
  height: 24px;
  padding: 0 8px;
  border: 1px solid var(--line);
  border-radius: 5px;
  font-size: 11px;
  background: #fff;
  color: var(--muted);
  cursor: pointer;
  white-space: nowrap;
  transition: color 0.1s, border-color 0.1s;
}

.ql-json-action:hover {
  color: var(--ink);
  border-color: rgba(0, 0, 0, 0.22);
}

.ql-json-tree-scroll {
  flex: 1;
  overflow: auto;
  min-height: 0;
  padding: 6px 0;
}

.ql-json-raw-wrap {
  flex: 1;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.ql-json-parse-err {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  height: 100%;
  padding: 20px;
  color: var(--muted);
  font-size: 12px;
}

.ql-json-parse-err code {
  font-size: 11px;
  color: #ff3b30;
  font-family: 'SFMono-Regular', Menlo, monospace;
  max-width: 300px;
  text-align: center;
  word-break: break-word;
  background: #fff5f5;
  padding: 6px 10px;
  border-radius: 5px;
  border: 1px solid rgba(255, 59, 48, 0.2);
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
