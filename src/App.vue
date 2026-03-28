<template>
  <div class="app-shell" :data-platform="platformData">
    <header class="app-titlebar" data-tauri-drag-region @mousedown="onTitlebarMousedown">
      <div class="titlebar-left">
        <button type="button" class="titlebar-brand" title="Home" @click="openWelcome">
          <span class="titlebar-brand-icon"><FolderOpen :size="14" /></span>
          <span class="titlebar-brand-name">fm</span>
        </button>
      </div>
      <div class="titlebar-center">
        <div class="titlebar-path-wrap">
          <span class="titlebar-path" :title="currentPath">{{ titleBarLocationLabel }}</span>
          <button type="button" class="titlebar-path-search" title="Search (F6)" @click="navigateTo(FM_SEARCH)">
            <Search :size="14" />
          </button>
        </div>
      </div>
      <div class="titlebar-right">
        <button
          type="button"
          class="titlebar-btn"
          title="Keyboard Shortcuts"
          @click.stop="shortcutsOpen = true"
        >
          <CircleHelp :size="14" />
        </button>
        <button
          type="button"
          class="titlebar-btn"
          title="New file or folder"
          @click.stop="onNewItemClick($event)"
        >
          <Plus :size="14" />
        </button>
        <div class="titlebar-win-controls">
          <WinControls />
        </div>
      </div>
    </header>
    <Sidebar
      :sections="sidebarSections"
      :current-path="currentPath"
      @resize-start="startResize"
      @navigate="navigateTo"
      @unpin="onUnpinFavorite"
      @reorder-pinned="onReorderPinned"
      @unmount="onUnmountDrive"
    />
    <main class="main">
      <Toolbar
        ref="toolbarRef"
        :current-path="currentPath"
        :folder-entries="entries"
        :manual-history="manualPathHistory"
        :transfer-jobs="transferJobs"
        :indexing="indexing"
        :index-done="indexDone"
        :can-go-back="canGoBack"
        :can-go-forward="canGoForward"
        :can-go-up="canGoUp"
        @navigate-up="navigateUp"
        @navigate-back="navigateBack"
        @navigate-forward="navigateForward"
        @navigate-path="navigateTo"
        @navigate-path-manual="navigateToFromManual"
        @delete-manual-history="deleteManualHistoryPath"
        @open-failed="onAddressBarOpenFailed"
        @open-path="openFile"
        @cancel-transfer="onCancelTransfer"
        @pause-transfer="onPauseTransfer"
        @resume-transfer="onResumeTransfer"
        @cancel-index="invoke('cancel_index_cmd')"
      />
      <TrashToolbar
        v-if="!showWelcome && isTrashView"
        :selected-count="selectedEntries.length"
        :can-empty-trash="canEmptyTrash"
        :show-hidden="showHidden"
        :show-extensions="showExtensions"
        :show-selection-checkboxes="showSelectionCheckboxes"
        :view-mode="viewMode"
        :sort-by="sortBy"
        :sort-dir="sortDir"
        :on-delete="deleteSelected"
        :on-empty-trash="emptyTrash"
        @select-all="selectAll"
        @deselect-all="deselectAll"
        @select-inverse="selectInverse"
        @update:show-hidden="setShowHidden"
        @update:show-extensions="setShowExtensions"
        @update:show-selection-checkboxes="setShowSelectionCheckboxes"
        @update:view-mode="onViewModeChange"
        @update:sort-by="setSortBy"
        @update:sort-dir="setSortDir"
      />
      <ActionToolbar
        v-else-if="!showWelcome && !isSearchPath(currentPath)"
        :show-hidden="showHidden"
        :show-extensions="showExtensions"
        :show-selection-checkboxes="showSelectionCheckboxes"
        :view-mode="viewMode"
        :selected-count="selectedEntries.length"
        :sort-by="sortBy"
        :sort-dir="sortDir"
        :on-delete="deleteSelected"
        :can-paste="!!clipboardPaths.length"
        @update:show-hidden="setShowHidden"
        @update:show-extensions="setShowExtensions"
        @update:show-selection-checkboxes="setShowSelectionCheckboxes"
        @update:view-mode="onViewModeChange"
        @update:sort-by="setSortBy"
        @update:sort-dir="setSortDir"
        @select-all="selectAll"
        @deselect-all="deselectAll"
        @select-inverse="selectInverse"
        @cut="onCut"
        @copy="onCopy"
        @paste="onPaste"
      />
      <WelcomePage
        v-if="showWelcome"
        :sections="sidebarSections"
        @navigate="navigateTo"
      />
      <SearchView
        v-else-if="isSearchPath(currentPath)"
        style="grid-row: 3; min-height: 0;"
        :current-path="lastRealPath"
        @navigate="navigateTo"
        @open-file="openFile"
        @update:result-count="searchResultCount = $event"
      />
      <MainContent
        v-else
        ref="mainContentRef"
        :view-mode="viewMode"
        :grid-zoom="gridZoom"
        :current-path="currentPath"
        :entries="sortedEntries"
        :loading="loading"
        :show-extensions="showExtensions"
        :show-selection-checkboxes="showSelectionCheckboxes"
        :rename-entry="renameEntry"
        :create-file="createFile"
        :create-folder="createFolder"
        :request-delete="deleteSelected"
        :cut-paths="clipboardOp === 'cut' ? clipboardPaths : []"
        @open-dir="navigateTo"
        @open-file="openFile"
        @selection-change="onSelectionChange"
        @remove-draft="removeDraftEntry"
        @show-properties="onShowProperties"
        @quicklook="onQuicklook"
        @cursor-changed="onCursorChanged"
      />
      <StatusBar
        :shown-count="isSearchPath(currentPath) ? searchResultCount : sortedEntries.length"
        :selected-count="selectedEntries.length"
        :show-selected-size="showSelectionSize"
        :selected-size-bytes="selectionSizeBytes"
      />
      <div v-if="propertiesEntries.length" class="props-backdrop" @click="propertiesEntries = []" />
      <DetailsPane :entries="propertiesEntries" @close="propertiesEntries = []" />
    </main>
  </div>
  <KeyboardShortcutsModal :open="shortcutsOpen" @close="shortcutsOpen = false" />
