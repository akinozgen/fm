<template>
  <div v-if="isVisible">
    <!-- Leaf or empty container — shown inline, no expand toggle -->
    <div v-if="isLeaf || !childCount" class="jtn-row" :style="rowStyle">
      <span class="jtn-spacer" />
      <template v-if="nodeKey !== null">
        <span class="jtn-key" v-html="hKey" />
        <span class="jtn-colon">:&nbsp;</span>
      </template>
      <span v-if="isLeaf" :class="['jtn-val', `jtn-val--${vType}`]" v-html="hVal" />
      <span v-else class="jtn-bracket jtn-empty">{{ isArray ? '[]' : '{}' }}</span>
    </div>

    <!-- Non-empty container -->
    <template v-else>
      <div class="jtn-row jtn-row--expandable" :style="rowStyle" @click.stop="toggle">
        <span class="jtn-arrow" :class="{ 'jtn-arrow--open': isOpen }">›</span>
        <template v-if="nodeKey !== null">
          <span class="jtn-key" v-html="hKey" />
          <span class="jtn-colon">:&nbsp;</span>
        </template>
        <span class="jtn-bracket">{{ isArray ? '[' : '{' }}</span>
        <span v-if="!isOpen" class="jtn-preview">&thinsp;…&thinsp;{{ childCount }}&thinsp;</span>
        <span v-if="!isOpen" class="jtn-bracket">{{ isArray ? ']' : '}' }}</span>
      </div>
      <template v-if="isOpen">
        <JsonTreeNode
          v-for="(v, k) in value"
          :key="k"
          :node-key="isArray ? Number(k) : String(k)"
          :value="v"
          :depth="depth + 1"
          :search-query="searchQuery"
          :expand-seq="expandSeq"
          :expand-dir="expandDir"
        />
        <div class="jtn-row jtn-row--close" :style="rowStyle">
          <span class="jtn-spacer" />
          <span class="jtn-bracket">{{ isArray ? ']' : '}' }}</span>
        </div>
      </template>
    </template>
  </div>
</template>

<script setup>
import { ref, computed, watch } from 'vue';
import JsonTreeNode from './JsonTreeNode.vue';

const props = defineProps({
  nodeKey:     { default: null },
  value:       { required: true },
  depth:       { type: Number,  default: 0 },
  searchQuery: { type: String,  default: '' },
  expandSeq:   { type: Number,  default: 0 },
  expandDir:   { type: Boolean, default: true },
});

const isLeaf  = computed(() => props.value === null || typeof props.value !== 'object');
const isArray = computed(() => !isLeaf.value && Array.isArray(props.value));

const childCount = computed(() => {
  if (isLeaf.value) return 0;
  return isArray.value ? props.value.length : Object.keys(props.value).length;
});

const vType = computed(() => {
  if (props.value === null) return 'null';
  return typeof props.value;
});

// ── Expand / collapse ─────────────────────────────────────────────────────────
const open = ref(props.depth < 2);
function toggle() { open.value = !open.value; }

watch(() => props.expandSeq, () => {
  if (!isLeaf.value && childCount.value > 0) open.value = props.expandDir;
});

// Auto-expand when a descendant matches the search query
const isOpen = computed(() => {
  if (isLeaf.value || !childCount.value) return false;
  if (props.searchQuery) {
    const lq = props.searchQuery.toLowerCase();
    const entries = isArray.value
      ? [...props.value.entries()]
      : Object.entries(props.value);
    if (entries.some(([k, v]) => nodeMatch(v, String(k), lq))) return true;
  }
  return open.value;
});

// ── Search / filter ───────────────────────────────────────────────────────────
function nodeMatch(val, key, lq) {
  if (key !== null && key.toLowerCase().includes(lq)) return true;
  if (val === null) return 'null'.includes(lq);
  if (typeof val !== 'object') return String(val).toLowerCase().includes(lq);
  const entries = Array.isArray(val) ? [...val.entries()] : Object.entries(val);
  return entries.some(([k, v]) => nodeMatch(v, String(k), lq));
}

const isVisible = computed(() => {
  if (!props.searchQuery) return true;
  const k = props.nodeKey !== null ? String(props.nodeKey) : null;
  return nodeMatch(props.value, k, props.searchQuery.toLowerCase());
});

// ── Display helpers ───────────────────────────────────────────────────────────
const rowStyle = computed(() => ({ paddingLeft: props.depth * 14 + 10 + 'px' }));

function esc(s) {
  return String(s)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');
}

function hl(text, q) {
  const e = esc(text);
  if (!q) return e;
  const re = new RegExp(q.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'gi');
  return e.replace(re, m => `<mark class="jtn-hl">${m}</mark>`);
}

const hKey = computed(() => hl(String(props.nodeKey), props.searchQuery));
const hVal = computed(() => {
  if (props.value === null) return esc('null');
  if (typeof props.value === 'string') return `"${hl(props.value, props.searchQuery)}"`;
  return hl(String(props.value), props.searchQuery);
});
</script>

<style scoped>
.jtn-row {
  display: flex;
  align-items: baseline;
  min-height: 20px;
  padding-right: 14px;
  white-space: nowrap;
  font-size: 12px;
  font-family: 'SFMono-Regular', Menlo, 'Cascadia Code', Consolas, monospace;
  line-height: 1.6;
}

.jtn-row--expandable {
  cursor: pointer;
  border-radius: 3px;
}

.jtn-row--expandable:hover {
  background: rgba(0, 0, 0, 0.04);
}

.jtn-spacer {
  display: inline-block;
  width: 14px;
  flex-shrink: 0;
}

.jtn-arrow {
  display: inline-block;
  width: 14px;
  flex-shrink: 0;
  font-size: 11px;
  color: #bbb;
  transform: rotate(0deg);
  transition: transform 0.12s;
  user-select: none;
  -webkit-user-select: none;
}

.jtn-arrow--open {
  transform: rotate(90deg);
}

.jtn-key {
  color: #7c3aed;
  font-weight: 500;
  flex-shrink: 0;
}

.jtn-colon {
  color: #bbb;
  flex-shrink: 0;
}

.jtn-val--string  { color: #15803d; }
.jtn-val--number  { color: #1d4ed8; }
.jtn-val--boolean { color: #c2410c; }
.jtn-val--null    { color: #9ca3af; font-style: italic; }

.jtn-bracket {
  color: #6b7280;
  font-weight: 600;
  flex-shrink: 0;
}

.jtn-empty {
  opacity: 0.55;
}

.jtn-preview {
  color: #9ca3af;
  font-size: 11px;
  font-style: italic;
  flex-shrink: 0;
}

/* highlight marks rendered via v-html */
:deep(.jtn-hl) {
  background: #fef08a;
  border-radius: 2px;
  padding: 0 1px;
  color: inherit;
  font-style: normal;
}
</style>
