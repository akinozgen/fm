<template>
  <div class="file-icon" ref="iconEl">
    <img
      v-if="iconUrl"
      :src="iconUrl"
      :alt="altText"
      :class="{ thumbnail: isThumbnail }"
      @error="onImgError"
    />
    <component v-else :is="fallback" :size="16" />
  </div>
</template>

<script setup>
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { File, Folder } from 'lucide-vue-next';
import { getFileIcon, isThumbnailKey } from '../lib/iconCache';

const props = defineProps({
  path: { type: String, required: true },
  isDir: { type: Boolean, default: false },
  size: { type: Number, default: 16 }
});

const iconUrl = ref(null);
const isThumbnail = ref(false);
const iconEl = ref(null);
const fallback = computed(() => (props.isDir ? Folder : File));
const altText = computed(() => (props.isDir ? 'Folder' : 'File'));

async function loadIcon() {
  const url = await getFileIcon(props.path, props.size);
  iconUrl.value = url;
  // isThumbnailKey is synchronously readable after getFileIcon resolves
  isThumbnail.value = !!url && isThumbnailKey(props.path, props.size);
}

function onImgError() {
  iconUrl.value = null;
  isThumbnail.value = false;
}

// Grid view (size >= 48) defers loading until the element enters the viewport.
// List view loads immediately (OS icons are tiny, no need to defer).
let observer = null;

function setupObserver() {
  observer?.disconnect();
  observer = null;

  if (props.size >= 48 && iconEl.value) {
    observer = new IntersectionObserver(([entry]) => {
      if (entry.isIntersecting) {
        loadIcon();
        observer.disconnect();
        observer = null;
      }
    }, { rootMargin: '150px' });
    observer.observe(iconEl.value);
  } else {
    loadIcon();
  }
}

onMounted(setupObserver);
onBeforeUnmount(() => { observer?.disconnect(); observer = null; });
watch(() => [props.path, props.size], () => {
  iconUrl.value = null;
  isThumbnail.value = false;
  setupObserver();
});
</script>