</template>

<script setup>
import { invoke } from '@tauri-apps/api/core';
import { listen, emit } from '@tauri-apps/api/event';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import ActionToolbar from './components/ActionToolbar.vue';
import DetailsPane from './components/DetailsPane.vue';
import MainContent from './components/MainContent.vue';
import StatusBar from './components/StatusBar.vue';
import TrashToolbar from './components/TrashToolbar.vue';
import {
  clearManualPathHistory,
  getDirectoryPrefs,
  loadGlobalPrefs,
  loadManualPathHistory,
  removeManualPath,
  pushManualPath,
  saveGlobalPrefs,
  setDirectoryPrefs
} from './lib/appPreferences';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { clearThumbnailQueue } from './lib/iconCache';
import { listenFileContextMenu } from './lib/contextMenu';
import { paste, cancelTransfer, pauseTransfer, resumeTransfer, listenTransferProgress, listenTransferDone } from './lib/transfer';
import { setupKeybindings } from './lib/keybindings';
import { bootstrapPreferencesStore } from './lib/preferencesStore';
import {
  FM_TRASH,
  FM_WELCOME,
  FM_SEARCH,
  canonicalizePath,
  createDraftPath,
  getVirtualPathLabel,
  isDraftPath,
  isSearchPath,
  isTrashPath,
  isVirtualPath,
  isWelcomePath,
  normalizePath
} from './lib/virtualPaths';
import { CircleHelp, FolderOpen, Plus, Search } from 'lucide-vue-next';
import Sidebar from './components/Sidebar.vue';
import Toolbar from './components/Toolbar.vue';
import KeyboardShortcutsModal from './components/KeyboardShortcutsModal.vue';
import WelcomePage from './components/WelcomePage.vue';
import WinControls from './components/WinControls.vue';
import SearchView from './components/SearchView.vue';

const platformData = typeof navigator !== 'undefined' && /Mac|Darwin/.test(navigator.platform || navigator.userAgent || '') ? 'macos' : '';

const viewMode = ref('list');
const sortBy  = ref('name'); // 'name' | 'type' | 'size' | 'modified'
const sortDir = ref('asc');  // 'asc' | 'desc'
const gridZoom = ref(110); // tile cell min-width in px; icon size derived from this
const sidebarWidth = ref(220);
const sidebarSections = ref([]);
const currentPath = ref(FM_WELCOME);
const entries = ref([]);
const loading = ref(false);
const activeRequestId = ref(null);
const showHidden = ref(false);
const showExtensions = ref(true);
const showSelectionCheckboxes = ref(false);
const showWelcome = ref(true);
const manualPathHistory = ref([]);
const mainContentRef = ref(null);
const toolbarRef = ref(null);
const propertiesEntries = ref([]);
const selectedPaths = ref([]);

const clipboardPaths   = ref([]);
const clipboardOp      = ref('');  // 'cut' | 'copy' | ''
const transferJobs     = ref([]);  // { id, op, done, total, bytes_done, bytes_total, current, paused }[]
const quicklookOpen    = ref(false);
const indexing         = ref(false);
const indexDone        = ref(0);
const lastRealPath     = ref('');
const searchResultCount = ref(0);
const shortcutsOpen     = ref(false);

const pathHistory = ref([]);
const historyIndex = ref(-1);
const unlistenFns = [];

const canGoBack    = computed(() => historyIndex.value > 0);
const canGoForward = computed(() => historyIndex.value < pathHistory.value.length - 1);
const canGoUp      = computed(() => !showWelcome.value && !!currentPath.value && !isWelcomePath(currentPath.value));

let resizing = false;
let teardownKeybindings = () => {};
let directoryWatchDebounceTimer = 0;

const sortedEntries = computed(() => {
  const dir = sortDir.value === 'asc' ? 1 : -1;
  return [...entries.value].sort((a, b) => {
    // Drafts always first
    if (!!a.draft !== !!b.draft) return a.draft ? -1 : 1;
    // Folders always before files
    if (a.is_dir !== b.is_dir) return a.is_dir ? -1 : 1;

    let primary = 0;
    if (sortBy.value === 'name') {
      primary = a.name.localeCompare(b.name, undefined, { sensitivity: 'base' });
    } else if (sortBy.value === 'type') {
      const extA = (a.ext || '').toLowerCase();
      const extB = (b.ext || '').toLowerCase();
      primary = extA.localeCompare(extB, undefined, { sensitivity: 'base' });
      if (primary === 0) primary = a.name.localeCompare(b.name, undefined, { sensitivity: 'base' });
    } else if (sortBy.value === 'size') {
      primary = (a.size ?? 0) - (b.size ?? 0);
      if (primary === 0) primary = a.name.localeCompare(b.name, undefined, { sensitivity: 'base' });
    } else if (sortBy.value === 'modified') {
      primary = (Number(a.modified_ms) || 0) - (Number(b.modified_ms) || 0);
      if (primary === 0) primary = a.name.localeCompare(b.name, undefined, { sensitivity: 'base' });
    }
    return primary * dir;
  });
});

const selectedEntries = computed(() => {
  if (selectedPaths.value.length === 0) return [];
  const selectedSet = new Set(selectedPaths.value);
  return sortedEntries.value.filter((entry) => selectedSet.has(entry.path));
});

