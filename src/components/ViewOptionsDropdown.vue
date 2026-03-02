<template>
  <div ref="dropdownWrapRef" class="action-dropdown-wrap">
    <button class="op-btn op-btn-dropdown op-btn-dropdown-icon" @click.stop="toggleMenu" title="View Options">
      <SlidersHorizontal :size="13" />
      <ChevronDown :size="12" />
    </button>
    <div v-if="menuOpen" class="op-dropdown" @click.stop>
      <div class="op-view-toggle-wrap">
        <button
          type="button"
          class="op-view-toggle-btn"
          :class="{ active: viewMode === 'grid' }"
          @click="$emit('update:view-mode', 'grid')"
        >
          <LayoutGrid :size="14" />
          <span>Grid</span>
        </button>
        <button
          type="button"
          class="op-view-toggle-btn"
          :class="{ active: viewMode === 'list' }"
          @click="$emit('update:view-mode', 'list')"
        >
          <List :size="14" />
          <span>List</span>
        </button>
      </div>
      <div class="op-divider"></div>
      <div class="op-sort-section" style="padding-bottom: 2px">
        <span class="op-sort-label">Display</span>
      </div>
      <label class="op-check">
        <input
          class="op-check-input"
          type="checkbox"
          :checked="showHidden"
          @change="$emit('update:show-hidden', $event.target.checked)"
        />
        <span class="op-check-mark" aria-hidden="true"></span>
        <span class="op-check-label">Show Hidden Files</span>
      </label>
      <label class="op-check">
        <input
          class="op-check-input"
          type="checkbox"
          :checked="showExtensions"
          @change="$emit('update:show-extensions', $event.target.checked)"
        />
        <span class="op-check-mark" aria-hidden="true"></span>
        <span class="op-check-label">Show File Extensions</span>
      </label>
      <label class="op-check">
        <input
          class="op-check-input"
          type="checkbox"
          :checked="showSelectionCheckboxes"
          @change="$emit('update:show-selection-checkboxes', $event.target.checked)"
        />
        <span class="op-check-mark" aria-hidden="true"></span>
        <span class="op-check-label">Show Selection Checkboxes</span>
      </label>

      <div class="op-divider"></div>

      <div class="op-sort-section">
        <span class="op-sort-label">Sort by</span>
        <div class="op-sort-group">
          <button
            v-for="field in SORT_FIELDS"
            :key="field.value"
            class="op-sort-btn"
            :class="{ active: sortBy === field.value }"
            @click="$emit('update:sort-by', field.value)"
          >{{ field.label }}</button>
        </div>
        <div class="op-sort-dir">
          <button
            class="op-sort-dir-btn"
            :class="{ active: sortDir === 'asc' }"
            @click="$emit('update:sort-dir', 'asc')"
          >
            <ArrowUpNarrowWide :size="12" />
            Ascending
          </button>
          <button
            class="op-sort-dir-btn"
            :class="{ active: sortDir === 'desc' }"
            @click="$emit('update:sort-dir', 'desc')"
          >
            <ArrowDownWideNarrow :size="12" />
            Descending
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { onBeforeUnmount, onMounted, ref } from 'vue';
import {
  ArrowDownWideNarrow,
  ArrowUpNarrowWide,
  ChevronDown,
  LayoutGrid,
  List,
  SlidersHorizontal,
} from 'lucide-vue-next';

const SORT_FIELDS = [
  { value: 'name',     label: 'Name' },
  { value: 'type',     label: 'Type' },
  { value: 'size',     label: 'Size' },
  { value: 'modified', label: 'Date' },
];

defineProps({
  viewMode: { type: String, required: true },
  showHidden: { type: Boolean, required: true },
  showExtensions: { type: Boolean, required: true },
  showSelectionCheckboxes: { type: Boolean, required: true },
  sortBy: { type: String, default: 'name' },
  sortDir: { type: String, default: 'asc' },
});

defineEmits([
  'update:view-mode',
  'update:show-hidden',
  'update:show-extensions',
  'update:show-selection-checkboxes',
  'update:sort-by',
  'update:sort-dir',
]);

const menuOpen = ref(false);
const dropdownWrapRef = ref(null);

function onWindowClick(e) {
  if (dropdownWrapRef.value && dropdownWrapRef.value.contains(e.target)) return;
  menuOpen.value = false;
}

function toggleMenu() {
  menuOpen.value = !menuOpen.value;
}

onMounted(() => {
  window.addEventListener('click', onWindowClick, true);
});

onBeforeUnmount(() => {
  window.removeEventListener('click', onWindowClick, true);
});
</script>
