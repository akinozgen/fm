<template>
  <div class="action-toolbar">
    <div class="action-segment">
      <button class="op-btn" title="Select All" @click="$emit('select-all')">
        <CheckSquare :size="13" />
        <span>Select All</span>
      </button>
      <button class="op-btn" title="Deselect" @click="$emit('deselect-all')">
        <Square :size="13" />
        <span>Deselect</span>
      </button>
      <button class="op-btn" title="Select Inverse" @click="$emit('select-inverse')">
        <Shuffle :size="13" />
        <span>Inverse</span>
      </button>
    </div>
    <div class="action-segment">
      <button class="op-btn op-btn-icon" title="Cut" aria-label="Cut">
        <Scissors :size="14" />
      </button>
      <button class="op-btn op-btn-icon" title="Copy" aria-label="Copy">
        <Copy :size="14" />
      </button>
      <button class="op-btn op-btn-icon" title="Paste" aria-label="Paste">
        <ClipboardPaste :size="14" />
      </button>
      <button class="op-btn op-btn-icon" title="Delete" aria-label="Delete" @click.stop.prevent="onDelete">
        <Trash2 :size="14" />
      </button>
      <button
        v-if="isTrashView"
        class="op-btn"
        title="Empty Trash"
        aria-label="Empty Trash"
        :disabled="!canEmptyTrash"
        @click.stop.prevent="onEmptyTrash"
      >
        <Trash2 :size="14" />
        <span>Empty Trash</span>
      </button>
    </div>
    <div class="action-dropdown-wrap">
      <button class="op-btn op-btn-dropdown" @click.stop="toggleMenu">
        <SlidersHorizontal :size="13" />
        View Options
        <ChevronDown :size="12" />
      </button>
      <div v-if="menuOpen" class="op-dropdown" @click.stop>
        <label class="op-check">
          <input
            class="op-check-input"
            type="checkbox"
            :checked="showHidden"
            @change="$emit('update:show-hidden', $event.target.checked)"
          />
          <span class="op-check-mark" aria-hidden="true"></span>
          <span class="op-check-label">Show Hidden Files</span>
        </label>
        <label class="op-check">
          <input
            class="op-check-input"
            type="checkbox"
            :checked="showExtensions"
            @change="$emit('update:show-extensions', $event.target.checked)"
          />
          <span class="op-check-mark" aria-hidden="true"></span>
          <span class="op-check-label">Show File Extensions</span>
        </label>
        <label class="op-check">
          <input
            class="op-check-input"
            type="checkbox"
            :checked="showSelectionCheckboxes"
            @change="$emit('update:show-selection-checkboxes', $event.target.checked)"
          />
          <span class="op-check-mark" aria-hidden="true"></span>
          <span class="op-check-label">Show Selection Checkboxes</span>
        </label>
      </div>
    </div>
  </div>
</template>

<script setup>
import { onBeforeUnmount, onMounted, ref } from 'vue';
import {
  CheckSquare,
  ChevronDown,
  ClipboardPaste,
  Copy,
  Scissors,
  Shuffle,
  SlidersHorizontal,
  Square,
  Trash2
} from 'lucide-vue-next';

const menuOpen = ref(false);
defineEmits([
  'update:show-hidden',
  'update:show-extensions',
  'update:show-selection-checkboxes',
  'select-all',
  'deselect-all',
  'select-inverse'
]);
defineProps({
  showHidden: {
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
  onDelete: {
    type: Function,
    required: true
  },
  onEmptyTrash: {
    type: Function,
    required: true
  },
  isTrashView: {
    type: Boolean,
    required: true
  },
  canEmptyTrash: {
    type: Boolean,
    required: true
  }
});

function toggleMenu() {
  menuOpen.value = !menuOpen.value;
}

function onWindowClick() {
  menuOpen.value = false;
}

onMounted(() => {
  window.addEventListener('click', onWindowClick);
});

onBeforeUnmount(() => {
  window.removeEventListener('click', onWindowClick);
});
</script>