const showSelectionSize = computed(() => {
  if (selectedEntries.value.length === 0) return false;
  return selectedEntries.value.every((entry) => !entry.is_dir);
});

const selectionSizeBytes = computed(() => {
  if (!showSelectionSize.value) return 0;
  let total = 0;
  for (const entry of selectedEntries.value) {
    if (typeof entry.size === 'number' && Number.isFinite(entry.size) && entry.size > 0) {
      total += entry.size;
    }
  }
  return total;
});

const isTrashView = computed(() => isTrashPath(currentPath.value));
const canEmptyTrash = computed(() => isTrashView.value && entries.value.length > 0);

const titleBarLocationLabel = computed(() => {
  const normalized = normalizePath(currentPath.value);
  if (!normalized) return 'No location';
  if (isVirtualPath(normalized)) {
    return getVirtualPathLabel(normalized) || normalized;
  }
  const parts = normalized.split('/').filter(Boolean);
  if (parts.length === 0) return '/';
  return parts[parts.length - 1];
});

function startResize(event) {
  resizing = true;
  const startX = event.clientX;
  const startWidth = sidebarWidth.value;

  const onMove = (e) => {
    if (!resizing) return;
    const next = Math.min(320, Math.max(160, startWidth + (e.clientX - startX)));
    sidebarWidth.value = next;
    document.documentElement.style.setProperty('--sidebar-width', `${next}px`);
  };

  const onUp = () => {
    resizing = false;
    window.removeEventListener('mousemove', onMove);
    window.removeEventListener('mouseup', onUp);
  };

  window.addEventListener('mousemove', onMove);
  window.addEventListener('mouseup', onUp);
}

function recordHistory(path) {
  const next = canonicalizePath(path);
  const current = pathHistory.value[historyIndex.value];
  if (current === next) return;

  const head = pathHistory.value.slice(0, historyIndex.value + 1);
  head.push(next);
  pathHistory.value = head;
  historyIndex.value = pathHistory.value.length - 1;
}

async function cancelActiveRequest() {
  if (activeRequestId.value === null) return;
  const requestId = activeRequestId.value;
  activeRequestId.value = null;
  try {
    await invoke('cancel_cmd', { requestId });
  } catch {
    // Ignore cancellation errors; request may already be done.
  }
}

async function navigateTo(path, options = {}) {
  const { shouldRecordHistory = true, includeHidden = showHidden.value } = options;
  const nextPath = canonicalizePath(path);
  if (!nextPath) return;

  if (isWelcomePath(nextPath)) {
    await openWelcome({ shouldRecordHistory });
    return;
  }

  if (isSearchPath(nextPath)) {
    await cancelActiveRequest();
    showWelcome.value = false;
    currentPath.value = FM_SEARCH;
    entries.value = [];
    loading.value = false;
    if (shouldRecordHistory) recordHistory(FM_SEARCH);
    return;
  }

  await cancelActiveRequest();

  showWelcome.value = false;
  currentPath.value = nextPath;
  lastRealPath.value = nextPath;
  entries.value = [];
  clearThumbnailQueue();
  selectedPaths.value = [];
  propertiesEntries.value = [];
  loading.value = true;

  if (isTrashPath(nextPath)) {
    try {
      await invoke('stop_dir_watch_cmd');
    } catch {
      // Ignore watcher stop errors.
    }
    if (shouldRecordHistory) {
      recordHistory(nextPath);
    }
    try {
      const trashEntries = await invoke('list_trash_entries_cmd');
      entries.value = Array.isArray(trashEntries) ? trashEntries : [];
    } catch (error) {
      console.error('trash listing failed', error);
      entries.value = [];
    } finally {
      loading.value = false;
    }
    return;
  }

  const dirPrefs = await getDirectoryPrefs(nextPath);
  if (dirPrefs?.viewMode === 'grid' || dirPrefs?.viewMode === 'list') {
    viewMode.value = dirPrefs.viewMode;
  }
  sortBy.value = dirPrefs?.sortBy || 'name';
  sortDir.value = (dirPrefs?.sortDir === 'asc' || dirPrefs?.sortDir === 'desc') ? dirPrefs.sortDir : 'asc';

  if (shouldRecordHistory) {
    recordHistory(nextPath);
  }

  try {
    const requestId = await invoke('read_dir_cmd', {
      path: nextPath,
      opts: {
        recursive: false,
        include_hidden: includeHidden,
        chunk_size: 200
      }
    });
    activeRequestId.value = requestId;
    try {
      await invoke('start_dir_watch_cmd', { path: nextPath });
    } catch {
      // Ignore watcher start errors (unsupported/fs edge cases).
    }
  } catch {
    loading.value = false;
  }
}

async function navigateToFromManual(path) {
  await navigateTo(path);
  await pushManualPath(canonicalizePath(path));
  manualPathHistory.value = await loadManualPathHistory();
}

async function deleteManualHistoryPath(path) {
  await removeManualPath(path);
  manualPathHistory.value = await loadManualPathHistory();
}

async function onUnpinFavorite(path) {
  try {
    await invoke('remove_pinned_favorite_cmd', { path });
    await loadSidebar();
  } catch {
    // ignore
  }
}

function sidebarHomePath() {
  for (const section of sidebarSections.value) {
    for (const item of (section.items || [])) {
      if (item.kind === 'home') return item.path;
    }
  }
  return null;
}

function redirectIfUnderPath(unmountedPath) {
  const cur = currentPath.value;
  if (!cur) return;
  const norm = unmountedPath.replace(/\/$/, '');
  if (cur === norm || cur.startsWith(norm + '/') || cur.startsWith(norm + '\\')) {
    const home = sidebarHomePath();
    if (home) navigateTo(home, { shouldRecordHistory: false });
  }
}

