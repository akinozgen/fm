<template>
  <div class="editor-shell" :data-platform="isMac ? 'macos' : 'other'">

    <!-- ── Titlebar ──────────────────────────────────────────────────────── -->
    <header
      class="ed-titlebar"
      @mousedown="onTitlebarMousedown"
      @mousemove="onTitlebarMousemove"
      @mouseup="onTitlebarMouseup"
    >
      <div class="ed-title-left">
        <span class="ed-filename">
          <span v-if="isDirty" class="ed-dirty" title="Unsaved changes">●</span>
          {{ fileName }}
        </span>
      </div>
      <div class="ed-title-right">
        <button
          class="ed-btn save-btn"
          :disabled="!isDirty || saving"
          title="Save (Ctrl+S / Cmd+S)"
          @click="save"
        >
          <Save :size="12" />
          <span>{{ saving ? 'Saving…' : 'Save' }}</span>
        </button>
        <WinControls v-if="!isMac" />
      </div>
    </header>

    <!-- ── Toolbar ───────────────────────────────────────────────────────── -->
    <div class="ed-toolbar">
      <button
        class="ed-tb-btn"
        :class="{ active: wordWrap }"
        title="Toggle Word Wrap (Alt+Z)"
        @click="toggleWordWrap"
      >
        <WrapText :size="13" />
        <span>Wrap</span>
      </button>
      <button class="ed-tb-btn" :class="{ active: showSearch }" title="Find &amp; Replace (Ctrl+F)" @click="openSearch(false)">
        <Search :size="13" />
        <span>Find</span>
      </button>
      <div class="ed-tb-spacer" />
      <span class="ed-tb-lang" :title="`Language: ${languageName}`">{{ languageName }}</span>
    </div>

    <!-- ── Disk-changed banner ───────────────────────────────────────────── -->
    <Transition name="banner">
      <div v-if="diskChanged" class="ed-disk-banner">
        <AlertCircle :size="13" />
        <span>File changed on disk</span>
        <div class="banner-actions">
          <button class="banner-btn" @click="reloadFromDisk">
            <RefreshCw :size="11" /> Reload
          </button>
          <button class="banner-btn" @click="openDiffModal">
            <GitMerge :size="11" /> Show Diff
          </button>
          <button class="banner-btn banner-close" title="Dismiss" @click="diskChanged = false">
            <X :size="11" />
          </button>
        </div>
      </div>
    </Transition>

    <!-- ── Error state ───────────────────────────────────────────────────── -->
    <div v-if="loadError" class="ed-error">{{ loadError }}</div>

    <!-- ── Editor area (pane + floating search panel) ────────────────────── -->
    <div class="ed-editor-area">
      <div v-show="!loadError" ref="editorContainer" class="ed-pane" />

      <!-- ── Custom search / replace panel ─────────────────────────────── -->
      <Transition name="search-slide">
        <div v-if="showSearch" class="sp-panel">

          <!-- Find row -->
          <div class="sp-row">
            <button
              class="sp-expand-btn"
              :class="{ expanded: showReplace }"
              :title="showReplace ? 'Hide Replace' : 'Show Replace'"
              @click="showReplace = !showReplace"
            >
              <ChevronRight :size="13" />
            </button>

            <div class="sp-field" :class="{ error: searchTerm && !regexError && matchInfo.total === 0, 'regex-error': regexError }">
              <input
                ref="searchInputRef"
                v-model="searchTerm"
                class="sp-input"
                placeholder="Find"
                autocomplete="off"
                spellcheck="false"
                @input="applySearchQuery"
                @keydown="onSearchKeydown"
              />
              <span v-if="searchTerm && !regexError" class="sp-count" :class="{ zero: matchInfo.total === 0 }">
                {{ matchInfo.total === 0 ? 'No results' : `${matchInfo.current} of ${matchInfo.total}` }}
              </span>
              <span v-if="regexError" class="sp-regex-err" title="Invalid regular expression">!</span>
            </div>

            <div class="sp-toggles">
              <button class="sp-toggle" :class="{ active: caseSensitive }" title="Match Case (Alt+C)" @click="toggleOpt('caseSensitive')">Aa</button>
              <button class="sp-toggle" :class="{ active: wholeWord }"    title="Match Whole Word (Alt+W)" @click="toggleOpt('wholeWord')">W</button>
              <button class="sp-toggle" :class="{ active: useRegex }"     title="Use Regular Expression (Alt+R)" @click="toggleOpt('useRegex')">.*</button>
            </div>

            <div class="sp-navbtns">
              <button class="sp-icon-btn" title="Previous Match (Shift+Enter)" @click="findPrevMatch">
                <ChevronUp :size="13" />
              </button>
              <button class="sp-icon-btn" title="Next Match (Enter)" @click="findNextMatch">
                <ChevronDown :size="13" />
              </button>
            </div>

            <button class="sp-icon-btn sp-close-btn" title="Close (Escape)" @click="closeSearch">
              <X :size="13" />
            </button>
          </div>

          <!-- Replace row (collapsible) -->
          <Transition name="replace-expand">
            <div v-if="showReplace" class="sp-row sp-replace-row">
              <div class="sp-expand-placeholder" />
              <div class="sp-field">
                <input
                  ref="replaceInputRef"
                  v-model="replaceTerm"
                  class="sp-input"
                  placeholder="Replace"
                  autocomplete="off"
                  spellcheck="false"
                  @keydown.enter.prevent="doReplaceNext"
                  @keydown.escape="closeSearch"
                />
              </div>
              <div class="sp-replace-actions">
                <button class="sp-action-btn" title="Replace (Enter)" @click="doReplaceNext">Replace</button>
                <button class="sp-action-btn" title="Replace All" @click="doReplaceAll">All</button>
              </div>
            </div>
          </Transition>

        </div>
      </Transition>
    </div>

    <!-- ── Diff / merge modal ────────────────────────────────────────────── -->
    <Transition name="modal">
      <div v-if="showDiffModal" class="ed-overlay" @click.self="closeDiffModal">
        <div class="ed-diff-panel">
          <div class="ed-diff-header">
            <div class="diff-title">
              <GitMerge :size="14" />
              <span>Your version vs On-disk version</span>
            </div>
            <div class="diff-header-right">
              <button class="ed-btn primary" @click="acceptDisk">Use Disk Version</button>
              <button class="ed-btn" @click="closeDiffModal">Keep My Version</button>
              <button class="ed-icon-btn" title="Close" @click="closeDiffModal"><X :size="14" /></button>
            </div>
          </div>
          <div class="diff-col-labels">
            <span>Your Version (editor)</span>
            <span>On-disk Version</span>
          </div>
          <div ref="diffContainer" class="ed-diff-body" />
        </div>
      </div>
    </Transition>

    <!-- ── Unsaved-changes close modal ───────────────────────────────────── -->
    <Transition name="modal">
      <div v-if="showCloseModal" class="ed-overlay">
        <div class="ed-modal-box">
          <p class="modal-msg">
            <strong>{{ fileName }}</strong> has unsaved changes. Save before closing?
          </p>
          <div class="modal-actions">
            <button class="ed-btn primary" @click="saveAndClose">Save</button>
            <button class="ed-btn danger" @click="discardAndClose">Discard</button>
            <button class="ed-btn" @click="showCloseModal = false">Cancel</button>
          </div>
        </div>
      </div>
    </Transition>

    <!-- ── Status bar ────────────────────────────────────────────────────── -->
    <footer class="ed-statusbar">
      <div class="sb-left">
        <span class="sb-item">{{ encoding }}</span>
        <span class="sb-sep">|</span>
        <span class="sb-item">{{ lineEnding }}</span>
      </div>
      <div class="sb-right">
        <span v-if="selection.chars > 0" class="sb-item sb-sel">
          {{ selection.chars }} chars ({{ selection.lines }}
          {{ selection.lines === 1 ? 'line' : 'lines' }}) selected
        </span>
        <span v-if="selection.chars > 0" class="sb-sep">|</span>
        <span class="sb-item">Ln {{ cursorPos.line }}, Col {{ cursorPos.col }}</span>
        <span class="sb-sep">|</span>
        <span class="sb-item">{{ totalLines }} lines</span>
      </div>
    </footer>

  </div>
