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
      <div v-if="transferJobs.length > 0" ref="transferWrapRef" class="transfer-btn-wrap">
        <button
          class="pill-btn transfer-progress-btn"
          title="Transfer in progress"
          aria-label="Transfer in progress"
          @click.stop="transferPopoutOpen = !transferPopoutOpen"
        >
          <svg class="transfer-progress-ring" viewBox="0 0 24 24" aria-hidden="true">
            <circle class="transfer-progress-bg" cx="12" cy="12" r="10" fill="none" stroke-width="2" />
            <circle
              class="transfer-progress-fill"
              cx="12"
              cy="12"
              r="10"
              fill="none"
              stroke-width="2"
              :stroke-dasharray="circumference"
              :stroke-dashoffset="strokeOffset"
            />
          </svg>
          <Copy v-if="!firstJob || firstJob.op !== 'move'" :size="12" class="transfer-progress-icon" />
          <Scissors v-else :size="12" class="transfer-progress-icon" />
        </button>
        <div v-if="transferPopoutOpen" class="transfer-popout" @click.stop>
          <div class="transfer-popout-list">
            <TransferBar
              v-for="job in transferJobs"
              :key="job.id"
              :active="true"
              :paused="job.paused"
              :progress="job"
              @cancel="$emit('cancel-transfer', job.id)"
              @pause="$emit('pause-transfer', job.id)"
              @resume="$emit('resume-transfer', job.id)"
            />
          </div>
        </div>
      </div>
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
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { ArrowUp, ChevronLeft, ChevronRight, Copy, LayoutGrid, List, Scissors } from 'lucide-vue-next';
import AddressBar from './AddressBar.vue';
import TransferBar from './TransferBar.vue';

const CIRCUMFERENCE = 2 * Math.PI * 10;

const emit = defineEmits([
  'update:view-mode',
  'navigate-up',
  'navigate-back',
  'navigate-forward',
  'navigate-path',
  'navigate-path-manual',
  'delete-manual-history',
  'cancel-transfer',
  'pause-transfer',
  'resume-transfer'
]);
const props = defineProps({
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
  },
  transferJobs: {
    type: Array,
    default: () => []
  }
});

const addressBarRef = ref(null);
const transferWrapRef = ref(null);
const transferPopoutOpen = ref(false);

const firstJob = computed(() => props.transferJobs[0] ?? null);

const fillPct = computed(() => {
  const job = firstJob.value;
  if (!job) return 0;
  const { bytes_done = 0, bytes_total = 0, done = 0, total = 0 } = job;
  if (bytes_total > 0) return Math.min(100, Math.round((bytes_done / bytes_total) * 100));
  return total > 0 ? Math.round((done / total) * 100) : 0;
});

const circumference = CIRCUMFERENCE;
const strokeOffset = computed(() => CIRCUMFERENCE * (1 - fillPct.value / 100));

watch(() => props.transferJobs.length, (len) => {
  if (len === 0) transferPopoutOpen.value = false;
});

function onWindowClick(e) {
  if (transferPopoutOpen.value && transferWrapRef.value && !transferWrapRef.value.contains(e.target)) {
    transferPopoutOpen.value = false;
  }
}

onMounted(() => {
  window.addEventListener('click', onWindowClick);
});

onBeforeUnmount(() => {
  window.removeEventListener('click', onWindowClick);
});

defineExpose({
  startAddressEditing: () => {
    addressBarRef.value?.startEditing?.();
  }
});
</script>