async function onUnmountDrive(path) {
  try {
    await invoke('unmount_drive_cmd', { path });
    redirectIfUnderPath(path);
    await loadSidebar();
  } catch (err) {
    console.error('unmount failed', err);
  }
}

async function onReorderPinned(newOrder) {
  try {
    await invoke('set_pinned_favorites_cmd', { paths: newOrder });
    await loadSidebar();
  } catch {
    // ignore
  }
}

async function openWelcome(options = {}) {
  const { shouldRecordHistory = true } = options;
  await cancelActiveRequest();
  try {
    await invoke('stop_dir_watch_cmd');
  } catch {
    // Ignore watcher stop errors.
  }
  showWelcome.value = true;
  currentPath.value = FM_WELCOME;
  entries.value = [];
  selectedPaths.value = [];
  loading.value = false;
  if (shouldRecordHistory) {
    recordHistory(FM_WELCOME);
  }
}

function focusAddressBar() {
  toolbarRef.value?.startAddressEditing?.();
}

function onAddressBarOpenFailed(message) {
  console.warn('[Address bar] Open failed:', message);
}

function onNewItemClick(event) {
  invoke('show_new_item_menu_cmd', { x: event.clientX, y: event.clientY });
}

async function startCreateFileDraft() {
  if (showWelcome.value || !currentPath.value) return;
  if (entries.value.some((entry) => entry.draft)) return;

  const draftPath = createDraftPath('file');
  entries.value.push({
    path: draftPath,
    name: 'New File',
    is_dir: false,
    size: 0,
    modified_ms: null,
    ext: '',
    hidden: false,
    draft: true,
    draft_kind: 'file'
  });

  await nextTick();
  mainContentRef.value?.startDraftRename?.(draftPath);
}

async function startCreateFolderDraft() {
  if (showWelcome.value || !currentPath.value) return;
  if (entries.value.some((entry) => entry.draft)) return;

  const draftPath = createDraftPath('dir');
  entries.value.push({
    path: draftPath,
    name: 'New Folder',
    is_dir: true,
    size: null,
    modified_ms: null,
    ext: '',
    hidden: false,
    draft: true,
    draft_kind: 'dir'
  });

  await nextTick();
  mainContentRef.value?.startDraftRename?.(draftPath);
}

function removeDraftEntry(path) {
  if (!path) return;
  entries.value = entries.value.filter((entry) => entry.path !== path);
}

function navigateBack() {
  if (historyIndex.value <= 0) return;
  historyIndex.value -= 1;
  const path = pathHistory.value[historyIndex.value];
  navigateTo(path, { shouldRecordHistory: false });
}

function navigateForward() {
  if (historyIndex.value >= pathHistory.value.length - 1) return;
  historyIndex.value += 1;
  const path = pathHistory.value[historyIndex.value];
  navigateTo(path, { shouldRecordHistory: false });
}

function navigateUp() {
  if (showWelcome.value) return;
  const path = normalizePath(currentPath.value);
  if (!path) return;
  if (isWelcomePath(path) || isTrashPath(path)) return;

  const isUnixRoot = path === '/';
  const isWinDrive = /^[A-Za-z]:$/.test(path);
  if (isUnixRoot || isWinDrive) return;

  const parts = path.split('/').filter(Boolean);
  if (path.startsWith('/')) {
    if (parts.length <= 1) {
      navigateTo('/');
      return;
    }
    navigateTo(`/${parts.slice(0, -1).join('/')}`);
    return;
  }

  if (parts.length <= 1) return;
  navigateTo(parts.slice(0, -1).join('/'));
}

function toggleHiddenFiles() {
  setShowHidden(!showHidden.value);
}

async function setShowHidden(value) {
  showHidden.value = value;
  await persistGlobalPrefs();
  if (currentPath.value) {
    navigateTo(currentPath.value, {
      shouldRecordHistory: false,
      includeHidden: showHidden.value
    });
  }
}

function refreshCurrentView() {
  if (!currentPath.value) return;
  if (isWelcomePath(currentPath.value)) return;
  navigateTo(currentPath.value, {
    shouldRecordHistory: false,
    includeHidden: showHidden.value
  });
}

async function setShowExtensions(value) {
  showExtensions.value = value;
  await persistGlobalPrefs();
}

async function setShowSelectionCheckboxes(value) {
  showSelectionCheckboxes.value = value;
  await persistGlobalPrefs();
}

async function setSortBy(value) {
  sortBy.value = value;
  if (currentPath.value) {
    await setDirectoryPrefs(currentPath.value, { sortBy: value });
  }
}

async function setSortDir(value) {
  sortDir.value = value;
  if (currentPath.value) {
    await setDirectoryPrefs(currentPath.value, { sortDir: value });
  }
}

function selectAll() {
  mainContentRef.value?.selectAllItems?.();
}

function deselectAll() {
  mainContentRef.value?.deselectAllItems?.();
}

function selectInverse() {
  mainContentRef.value?.invertSelection?.();
}

function onSelectionChange(paths) {
  selectedPaths.value = Array.isArray(paths) ? paths : [];
}

let deleteBusy = false;