</template>

<script setup>
import { ref, computed, onMounted, onBeforeUnmount, nextTick } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { listen } from '@tauri-apps/api/event';
import { EditorView } from '@codemirror/view';
import {
  SearchQuery,
  setSearchQuery,
  findNext as cmFindNext,
  findPrevious as cmFindPrev,
  replaceNext as cmReplaceNext,
  replaceAll as cmReplaceAll,
} from '@codemirror/search';
import { MergeView } from '@codemirror/merge';
import {
  createEditor,
  detectLanguage,
  wrapCompartment,
  languageCompartment,
  readonlyExtensions,
} from '@/lib/editorSetup.js';
import WinControls from './components/WinControls.vue';
import {
  WrapText, Search, AlertCircle, X, Save, RefreshCw, GitMerge,
  ChevronRight, ChevronUp, ChevronDown,
} from 'lucide-vue-next';

// ── Platform ──────────────────────────────────────────────────────────────────
const isMac = navigator.platform.toUpperCase().includes('MAC');

// ── DOM refs ──────────────────────────────────────────────────────────────────
const editorContainer = ref(null);
const diffContainer   = ref(null);
const searchInputRef  = ref(null);
const replaceInputRef = ref(null);

// ── Non-reactive imperative instances ────────────────────────────────────────
let editorView         = null;
let mergeViewInstance  = null;
let unlistenFileChange = null;
let unlistenClose      = null;
let justSaved          = false;
let justSavedTimer     = null;
let currentLangExt     = [];

