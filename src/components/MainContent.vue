<template>
  <div
    class="content"
    ref="contentRef"
    @click="clearSelection"
    @pointerdown="onContentPointerDown"
    @pointermove="onContentPointerMove"
    @pointerup="onContentPointerUp"
    @pointercancel="onContentPointerCancel"
    @contextmenu.prevent="onContentContextMenu"
    :class="{ selecting: isDragging }"
  >
    <div v-if="loading" class="status-line">Loading...</div>
    <div v-else-if="entries.length === 0" class="status-line">Empty folder</div>

    <section v-if="viewMode === 'grid'" class="grid">
      <button
        v-for="(entry, index) in entries"
        :key="entry.path"
        class="tile"
        :data-path="entry.path"
        :class="{ folder: entry.is_dir, file: !entry.is_dir, selected: isSelected(entry.path), cursor: isCursor(index), 'with-checkbox': showSelectionCheckboxes }"
        :title="truncatedTitle(entry.name, 24, entry.is_dir)"
        @click.stop="handleItemClick(entry, index, $event)"
        @dblclick.stop="openEntry(entry)"
      >
        <input
          v-if="showSelectionCheckboxes"
          class="item-checkbox grid-checkbox"
          type="checkbox"
          :checked="isSelected(entry.path)"
          @click.stop="toggleCheckbox(entry.path, index)"
        />
        <FileIcon :path="entry.path" :is-dir="entry.is_dir" :size="48" />
        <input
          v-if="isRenaming(entry.path)"
          :ref="setRenameInputRef"
          class="rename-input grid-rename-input"
          type="text"
          v-model="renameValue"
          @mousedown.stop
          @click.stop
          @dblclick.stop
          @keydown.enter.stop.prevent="submitRename(entry)"
          @keydown.esc.stop.prevent="cancelRename"
          @blur="submitRename(entry)"
        />
        <span
          v-else
          class="entry-name"
          :title="entry.name"
          @click.stop="onEntryNameClick(entry, index, $event)"
        >{{ displayName(entry.name, 24, entry.is_dir, showExtensions) }}</span>
      </button>
    </section>

    <section v-else class="list">
      <button
        v-for="(entry, index) in entries"
        :key="entry.path"
        class="list-row"
        :data-path="entry.path"
        :class="{ folder: entry.is_dir, file: !entry.is_dir, selected: isSelected(entry.path), cursor: isCursor(index), 'with-checkbox': showSelectionCheckboxes }"
        :title="truncatedTitle(entry.name, 52, entry.is_dir)"
        @click.stop="handleItemClick(entry, index, $event)"
        @dblclick.stop="openEntry(entry)"
      >
        <input
          v-if="showSelectionCheckboxes"
          class="item-checkbox"
          type="checkbox"
          :checked="isSelected(entry.path)"
          @click.stop="toggleCheckbox(entry.path, index)"
        />
        <FileIcon :path="entry.path" :is-dir="entry.is_dir" :size="16" />
        <input
          v-if="isRenaming(entry.path)"
          :ref="setRenameInputRef"
          class="rename-input list-rename-input"
          type="text"
          v-model="renameValue"
          @mousedown.stop
          @click.stop
          @dblclick.stop
          @keydown.enter.stop.prevent="submitRename(entry)"
          @keydown.esc.stop.prevent="cancelRename"
          @blur="submitRename(entry)"
        />
        <span
          v-else
          class="name"
          :title="entry.name"
          @click.stop="onEntryNameClick(entry, index, $event)"
        >{{ displayName(entry.name, 52, entry.is_dir, showExtensions) }}</span>
        <span class="meta">{{ entry.is_dir ? 'Folder' : (entry.ext || 'File').toUpperCase() }}</span>
        <span class="meta">{{ formatSize(entry.size) }}</span>
        <span class="meta">{{ formatModified(entry.modified_ms) }}</span>
      </button>
    </section>

    <div ref="selectionRectRef" class="selection-rect"></div>
  </div>
</template>