async function deleteSelected(options = {}) {
  if (deleteBusy) return;
  if (!currentPath.value) return;
  const fromTrashView = isTrashView.value;
  const permanent = !!options.permanent || fromTrashView;
  let targets;
  if (Array.isArray(options.targets) && options.targets.length > 0) {
    targets = options.targets.filter((path) => typeof path === 'string' && path && !isDraftPath(path));
  } else {
    const selectedFromView = mainContentRef.value?.getSelectedPaths?.() || [];
    const cursorPath = mainContentRef.value?.getCursorPath?.() || '';
    const effectiveSelection = selectedFromView.length > 0 ? selectedFromView : selectedPaths.value;
    targets = effectiveSelection.filter((path) => typeof path === 'string' && path && !isDraftPath(path));
    if (targets.length === 0 && cursorPath && !isDraftPath(cursorPath)) {
      targets.push(cursorPath);
    }
  }
  if (targets.length === 0) return;

  deleteBusy = true;
  const { confirm } = await import('@tauri-apps/plugin-dialog');
  const itemWord = targets.length === 1 ? 'item' : 'items';
  const allowed = await confirm(
    fromTrashView
      ? `Permanently delete ${targets.length} ${itemWord} from Trash? This cannot be undone.`
      : permanent
      ? `Permanently delete ${targets.length} ${itemWord}? This cannot be undone.`
      : `Delete ${targets.length} ${itemWord}?`,
    {
      title: permanent ? 'Confirm Permanent Delete' : 'Confirm Delete',
      kind: 'warning',
      okLabel: permanent ? 'Delete Permanently' : 'Delete',
      cancelLabel: 'Cancel'
    }
  );
  if (!allowed) {
    deleteBusy = false;
    return;
  }

  try {
    await invoke('delete_paths_cmd', { paths: targets, permanent });
  } catch (error) {
    console.error('delete failed', error);
    const { message } = await import('@tauri-apps/plugin-dialog');
    const text = typeof error === 'string' ? error : (error?.message ?? String(error));
    await message(text, { title: 'Delete failed', kind: 'error' });
  } finally {
    deleteBusy = false;
    await navigateTo(currentPath.value, {
      shouldRecordHistory: false,
      includeHidden: showHidden.value
    });
  }
}

async function emptyTrash() {
  if (!isTrashView.value) return;
  const { confirm } = await import('@tauri-apps/plugin-dialog');
  const allowed = await confirm('Permanently delete all items in Trash? This cannot be undone.', {
    title: 'Empty Trash',
    kind: 'warning',
    okLabel: 'Empty Trash',
    cancelLabel: 'Cancel'
  });
  if (!allowed) return;

  try {
    await invoke('empty_trash_cmd');
  } catch (error) {
    console.error('empty trash failed', error);
  } finally {
    await navigateTo(FM_TRASH, {
      shouldRecordHistory: false
    });
  }
}

async function onViewModeChange(mode) {
  viewMode.value = mode;
  if (currentPath.value) {
    await setDirectoryPrefs(currentPath.value, { viewMode: mode });
  }
}

const DISK_IMAGE_EXTS = ['.dmg', '.iso', '.img', '.cdr', '.toast'];
function isDiskImage(path) {
  const lower = path.toLowerCase();
  return DISK_IMAGE_EXTS.some((ext) => lower.endsWith(ext));
}

async function openFile(path) {
  if (isDiskImage(path)) {
    try {
      const mountPath = await invoke('mount_disk_image_cmd', { path });
      if (mountPath) { navigateTo(mountPath); return; }
    } catch {
      // fall through to regular open
    }
  }
  try {
    await invoke('open_path_cmd', { path });
  } catch {
    // ignore
  }
}

async function renameEntry(path, newName) {
  const renamedPath = await invoke('rename_path_cmd', { path, newName });
  if (currentPath.value) {
    await navigateTo(currentPath.value, {
      shouldRecordHistory: false,
      includeHidden: showHidden.value
    });
  }
  return renamedPath;
}

async function createFile(dirPath, fileName) {
  const createdPath = await invoke('create_empty_file_cmd', { dirPath, fileName });
  if (currentPath.value) {
    await navigateTo(currentPath.value, {
      shouldRecordHistory: false,
      includeHidden: showHidden.value
    });
  }
  return createdPath;
}

async function createFolder(dirPath, dirName) {
  const createdPath = await invoke('create_directory_cmd', { dirPath, dirName });
  if (currentPath.value) {
    await navigateTo(currentPath.value, {
      shouldRecordHistory: false,
      includeHidden: showHidden.value
    });
  }
  return createdPath;
}

async function loadSidebar() {
  try {
    const pinnedPaths = await invoke('get_pinned_favorites_cmd');
    const sections = await invoke('get_sidebar_cmd', { pinnedPaths });
    sidebarSections.value = sections;
  } catch {
    sidebarSections.value = [];
  }
  loading.value = false;
}

async function loadPreferences() {
  const prefs = await loadGlobalPrefs();
  showHidden.value = !!prefs.showHidden;
  showExtensions.value = prefs.showExtensions !== false;
  showSelectionCheckboxes.value = !!prefs.showSelectionCheckboxes;
  if (typeof prefs.gridZoom === 'number') gridZoom.value = prefs.gridZoom;
  manualPathHistory.value = await loadManualPathHistory();
}

async function persistGlobalPrefs() {
  await saveGlobalPrefs({
    showHidden: showHidden.value,
    showExtensions: showExtensions.value,
    showSelectionCheckboxes: showSelectionCheckboxes.value,
    gridZoom: gridZoom.value
  });
}

function onCut() {
  if (!selectedPaths.value.length) return;
  clipboardPaths.value = selectedPaths.value.slice();
  clipboardOp.value = 'cut';
}

function onCopy() {
  if (!selectedPaths.value.length) return;
  clipboardPaths.value = selectedPaths.value.slice();
  clipboardOp.value = 'copy';
}

async function onPaste() {
  if (!clipboardPaths.value.length || showWelcome.value || isTrashView.value) return;
  const op = clipboardOp.value === 'cut' ? 'move' : 'copy';
  const total = clipboardPaths.value.length;
  try {
    const jobId = await paste(clipboardPaths.value, currentPath.value, op);
    transferJobs.value = [
      ...transferJobs.value,
      {
        id: jobId,
        op,
        done: 0,
        total,
        bytes_done: 0,
        bytes_total: 0,
        current: '',
        paused: false
      }
    ];
  } catch (e) {
    console.error('paste failed', e);
  }
  if (op === 'move') {
    clipboardPaths.value = [];
    clipboardOp.value = '';
  }
}

