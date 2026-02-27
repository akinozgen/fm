<template>
  <div v-if="active" class="transfer-bar">
    <div class="transfer-bar-header">
      <component :is="opIcon" :size="13" class="transfer-bar-icon" />
      <span class="transfer-bar-label">{{ statusLabel }}</span>
      <div class="transfer-bar-actions">
        <button v-if="!paused" class="transfer-bar-btn" title="Pause" @click="$emit('pause')">
          <Pause :size="12" />
        </button>
        <button v-else class="transfer-bar-btn" title="Resume" @click="$emit('resume')">
          <Play :size="12" />
        </button>
        <button class="transfer-bar-btn transfer-bar-cancel" title="Cancel" @click="$emit('cancel')">
          <X :size="12" />
        </button>
      </div>
    </div>
    <div class="transfer-bar-file">{{ progress.current }}</div>
    <div class="transfer-bar-track">
      <div class="transfer-bar-fill" :style="{ width: fillPct + '%' }"></div>
    </div>
    <div class="transfer-bar-count">
      {{ sizeLabel }}
      <span v-if="progress.total > 0" class="transfer-bar-count-files">{{ progress.done }} of {{ progress.total }} files</span>
    </div>
  </div>
</template>

<script setup>
import { computed } from 'vue';
import { Copy, Pause, Play, Scissors, X } from 'lucide-vue-next';

function formatBytes(bytes) {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  const v = bytes / Math.pow(k, i);
  return `${v % 1 === 0 ? v : v.toFixed(1)} ${units[i]}`;
}

const props = defineProps({
  active:   { type: Boolean, default: false },
  paused:   { type: Boolean, default: false },
  progress: { type: Object,  default: () => ({ op: '', done: 0, total: 0, bytes_done: 0, bytes_total: 0, current: '' }) }
});
defineEmits(['cancel', 'pause', 'resume']);

const opLabel = computed(() => props.progress.op === 'move' ? 'Moving' : 'Copying');
const statusLabel = computed(() => {
  if (props.paused) return 'Paused';
  return props.progress.op === 'move' ? 'Moving' : 'Copying';
});
const opIcon  = computed(() => props.progress.op === 'move' ? Scissors : Copy);

const fillPct = computed(() => {
  const { bytes_done = 0, bytes_total = 0, done = 0, total = 0 } = props.progress;
  if (bytes_total > 0) return Math.min(100, Math.round((bytes_done / bytes_total) * 100));
  return total > 0 ? Math.round((done / total) * 100) : 0;
});

const sizeLabel = computed(() => {
  const { bytes_done = 0, bytes_total = 0 } = props.progress;
  return `${formatBytes(bytes_done)} / ${formatBytes(bytes_total)}`;
});
</script>