<script setup>
import { nextTick, onBeforeUnmount, onMounted, ref, toRefs, watch } from 'vue';
import { showNativeFileContextMenu } from '../lib/contextMenu';
import { resolveFileContextTarget } from '../lib/contextResolver';
import FileIcon from './FileIcon.vue';

const props = defineProps({
  currentPath: {
    type: String,
    required: true
  },
  viewMode: {
    type: String,
    required: true
  },
  entries: {
    type: Array,
    required: true
  },
  loading: {
    type: Boolean,
    required: true
  },
  showExtensions: {
    type: Boolean,
    required: true
  },
  showSelectionCheckboxes: {
    type: Boolean,
    required: true
  },
  renameEntry: {
    type: Function,
    required: true
  },
  createFile: {
    type: Function,
    required: true
  },
  createFolder: {
    type: Function,
    required: true
  },
  requestDelete: {
    type: Function,
    required: true
  }
});
const { viewMode, entries, loading, showExtensions, showSelectionCheckboxes } = toRefs(props);

const emit = defineEmits(['open-dir', 'open-file', 'selection-change', 'remove-draft']);
const selectedPaths = ref(new Set());
const anchorIndex = ref(null);
const cursorIndex = ref(-1);
const contentRef = ref(null);
const selectionRectRef = ref(null);
const isDragging = ref(false);
const renamingPath = ref('');
const renameValue = ref('');
const renameInputRef = ref(null);
const lastSelectionChangeAt = ref(0);

const drag = {
  active: false,
  pointerId: -1,
  startX: 0,
  startY: 0,
  currentX: 0,
  currentY: 0,
  hadSelectionDrag: false,
  additive: false,
  baseSelection: new Set()
};

let frameId = 0;
let pendingRect = null;
let selectableRects = [];
const suppressNextContentClick = ref(false);
const typeBuffer = ref('');
let typeBufferTimer = 0;

function isSelected(path) {
  return selectedPaths.value.has(path);
}

function emitSelectionChange() {
  emit('selection-change', Array.from(selectedPaths.value));
}

function clearSelection() {
  if (suppressNextContentClick.value) {
    suppressNextContentClick.value = false;
    return;
  }
  selectedPaths.value = new Set();
  anchorIndex.value = null;
  cursorIndex.value = -1;
}

function handleItemClick(entry, index, event) {
  cursorIndex.value = index;
  const additive = event.ctrlKey || event.metaKey;
  const range = event.shiftKey;

  if (range) {
    if (anchorIndex.value === null) {
      selectedPaths.value = new Set([entry.path]);
      anchorIndex.value = index;
      return;
    }

    const start = Math.min(anchorIndex.value, index);
    const end = Math.max(anchorIndex.value, index);
    const rangePaths = entries.value.slice(start, end + 1).map((e) => e.path);

    if (additive) {
      const next = new Set(selectedPaths.value);
      for (const path of rangePaths) next.add(path);
      selectedPaths.value = next;
    } else {
      selectedPaths.value = new Set(rangePaths);
    }
    return;
  }

  if (additive) {
    const next = new Set(selectedPaths.value);
    if (next.has(entry.path)) next.delete(entry.path);
    else next.add(entry.path);
    selectedPaths.value = next;
    anchorIndex.value = index;
    return;
  }

  selectedPaths.value = new Set([entry.path]);
  anchorIndex.value = index;
}

function toggleCheckbox(path, index) {
  cursorIndex.value = index;
  const next = new Set(selectedPaths.value);
  if (next.has(path)) next.delete(path);
  else next.add(path);
  selectedPaths.value = next;
  anchorIndex.value = index;
}

function selectByIndex(index) {
  const entry = entries.value[index];
  if (!entry) return;
  selectedPaths.value = new Set([entry.path]);
  anchorIndex.value = index;
  cursorIndex.value = index;
  scrollItemIntoView(entry.path);
}