function onCancelTransfer(jobId) {
  cancelTransfer(jobId);
}

function onPauseTransfer(jobId) {
  const job = transferJobs.value.find((j) => j.id === jobId);
  if (job) job.paused = true;
  pauseTransfer(jobId);
}

function onResumeTransfer(jobId) {
  const job = transferJobs.value.find((j) => j.id === jobId);
  if (job) job.paused = false;
  resumeTransfer(jobId);
}

async function rebuildAppMenu() {
  const volumes = [];
  const homeDirs = [];
  for (const section of sidebarSections.value) {
    for (const item of section.items) {
      if (item.kind === 'device' || item.kind === 'device_removable')
        volumes.push({ label: item.label, path: item.path });
      if (item.kind === 'home' || item.kind === 'folder')
        homeDirs.push({ label: item.label, path: item.path });
    }
  }
  const seen = new Set();
  const recentHistory = [...pathHistory.value].reverse().filter(p => {
    if (seen.has(p)) return false;
    seen.add(p);
    return true;
  }).slice(0, 5);
  try {
    await invoke('rebuild_app_menu_cmd', { params: {
      view_mode:       viewMode.value,
      show_hidden:     showHidden.value,
      show_extensions: showExtensions.value,
      show_checkboxes: showSelectionCheckboxes.value,
      sort_by:         sortBy.value,
      sort_dir:        sortDir.value,
      can_go_back:     historyIndex.value > 0,
      can_go_forward:  historyIndex.value < pathHistory.value.length - 1,
      can_go_up:       !showWelcome.value && !!currentPath.value && !isWelcomePath(currentPath.value),
      history_paths:   recentHistory,
      volumes,
      home_dirs:       homeDirs,
      has_selection:   selectedPaths.value.length > 0,
      has_clipboard:   clipboardPaths.value.length > 0,
    }});
  } catch { /* non-macOS or early init */ }
}

async function onQuicklook(path) {
  await invoke('open_quicklook_cmd', { path });
  quicklookOpen.value = true;
}

function onCursorChanged(path) {
  if (path) emit('fm://quicklook-navigate', path);
}

