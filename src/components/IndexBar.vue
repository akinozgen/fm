<template>
  <div v-if="indexing" ref="wrapRef" class="index-btn-wrap view-group">
    <button
      class="pill-btn index-progress-btn"
      title="Indexing filesystem…"
      aria-label="Indexing filesystem"
      @click.stop="open = !open"
    >
      <svg class="index-spinner" viewBox="0 0 24 24" aria-hidden="true">
        <circle class="index-spinner-track" cx="12" cy="12" r="10" fill="none" stroke-width="2" />
        <circle class="index-spinner-arc"   cx="12" cy="12" r="10" fill="none" stroke-width="2"
          stroke-dasharray="20 43" stroke-linecap="round" />
      </svg>
      <Search :size="12" class="index-progress-icon" />
    </button>
    <div v-if="open" class="index-popout" @click.stop>
      <span class="index-popout-label">Indexing… {{ indexDone.toLocaleString() }} files</span>
      <button class="index-cancel-btn" @click="$emit('cancel')">Cancel</button>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted, onBeforeUnmount } from 'vue';
import { Search } from 'lucide-vue-next';

defineProps({
  indexing: { type: Boolean, default: false },
  indexDone: { type: Number, default: 0 },
});
defineEmits(['cancel']);

const wrapRef = ref(null);
const open    = ref(false);

function onWindowClick(e) {
  if (open.value && wrapRef.value && !wrapRef.value.contains(e.target)) {
    open.value = false;
  }
}

onMounted(() => window.addEventListener('click', onWindowClick));
onBeforeUnmount(() => window.removeEventListener('click', onWindowClick));
</script>

<style scoped>
.index-btn-wrap {
  position: relative;
}

.index-progress-btn {
  display: flex;
  align-items: center;
  gap: 4px;
}

.index-spinner {
  width: 18px;
  height: 18px;
  animation: index-spin 1s linear infinite;
  flex-shrink: 0;
}

.index-spinner-track {
  stroke: rgba(128, 128, 128, 0.25);
}

.index-spinner-arc {
  stroke: currentColor;
  transform-origin: center;
}

@keyframes index-spin {
  to { transform: rotate(360deg); }
}

.index-progress-icon {
  opacity: 0.75;
}

.index-popout {
  position: absolute;
  top: calc(100% + 6px);
  right: 0;
  min-width: 220px;
  background: var(--panel, #fff);
  border: 1px solid var(--line);
  border-radius: 8px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.12);
  padding: 10px 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  z-index: 200;
}

.index-popout-label {
  font-size: 12px;
  color: var(--ink);
}

.index-cancel-btn {
  font-size: 11.5px;
  color: var(--muted);
  background: none;
  border: 1px solid var(--line);
  border-radius: 4px;
  padding: 3px 8px;
  cursor: pointer;
  align-self: flex-start;
}

.index-cancel-btn:hover {
  color: var(--ink);
}
</style>