function getPrimarySelectionIndex() {
  if (
    anchorIndex.value !== null &&
    anchorIndex.value >= 0 &&
    anchorIndex.value < entries.value.length
  ) {
    const anchored = entries.value[anchorIndex.value];
    if (anchored && selectedPaths.value.has(anchored.path)) {
      return anchorIndex.value;
    }
  }
  for (let i = 0; i < entries.value.length; i += 1) {
    if (selectedPaths.value.has(entries.value[i].path)) return i;
  }
  return -1;
}

function getKeyboardCursorIndex() {
  if (cursorIndex.value >= 0 && cursorIndex.value < entries.value.length) {
    return cursorIndex.value;
  }
  const selectedIndex = getPrimarySelectionIndex();
  if (selectedIndex >= 0) return selectedIndex;
  return entries.value.length > 0 ? 0 : -1;
}

function isCursor(index) {
  return index === cursorIndex.value;
}

function getGridColumnCount() {
  const content = contentRef.value;
  if (!content) return 1;
  const nodes = content.querySelectorAll('.tile[data-path]');
  if (nodes.length <= 1) return 1;
  const firstTop = nodes[0].offsetTop;
  let count = 1;
  for (let i = 1; i < nodes.length; i += 1) {
    if (nodes[i].offsetTop !== firstTop) break;
    count += 1;
  }
  return Math.max(1, count);
}

function scrollItemIntoView(path) {
  const content = contentRef.value;
  if (!content) return;
  const nodes = content.querySelectorAll('.tile[data-path], .list-row[data-path]');
  for (const node of nodes) {
    if (node.getAttribute('data-path') === path) {
      node.scrollIntoView({ block: 'nearest' });
      return;
    }
  }
}

function selectAllItems() {
  selectedPaths.value = new Set(entries.value.map((e) => e.path));
  anchorIndex.value = entries.value.length > 0 ? 0 : null;
}

function deselectAllItems() {
  selectedPaths.value = new Set();
  anchorIndex.value = null;
  cursorIndex.value = -1;
}

function invertSelection() {
  const next = new Set();
  for (const entry of entries.value) {
    if (!selectedPaths.value.has(entry.path)) {
      next.add(entry.path);
    }
  }
  selectedPaths.value = next;
}

function openEntry(entry) {
  if (renamingPath.value) return;
  if (entry.draft) return;
  if (entry.is_dir) {
    emit('open-dir', entry.path);
  } else {
    emit('open-file', entry.path);
  }
}

function isRenaming(path) {
  return renamingPath.value === path;
}

function setRenameInputRef(el) {
  if (el) {
    renameInputRef.value = el;
  }
}

function escapeName(value) {
  return value.replace(/[/\\]/g, '').trim();
}

async function startRename() {
  if (loading.value) return;
  if (renamingPath.value) return;

  const currentIndex = getKeyboardCursorIndex();
  if (currentIndex < 0 || currentIndex >= entries.value.length) return;
  const entry = entries.value[currentIndex];
  if (!entry) return;

  await startRenameForEntry(entry);
}

async function startRenameForEntry(entry) {
  if (!entry || loading.value || renamingPath.value) return;

  renamingPath.value = entry.path;
  renameValue.value = entry.name;
  await nextTick();
  const input = renameInputRef.value;
  if (input && typeof input.focus === 'function') {
    input.focus();
    input.select?.();
  }
}

function onEntryNameClick(entry, index, event) {
  if (renamingPath.value) return;

  const hasModifier = event.ctrlKey || event.metaKey || event.shiftKey || event.altKey;
  if (hasModifier) {
    handleItemClick(entry, index, event);
    return;
  }

  const selected = isSelected(entry.path);
  const longEnoughSinceSelection = Date.now() - lastSelectionChangeAt.value >= 650;
  const singleSelection = selectedPaths.value.size === 1;

  if (selected && singleSelection && longEnoughSinceSelection) {
    anchorIndex.value = index;
    cursorIndex.value = index;
    void startRenameForEntry(entry);
    return;
  }

  handleItemClick(entry, index, event);
}

