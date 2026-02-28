<template>
  <div ref="pathbarRef" class="pathbar" @contextmenu.prevent="openAddressMenu" @click="onPathbarClick">
    <div class="pathbar-content">
      <template v-if="!isEditing">
        <button
          v-for="crumb in crumbs"
          :key="crumb.path"
          class="crumb"
          :class="{ current: crumb.current }"
          @click="$emit('navigate', crumb.path)"
        >
          <component v-if="crumb.icon" :is="crumb.icon" :size="12" class="crumb-icon" />
          {{ crumb.label }}
        </button>
      </template>
      <input
        v-else
        ref="pathInputRef"
        v-model="editValue"
        class="path-input"
        spellcheck="false"
        autocomplete="off"
        @keydown="onInputKeydown"
        @keydown.esc.prevent="cancelEditing"
        @blur="cancelEditing"
      />
    </div>
    <div v-if="isEditing && dropdownItems.length > 0" class="path-history-dropdown">
      <div
        v-for="(item, idx) in dropdownItems"
        :key="item.type === 'history' ? item.path : item.path"
        class="path-history-item"
        :class="{ 'dropdown-selected': idx === dropdownHighlightIndex }"
      >
        <template v-if="item.type === 'history'">
          <button class="path-history-go" @mousedown.prevent="applyHistoryPath(item.path)">
            {{ item.path }}
          </button>
          <button
            class="path-history-delete"
            title="Remove from recent paths"
            @mousedown.prevent="removeHistoryPath(item.path)"
          >
            ×
          </button>
        </template>
        <template v-else>
          <button
            class="path-history-go path-autocomplete-item"
            :title="item.path"
            @mousedown.prevent="applyAutocompleteItem(item.path)"
          >
            {{ item.name }}
          </button>
        </template>
      </div>
    </div>
  </div>
</template>

<script setup>
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { HardDrive, House, Trash2 } from 'lucide-vue-next';
import { canonicalizePath, FM_TRASH, FM_WELCOME, getVirtualPathLabel, isTrashPath, isWelcomePath, normalizePath } from '../lib/virtualPaths';

const props = defineProps({
  currentPath: {
    type: String,
    required: true
  },
  folderEntries: {
    type: Array,
    default: () => []
  },
  manualHistory: {
    type: Array,
    required: true
  }
});

const emit = defineEmits(['navigate', 'navigate-manual', 'delete-manual-history', 'open-failed']);

const isEditing = ref(false);
const editValue = ref('');
const pathInputRef = ref(null);
const pathbarRef = ref(null);
const dropdownHighlightIndex = ref(0);

const crumbs = computed(() => {
  const raw = props.currentPath || '';
  const normalized = normalizePath(raw);
  if (isWelcomePath(normalized)) {
    return [
      {
        label: getVirtualPathLabel(FM_WELCOME),
        path: FM_WELCOME,
        icon: House,
        current: true
      }
    ];
  }
  if (isTrashPath(normalized)) {
    return [
      {
        label: getVirtualPathLabel(FM_TRASH),
        path: FM_TRASH,
        icon: Trash2,
        current: true
      }
    ];
  }
  const isUnixAbs = normalized.startsWith('/');
  const winDriveMatch = normalized.match(/^[A-Za-z]:/);

  const parts = normalized.split('/').filter(Boolean);
  const result = [];

  let current = '';
  if (winDriveMatch) {
    const driveRoot = `${winDriveMatch[0]}/`;
    current = driveRoot;
    result.push({
      label: winDriveMatch[0],
      path: driveRoot,
      current: parts.length === 1
    });
    parts.shift();
  } else if (isUnixAbs) {
    current = '/';
    result.push({
      label: '',
      path: '/',
      icon: HardDrive,
      current: parts.length === 0
    });
  }

  for (const [index, segment] of parts.entries()) {
    if (current === '' || current === '/') {
      current = `${current}${segment}`;
    } else {
      current = `${current}/${segment}`;
    }

    result.push({
      label: segment,
      path: current,
      current: index === parts.length - 1
    });
  }

  return result;
});

const filteredHistory = computed(() => {
  if (!isEditing.value) return [];
  const list = Array.isArray(props.manualHistory) ? props.manualHistory : [];
  const query = editValue.value.trim().toLowerCase();
  if (!query) return list.slice(0, 8);
  return list.filter((p) => p.toLowerCase().includes(query)).slice(0, 8);
});

const AUTOCOMPLETE_MAX_ITEMS = 80;

/** Last segment of input for folder autocomplete (prefix match in current folder). */
const autocompleteQuery = computed(() => {
  const raw = editValue.value;
  const sep = raw.includes('/') ? '/' : raw.includes('\\') ? '\\' : null;
  const segment = sep ? raw.slice(raw.lastIndexOf(sep) + 1) : raw;
  return segment.trim().toLowerCase();
});

