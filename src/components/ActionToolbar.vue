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
      <button class="op-btn op-btn-icon" title="Cut" aria-label="Cut" :disabled="!hasSelection" @click="$emit('cut')">
        <Scissors :size="14" />
      </button>
      <button class="op-btn op-btn-icon" title="Copy" aria-label="Copy" :disabled="!hasSelection" @click="$emit('copy')">
        <Copy :size="14" />
      </button>
      <button class="op-btn op-btn-icon" title="Paste" aria-label="Paste" :disabled="!canPaste" @click="$emit('paste')">
        <ClipboardPaste :size="14" />
      </button>
      <button class="op-btn op-btn-icon" title="Delete" aria-label="Delete" :disabled="!hasSelection" @click.stop.prevent="onDelete">
        <Trash2 :size="14" />
      </button>
    </div>
    <div class="action-dropdown-wrap">
      <button class="op-btn op-btn-dropdown" @click.stop="toggleMenu">
        <SlidersHorizontal :size="13" />
        View Options
        <ChevronDown :size="12" />
      </button>
      <div v-if="menuOpen" class="op-dropdown" @click.stop>
        <div class="op-sort-section" style="padding-bottom: 2px">
          <span class="op-sort-label">Display</span>
        </div>
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

        <div class="op-divider"></div>

        <div class="op-sort-section">
          <span class="op-sort-label">Sort by</span>
          <div class="op-sort-group">
            <button
              v-for="field in SORT_FIELDS"
              :key="field.value"
              class="op-sort-btn"
              :class="{ active: sortBy === field.value }"
              @click="$emit('update:sort-by', field.value)"
            >{{ field.label }}</button>
          </div>
          <div class="op-sort-dir">
            <button
              class="op-sort-dir-btn"
              :class="{ active: sortDir === 'asc' }"
              @click="$emit('update:sort-dir', 'asc')"
            >
              <ArrowUpNarrowWide :size="12" />
              Ascending
            </button>
            <button
              class="op-sort-dir-btn"
              :class="{ active: sortDir === 'desc' }"
              @click="$emit('update:sort-dir', 'desc')"
            >
              <ArrowDownWideNarrow :size="12" />
              Descending
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import {
  ArrowDownWideNarrow,
  ArrowUpNarrowWide,
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

const SORT_FIELDS = [
  { value: 'name',     label: 'Name' },
  { value: 'type',     label: 'Type' },
  { value: 'size',     label: 'Size' },
  { value: 'modified', label: 'Date' },
];

const props = defineProps({
  showHidden: { type: Boolean, required: true },
  showExtensions: { type: Boolean, required: true },
  showSelectionCheckboxes: { type: Boolean, required: true },
  selectedCount: { type: Number, default: 0 },
  sortBy: { type: String, default: 'name' },
  sortDir: { type: String, default: 'asc' },
  onDelete: { type: Function, required: true },
  canPaste: { type: Boolean, default: false }
});

const hasSelection = computed(() => props.selectedCount > 0);

const menuOpen = ref(false);
defineEmits([
  'update:show-hidden',
  'update:show-extensions',
  'update:show-selection-checkboxes',
  'update:sort-by',
  'update:sort-dir',
  'select-all',
  'deselect-all',
  'select-inverse',
  'cut',
  'copy',
  'paste'
]);

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
