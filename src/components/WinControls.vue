<template>
  <div class="win-controls">
    <button class="wc-btn wc-min" title="Minimize" @click.stop="minimize">
      <Minus :size="10" />
    </button>
    <button class="wc-btn wc-max" :title="maximized ? 'Restore' : 'Maximize'" @click.stop="toggleMax">
      <Minimize2 v-if="maximized" :size="10" />
      <Maximize2 v-else :size="10" />
    </button>
    <button class="wc-btn wc-close" title="Close" @click.stop="close">
      <X :size="10" />
    </button>
  </div>
</template>

<script setup>
import { onBeforeUnmount, onMounted, ref } from 'vue';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { Maximize2, Minimize2, Minus, X } from 'lucide-vue-next';

const maximized = ref(false);
let unlistenResize = null;

const win = getCurrentWindow();

async function minimize() {
  await win.minimize();
}

async function toggleMax() {
  await win.toggleMaximize();
}

async function close() {
  await win.close();
}

onMounted(async () => {
  maximized.value = await win.isMaximized();
  unlistenResize = await win.onResized(async () => {
    maximized.value = await win.isMaximized();
  });
});

onBeforeUnmount(() => {
  unlistenResize?.();
});
</script>
