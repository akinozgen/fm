<template>
  <div class="file-icon">
    <img v-if="iconUrl" :src="iconUrl" :alt="altText" />
    <component v-else :is="fallback" :size="16" />
  </div>
</template>

<script setup>
import { computed, onMounted, ref, watch } from 'vue';
import { File, Folder } from 'lucide-vue-next';
import { getFileIcon } from '../lib/iconCache';

const props = defineProps({
  path: {
    type: String,
    required: true
  },
  isDir: {
    type: Boolean,
    default: false
  },
  size: {
    type: Number,
    default: 16
  }
});

const iconUrl = ref(null);
const fallback = computed(() => (props.isDir ? Folder : File));
const altText = computed(() => (props.isDir ? 'Folder' : 'File'));

async function loadIcon() {
  iconUrl.value = await getFileIcon(props.path, props.size);
}

onMounted(loadIcon);
watch(() => [props.path, props.size], loadIcon);
</script>