function cancelRename() {
  const draftPath = renamingPath.value;
  const entry = entries.value.find((it) => it.path === draftPath);
  renamingPath.value = '';
  renameValue.value = '';
  renameInputRef.value = null;
  if (entry?.draft) {
    emit('remove-draft', draftPath);
  }
}

async function submitRename(entry) {
  if (!entry || !isRenaming(entry.path)) return;
  const candidate = escapeName(renameValue.value);
  if (!candidate || (!entry.draft && candidate === entry.name)) {
    cancelRename();
    return;
  }

  try {
    const renamedPath = entry.draft
      ? entry.draft_kind === 'dir'
        ? await props.createFolder(props.currentPath, candidate)
        : await props.createFile(props.currentPath, candidate)
      : await props.renameEntry(entry.path, candidate);
    selectedPaths.value = new Set([renamedPath]);
    const idx = entries.value.findIndex((it) => it.path === entry.path);
    if (idx >= 0) {
      anchorIndex.value = idx;
      cursorIndex.value = idx;
    }
  } catch (error) {
    console.error('rename failed', error);
  } finally {
    cancelRename();
  }
}

function submitActiveRename() {
  if (!renamingPath.value) return;
  const entry = entries.value.find((it) => it.path === renamingPath.value);
  if (!entry) {
    cancelRename();
    return;
  }
  void submitRename(entry);
}

function displayName(name, maxTotal, isDir, withExtension) {
  if (!name) return '';
  if (isDir || withExtension === false) {
    if (withExtension === false && !isDir) {
      const withoutExt = stripExtension(name);
      return truncatePlain(withoutExt, maxTotal);
    }
    return truncatePlain(name, maxTotal);
  }

  const lastDot = name.lastIndexOf('.');
  if (lastDot <= 0 || lastDot === name.length - 1) {
    return truncatePlain(name, maxTotal);
  }

  const base = name.slice(0, lastDot);
  const ext = name.slice(lastDot + 1);
  const extPart = `.${ext}`;
  const allowedBase = maxTotal - extPart.length;

  if (allowedBase <= 3) {
    return truncatePlain(name, maxTotal);
  }
  if (base.length <= allowedBase) {
    return name;
  }
  return `${base.slice(0, allowedBase - 1)}…${extPart}`;
}

function truncatedTitle(name, maxTotal, isDir) {
  const withExt = showExtensions.value;
  const source = !isDir && !withExt ? stripExtension(name) : name;
  const shown = displayName(name, maxTotal, isDir, withExt);
  return shown === source ? '' : name;
}

function stripExtension(name) {
  const lastDot = name.lastIndexOf('.');
  if (lastDot <= 0) return name;
  return name.slice(0, lastDot);
}

function truncatePlain(value, max) {
  if (value.length <= max) return value;
  if (max <= 1) return '…';
  return `${value.slice(0, max - 1)}…`;
}

function formatSize(size) {
  if (size === null || size === undefined) return '-';
  if (size < 1024) return `${size} B`;
  if (size < 1024 * 1024) return `${Math.round(size / 1024)} KB`;
  if (size < 1024 * 1024 * 1024) return `${(size / (1024 * 1024)).toFixed(1)} MB`;
  return `${(size / (1024 * 1024 * 1024)).toFixed(1)} GB`;
}

function formatModified(modifiedMs) {
  if (!modifiedMs) return '-';
  const date = new Date(Number(modifiedMs));
  if (Number.isNaN(date.getTime())) return '-';
  return date.toLocaleDateString();
}

function onContentPointerDown(event) {
  if (event.button !== 0 || !event.isPrimary) return;
  const content = contentRef.value;
  if (!content) return;
  if (renamingPath.value && !event.target.closest('.rename-input')) {
    submitActiveRename();
  }
  if (event.target.closest('.tile, .list-row')) return;

  const { x, y } = pointInContent(event);
  drag.additive = event.ctrlKey || event.metaKey;
  drag.baseSelection = new Set(selectedPaths.value);
  if (!drag.additive) {
    selectedPaths.value = new Set();
    anchorIndex.value = null;
  }
  isDragging.value = true;
  drag.active = true;
  drag.pointerId = event.pointerId;
  drag.startX = x;
  drag.startY = y;
  drag.currentX = x;
  drag.currentY = y;
  drag.hadSelectionDrag = false;
  selectableRects = collectSelectableRects();

  content.setPointerCapture(event.pointerId);
  pendingRect = { visible: false, x, y, width: 0, height: 0 };
  scheduleRectPaint();
  event.preventDefault();
}