// ── Core editor state ─────────────────────────────────────────────────────────
const filePath     = ref('');
const loadError    = ref('');
const saving       = ref(false);
const isDirty      = ref(false);
const wordWrap     = ref(false);
const diskChanged  = ref(false);
const showDiffModal  = ref(false);
const showCloseModal = ref(false);
const languageName = ref('Plain Text');
const encoding     = ref('UTF-8');
const lineEnding   = ref('LF');
const cursorPos    = ref({ line: 1, col: 1 });
const selection    = ref({ chars: 0, lines: 0 });
const totalLines   = ref(0);

// ── Search state ──────────────────────────────────────────────────────────────
const showSearch    = ref(false);
const showReplace   = ref(false);
const searchTerm    = ref('');
const replaceTerm   = ref('');
const caseSensitive = ref(false);
const wholeWord     = ref(false);
const useRegex      = ref(false);
const matchInfo     = ref({ total: 0, current: 0 });
const regexError    = ref(false);

const fileName = computed(() => {
  if (!filePath.value) return 'Loading\u2026';
  const sep = filePath.value.includes('\\') ? '\\' : '/';
  return filePath.value.split(sep).filter(Boolean).pop() || filePath.value;
});

// ── CodeMirror update handler ─────────────────────────────────────────────────
function handleEditorUpdate(update) {
  if (update.selectionSet || update.docChanged) {
    const state = update.state;
    const sel   = state.selection.main;
    const line  = state.doc.lineAt(sel.head);
    cursorPos.value = { line: line.number, col: sel.head - line.from + 1 };

    if (sel.from !== sel.to) {
      selection.value = {
        chars: state.sliceDoc(sel.from, sel.to).length,
        lines: state.doc.lineAt(sel.to).number - state.doc.lineAt(sel.from).number + 1,
      };
    } else {
      selection.value = { chars: 0, lines: 0 };
    }

    // Refresh match position when cursor moves while search is open
    if (showSearch.value && searchTerm.value && !regexError.value) {
      refreshMatchCount(buildQuery());
    }
  }

  if (update.docChanged) {
    isDirty.value      = true;
    totalLines.value   = update.state.doc.lines;
    const sample       = update.state.sliceDoc(0, Math.min(2000, update.state.doc.length));
    lineEnding.value   = sample.includes('\r\n') ? 'CRLF' : 'LF';
  }
}

// ── Save ──────────────────────────────────────────────────────────────────────
async function save() {
  if (!isDirty.value || saving.value || !editorView) return;
  saving.value = true;
  justSaved = true;
  clearTimeout(justSavedTimer);
  try {
    const content = editorView.state.doc.toString();
    await invoke('write_text_file_cmd', { path: filePath.value, content });
    isDirty.value = false;
    justSavedTimer = setTimeout(() => { justSaved = false; }, 2000);
  } catch (err) {
    justSaved = false;
    console.error('Save failed:', err);
  } finally {
    saving.value = false;
  }
}

// ── Word wrap ─────────────────────────────────────────────────────────────────
function toggleWordWrap() {
  wordWrap.value = !wordWrap.value;
  editorView?.dispatch({
    effects: wrapCompartment.reconfigure(wordWrap.value ? EditorView.lineWrapping : []),
  });
}

