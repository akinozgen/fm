<template>
  <footer class="statusbar">
    <span class="statusbar-item">Items: {{ shownCount }}</span>
    <span class="statusbar-item">Selected: {{ selectedCount }}</span>
    <span v-if="showSelectedSize" class="statusbar-item">Selection Size: {{ formatSize(selectedSizeBytes) }}</span>
  </footer>
</template>

<script setup>
defineProps({
  shownCount: {
    type: Number,
    required: true
  },
  selectedCount: {
    type: Number,
    required: true
  },
  showSelectedSize: {
    type: Boolean,
    required: true
  },
  selectedSizeBytes: {
    type: Number,
    required: true
  }
});

function formatSize(size) {
  if (!Number.isFinite(size) || size <= 0) return '0 B';
  if (size < 1024) return `${size} B`;
  if (size < 1024 * 1024) return `${Math.round(size / 1024)} KB`;
  if (size < 1024 * 1024 * 1024) return `${(size / (1024 * 1024)).toFixed(1)} MB`;
  return `${(size / (1024 * 1024 * 1024)).toFixed(1)} GB`;
}
</script>