function collectSelectableRects() {
  const content = contentRef.value;
  if (!content) return [];
  const contentRect = content.getBoundingClientRect();
  const nodes = content.querySelectorAll('.tile[data-path], .list-row[data-path]');
  const result = [];
  for (const node of nodes) {
    const rect = node.getBoundingClientRect();
    const path = node.getAttribute('data-path');
    if (!path) continue;
    result.push({
      path,
      left: rect.left - contentRect.left + content.scrollLeft,
      top: rect.top - contentRect.top + content.scrollTop,
      right: rect.right - contentRect.left + content.scrollLeft,
      bottom: rect.bottom - contentRect.top + content.scrollTop
    });
  }
  return result;
}

function pointInContent(event) {
  const content = contentRef.value;
  const rect = content.getBoundingClientRect();
  return {
    x: event.clientX - rect.left + content.scrollLeft,
    y: event.clientY - rect.top + content.scrollTop
  };
}

function onContentPointerMove(event) {
  if (!drag.active || event.pointerId !== drag.pointerId || !contentRef.value) return;
  const content = contentRef.value;
  const { x, y } = pointInContent(event);
  const minX = content.scrollLeft;
  const minY = content.scrollTop;
  const maxX = content.scrollLeft + content.clientWidth;
  const maxY = content.scrollTop + content.clientHeight;

  drag.currentX = Math.min(maxX, Math.max(minX, x));
  drag.currentY = Math.min(maxY, Math.max(minY, y));

  const left = Math.min(drag.startX, drag.currentX);
  const top = Math.min(drag.startY, drag.currentY);
  const width = Math.abs(drag.currentX - drag.startX);
  const height = Math.abs(drag.currentY - drag.startY);
  pendingRect = {
    visible: width > 2 || height > 2,
    x: left,
    y: top,
    width,
    height
  };
  if (pendingRect.visible) {
    drag.hadSelectionDrag = true;
  }
  scheduleRectPaint();
}

function onContentPointerUp(event) {
  if (!drag.active || event.pointerId !== drag.pointerId) return;
  finishDrag();
}

function onContentPointerCancel(event) {
  if (!drag.active || event.pointerId !== drag.pointerId) return;
  finishDrag();
}

function onContentContextMenu(event) {
  const context = resolveFileContextTarget(event, entries.value);
  void showNativeFileContextMenu({
    x: event.clientX,
    y: event.clientY,
    kind: context.kind,
    path: context.path
  });
}

