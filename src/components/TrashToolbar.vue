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
  </div>
</template>

<script setup>
import { computed } from 'vue';
import { CheckSquare, Shuffle, Square, Trash2 } from 'lucide-vue-next';

const props = defineProps({
  selectedCount: { type: Number, default: 0 },
  canEmptyTrash: { type: Boolean, required: true },
  onDelete: { type: Function, required: true },
  onEmptyTrash: { type: Function, required: true }
});

defineEmits(['select-all', 'deselect-all', 'select-inverse']);

const hasSelection = computed(() => props.selectedCount > 0);
</script>
