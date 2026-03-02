<template>
  <div class="welcome-page">
    <section class="welcome-hero">
      <h1 class="welcome-title">fm</h1>
      <p class="welcome-subtitle">Open a folder or drive to get started.</p>
    </section>

    <section class="welcome-section" v-if="pinnedItems.length > 0">
      <div class="welcome-section-head">
        <Pin :size="12" />
        <span>Favorites</span>
      </div>
      <div class="welcome-grid">
        <button
          v-for="item in pinnedItems"
          :key="item.path"
          class="welcome-card welcome-card--compact"
          @click="$emit('navigate', item.path)"
        >
          <Folder :size="16" class="welcome-card-icon" />
          <span class="welcome-card-title">{{ item.label }}</span>
        </button>
      </div>
    </section>

    <section class="welcome-section">
      <div class="welcome-section-head">
        <Star :size="12" />
        <span>Quick access</span>
      </div>
      <div class="welcome-grid">
        <button
          v-for="item in quickAccessItems"
          :key="item.path"
          class="welcome-card welcome-card--compact"
          @click="$emit('navigate', item.path)"
        >
          <component :is="iconFor(item)" :size="16" class="welcome-card-icon" />
          <span class="welcome-card-title">{{ item.label }}</span>
        </button>
      </div>
    </section>

    <section class="welcome-section">
      <div class="welcome-section-head">
        <HardDrive :size="12" />
        <span>Drives</span>
      </div>
      <div class="welcome-grid">
        <button
          v-for="item in drives"
          :key="item.path"
          class="welcome-card welcome-card--compact"
          @click="$emit('navigate', item.path)"
        >
          <HardDrive :size="16" class="welcome-card-icon" />
          <span class="welcome-card-title">{{ item.label }}</span>
        </button>
      </div>
    </section>

    <section class="welcome-section" v-if="removable.length > 0">
      <div class="welcome-section-head">
        <Usb :size="12" />
        <span>Removable</span>
      </div>
      <div class="welcome-grid">
        <button
          v-for="item in removable"
          :key="item.path"
          class="welcome-card welcome-card--compact removable"
          @click="$emit('navigate', item.path)"
        >
          <Usb :size="16" class="welcome-card-icon" />
          <span class="welcome-card-title">{{ item.label }}</span>
        </button>
      </div>
    </section>
  </div>
</template>

<script setup>
import { computed } from 'vue';
import { Download, FileText, Folder, HardDrive, Home, Music, PictureInPicture2, Pin, Star, Trash2, Usb, Video } from 'lucide-vue-next';

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
</script>
