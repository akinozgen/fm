<template>
  <div ref="dropdownWrapRef" class="action-dropdown-wrap">
    <button class="op-btn op-btn-dropdown op-btn-dropdown-icon" title="View Options" @click.stop="toggleMenu">
      <SlidersHorizontal :size="13" />
      <ChevronDown :size="12" />
    </button>
    <div v-if="menuOpen" class="op-dropdown" @click.stop>

      <!-- View mode: compact inline segmented control -->
      <div class="op-row">
        <span class="op-row-label">View</span>
        <div class="op-seg">
          <button
            type="button"
            class="op-seg-btn"
            :class="{ active: viewMode === 'grid' }"
            @click="$emit('update:view-mode', 'grid')"
          >
            <LayoutGrid :size="11" />
            Grid
          </button>
          <button
            type="button"
            class="op-seg-btn"
            :class="{ active: viewMode === 'list' }"
            @click="$emit('update:view-mode', 'list')"
          >
            <List :size="11" />
            List
          </button>
        </div>
      </div>

      <!-- Grid size (only shown in grid mode) -->
      <div v-if="viewMode === 'grid'" class="op-row">
        <span class="op-row-label">Size</span>
        <div class="op-seg">
          <button
            type="button"
            class="op-seg-btn"
            :class="{ active: gridSizeLabel === 'S' }"
            @click="$emit('update:grid-zoom', GRID_SIZE_S)"
          >S</button>
          <button
            type="button"
            class="op-seg-btn"
            :class="{ active: gridSizeLabel === 'M' }"
            @click="$emit('update:grid-zoom', GRID_SIZE_M)"
          >M</button>
          <button
            type="button"
            class="op-seg-btn"
            :class="{ active: gridSizeLabel === 'L' }"
            @click="$emit('update:grid-zoom', GRID_SIZE_L)"
          >L</button>
        </div>
      </div>

      <div class="op-divider"></div>

      <!-- Display toggles -->
      <label class="op-toggle-row">
        <input type="checkbox" class="op-check-input" :checked="showHidden" @change="$emit('update:show-hidden', $event.target.checked)" />
        <span class="op-toggle-label">Hidden Files</span>
        <span class="op-switch" aria-hidden="true"></span>
      </label>
      <label class="op-toggle-row">
        <input type="checkbox" class="op-check-input" :checked="showExtensions" @change="$emit('update:show-extensions', $event.target.checked)" />
        <span class="op-toggle-label">File Extensions</span>
        <span class="op-switch" aria-hidden="true"></span>
      </label>
      <label class="op-toggle-row">
        <input type="checkbox" class="op-check-input" :checked="showSelectionCheckboxes" @change="$emit('update:show-selection-checkboxes', $event.target.checked)" />
        <span class="op-toggle-label">Selection Checkboxes</span>
        <span class="op-switch" aria-hidden="true"></span>
      </label>

      <div class="op-divider"></div>

      <!-- Sort field -->
      <div class="op-row">
        <span class="op-row-label">Sort</span>
        <div class="op-seg op-seg-sort">
          <button
            v-for="field in SORT_FIELDS"
            :key="field.value"
            class="op-seg-btn"
            :class="{ active: sortBy === field.value }"
            @click="$emit('update:sort-by', field.value)"
          >{{ field.label }}</button>
        </div>
      </div>

      <!-- Sort direction -->
      <div class="op-row">
        <span class="op-row-label">Order</span>
        <div class="op-seg">
          <button
            class="op-seg-btn"
            :class="{ active: sortDir === 'asc' }"
            @click="$emit('update:sort-dir', 'asc')"
          >
            <ArrowUpNarrowWide :size="11" />
            Asc
          </button>
          <button
            class="op-seg-btn"
            :class="{ active: sortDir === 'desc' }"
            @click="$emit('update:sort-dir', 'desc')"
          >
            <ArrowDownWideNarrow :size="11" />
            Desc
          </button>
        </div>
      </div>

    </div>
  </div>
</template>

<script setup>
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
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

const GRID_SIZE_S = 88;
const GRID_SIZE_M = 128;
const GRID_SIZE_L = 176;

const props = defineProps({
  viewMode: { type: String, required: true },
  gridZoom: { type: Number, default: 110 },
  showHidden: { type: Boolean, required: true },
  showExtensions: { type: Boolean, required: true },
  showSelectionCheckboxes: { type: Boolean, required: true },
  sortBy: { type: String, default: 'name' },
  sortDir: { type: String, default: 'asc' },
});

defineEmits([
  'update:view-mode',
  'update:grid-zoom',
  'update:show-hidden',
  'update:show-extensions',
  'update:show-selection-checkboxes',
  'update:sort-by',
  'update:sort-dir',
]);

// Map continuous zoom value to the nearest S/M/L bucket for active indicator
const gridSizeLabel = computed(() => {
  if (props.gridZoom < 108) return 'S';
  if (props.gridZoom < 152) return 'M';
  return 'L';
});

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
