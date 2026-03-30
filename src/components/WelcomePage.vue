<template>
  <div class="welcome-page">

    <!-- Pinned favorites strip -->
    <div v-if="pinnedItems.length" class="wp-pinned">
      <span class="wp-pinned-label">Favorites</span>
      <div class="wp-pinned-chips">
        <button
          v-for="item in pinnedItems"
          :key="item.path"
          class="wp-pinned-chip"
          :title="item.path"
          @click="$emit('navigate', item.path)"
        >
          <Folder :size="11" />
          {{ item.label }}
        </button>
      </div>
    </div>

    <!-- Single-column body -->
    <div class="wp-body">

      <!-- Quick Access: 3-column tile grid -->
      <div class="wp-section">
        <p class="wp-section-head">Quick Access</p>
        <div class="wp-tile-grid">
          <button
            v-for="item in quickAccessItems"
            :key="item.path"
            class="wp-tile"
            @click="$emit('navigate', item.path)"
          >
            <component :is="iconFor(item)" :size="18" class="wp-tile-icon" :class="iconClass(item)" />
            <span class="wp-tile-label">{{ item.label }}</span>
          </button>
        </div>
      </div>

      <!-- Storage: drives + removable as bordered cards -->
      <div v-if="drives.length || removable.length" class="wp-section">
        <p class="wp-section-head">Storage</p>
        <div class="wp-device-list">
          <button
            v-for="item in drives"
            :key="item.path"
            class="wp-device-card"
            @click="$emit('navigate', item.path)"
          >
            <HardDrive :size="16" class="wp-device-icon" />
            <div class="wp-device-info">
              <span class="wp-device-label">{{ item.label }}</span>
              <span class="wp-device-path">{{ item.path }}</span>
            </div>
          </button>

          <div
            v-for="item in removable"
            :key="item.path"
            class="wp-device-card wp-device-card--removable"
          >
            <button class="wp-device-card-main" @click="$emit('navigate', item.path)">
              <Usb :size="16" class="wp-device-icon wp-device-icon--removable" />
              <div class="wp-device-info">
                <span class="wp-device-label">{{ item.label }}</span>
                <span class="wp-device-path">{{ item.path }}</span>
              </div>
            </button>
            <button class="wp-eject-btn" title="Unmount" @click.stop="unmount(item.path)">
              <Unplug :size="12" />
            </button>
          </div>
        </div>
      </div>

    </div>
  </div>
</template>

<script setup>
import { computed } from 'vue';
import { Download, FileText, Folder, HardDrive, Home, Music, PictureInPicture2, Trash2, Unplug, Usb, Video } from 'lucide-vue-next';
import { invoke } from '@tauri-apps/api/core';

const props = defineProps({
  sections: {
    type: Array,
    required: true
  }
});

defineEmits(['navigate']);

const userSection = computed(() => {
  return props.sections.find((s) => s.title.startsWith('User')) || props.sections.find((s) => s.title === 'Favorites') || null;
});

const pinnedItems = computed(() => (userSection.value?.items || []).filter((i) => i.kind === 'pinned'));
const quickAccessItems = computed(() => (userSection.value?.items || []).filter((i) => i.kind !== 'pinned'));
const drives = computed(() => findSection('Drives'));
const removable = computed(() => findSection('Removable'));

function findSection(title) {
  const section = props.sections.find((s) => s.title === title);
  return section?.items || [];
}

function iconFor(item) {
  if (item.kind === 'home') return Home;
  if (item.kind === 'trash') return Trash2;
  if (item.label === 'Downloads') return Download;
  if (item.label === 'Documents') return FileText;
  if (item.label === 'Pictures') return PictureInPicture2;
  if (item.label === 'Music') return Music;
  if (item.label === 'Videos') return Video;
  return Folder;
}

function iconClass(item) {
  if (item.kind === 'trash') return 'wp-tile-icon--muted';
  return '';
}

async function unmount(path) {
  try {
    await invoke('unmount_drive_cmd', { path });
  } catch (e) {
    console.error('unmount failed:', e);
  }
}
</script>
