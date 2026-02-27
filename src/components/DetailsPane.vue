<template>
  <aside class="details" :class="{ collapsed }">
    <div class="details-header">
      <div class="details-title">Details</div>
      <button class="icon-btn" title="Toggle details" @click="$emit('toggle')">
        <PanelRightClose v-if="!collapsed" :size="14" />
        <PanelRightOpen v-else :size="14" />
      </button>
    </div>
    <div class="details-body">
      <div class="detail-row"><span>Name</span><span>src</span></div>
      <div class="detail-row"><span>Type</span><span>Folder</span></div>
      <div class="detail-row"><span>Size</span><span>—</span></div>
      <div class="detail-row"><span>Modified</span><span>Today</span></div>
    </div>
  </aside>
</template>

<script setup>
import { PanelRightClose, PanelRightOpen } from 'lucide-vue-next';

defineEmits(['toggle']);
defineProps({
  collapsed: {
    type: Boolean,
    required: true
  }
});
</script>
