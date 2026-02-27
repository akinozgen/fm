<template>
  <div class="app-shell">
    <Sidebar
      :sections="sidebarSections"
      :current-path="currentPath"
      @resize-start="startResize"
      @navigate="navigateTo"
      @go-welcome="openWelcome"
      @quick-add="startCreateFileDraft"
    />
    <main class="main">
      <Toolbar
        ref="toolbarRef"
        :current-path="currentPath"
        :manual-history="manualPathHistory"
        :view-mode="viewMode"
        @update:view-mode="onViewModeChange"
        @navigate-up="navigateUp"
        @navigate-back="navigateBack"
        @navigate-forward="navigateForward"
        @navigate-path="navigateTo"
        @navigate-path-manual="navigateToFromManual"
        @delete-manual-history="deleteManualHistoryPath"
      />
      <ActionToolbar
        v-if="!showWelcome"
        :show-hidden="showHidden"
        :show-extensions="showExtensions"
        :show-selection-checkboxes="showSelectionCheckboxes"
        :on-delete="deleteSelected"
        :on-empty-trash="emptyTrash"
        :is-trash-view="isTrashView"
        :can-empty-trash="canEmptyTrash"
        @update:show-hidden="setShowHidden"
        @update:show-extensions="setShowExtensions"
        @update:show-selection-checkboxes="setShowSelectionCheckboxes"
        @select-all="selectAll"
        @deselect-all="deselectAll"
        @select-inverse="selectInverse"
      />
      <WelcomePage
        v-if="showWelcome"
        :sections="sidebarSections"
        @navigate="navigateTo"
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
        @open-dir="navigateTo"
        @open-file="openFile"
        @selection-change="onSelectionChange"
        @remove-draft="removeDraftEntry"
      />
      <StatusBar
        :shown-count="sortedEntries.length"
        :selected-count="selectedEntries.length"
        :show-selected-size="showSelectionSize"
        :selected-size-bytes="selectionSizeBytes"
      />
    </main>
  </div>
</template>

<script setup>
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from 'vue';
import ActionToolbar from './components/ActionToolbar.vue';
import MainContent from './components/MainContent.vue';
import StatusBar from './components/StatusBar.vue';
import {
  getDirectoryPrefs,
  loadGlobalPrefs,
  loadManualPathHistory,
  removeManualPath,
  pushManualPath,
  saveGlobalPrefs,
  setDirectoryPrefs
} from './lib/appPreferences';
import { clearThumbnailQueue } from './lib/iconCache';
import { listenFileContextMenu } from './lib/contextMenu';
import { setupKeybindings } from './lib/keybindings';
import { bootstrapPreferencesStore } from './lib/preferencesStore';
import {
  FM_TRASH,
  FM_WELCOME,
  canonicalizePath,
  createDraftPath,
  isDraftPath,
  isTrashPath,
  isWelcomePath,
  normalizePath
} from './lib/virtualPaths';
import Sidebar from './components/Sidebar.vue';
import Toolbar from './components/Toolbar.vue';
import WelcomePage from './components/WelcomePage.vue';

const viewMode = ref('list');
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
const selectedPaths = ref([]);

const pathHistory = ref([]);
const historyIndex = ref(-1);
const unlistenFns = [];

let resizing = false;
let teardownKeybindings = () => {};
let directoryWatchDebounceTimer = 0;

const sortedEntries = computed(() => {
  return [...entries.value].sort((a, b) => {
    if (!!a.draft !== !!b.draft) {
      return a.draft ? -1 : 1;
    }
    if (a.is_dir !== b.is_dir) {
      return a.is_dir ? -1 : 1;
    }
    return a.name.localeCompare(b.name);
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

  await cancelActiveRequest();

  showWelcome.value = false;
  currentPath.value = nextPath;
  entries.value = [];
  clearThumbnailQueue();
  selectedPaths.value = [];
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

async function deleteSelected(options = {}) {
  if (!currentPath.value) return;
  const fromTrashView = isTrashView.value;
  const permanent = !!options.permanent || fromTrashView;
  const selectedFromView = mainContentRef.value?.getSelectedPaths?.() || [];
  const cursorPath = mainContentRef.value?.getCursorPath?.() || '';
  const effectiveSelection = selectedFromView.length > 0 ? selectedFromView : selectedPaths.value;
  const targets = effectiveSelection.filter((path) => typeof path === 'string' && path && !isDraftPath(path));
  if (targets.length === 0 && cursorPath && !isDraftPath(cursorPath)) {
    targets.push(cursorPath);
  }
  if (targets.length === 0) return;

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
  if (!allowed) return;

  try {
    await invoke('delete_paths_cmd', { paths: targets, permanent });
  } catch (error) {
    console.error('delete failed', error);
  } finally {
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

async function openFile(path) {
  try {
    await invoke('open_path_cmd', { path });
  } catch {
    // Ignore open errors for now; UI feedback can be added later.
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
    const sections = await invoke('get_sidebar_cmd');
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

async function hookEvents() {
  const unlistenMenu = await listen('fm://address-menu', async (event) => {
    if (event.payload === 'copy') {
      await writeText(currentPath.value);
    }
    if (event.payload === 'clear') {
      pathHistory.value = [];
      historyIndex.value = -1;
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
    if (!payload.kind) return;
    console.info('file context menu click', payload);
  });

  unlistenFns.push(unlistenMenu, unlistenChunk, unlistenDirChanged, unlistenContextInfo);
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
    onFocusAddressBar: () => toolbarRef.value?.startAddressEditing?.(),
    onRefresh: refreshCurrentView
  });
  await hookEvents();
  await loadSidebar();
});

onBeforeUnmount(async () => {
  window.removeEventListener('wheel', onGridZoomWheel);
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