// ── Search helpers ────────────────────────────────────────────────────────────
function buildQuery() {
  return new SearchQuery({
    search:        searchTerm.value,
    replace:       replaceTerm.value,
    caseSensitive: caseSensitive.value,
    regexp:        useRegex.value,
    wholeWord:     wholeWord.value,
  });
}

function refreshMatchCount(q) {
  if (!searchTerm.value || !q.valid || !editorView) {
    matchInfo.value = { total: 0, current: 0 };
    return;
  }
  try {
    const cursor = q.getCursor(editorView.state.doc);
    const curPos = editorView.state.selection.main.from;
    let total = 0, current = 0;
    while (!cursor.next().done) {
      total++;
      if (cursor.value.from <= curPos) current = total;
    }
    matchInfo.value = { total, current: current || (total > 0 ? 1 : 0) };
  } catch {
    matchInfo.value = { total: 0, current: 0 };
  }
}

function applySearchQuery() {
  if (!editorView) return;
  regexError.value = false;
  const q = buildQuery();
  if (useRegex.value && searchTerm.value && !q.valid) {
    regexError.value = true;
    matchInfo.value  = { total: 0, current: 0 };
    return;
  }
  editorView.dispatch({ effects: setSearchQuery.of(q) });
  refreshMatchCount(q);
}

function toggleOpt(key) {
  if (key === 'caseSensitive') caseSensitive.value = !caseSensitive.value;
  else if (key === 'wholeWord') wholeWord.value = !wholeWord.value;
  else if (key === 'useRegex')  useRegex.value  = !useRegex.value;
  applySearchQuery();
}

// ── Search open / close ───────────────────────────────────────────────────────
function openSearch(withReplace = false) {
  // Pre-fill with current editor selection if we have nothing yet
  if (editorView && !searchTerm.value) {
    const sel = editorView.state.selection.main;
    if (!sel.empty) {
      const txt = editorView.state.sliceDoc(sel.from, sel.to);
      if (txt.length < 200 && !txt.includes('\n')) searchTerm.value = txt;
    }
  }
  showSearch.value  = true;
  showReplace.value = withReplace;
  nextTick(() => {
    searchInputRef.value?.focus();
    searchInputRef.value?.select();
    if (searchTerm.value) applySearchQuery();
  });
}

function closeSearch() {
  showSearch.value = false;
  // Clear highlights by resetting query
  if (editorView) {
    editorView.dispatch({ effects: setSearchQuery.of(new SearchQuery({ search: '' })) });
  }
  editorView?.focus();
}

// ── Search navigation ─────────────────────────────────────────────────────────
function findNextMatch() {
  if (!editorView || !searchTerm.value) return;
  applySearchQuery();
  cmFindNext(editorView);
  nextTick(() => refreshMatchCount(buildQuery()));
}

function findPrevMatch() {
  if (!editorView || !searchTerm.value) return;
  applySearchQuery();
  cmFindPrev(editorView);
  nextTick(() => refreshMatchCount(buildQuery()));
}

function doReplaceNext() {
  if (!editorView) return;
  applySearchQuery();
  cmReplaceNext(editorView);
  nextTick(() => refreshMatchCount(buildQuery()));
}

function doReplaceAll() {
  if (!editorView) return;
  applySearchQuery();
  cmReplaceAll(editorView);
  nextTick(() => refreshMatchCount(buildQuery()));
}

function onSearchKeydown(event) {
  if (event.key === 'Enter' && !event.shiftKey) {
    event.preventDefault(); findNextMatch();
  } else if (event.key === 'Enter' && event.shiftKey) {
    event.preventDefault(); findPrevMatch();
  } else if (event.key === 'Escape') {
    event.preventDefault(); closeSearch();
  } else if (event.key === 'Tab' && showReplace.value) {
    event.preventDefault(); replaceInputRef.value?.focus();
  }
}

// ── Reload from disk ──────────────────────────────────────────────────────────
async function reloadFromDisk() {
  try {
    const content = await invoke('read_text_file_cmd', { path: filePath.value });
    editorView.dispatch({ changes: { from: 0, to: editorView.state.doc.length, insert: content } });
    isDirty.value    = false;
    diskChanged.value = false;
    totalLines.value = editorView.state.doc.lines;
    lineEnding.value = content.includes('\r\n') ? 'CRLF' : 'LF';
  } catch (err) {
    console.error('Reload failed:', err);
  }
}