function onWindowKeyDown(event) {
  const target = event.target;
  if (
    target &&
    (target.tagName === 'INPUT' ||
      target.tagName === 'TEXTAREA' ||
      target.isContentEditable)
  ) {
    return;
  }

  if (renamingPath.value) {
    return;
  }

  if (!event.altKey && !event.shiftKey && (event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'a') {
    selectAllItems();
    event.preventDefault();
    return;
  }

  if (!event.ctrlKey && !event.metaKey && !event.altKey && event.key === 'Delete') {
    event.preventDefault();
    void props.requestDelete({ permanent: !!event.shiftKey });
    return;
  }

  if (!event.ctrlKey && !event.metaKey && !event.altKey && !event.shiftKey && event.key === 'F2') {
    event.preventDefault();
    startRename();
    return;
  }

  const listLen = entries.value.length;
  const supportsArrows =
    event.key === 'ArrowDown' ||
    event.key === 'ArrowUp' ||
    (viewMode.value === 'grid' && (event.key === 'ArrowLeft' || event.key === 'ArrowRight'));

  if (listLen > 0 && supportsArrows && !event.metaKey && !event.altKey) {
    let currentIndex = getKeyboardCursorIndex();
    if (currentIndex < 0) currentIndex = 0;

    let targetIndex = currentIndex;
    if (viewMode.value === 'list') {
      if (event.key === 'ArrowDown') targetIndex = Math.min(listLen - 1, currentIndex + 1);
      else if (event.key === 'ArrowUp') targetIndex = Math.max(0, currentIndex - 1);
    } else if (viewMode.value === 'grid') {
      const columns = getGridColumnCount();
      if (event.key === 'ArrowRight') targetIndex = Math.min(listLen - 1, currentIndex + 1);
      else if (event.key === 'ArrowLeft') targetIndex = Math.max(0, currentIndex - 1);
      else if (event.key === 'ArrowDown') targetIndex = Math.min(listLen - 1, currentIndex + columns);
      else if (event.key === 'ArrowUp') targetIndex = Math.max(0, currentIndex - columns);
    }

    if (event.shiftKey) {
      const baseIndex = anchorIndex.value !== null ? anchorIndex.value : currentIndex;
      const start = Math.min(baseIndex, targetIndex);
      const end = Math.max(baseIndex, targetIndex);
      const rangePaths = entries.value.slice(start, end + 1).map((e) => e.path);

      if (event.ctrlKey) {
        const next = new Set(selectedPaths.value);
        for (const path of rangePaths) next.add(path);
        selectedPaths.value = next;
      } else {
        selectedPaths.value = new Set(rangePaths);
      }

      anchorIndex.value = baseIndex;
      cursorIndex.value = targetIndex;
      const entry = entries.value[targetIndex];
      if (entry) scrollItemIntoView(entry.path);
      event.preventDefault();
      return;
    }

    if (event.ctrlKey) {
      cursorIndex.value = targetIndex;
      const entry = entries.value[targetIndex];
      if (entry) scrollItemIntoView(entry.path);
      event.preventDefault();
      return;
    }

    if (!event.ctrlKey) {
      selectByIndex(targetIndex);
      event.preventDefault();
      return;
    }
  }

  if (!event.metaKey && !event.altKey && !event.shiftKey && (event.key === ' ' || event.code === 'Space')) {
    const index = getKeyboardCursorIndex();
    const entry = entries.value[index];
    if (!entry) return;
    const next = new Set(selectedPaths.value);
    if (next.has(entry.path)) next.delete(entry.path);
    else next.add(entry.path);
    selectedPaths.value = next;
    anchorIndex.value = index;
    cursorIndex.value = index;
    scrollItemIntoView(entry.path);
    event.preventDefault();
    return;
  }

  if (event.key === 'Enter') {
    const selected = Array.from(selectedPaths.value);
    if (selected.length === 0) return;
    const selectedPath = selected[0];
    const entry = entries.value.find((e) => e.path === selectedPath);
    if (!entry) return;
    openEntry(entry);
    event.preventDefault();
    return;
  }

  if (event.ctrlKey || event.metaKey || event.altKey) return;
  if (event.key.length !== 1) return;

  const typed = event.key.toLowerCase();
  if (!typed.trim()) return;

  typeBuffer.value += typed;
  if (typeBufferTimer) window.clearTimeout(typeBufferTimer);
  typeBufferTimer = window.setTimeout(() => {
    typeBuffer.value = '';
    typeBufferTimer = 0;
  }, 900);

  const list = entries.value;
  if (list.length === 0) return;

  const selected = Array.from(selectedPaths.value);
  const currentPath = selected.length > 0 ? selected[0] : '';
  const currentIndex = list.findIndex((e) => e.path === currentPath);
  const startIndex = currentIndex >= 0 ? (currentIndex + 1) % list.length : 0;

  const matches = (name) => name.toLowerCase().startsWith(typeBuffer.value);
  let foundIndex = -1;

  for (let i = 0; i < list.length; i += 1) {
    const idx = (startIndex + i) % list.length;
    if (matches(list[idx].name)) {
      foundIndex = idx;
      break;
    }
  }

  if (foundIndex >= 0) {
    selectByIndex(foundIndex);
    event.preventDefault();
  }
}

function finishDrag() {
  const content = contentRef.value;
  if (content && drag.pointerId >= 0 && content.hasPointerCapture(drag.pointerId)) {
    content.releasePointerCapture(drag.pointerId);
  }
  drag.active = false;
  drag.pointerId = -1;
  drag.additive = false;
  drag.baseSelection = new Set();
  if (drag.hadSelectionDrag) {
    suppressNextContentClick.value = true;
  }
  isDragging.value = false;
  selectableRects = [];
  pendingRect = { visible: false, x: 0, y: 0, width: 0, height: 0 };
  scheduleRectPaint();
}

function scheduleRectPaint() {
  if (frameId) return;
  frameId = window.requestAnimationFrame(() => {
    frameId = 0;
    const el = selectionRectRef.value;
    if (!el || !pendingRect) return;

    if (!pendingRect.visible) {
      el.classList.remove('visible');
      return;
    }

    const rectRight = pendingRect.x + pendingRect.width;
    const rectBottom = pendingRect.y + pendingRect.height;
    const nextSelection = new Set();
    for (const item of selectableRects) {
      const intersects =
        item.right >= pendingRect.x &&
        item.left <= rectRight &&
        item.bottom >= pendingRect.y &&
        item.top <= rectBottom;
      if (intersects) nextSelection.add(item.path);
    }
    if (drag.additive) {
      const toggled = new Set(drag.baseSelection);
      for (const path of nextSelection) {
        if (toggled.has(path)) toggled.delete(path);
        else toggled.add(path);
      }
      selectedPaths.value = toggled;
    } else {
      selectedPaths.value = nextSelection;
    }

    el.classList.add('visible');
    el.style.left = `${pendingRect.x}px`;
    el.style.top = `${pendingRect.y}px`;
    el.style.width = `${pendingRect.width}px`;
    el.style.height = `${pendingRect.height}px`;
  });
}

onBeforeUnmount(() => {
  window.removeEventListener('keydown', onWindowKeyDown);
  if (frameId) {
    window.cancelAnimationFrame(frameId);
    frameId = 0;
  }
  if (typeBufferTimer) {
    window.clearTimeout(typeBufferTimer);
    typeBufferTimer = 0;
  }
  finishDrag();
  cancelRename();
});

onMounted(() => {
  window.addEventListener('keydown', onWindowKeyDown);
});

watch(
  () => selectedPaths.value,
  () => {
    lastSelectionChangeAt.value = Date.now();
    emitSelectionChange();
  },
  { immediate: true }
);

watch(
  () => entries.value,
  (nextEntries) => {
    if (renamingPath.value && !nextEntries.some((entry) => entry.path === renamingPath.value)) {
      cancelRename();
    }

    const allowed = new Set(nextEntries.map((entry) => entry.path));
    const pruned = new Set(Array.from(selectedPaths.value).filter((path) => allowed.has(path)));
    if (pruned.size !== selectedPaths.value.size) {
      selectedPaths.value = pruned;
    } else {
      emitSelectionChange();
    }

    if (anchorIndex.value !== null && (anchorIndex.value < 0 || anchorIndex.value >= nextEntries.length)) {
      anchorIndex.value = null;
    }
    if (cursorIndex.value < 0 || cursorIndex.value >= nextEntries.length) {
      cursorIndex.value = nextEntries.length > 0 ? 0 : -1;
    }
  },
  { immediate: true }
);

defineExpose({
  selectAllItems,
  deselectAllItems,
  invertSelection,
  getSelectedPaths: () => Array.from(selectedPaths.value),
  getCursorPath: () => {
    const index = getKeyboardCursorIndex();
    const entry = entries.value[index];
    return entry?.path || '';
  },
  startDraftRename: (path) => {
    const entry = entries.value.find((it) => it.path === path);
    if (!entry) return;
    void startRenameForEntry(entry);
  }
});
</script>
