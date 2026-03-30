<template>
  <div class="action-toolbar">

    <!-- Selection segment: adapts based on whether anything is selected -->
    <div class="action-segment selection-segment">
      <template v-if="hasSelection">
        <div class="selection-chip">
          <CheckSquare :size="11" />
          <span>{{ selectedCount }} selected</span>
          <button class="chip-clear" title="Deselect all" @click="$emit('deselect-all')">
            <X :size="10" />
          </button>
        </div>
        <div class="segment-divider" />
        <button class="op-btn op-btn-xs" title="Select all" @click="$emit('select-all')">All</button>
        <button class="op-btn op-btn-xs" title="Invert selection" @click="$emit('select-inverse')">Invert</button>
      </template>
      <template v-else>
        <button class="op-btn" title="Select all" @click="$emit('select-all')">
          <Square :size="12" />
          <span>Select All</span>
        </button>
      </template>
    </div>

    <!-- Clipboard actions -->
    <div class="action-segment">
      <button class="op-btn op-btn-icon" title="Cut (⌘X)" aria-label="Cut" :disabled="!hasSelection" @click="$emit('cut')">
        <Scissors :size="13" />
      </button>
      <button class="op-btn op-btn-icon" title="Copy (⌘C)" aria-label="Copy" :disabled="!hasSelection" @click="$emit('copy')">
        <Copy :size="13" />
      </button>
      <button class="op-btn op-btn-icon" title="Paste (⌘V)" aria-label="Paste" :disabled="!canPaste" @click="$emit('paste')">
        <ClipboardPaste :size="13" />
      </button>
    </div>

    <!-- Destructive action: visually isolated -->
    <div class="action-segment">
      <button class="op-btn op-btn-icon op-btn-danger" title="Delete" aria-label="Delete" :disabled="!hasSelection" @click.stop.prevent="onDelete">
        <Trash2 :size="13" />
      </button>
    </div>

    <ViewOptionsDropdown
      :view-mode="viewMode"
      :grid-zoom="gridZoom"
      :show-hidden="showHidden"
      :show-extensions="showExtensions"
      :show-selection-checkboxes="showSelectionCheckboxes"
      :sort-by="sortBy"
      :sort-dir="sortDir"
      @update:view-mode="$emit('update:view-mode', $event)"
      @update:grid-zoom="$emit('update:grid-zoom', $event)"
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
  Square,
  Trash2,
  X
} from 'lucide-vue-next';
import ViewOptionsDropdown from '@/components/ViewOptionsDropdown.vue';

const props = defineProps({
  showHidden: { type: Boolean, required: true },
  showExtensions: { type: Boolean, required: true },
  showSelectionCheckboxes: { type: Boolean, required: true },
  viewMode: { type: String, required: true },
  gridZoom: { type: Number, default: 110 },
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
  'update:grid-zoom',
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