async function hookEvents() {
  const unlistenMenu = await listen('fm://address-menu', async (event) => {
    if (event.payload === 'copy') {
      await writeText(currentPath.value);
    }
    if (event.payload === 'clear') {
      pathHistory.value = [];
      historyIndex.value = -1;
      manualPathHistory.value = [];
      await clearManualPathHistory();
    }
  });

  const unlistenChunk = await listen('fm://dir-chunk', (event) => {
    const chunk = event.payload;
    if (!chunk) return;

    const requestId = chunk.request_id ?? chunk.requestId;
    const rootPath = normalizePath(chunk.root_path ?? chunk.rootPath ?? '');
    const targetPath = normalizePath(currentPath.value);

    if (rootPath && rootPath !== targetPath) return;

    if (activeRequestId.value === null && typeof requestId === 'number') {
      activeRequestId.value = requestId;
    }

    if (requestId !== activeRequestId.value) return;

    entries.value.push(...chunk.entries);
    if (chunk.done || chunk.error) {
      loading.value = false;
      activeRequestId.value = null;
    }
  });

  const unlistenDirChanged = await listen('fm://dir-changed', (event) => {
    const watchedPath = normalizePath(event.payload || '');
    const current = normalizePath(currentPath.value);
    if (!watchedPath || !current) return;
    if (isWelcomePath(current) || isTrashPath(current)) return;
    if (watchedPath !== current) return;

    if (directoryWatchDebounceTimer) {
      window.clearTimeout(directoryWatchDebounceTimer);
      directoryWatchDebounceTimer = 0;
    }
    directoryWatchDebounceTimer = window.setTimeout(() => {
      directoryWatchDebounceTimer = 0;
      refreshCurrentView();
    }, 180);
  });

  const unlistenContextInfo = await listenFileContextMenu((payload) => {
    const { action, kind, paths = [] } = payload;
    if (!action) return;
    const singlePath = paths[0] ?? null;
    if (action === 'show_in_folder') {
      if (!singlePath) return;
      const sep = singlePath.includes('/') ? '/' : '\\';
      const parent = singlePath.split(sep).slice(0, -1).join(sep) || sep;
      navigateTo(parent);
      return;
    } else if (action === 'open') {
      if (!singlePath) return;
      if (kind === 'dir' || kind === 'sidebar_item') navigateTo(singlePath);
      else void openFile(singlePath);
    } else if (action === 'browse') {
      if (singlePath) navigateTo(singlePath);
    } else if (action === 'new_folder') {
      startCreateFolderDraft();
    } else if (action === 'new_file') {
      startCreateFileDraft();
    } else if (action === 'refresh') {
      refreshCurrentView();
    } else if (action === 'delete') {
      void deleteSelected(paths.length > 0 ? { targets: paths } : {});
    } else if (action === 'rename') {
      if (singlePath) mainContentRef.value?.startDraftRename?.(singlePath);
    } else if (action === 'cut') {
      clipboardPaths.value = paths.slice();
      clipboardOp.value = 'cut';
    } else if (action === 'copy') {
      clipboardPaths.value = paths.slice();
      clipboardOp.value = 'copy';
    } else if (action === 'paste') {
      void onPaste();
    } else if (action === 'properties') {
      const found = paths.map((p) => {
        const existing = entries.value.find((e) => e.path === p);
        if (existing) return existing;
        // Synthesize for paths outside the current listing (e.g. sidebar items)
        if (!p) return null;
        const sep = p.includes('\\') ? '\\' : '/';
        const name = p.split(sep).filter(Boolean).pop() || p;
        const isDir = kind === 'dir' || kind === 'sidebar_item' || kind === 'dirs';
        return { path: p, name, is_dir: isDir, ext: null, size: null, modified_ms: null };
      }).filter(Boolean);
      propertiesEntries.value = found;
    } else if (action === 'open_terminal') {
      const target = singlePath ?? currentPath.value;
      if (target) void invoke('open_in_terminal_cmd', { path: target });
    } else if (action === 'edit') {
      if (singlePath) void invoke('open_editor_cmd', { path: singlePath });
    } else if (action === 'mount') {
      if (singlePath) {
        invoke('mount_disk_image_cmd', { path: singlePath })
          .then((mountPath) => { if (mountPath) navigateTo(mountPath); })
          .catch((err) => console.error('mount failed', err));
      }
    } else if (action === 'extract') {
      if (singlePath) void invoke('open_extract_dialog_cmd', { path: singlePath });
    } else if (action === 'archive') {
      void invoke('open_archive_dialog_cmd', { paths, destDir: currentPath.value ?? '', selectionKind: kind });
    } else if (action === 'pin_to_favorites' && singlePath) {
      void invoke('add_pinned_favorite_cmd', { path: singlePath })
        .then(() => loadSidebar())
        .catch((err) => console.error('add_pinned_favorite_cmd failed', err));
    } else if (action === 'unpin_from_favorites' && singlePath) {
      void invoke('remove_pinned_favorite_cmd', { path: singlePath })
        .then(() => loadSidebar())
        .catch((err) => console.error('remove_pinned_favorite_cmd failed', err));
    }
  });

  const unlistenProgress = await listenTransferProgress((p) => {
    const jobId = p.job_id ?? p.jobId;
    if (jobId == null) return;
    const job = transferJobs.value.find((j) => j.id === jobId);
    if (job) {
      job.op = p.op ?? job.op;
      job.done = p.done ?? 0;
      job.total = p.total ?? job.total;
      job.bytes_done = p.bytes_done ?? 0;
      job.bytes_total = p.bytes_total ?? job.bytes_total;
      job.current = p.current ?? '';
    }
  });
  const unlistenDone = await listenTransferDone((d) => {
    const jobId = d.job_id ?? d.jobId;
    if (jobId != null) {
      transferJobs.value = transferJobs.value.filter((j) => j.id !== jobId);
    }
    if (!d.cancelled) refreshCurrentView();
    if (d.errors && d.errors.length) console.error('Transfer errors:', d.errors);
  });

  const unlistenDisksChanged = await listen('fm://disks-changed', async () => {
    await loadSidebar();
    // If the current directory no longer exists (e.g. Finder unmounted a volume
    // we were browsing), redirect to home rather than showing an empty view.
    if (currentPath.value && currentPath.value !== FM_WELCOME) {
      const valid = await invoke('is_valid_dir_cmd', { path: currentPath.value }).catch(() => false);
      if (!valid) {
        const home = sidebarHomePath();
        if (home) navigateTo(home, { shouldRecordHistory: false });
      }
    }
  });

  const unlistenNewItem = await listen('fm://new-item-menu', (event) => {
    if (event.payload === 'file') startCreateFileDraft();
    else if (event.payload === 'folder') startCreateFolderDraft();
  });

  const unlistenMenuAction = await listen('fm://menu-action', ({ payload }) => {
    if      (payload === 'cut')               onCut();
    else if (payload === 'copy')              onCopy();
    else if (payload === 'paste')             onPaste();
    else if (payload === 'select_all')        selectAll();
    else if (payload === 'invert_selection')  selectInverse();
    else if (payload === 'clear_selection')   deselectAll();
    else if (payload === 'history_back')      navigateBack();
    else if (payload === 'history_forward')   navigateForward();
    else if (payload === 'history_up')        navigateUp();
    else if (payload === 'focus_address')     focusAddressBar();
    else if (payload === 'toggle_hidden')     setShowHidden(!showHidden.value);
    else if (payload === 'toggle_extensions') showExtensions.value = !showExtensions.value;
    else if (payload === 'toggle_checkboxes') showSelectionCheckboxes.value = !showSelectionCheckboxes.value;
    else if (payload.startsWith('view_mode:'))  viewMode.value = payload.slice(10);
    else if (payload.startsWith('sort_by:'))    sortBy.value = payload.slice(8);
    else if (payload.startsWith('sort_dir:'))   sortDir.value = payload.slice(9);
    else if (payload.startsWith('navigate:'))   navigateTo(payload.slice(9));
  });

  const unlistenIndexProgress = await listen('fm://index-progress', ({ payload }) => {
    indexing.value = true;
    indexDone.value = payload.done ?? 0;
  });
  const unlistenIndexDone = await listen('fm://index-done', () => {
    indexing.value = false;
  });


  unlistenFns.push(unlistenMenu, unlistenChunk, unlistenDirChanged, unlistenContextInfo, unlistenProgress, unlistenDone, unlistenDisksChanged, unlistenNewItem, unlistenMenuAction, unlistenIndexProgress, unlistenIndexDone);
}

// Keys that have no meaning in a file manager but trigger browser defaults.
// Checked in capture phase so preventDefault() fires before any handler sees them.
// Our own handlers (keybindings.js, MainContent) still receive the event normally.
const BLOCKED_CTRL_KEYS = new Set([
  'f',  // find bar
  'p',  // print
  's',  // save page
  'u',  // view source
  'l',  // browser address bar  (we use F6)
  'd',  // bookmark
  't',  // new tab
  'n',  // new window           (Ctrl+Shift+N = our New Folder — not affected)
  'w',  // close tab
  'j',  // downloads (Chrome/Edge)
  'g',  // find next
  'k',  // search bar (Firefox)
  'e',  // search bar (Chrome)
]);

