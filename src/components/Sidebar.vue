<template>
  <aside class="sidebar">
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
            @contextmenu.prevent="onItemContextMenu($event, item)"
          >
            <component :is="resolveIcon(item)" :size="14" />
            <span>{{ driveDisplayText(item) }}</span>
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
  HardDrive,
  Home,
  Monitor,
  Music,
  PictureInPicture2,
  Trash2,
  Usb,
  Video
} from 'lucide-vue-next';
import { reactive } from 'vue';
import { isVirtualPath, normalizePath } from '../lib/virtualPaths';
import { showNativeFileContextMenu } from '../lib/contextMenu';

const emit = defineEmits(['resize-start', 'navigate']);
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

function driveDisplayText(item) {
  if (item.kind === 'device' || item.kind === 'device_removable') {
    return `${item.path} ${item.label}`.trim();
  }
  return item.label;
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

function onItemContextMenu(event, item) {
  if (isVirtualPath(item.path)) return;
  void showNativeFileContextMenu({
    x: event.clientX,
    y: event.clientY,
    kind: 'sidebar_item',
    paths: [item.path]
  });
}
</script>
