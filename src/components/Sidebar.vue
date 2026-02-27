<template>
  <aside class="sidebar">
    <div class="sidebar-header">
      <div class="sidebar-brand">
        <button class="brand-badge brand-home-btn" title="Go Home" @click="goHome">
          <FolderOpen :size="14" />
        </button>
        <div class="brand-copy">
          <div class="brand">fm</div>
          <div class="brand-subtitle">{{ currentLocationLabel }}</div>
        </div>
      </div>
      <div class="sidebar-header-actions">
        <button class="ghost-btn" title="Search">
          <Search :size="13" />
        </button>
        <button class="ghost-btn" title="New File" @click="$emit('quick-add')">
          <Plus :size="13" />
        </button>
      </div>
    </div>
    <div class="treeview">
      <div
        v-for="section in sections"
        :key="section.title"
        class="tree-group"
      >
        <button class="tree-section" @click="toggleSection(section.title)">
          <ChevronDown v-if="!collapsed[section.title]" :size="12" />
          <ChevronRight v-else :size="12" />
          <span>{{ section.title }}</span>
        </button>
        <div v-if="!collapsed[section.title]" class="tree-items">
          <button
            v-for="item in section.items"
            :key="item.path"
            class="tree-item"
            :class="{ active: isActive(item.path) }"
            @click="$emit('navigate', item.path)"
          >
            <component :is="resolveIcon(item)" :size="14" />
            <span>{{ item.label }}</span>
          </button>
        </div>
      </div>
    </div>
    <div class="sidebar-resizer" @mousedown="$emit('resize-start', $event)" title="Resize sidebar"></div>
  </aside>
</template>

<script setup>
import {
  ChevronDown,
  ChevronRight,
  Download,
  FileText,
  Folder,
  FolderOpen,
  HardDrive,
  Home,
  Monitor,
  Music,
  PictureInPicture2,
  Plus,
  Search,
  Trash2,
  Usb,
  Video
} from 'lucide-vue-next';
import { computed, reactive } from 'vue';
import { getVirtualPathLabel, isVirtualPath, normalizePath } from '../lib/virtualPaths';

const emit = defineEmits(['resize-start', 'navigate', 'go-welcome', 'quick-add']);
const props = defineProps({
  sections: {
    type: Array,
    required: true
  },
  currentPath: {
    type: String,
    required: true
  }
});

const collapsed = reactive({});

function toggleSection(title) {
  collapsed[title] = !collapsed[title];
}

function resolveIcon(item) {
  if (item.kind === 'home') return Home;
  if (item.label === 'Desktop') return Monitor;
  if (item.label === 'Downloads') return Download;
  if (item.label === 'Documents') return FileText;
  if (item.label === 'Pictures') return PictureInPicture2;
  if (item.label === 'Music') return Music;
  if (item.label === 'Videos') return Video;
  if (item.kind === 'trash') return Trash2;
  if (item.kind === 'device_removable') return Usb;
  if (item.kind === 'device') return HardDrive;
  return Folder;
}

function isActive(path) {
  return normalizePath(path) === normalizePath(props.currentPath);
}

function goHome() {
  emit('go-welcome');
}

const currentLocationLabel = computed(() => {
  const normalized = normalizePath(props.currentPath);
  if (!normalized) return 'No location';
  if (isVirtualPath(normalized)) {
    return getVirtualPathLabel(normalized) || normalized;
  }
  const parts = normalized.split('/').filter(Boolean);
  if (parts.length === 0) return '/';
  return parts[parts.length - 1];
});
</script>
