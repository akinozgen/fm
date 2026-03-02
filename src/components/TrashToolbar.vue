<template>
  <div class="action-toolbar trash-toolbar">
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
      <button
        class="op-btn"
        title="Delete selected items permanently"
        :disabled="!hasSelection"
        @click.stop.prevent="onDelete"
      >
        <Trash2 :size="13" />
        <span>Delete</span>
      </button>
      <button
        class="op-btn op-btn-danger"
        title="Permanently delete all items in Trash"
        :disabled="!canEmptyTrash"
        @click.stop.prevent="onEmptyTrash"
      >
        <Trash2 :size="13" />
        <span>Empty Trash</span>
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
import { CheckSquare, Shuffle, Square, Trash2 } from 'lucide-vue-next';
import ViewOptionsDropdown from '@/components/ViewOptionsDropdown.vue';

const props = defineProps({
  selectedCount: { type: Number, default: 0 },
  canEmptyTrash: { type: Boolean, required: true },
  onDelete: { type: Function, required: true },
  onEmptyTrash: { type: Function, required: true },
  viewMode: { type: String, default: 'list' },
  showHidden: { type: Boolean, default: false },
  showExtensions: { type: Boolean, default: true },
  showSelectionCheckboxes: { type: Boolean, default: false },
  sortBy: { type: String, default: 'name' },
  sortDir: { type: String, default: 'asc' },
});

defineEmits([
  'select-all',
  'deselect-all',
  'select-inverse',
  'update:view-mode',
  'update:show-hidden',
  'update:show-extensions',
  'update:show-selection-checkboxes',
  'update:sort-by',
  'update:sort-dir',
]);

const hasSelection = computed(() => props.selectedCount > 0);
</script>
