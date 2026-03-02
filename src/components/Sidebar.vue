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
          <div
            v-for="item in section.items"
            :key="item.path"
            class="tree-item"
            :class="{ active: isActive(item.path), 'tree-item-dragging': draggedPinnedPath === item.path, 'tree-item-drop-target': dropTargetPath === item.path }"
            role="button"
            tabindex="0"
            :draggable="item.kind === 'pinned'"
            @click="$emit('navigate', item.path)"
            @contextmenu.prevent="onItemContextMenu($event, item)"
            @keydown.enter.prevent="$emit('navigate', item.path)"
            @keydown.space.prevent="$emit('navigate', item.path)"
            @dragstart="item.kind === 'pinned' && onPinnedDragStart($event, item, section)"
            @dragend="onPinnedDragEnd"
            @dragover.prevent="item.kind === 'pinned' && onPinnedDragOver($event, item)"
            @dragleave="onPinnedDragLeave($event, item)"
            @drop.prevent="item.kind === 'pinned' && onPinnedDrop($event, item, section)"
          >
            <component :is="resolveIcon(item)" :size="14" />
            <span class="tree-item-label">{{ driveDisplayText(item) }}</span>
            <button
              v-if="item.kind === 'pinned'"
              type="button"
              class="tree-item-unpin"
              title="Unpin from Favorites"
              @click.stop="$emit('unpin', item.path)"
            >
              <PinOff :size="12" />
            </button>
          </div>
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
  PinOff,
  Trash2,
  Usb,
  Video
} from 'lucide-vue-next';
import { reactive, ref } from 'vue';
import { isVirtualPath, normalizePath } from '../lib/virtualPaths';
import { showNativeFileContextMenu } from '../lib/contextMenu';

const emit = defineEmits(['resize-start', 'navigate', 'unpin', 'reorder-pinned']);
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
const draggedPinnedPath = ref(null);
const dropTargetPath = ref(null);

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
    paths: [item.path],
    isPinned: item.kind === 'pinned'
  });
}

function pinnedPathsInOrder(section) {
  return (section.items || []).filter((i) => i.kind === 'pinned').map((i) => i.path);
}

function onPinnedDragStart(event, item, section) {
  draggedPinnedPath.value = item.path;
  if (event.dataTransfer) {
    event.dataTransfer.effectAllowed = 'move';
    event.dataTransfer.setData('text/plain', item.path);
    event.dataTransfer.setData('application/x-fm-pinned-path', item.path);
  }
}

function onPinnedDragEnd() {
  draggedPinnedPath.value = null;
  dropTargetPath.value = null;
}

function onPinnedDragOver(event, item) {
  if (!draggedPinnedPath.value || item.path === draggedPinnedPath.value) return;
  dropTargetPath.value = item.path;
  if (event.dataTransfer) event.dataTransfer.dropEffect = 'move';
}

function onPinnedDragLeave(event, item) {
  if (event.relatedTarget && event.currentTarget?.contains?.(event.relatedTarget)) return;
  dropTargetPath.value = null;
}

function onPinnedDrop(event, dropItem, section) {
  dropTargetPath.value = null;
  const path = draggedPinnedPath.value;
  draggedPinnedPath.value = null;
  if (!path || path === dropItem.path) return;
  const order = pinnedPathsInOrder(section);
  const fromIdx = order.indexOf(path);
  const toIdx = order.indexOf(dropItem.path);
  if (fromIdx === -1 || toIdx === -1) return;
  const next = order.slice();
  next.splice(fromIdx, 1);
  next.splice(toIdx, 0, path);
  emit('reorder-pinned', next);
}
</script>