// ── Diff modal ────────────────────────────────────────────────────────────────
async function openDiffModal() {
  showDiffModal.value = true;
  await nextTick();
  let diskText = '';
  try { diskText = await invoke('read_text_file_cmd', { path: filePath.value }); } catch { /* ok */ }
  const ro = [...readonlyExtensions(), ...currentLangExt];
  mergeViewInstance = new MergeView({
    a: { doc: editorView ? editorView.state.doc.toString() : '', extensions: ro },
    b: { doc: diskText, extensions: ro },
    parent: diffContainer.value,
  });
}

function closeDiffModal() {
  showDiffModal.value = false;
  mergeViewInstance?.destroy();
  mergeViewInstance = null;
}

function acceptDisk() {
  if (mergeViewInstance && editorView) {
    const content = mergeViewInstance.b.state.doc.toString();
    editorView.dispatch({ changes: { from: 0, to: editorView.state.doc.length, insert: content } });
    isDirty.value     = false;
    diskChanged.value = false;
  }
  closeDiffModal();
}

// ── Close window ──────────────────────────────────────────────────────────────

// Actually close: remove the listener first so the re-triggered close event
// has no handler and the window closes for real.
async function doClose() {
  unlistenClose?.();
  unlistenClose = null;
  await getCurrentWindow().close();
}

function requestClose() {
  if (isDirty.value) {
    showCloseModal.value = true;
  } else {
    doClose();
  }
}

async function saveAndClose() {
  await save();
  doClose();
}

async function discardAndClose() {
  showCloseModal.value = false;
  doClose();
}

function onGlobalKeydown(e) {
  if ((e.metaKey || e.ctrlKey) && e.key === 'w') {
    e.preventDefault();
    requestClose();
  }
}

// ── Titlebar drag ─────────────────────────────────────────────────────────────
let pendingDrag = false;

function onTitlebarMousedown(event) {
  if (event.button !== 0) return;
  if (event.target.closest('button, a, input, select, [role="button"]')) return;
  if (event.detail === 2) {
    const win = getCurrentWindow();
    win.isMaximized().then((max) => (max ? win.unmaximize() : win.maximize()));
    return;
  }
  pendingDrag = true;
}
function onTitlebarMousemove() {
  if (!pendingDrag) return;
  pendingDrag = false;
  void getCurrentWindow().startDragging();
}
function onTitlebarMouseup() { pendingDrag = false; }

// ── Lifecycle ─────────────────────────────────────────────────────────────────
onMounted(async () => {
  const win = getCurrentWindow();

  window.addEventListener('keydown', onGlobalKeydown);

  unlistenClose = await win.onCloseRequested((event) => {
    event.preventDefault();
    requestClose();
  });

  let path;
  try {
    path = await invoke('get_editor_path_cmd', { label: win.label });
    filePath.value = path;
  } catch (err) { loadError.value = String(err); return; }

  let content = '';
  try {
    content = await invoke('read_text_file_cmd', { path });
  } catch (err) { loadError.value = String(err); return; }

  lineEnding.value = content.includes('\r\n') ? 'CRLF' : 'LF';

  const { name, extension } = await detectLanguage(fileName.value);
  languageName.value = name;
  currentLangExt     = extension;

  editorView = createEditor({
    parent:        editorContainer.value,
    doc:           content,
    onUpdate:      handleEditorUpdate,
    onSave:        save,
    onToggleWrap:  toggleWordWrap,
    onOpenSearch:  () => openSearch(false),
    onOpenReplace: () => openSearch(true),
  });

  totalLines.value = editorView.state.doc.lines;
  if (extension.length > 0) {
    editorView.dispatch({ effects: languageCompartment.reconfigure(extension) });
  }
  editorView.focus();

  try { await invoke('watch_editor_file_cmd', { label: win.label, path }); } catch { /* ok */ }

  unlistenFileChange = await listen('fm://editor-file-changed', () => {
    if (justSaved || diskChanged.value) return;
    diskChanged.value = true;
  });
});

onBeforeUnmount(async () => {
  window.removeEventListener('keydown', onGlobalKeydown);
  unlistenClose?.();
  unlistenFileChange?.();
  editorView?.destroy();
  closeDiffModal();
  clearTimeout(justSavedTimer);
  const win = getCurrentWindow();
  try { await invoke('unwatch_editor_file_cmd', { label: win.label }); } catch { /* ok */ }
});
</script>