/** True when input ends with path separator → show full folder list without typing. */
const inputEndsWithSeparator = computed(() => {
  const raw = editValue.value.trim();
  return raw.endsWith('/') || raw.endsWith('\\');
});

const folderSuggestions = computed(() => {
  if (!isEditing.value) return [];
  const entries = (Array.isArray(props.folderEntries) ? props.folderEntries : []).filter(
    (e) => !e.draft && e.name
  );
  const query = autocompleteQuery.value;
  if (inputEndsWithSeparator.value) {
    return entries.slice(0, AUTOCOMPLETE_MAX_ITEMS);
  }
  if (!query) return [];
  return entries
    .filter((e) => e.name.toLowerCase().startsWith(query))
    .slice(0, AUTOCOMPLETE_MAX_ITEMS);
});

/** Unified dropdown: history when input unchanged/empty, folder autocomplete when trailing / or typing. */
const dropdownItems = computed(() => {
  if (!isEditing.value) return [];
  const raw = editValue.value.trim();
  const isUnchanged = raw === normalizePath(props.currentPath || '') || raw === '';
  if (isUnchanged) {
    return filteredHistory.value.map((p) => ({ type: 'history', path: p }));
  }
  if (folderSuggestions.value.length > 0) {
    return folderSuggestions.value.map((e) => ({ type: 'folder', path: e.path, name: e.name }));
  }
  return filteredHistory.value.map((p) => ({ type: 'history', path: p }));
});

function openAddressMenu(event) {
  invoke('show_address_menu_cmd', {
    x: event.clientX,
    y: event.clientY
  });
}

function onPathbarClick(event) {
  if (isEditing.value) return;
  if (event.target.closest('.crumb')) return;
  if (event.target.closest('.path-history-dropdown')) return;
  if (event.target.closest('.path-input')) return;
  startEditing();
}

function startEditing() {
  isEditing.value = true;
  editValue.value = props.currentPath || '';
  nextTick(() => {
    const input = pathInputRef.value;
    if (input) {
      input.focus();
      input.select();
    }
  });
}

function cancelEditing() {
  isEditing.value = false;
}

async function submitPath() {
  const rawCandidate = editValue.value.trim();
  if (!rawCandidate) {
    cancelEditing();
    return;
  }

  const normalized = normalizePath(rawCandidate);
  const canonical = canonicalizePath(normalized);

  if (canonical === FM_WELCOME || canonical === FM_TRASH) {
    emit('navigate-manual', canonical);
    cancelEditing();
    return;
  }

  let candidate = rawCandidate;
  try {
    candidate = await invoke('expand_path_cmd', { path: rawCandidate });
  } catch {
    candidate = rawCandidate;
  }

  const valid = await invoke('is_valid_dir_cmd', { path: candidate });
  if (valid) {
    emit('navigate-manual', candidate);
    cancelEditing();
    return;
  }

  try {
    await invoke('open_or_run_cmd', { input: candidate });
  } catch (err) {
    emit('open-failed', typeof err === 'string' ? err : String(err));
  }
  cancelEditing();
}

function applyHistoryPath(path) {
  emit('navigate-manual', path);
  cancelEditing();
}

function applyAutocompleteItem(path) {
  emit('navigate-manual', path);
  cancelEditing();
}

function onInputKeydown(e) {
  const items = dropdownItems.value;
  if (items.length > 0) {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      dropdownHighlightIndex.value = Math.min(dropdownHighlightIndex.value + 1, items.length - 1);
      return;
    }
    if (e.key === 'ArrowUp') {
      e.preventDefault();
      dropdownHighlightIndex.value = Math.max(0, dropdownHighlightIndex.value - 1);
      return;
    }
    if (e.key === 'Enter') {
      e.preventDefault();
      const idx = dropdownHighlightIndex.value;
      if (idx >= 0 && idx < items.length) {
        const item = items[idx];
        if (item.type === 'history') {
          applyHistoryPath(item.path);
        } else {
          applyAutocompleteItem(item.path);
        }
        return;
      }
    }
  }
  if (e.key === 'Enter') {
    e.preventDefault();
    submitPath();
  }
}

function removeHistoryPath(path) {
  emit('delete-manual-history', path);
}

function onWindowPointerDown(event) {
  if (!isEditing.value) return;
  const root = pathbarRef.value;
  if (!root) return;
  if (root.contains(event.target)) return;
  cancelEditing();
}

watch(
  () => props.currentPath,
  () => {
    if (!isEditing.value) {
      editValue.value = props.currentPath || '';
    }
  }
);

watch(dropdownItems, (items) => {
  dropdownHighlightIndex.value = 0;
});

onMounted(() => {
  window.addEventListener('pointerdown', onWindowPointerDown, true);
});

onBeforeUnmount(() => {
  window.removeEventListener('pointerdown', onWindowPointerDown, true);
});

defineExpose({
  startEditing
});
</script>
