<template>
  <Teleport to="body">
    <Transition name="ks-fade">
      <div v-if="open" class="ks-backdrop" @mousedown.self="$emit('close')">
        <div class="ks-modal" role="dialog" aria-label="Keyboard Shortcuts">

          <header class="ks-header">
            <span class="ks-title">Keyboard Shortcuts</span>
            <button class="ks-close" @click="$emit('close')">
              <X :size="14" />
            </button>
          </header>

          <div class="ks-body">
            <div v-for="group in groups" :key="group.label" class="ks-group">
              <div class="ks-group-label">{{ group.label }}</div>
              <div class="ks-rows">
                <div v-for="shortcut in group.shortcuts" :key="shortcut.action" class="ks-row">
                  <span class="ks-action">{{ shortcut.action }}</span>
                  <span class="ks-keys">
                    <kbd v-for="(key, i) in shortcut.keys" :key="i" class="ks-key">{{ key }}</kbd>
                  </span>
                </div>
              </div>
            </div>
          </div>

        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup>
import { onMounted, onBeforeUnmount } from 'vue';
import { X } from 'lucide-vue-next';

defineProps({ open: Boolean });
const emit = defineEmits(['close']);

const isMac = navigator.platform.toUpperCase().includes('MAC');
const mod = isMac ? '⌘' : 'Ctrl';
const alt = isMac ? '⌥' : 'Alt';

const groups = [
  {
    label: 'Navigation',
    shortcuts: [
      { action: 'Go Back',                keys: [`${alt} ←`, 'Backspace'] },
      { action: 'Go Forward',             keys: [`${alt} →`] },
      { action: 'Open Search',            keys: ['F6'] },
      { action: 'Refresh',                keys: ['F5', `${mod} R`] },
    ],
  },
  {
    label: 'File Operations',
    shortcuts: [
      { action: 'New File or Folder',     keys: [`${mod} ⇧ N`] },
      { action: 'Rename',                 keys: ['F2'] },
      { action: 'Move to Trash',          keys: ['Delete'] },
      { action: 'Delete Permanently',     keys: ['⇧ Delete'] },
      { action: 'Cut',                    keys: [`${mod} X`] },
      { action: 'Copy',                   keys: [`${mod} C`] },
      { action: 'Paste',                  keys: [`${mod} V`] },
    ],
  },
  {
    label: 'Selection',
    shortcuts: [
      { action: 'Select All',             keys: [`${mod} A`] },
      { action: 'Toggle Selection',       keys: ['Space'] },
      { action: 'Range Select',           keys: ['⇧ ↑ / ↓'] },
      { action: 'Move Cursor',            keys: ['↑  ↓  ←  →'] },
    ],
  },
  {
    label: 'View',
    shortcuts: [
      { action: 'Toggle Hidden Files',    keys: [`${mod} H`] },
      { action: 'Zoom In Grid',           keys: [`${mod} Scroll ↑`] },
      { action: 'Zoom Out Grid',          keys: [`${mod} Scroll ↓`] },
    ],
  },
  {
    label: 'Quick Actions',
    shortcuts: [
      { action: 'Quick Look',             keys: ['⇧ Space'] },
      { action: 'Open / Enter Folder',    keys: ['↩ Enter'] },
      { action: 'Show Properties',        keys: [`${alt} ↩`] },
      { action: 'Close / Dismiss',        keys: ['Esc'] },
    ],
  },
  {
    label: 'Editor',
    shortcuts: [
      { action: 'Save',                   keys: [`${mod} S`] },
      { action: 'Find',                   keys: [`${mod} F`] },
      { action: 'Replace',                keys: [`${mod} H`] },
      { action: 'Find Next',              keys: [`${mod} G`] },
      { action: 'Find Previous',          keys: [`${mod} ⇧ G`] },
      { action: 'Toggle Word Wrap',       keys: [`${alt} Z`] },
      { action: 'Close Editor',           keys: [`${mod} W`] },
    ],
  },
];

function onKeyDown(e) {
  if (e.key === 'Escape') emit('close');
}

onMounted(() => window.addEventListener('keydown', onKeyDown));
onBeforeUnmount(() => window.removeEventListener('keydown', onKeyDown));
</script>

<style scoped>
.ks-backdrop {
  position: fixed;
  inset: 0;
  z-index: 1000;
  background: rgba(0, 0, 0, 0.35);
  display: flex;
  align-items: center;
  justify-content: center;
}

.ks-modal {
  background: var(--panel);
  border: 1px solid var(--line);
  border-radius: 10px;
  width: 560px;
  max-height: 80vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  box-shadow: 0 8px 40px rgba(0, 0, 0, 0.18);
}

.ks-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 16px 12px;
  border-bottom: 1px solid var(--line);
  flex-shrink: 0;
}

.ks-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--ink);
}

.ks-close {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border: none;
  background: transparent;
  border-radius: 4px;
  color: var(--muted);
  cursor: pointer;
  transition: background 0.1s, color 0.1s;
}

.ks-close:hover {
  background: var(--line);
  color: var(--ink);
}

.ks-body {
  overflow-y: auto;
  padding: 12px 16px 16px;
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 20px 24px;
  align-content: start;
}

.ks-group {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.ks-group-label {
  font-size: 10px;
  font-weight: 600;
  color: var(--muted);
  text-transform: uppercase;
  letter-spacing: 0.07em;
  margin-bottom: 2px;
}

.ks-rows {
  display: flex;
  flex-direction: column;
  gap: 0;
}

.ks-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  height: 28px;
  padding: 0 6px;
  border-radius: 4px;
}

.ks-row:hover {
  background: var(--bg);
}

.ks-action {
  font-size: 12px;
  color: var(--ink);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  min-width: 0;
}

.ks-keys {
  display: flex;
  align-items: center;
  gap: 3px;
  flex-shrink: 0;
}

.ks-key {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 22px;
  height: 20px;
  padding: 0 5px;
  background: var(--bg);
  border: 1px solid var(--line);
  border-bottom-width: 2px;
  border-radius: 4px;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
  font-size: 10.5px;
  color: var(--ink);
  white-space: nowrap;
  user-select: none;
  -webkit-user-select: none;
}

/* Transition */
.ks-fade-enter-active,
.ks-fade-leave-active {
  transition: opacity 0.15s ease;
}

.ks-fade-enter-active .ks-modal,
.ks-fade-leave-active .ks-modal {
  transition: transform 0.15s ease, opacity 0.15s ease;
}

.ks-fade-enter-from,
.ks-fade-leave-to {
  opacity: 0;
}

.ks-fade-enter-from .ks-modal,
.ks-fade-leave-to .ks-modal {
  transform: scale(0.96);
  opacity: 0;
}
</style>