<style>
:root {
  --bg: #f5f5f7; --panel: #ffffff; --ink: #1c1c1e;
  --muted: #6e6e73; --accent: #0a84ff; --line: #d2d2d7; --danger: #ff375f;
}
* { box-sizing: border-box; }
html, body { background: var(--bg); margin: 0; height: 100vh; overflow: hidden; }
body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; color: var(--ink); }
#app { height: 100vh; overflow: hidden; }
</style>

<style scoped>
/* ── Shell ───────────────────────────────────────────────────────────────────  */
.editor-shell {
  display: flex; flex-direction: column; height: 100vh;
  background: var(--panel); overflow: hidden;
  user-select: none; -webkit-user-select: none;
}

/* ── Titlebar ────────────────────────────────────────────────────────────────  */
.ed-titlebar {
  display: flex; align-items: center; justify-content: space-between;
  height: 36px; padding: 0 10px 0 14px;
  background: var(--bg); border-bottom: 1px solid var(--line); flex-shrink: 0; gap: 8px;
}
[data-platform="macos"] .ed-titlebar { padding-left: 80px; }
.ed-title-left { flex: 1; min-width: 0; display: flex; align-items: center; }
.ed-title-right { display: flex; align-items: center; gap: 6px; flex-shrink: 0; }
.ed-filename {
  font-size: 13px; font-weight: 500; color: var(--ink);
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
  display: flex; align-items: center; gap: 5px;
}
.ed-dirty { color: var(--accent); font-size: 11px; line-height: 1; }

