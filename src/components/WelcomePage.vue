<template>
  <div class="welcome-page">

    <!-- Pinned strip — only shown when there are pinned items -->
    <div v-if="pinnedItems.length" class="wp-pinned">
      <Pin :size="10" class="wp-pinned-meta-icon" />
      <span class="wp-pinned-meta">Favorites</span>
      <button
        v-for="item in pinnedItems"
        :key="item.path"
        class="wp-pinned-item"
        @click="$emit('navigate', item.path)"
      >
        <Folder :size="12" />
        {{ item.label }}
      </button>
    </div>

    <!-- Two-column body -->
    <div class="wp-body">

      <!-- Left: Locations -->
      <div class="wp-col">
        <p class="wp-col-head">Locations</p>
        <div class="wp-rows">
          <button
            v-for="item in quickAccessItems"
            :key="item.path"
            class="wp-row"
            @click="$emit('navigate', item.path)"
          >
            <component :is="iconFor(item)" :size="15" class="wp-row-icon" :class="iconClass(item)" />
            <span class="wp-row-label">{{ item.label }}</span>
          </button>
        </div>
      </div>

      <!-- Right: Drives + Removable -->
      <div class="wp-col wp-col--right">
        <template v-if="drives.length">
          <p class="wp-col-head">Devices</p>
          <div class="wp-rows">
            <button
              v-for="item in drives"
              :key="item.path"
              class="wp-row"
              @click="$emit('navigate', item.path)"
            >
              <HardDrive :size="15" class="wp-row-icon wp-row-icon--device" />
              <span class="wp-row-label">{{ item.label }}</span>
              <span class="wp-row-sub">{{ item.path }}</span>
            </button>
          </div>
        </template>

        <template v-if="removable.length">
          <p class="wp-col-head" :class="{ 'wp-col-head--gap': drives.length }">Removable</p>
          <div class="wp-rows">
            <div
              v-for="item in removable"
              :key="item.path"
              class="wp-row wp-row--removable"
            >
              <button class="wp-row-main" @click="$emit('navigate', item.path)">
                <Usb :size="15" class="wp-row-icon wp-row-icon--device" />
                <span class="wp-row-label">{{ item.label }}</span>
                <span class="wp-row-sub">{{ item.path }}</span>
              </button>
              <button class="wp-eject-btn" title="Unmount" @click.stop="unmount(item.path)">
                <svg width="12" height="12" viewBox="0 0 12 12" fill="none" xmlns="http://www.w3.org/2000/svg">
                  <path d="M2 9.5h8M6 1.5 2 7h8L6 1.5Z" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>
                </svg>
              </button>
            </div>
          </div>
        </template>
      </div>

    </div>
  </div>
</template>

<script setup>
import { computed } from 'vue';
import { Download, FileText, Folder, HardDrive, Home, Music, PictureInPicture2, Pin, Trash2, Usb, Video } from 'lucide-vue-next';
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
  if (item.kind === 'trash') return 'wp-row-icon--trash';
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
