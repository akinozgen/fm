<template>
  <div class="toolbar">
    <div class="nav-group">
      <button class="icon-btn" title="Back" @click="$emit('navigate-back')"><ChevronLeft :size="14" /></button>
      <button class="icon-btn" title="Forward" @click="$emit('navigate-forward')"><ChevronRight :size="14" /></button>
      <button class="icon-btn" title="Up" @click="$emit('navigate-up')"><ArrowUp :size="14" /></button>
    </div>
    <AddressBar
      ref="addressBarRef"
      :current-path="currentPath"
      :manual-history="manualHistory"
      @navigate="(path) => $emit('navigate-path', path)"
      @navigate-manual="(path) => $emit('navigate-path-manual', path)"
      @delete-manual-history="(path) => $emit('delete-manual-history', path)"
    />
    <div class="view-group">
      <button
        class="pill-btn"
        :class="{ active: viewMode === 'grid' }"
        @click="$emit('update:view-mode', 'grid')"
      >
        <LayoutGrid :size="14" />
      </button>
      <button
        class="pill-btn"
        :class="{ active: viewMode === 'list' }"
        @click="$emit('update:view-mode', 'list')"
      >
        <List :size="14" />
      </button>
    </div>
  </div>
</template>

<script setup>
import { ref } from 'vue';
import { ArrowUp, ChevronLeft, ChevronRight, LayoutGrid, List } from 'lucide-vue-next';
import AddressBar from './AddressBar.vue';

defineEmits([
  'update:view-mode',
  'navigate-up',
  'navigate-back',
  'navigate-forward',
  'navigate-path',
  'navigate-path-manual',
  'delete-manual-history'
]);
defineProps({
  currentPath: {
    type: String,
    required: true
  },
  manualHistory: {
    type: Array,
    required: true
  },
  viewMode: {
    type: String,
    required: true
  }
});

const addressBarRef = ref(null);

defineExpose({
  startAddressEditing: () => {
    addressBarRef.value?.startEditing?.();
  }
});
</script>