/* ── Shared buttons ──────────────────────────────────────────────────────────  */
.ed-btn {
  display: inline-flex; align-items: center; gap: 5px;
  font-size: 12px; font-family: inherit; padding: 4px 10px;
  border-radius: 5px; border: 1px solid var(--line);
  background: var(--panel); color: var(--ink); cursor: default;
  transition: background 0.1s; flex-shrink: 0;
}
.ed-btn:hover:not(:disabled) { background: #ebebf0; }
.ed-btn:disabled { opacity: 0.4; }
.ed-btn.primary { background: var(--accent); color: #fff; border-color: var(--accent); }
.ed-btn.primary:hover:not(:disabled) { background: #0070e0; }
.ed-btn.danger  { background: var(--danger); color: #fff; border-color: var(--danger); }
.ed-btn.danger:hover:not(:disabled)  { background: #cc002a; }
.ed-icon-btn {
  display: inline-flex; align-items: center; justify-content: center;
  width: 26px; height: 26px; border-radius: 5px;
  border: 1px solid transparent; background: transparent; color: var(--muted); cursor: default;
}
.ed-icon-btn:hover { background: #ebebf0; color: var(--ink); }

/* ── Toolbar ─────────────────────────────────────────────────────────────────  */
.ed-toolbar {
  display: flex; align-items: center; gap: 4px; height: 30px; padding: 0 10px;
  background: var(--bg); border-bottom: 1px solid var(--line); flex-shrink: 0;
}
.ed-tb-btn {
  display: inline-flex; align-items: center; gap: 4px;
  font-size: 11.5px; font-family: inherit; padding: 3px 7px;
  border-radius: 4px; border: 1px solid transparent;
  background: transparent; color: var(--muted); cursor: default;
  transition: background 0.1s, color 0.1s;
}
.ed-tb-btn:hover { background: #ebebf0; color: var(--ink); }
.ed-tb-btn.active { background: #0a84ff18; color: var(--accent); border-color: #0a84ff33; }
.ed-tb-spacer { flex: 1; }
.ed-tb-lang { font-size: 11px; color: var(--muted); padding: 0 4px; white-space: nowrap; }

/* ── Disk-changed banner ─────────────────────────────────────────────────────  */
.ed-disk-banner {
  display: flex; align-items: center; gap: 8px; padding: 5px 12px;
  background: #fff3cd; border-bottom: 1px solid #f0c040;
  color: #7a5900; font-size: 12px; flex-shrink: 0;
}
.banner-actions { display: flex; align-items: center; gap: 4px; margin-left: auto; }
.banner-btn {
  display: inline-flex; align-items: center; gap: 4px;
  font-size: 11px; font-family: inherit; padding: 2px 8px;
  border-radius: 4px; border: 1px solid #c09000;
  background: transparent; color: #7a5900; cursor: default;
}
.banner-btn:hover { background: #ffe89a; }
.banner-btn.banner-close { padding: 2px 5px; border-color: transparent; }
.banner-enter-active,.banner-leave-active { transition: max-height .2s ease,opacity .2s ease; max-height: 48px; overflow: hidden; }
.banner-enter-from,.banner-leave-to { max-height: 0; opacity: 0; }

/* ── Error ───────────────────────────────────────────────────────────────────  */
.ed-error {
  flex: 1; display: flex; align-items: center; justify-content: center;
  padding: 24px; color: var(--danger); font-size: 13px; text-align: center;
}

/* ── Editor area wrapper ─────────────────────────────────────────────────────  */
.ed-editor-area {
  flex: 1; position: relative; overflow: hidden;
  display: flex; flex-direction: column;
}

/* ── CodeMirror pane ─────────────────────────────────────────────────────────  */
.ed-pane {
  flex: 1; overflow: hidden; display: flex; flex-direction: column;
  user-select: text; -webkit-user-select: text;
}
.ed-pane :deep(.cm-editor)  { height: 100%; }
.ed-pane :deep(.cm-scroller) { height: 100%; }

/* ── Search / replace panel ──────────────────────────────────────────────────  */
.sp-panel {
  position: absolute; top: 8px; right: 14px; z-index: 50;
  width: 430px;
  background: #ffffff;
  border: 1px solid #c8c8cc;
  border-radius: 8px;
  box-shadow: 0 6px 24px rgba(0,0,0,0.14), 0 1px 4px rgba(0,0,0,0.08);
  overflow: hidden;
  font-size: 12px;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
}

.sp-row {
  display: flex; align-items: center; gap: 5px; padding: 6px 8px;
}

/* Expand / collapse replace toggle */
.sp-expand-btn {
  display: flex; align-items: center; justify-content: center;
  width: 20px; height: 20px; border-radius: 3px;
  border: none; background: transparent; color: var(--muted);
  cursor: default; flex-shrink: 0;
  transition: transform 0.15s ease, background 0.1s;
}
.sp-expand-btn:hover { background: #ebebf0; color: var(--ink); }
.sp-expand-btn.expanded { transform: rotate(90deg); }
.sp-expand-placeholder { width: 20px; flex-shrink: 0; }

/* Input field */
.sp-field {
  flex: 1; min-width: 0; display: flex; align-items: center;
  background: #f5f5f7; border: 1px solid #d2d2d7; border-radius: 5px;
  padding: 0 7px; height: 26px; transition: border-color 0.15s, box-shadow 0.15s;
}
.sp-field:focus-within { border-color: var(--accent); box-shadow: 0 0 0 2px #0a84ff22; background: #fff; }
.sp-field.error    { border-color: var(--danger); }
.sp-field.error:focus-within { box-shadow: 0 0 0 2px #ff375f22; }
.sp-field.regex-error { border-color: #ff9500; }
.sp-field.regex-error:focus-within { box-shadow: 0 0 0 2px #ff950022; }

.sp-input {
  flex: 1; min-width: 0; border: none; background: transparent;
  outline: none; font-size: 12px; font-family: inherit; color: var(--ink);
}
.sp-input::placeholder { color: #b0b0b6; }

.sp-count {
  font-size: 11px; color: var(--muted); white-space: nowrap;
  padding-left: 6px; flex-shrink: 0;
}
.sp-count.zero { color: var(--danger); }

.sp-regex-err {
  font-size: 11px; font-weight: 700; color: #ff9500;
  padding-left: 5px; flex-shrink: 0; cursor: default;
}

/* Toggle buttons (Aa, W, .*) */
.sp-toggles { display: flex; align-items: center; gap: 2px; flex-shrink: 0; }

.sp-toggle {
  display: flex; align-items: center; justify-content: center;
  height: 22px; min-width: 24px; padding: 0 5px;
  border-radius: 4px; border: 1px solid transparent;
  background: transparent; color: var(--muted);
  font-size: 11px; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
  font-weight: 500; cursor: default; letter-spacing: 0.01em;
  transition: background 0.1s, color 0.1s;
}
.sp-toggle:hover { background: #ebebf0; color: var(--ink); }
.sp-toggle.active {
  background: #0a84ff1a; color: var(--accent);
  border-color: #0a84ff44;
}

/* Nav prev/next buttons */
.sp-navbtns { display: flex; align-items: center; gap: 2px; flex-shrink: 0; }

.sp-icon-btn {
  display: flex; align-items: center; justify-content: center;
  width: 24px; height: 24px; border-radius: 4px;
  border: none; background: transparent; color: var(--muted); cursor: default;
  transition: background 0.1s, color 0.1s;
}
.sp-icon-btn:hover { background: #ebebf0; color: var(--ink); }
.sp-close-btn { margin-left: 1px; }

/* Replace row */
.sp-replace-row { border-top: 1px solid #ebebf0; }
.sp-replace-actions { display: flex; align-items: center; gap: 4px; flex-shrink: 0; }
.sp-action-btn {
  height: 24px; padding: 0 9px; border-radius: 4px;
  border: 1px solid #d2d2d7; background: #ffffff; color: var(--ink);
  font-size: 11px; font-family: inherit; cursor: default; white-space: nowrap;
  transition: background 0.1s;
}
.sp-action-btn:hover { background: #ebebf0; }

/* Panel animation */
.search-slide-enter-active,.search-slide-leave-active {
  transition: opacity 0.12s ease, transform 0.12s ease;
}
.search-slide-enter-from,.search-slide-leave-to {
  opacity: 0; transform: translateY(-6px) scale(0.98);
}

/* Replace row expand animation */
.replace-expand-enter-active,.replace-expand-leave-active {
  transition: max-height 0.15s ease, opacity 0.12s ease;
  max-height: 44px; overflow: hidden;
}
.replace-expand-enter-from,.replace-expand-leave-to { max-height: 0; opacity: 0; }

/* ── Overlay (diff + close modal) ────────────────────────────────────────────  */
.ed-overlay {
  position: absolute; inset: 0; background: rgba(0,0,0,0.35);
  display: flex; align-items: center; justify-content: center; z-index: 100;
}

/* ── Diff panel ──────────────────────────────────────────────────────────────  */
.ed-diff-panel {
  display: flex; flex-direction: column; width: 96%; height: 90%;
  background: var(--panel); border-radius: 10px; overflow: hidden;
  box-shadow: 0 8px 32px rgba(0,0,0,0.28);
}
.ed-diff-header {
  display: flex; align-items: center; justify-content: space-between;
  padding: 10px 14px; background: var(--bg);
  border-bottom: 1px solid var(--line); flex-shrink: 0; gap: 12px;
}
.diff-title { display: flex; align-items: center; gap: 7px; font-size: 13px; font-weight: 500; }
.diff-header-right { display: flex; align-items: center; gap: 6px; }
.diff-col-labels { display: flex; background: var(--bg); border-bottom: 1px solid var(--line); flex-shrink: 0; }
.diff-col-labels span {
  flex: 1; padding: 4px 12px; font-size: 11px; font-weight: 500;
  color: var(--muted); text-align: center;
}
.diff-col-labels span:first-child { border-right: 1px solid var(--line); }
.ed-diff-body {
  flex: 1; overflow: hidden; user-select: text; -webkit-user-select: text;
}
.ed-diff-body :deep(.cm-mergeView)       { height: 100%; }
.ed-diff-body :deep(.cm-mergeViewEditor) { height: 100%; }
.ed-diff-body :deep(.cm-editor)          { height: 100%; }
.ed-diff-body :deep(.cm-scroller)        { height: 100%; }

/* ── Close modal ─────────────────────────────────────────────────────────────  */
.ed-modal-box {
  background: var(--panel); border-radius: 10px;
  padding: 20px 24px; width: 360px;
  box-shadow: 0 8px 32px rgba(0,0,0,0.28);
}
.modal-msg { margin: 0 0 16px; font-size: 13px; line-height: 1.5; }
.modal-actions { display: flex; gap: 8px; justify-content: flex-end; }
.modal-enter-active,.modal-leave-active { transition: opacity 0.15s ease; }
.modal-enter-from,.modal-leave-to { opacity: 0; }

/* ── Status bar ──────────────────────────────────────────────────────────────  */
.ed-statusbar {
  display: flex; align-items: center; justify-content: space-between;
  height: 22px; padding: 0 10px;
  background: var(--bg); border-top: 1px solid var(--line);
  flex-shrink: 0; font-size: 11px; color: var(--muted); gap: 6px;
}
.sb-left,.sb-right { display: flex; align-items: center; gap: 6px; }
.sb-item { white-space: nowrap; }
.sb-sep { color: var(--line); }
.sb-sel { color: var(--accent); }
</style>
