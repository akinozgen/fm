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
        @keydown.enter.prevent="submitPath"
        @keydown.esc.prevent="cancelEditing"
        @blur="cancelEditing"
      />
    </div>
    <div v-if="isEditing && filteredHistory.length > 0" class="path-history-dropdown">
      <div
        v-for="path in filteredHistory"
        :key="path"
        class="path-history-item"
      >
        <button class="path-history-go" @mousedown.prevent="applyHistoryPath(path)">
          {{ path }}
        </button>
        <button
          class="path-history-delete"
          title="Remove from recent paths"
          @mousedown.prevent="removeHistoryPath(path)"
        >
          ×
        </button>
      </div>
    </div>
  </div>
</template>

<script setup>
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { HardDrive, House, Trash2 } from 'lucide-vue-next';
import { FM_TRASH, FM_WELCOME, getVirtualPathLabel, isTrashPath, isWelcomePath, normalizePath } from '../lib/virtualPaths';

const props = defineProps({
  currentPath: {
    type: String,
    required: true
  },
  manualHistory: {
    type: Array,
    required: true
  }
});

const emit = defineEmits(['navigate', 'navigate-manual', 'delete-manual-history']);

const isEditing = ref(false);
const editValue = ref('');
const pathInputRef = ref(null);
const pathbarRef = ref(null);

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
    current = winDriveMatch[0];
    result.push({
      label: winDriveMatch[0],
      path: current,
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

  const normalizedCandidate = normalizePath(rawCandidate);
  if (isTrashPath(normalizedCandidate)) {
    emit('navigate-manual', FM_TRASH);
    cancelEditing();
    return;
  }
  if (isWelcomePath(normalizedCandidate)) {
    emit('navigate-manual', FM_WELCOME);
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
  }
  cancelEditing();
}

function applyHistoryPath(path) {
  emit('navigate-manual', path);
  cancelEditing();
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
