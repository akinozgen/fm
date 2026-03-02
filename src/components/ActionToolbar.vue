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
    <ViewOptionsDropdown
      :view-mode="viewMode"
      :show-hidden="showHidden"
      :show-extensions="showExtensions"
      :show-selection-checkboxes="showSelectionCheckboxes"
      :sort-by="sortBy"
      :sort-dir="sortDir"
      @update:view-mode="$emit('update:view-mode', $event)"
      @update:show-hidden="$emit('update:show-hidden', $event)"
      @update:show-extensions="$emit('update:show-extensions', $event)"
      @update:show-selection-checkboxes="$emit('update:show-selection-checkboxes', $event)"
      @update:sort-by="$emit('update:sort-by', $event)"
      @update:sort-dir="$emit('update:sort-dir', $event)"
    />
  </div>
</template>

<script setup>
import { computed } from 'vue';
import {
  CheckSquare,
  ClipboardPaste,
  Copy,
  Scissors,
  Shuffle,
  Square,
  Trash2
} from 'lucide-vue-next';
import ViewOptionsDropdown from '@/components/ViewOptionsDropdown.vue';

const props = defineProps({
  showHidden: { type: Boolean, required: true },
  showExtensions: { type: Boolean, required: true },
  showSelectionCheckboxes: { type: Boolean, required: true },
  viewMode: { type: String, required: true },
  selectedCount: { type: Number, default: 0 },
  sortBy: { type: String, default: 'name' },
  sortDir: { type: String, default: 'asc' },
  onDelete: { type: Function, required: true },
  canPaste: { type: Boolean, default: false }
});

const hasSelection = computed(() => props.selectedCount > 0);

defineEmits([
  'update:show-hidden',
  'update:show-extensions',
  'update:show-selection-checkboxes',
  'update:view-mode',
  'update:sort-by',
  'update:sort-dir',
  'select-all',
  'deselect-all',
  'select-inverse',
  'cut',
  'copy',
  'paste'
]);

</script>