function blockBrowserShortcuts(event) {
  const ctrl = event.ctrlKey || event.metaKey;
  const key  = event.key.toLowerCase();

  // F11 — fullscreen toggle
  if (!ctrl && !event.shiftKey && !event.altKey && event.key === 'F11') {
    event.preventDefault();
    return;
  }

  // F1 — browser help
  if (!ctrl && !event.shiftKey && !event.altKey && event.key === 'F1') {
    event.preventDefault();
    return;
  }

  if (!ctrl || event.altKey) return;

  // Ctrl-only shortcuts to block (shift must NOT be held, so Ctrl+Shift+N stays ours)
  if (!event.shiftKey && BLOCKED_CTRL_KEYS.has(key)) {
    event.preventDefault();
    return;
  }

  // Browser zoom: Ctrl+- / Ctrl+= / Ctrl+0
  if (!event.shiftKey && (key === '-' || key === '=' || key === '0')) {
    event.preventDefault();
  }
}

let titlebarPendingDrag = false;

function onTitlebarMousedown(event) {
  if (event.button !== 0) return;
  if (event.target.closest('button, a, input, select, [role="button"]')) return;
  const win = getCurrentWindow();
  if (event.detail === 2) {
    void win.isMaximized().then((max) => (max ? win.unmaximize() : win.maximize()));
    return;
  }
  titlebarPendingDrag = true;
}

function onTitlebarMousemove() {
  if (!titlebarPendingDrag) return;
  titlebarPendingDrag = false;
  void getCurrentWindow().startDragging();
}

function onTitlebarMouseup() {
  titlebarPendingDrag = false;
}

function onAppKeyDown(event) {
  if (event.key === 'F12' || (event.altKey && event.metaKey && event.key === 'i')) {
    invoke('open_devtools_cmd');
    event.preventDefault();
    return;
  }
  if (event.key !== 'Escape') return;
  if (propertiesEntries.value.length) {
    propertiesEntries.value = [];
    event.preventDefault();
    return;
  }
  if (selectedPaths.value.length > 0) {
    mainContentRef.value?.deselectAllItems?.();
    event.preventDefault();
  }
}

function onShowProperties(paths) {
  const found = paths.map((p) => entries.value.find((e) => e.path === p)).filter(Boolean);
  if (found.length > 0) propertiesEntries.value = found;
}

// ── Grid zoom (Ctrl+scroll) ───────────────────────────────────────────────────
const GRID_ZOOM_MIN = 80;
const GRID_ZOOM_MAX = 220;
const GRID_ZOOM_STEP = 16;

let gridZoomSaveTimer = 0;
function scheduleGridZoomSave() {
  if (gridZoomSaveTimer) clearTimeout(gridZoomSaveTimer);
  gridZoomSaveTimer = setTimeout(persistGlobalPrefs, 800);
}

function onGridZoomWheel(e) {
  if (!e.ctrlKey || viewMode.value !== 'grid') return;
  e.preventDefault();
  const dir = e.deltaY < 0 ? 1 : -1; // scroll up = zoom in
  gridZoom.value = Math.min(GRID_ZOOM_MAX, Math.max(GRID_ZOOM_MIN, gridZoom.value + dir * GRID_ZOOM_STEP));
  scheduleGridZoomSave();
}
// ─────────────────────────────────────────────────────────────────────────────

onMounted(async () => {
  document.documentElement.style.setProperty('--sidebar-width', `${sidebarWidth.value}px`);
  window.addEventListener('wheel', onGridZoomWheel, { passive: false });
  window.addEventListener('keydown', blockBrowserShortcuts, { capture: true });
  window.addEventListener('keydown', onAppKeyDown);
  window.addEventListener('mousemove', onTitlebarMousemove);
  window.addEventListener('mouseup', onTitlebarMouseup);
  try {
    await bootstrapPreferencesStore();
  } catch (error) {
    console.error('preferences store bootstrap failed', error);
  }
  await loadPreferences();
  teardownKeybindings = setupKeybindings({
    onToggleHidden: toggleHiddenFiles,
    onNavigateBack: navigateBack,
    onNavigateForward: navigateForward,
    onNewFolder: startCreateFolderDraft,
    onFocusAddressBar: () => navigateTo(FM_SEARCH),
    onRefresh: refreshCurrentView,
    onCut,
    onCopy,
    onPaste
  });
  await hookEvents();
  await loadSidebar();
  await rebuildAppMenu();

  watch(
    [viewMode, showHidden, showExtensions, showSelectionCheckboxes,
     sortBy, sortDir, historyIndex, pathHistory, sidebarSections,
     selectedPaths, clipboardPaths],
    rebuildAppMenu,
    { deep: true }
  );
});

onBeforeUnmount(async () => {
  window.removeEventListener('wheel', onGridZoomWheel);
  window.removeEventListener('keydown', blockBrowserShortcuts, { capture: true });
  window.removeEventListener('keydown', onAppKeyDown);
  window.removeEventListener('mousemove', onTitlebarMousemove);
  window.removeEventListener('mouseup', onTitlebarMouseup);
  if (gridZoomSaveTimer) clearTimeout(gridZoomSaveTimer);
  resizing = false;
  if (directoryWatchDebounceTimer) {
    window.clearTimeout(directoryWatchDebounceTimer);
    directoryWatchDebounceTimer = 0;
  }
  teardownKeybindings();
  for (const unlisten of unlistenFns) {
    unlisten();
  }
  try {
    await invoke('stop_dir_watch_cmd');
  } catch {
    // Ignore watcher stop errors.
  }
  await cancelActiveRequest();
});
</script>
