<template>
  <div class="welcome-page">
    <section class="welcome-hero">
      <h1 class="welcome-title">Welcome to fm</h1>
      <p class="welcome-subtitle">Jump into your folders and volumes.</p>
    </section>

    <section class="welcome-section">
      <div class="welcome-section-head">
        <Star :size="14" />
        <span>{{ userSection?.title || 'User' }}</span>
      </div>
      <div class="welcome-grid">
        <button
          v-for="item in userItems"
          :key="item.path"
          class="welcome-card"
          @click="$emit('navigate', item.path)"
        >
          <component :is="iconFor(item)" :size="20" />
          <span class="welcome-card-title">{{ item.label }}</span>
          <span class="welcome-card-path">{{ item.path }}</span>
        </button>
      </div>
    </section>

    <section class="welcome-section">
      <div class="welcome-section-head">
        <HardDrive :size="14" />
        <span>Drives</span>
      </div>
      <div class="welcome-grid">
        <button
          v-for="item in drives"
          :key="item.path"
          class="welcome-card"
          @click="$emit('navigate', item.path)"
        >
          <HardDrive :size="20" />
          <span class="welcome-card-title">{{ item.label }}</span>
          <span class="welcome-card-path">{{ item.path }}</span>
        </button>
      </div>
    </section>

    <section class="welcome-section" v-if="removable.length > 0">
      <div class="welcome-section-head">
        <Usb :size="14" />
        <span>Removable</span>
      </div>
      <div class="welcome-grid">
        <button
          v-for="item in removable"
          :key="item.path"
          class="welcome-card removable"
          @click="$emit('navigate', item.path)"
        >
          <Usb :size="20" />
          <span class="welcome-card-title">{{ item.label }}</span>
          <span class="welcome-card-path">{{ item.path }}</span>
        </button>
      </div>
    </section>
  </div>
</template>

<script setup>
import { computed } from 'vue';
import { Download, FileText, Folder, HardDrive, Home, Music, PictureInPicture2, Star, Usb, Video } from 'lucide-vue-next';

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
const userItems = computed(() => userSection.value?.items || []);
const drives = computed(() => findSection('Drives'));
const removable = computed(() => findSection('Removable'));

function findSection(title) {
  const section = props.sections.find((s) => s.title === title);
  return section?.items || [];
}

function iconFor(item) {
  if (item.kind === 'home') return Home;
  if (item.label === 'Downloads') return Download;
  if (item.label === 'Documents') return FileText;
  if (item.label === 'Pictures') return PictureInPicture2;
  if (item.label === 'Music') return Music;
  if (item.label === 'Videos') return Video;
  return Folder;
}
</script>
