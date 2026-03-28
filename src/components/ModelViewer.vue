<template>
  <div ref="container" class="mv-wrap">
    <div v-if="state === 'loading'" class="mv-overlay">Loading…</div>
    <div v-else-if="state === 'error'" class="mv-overlay mv-overlay--err">{{ errorMsg }}</div>
  </div>
</template>

<script setup>
import { ref, onMounted, onBeforeUnmount } from 'vue';
import { convertFileSrc } from '@tauri-apps/api/core';

const props = defineProps({
  path: { type: String, required: true },
});

const container = ref(null);
const state     = ref('loading'); // 'loading' | 'ready' | 'error'
const errorMsg  = ref('');

let renderer = null;
let animId   = null;
let ro       = null;

onMounted(async () => {
  try {
    // Dynamic imports keep Three.js out of non-3D bundles
    const THREE       = await import('three');
    const { GLTFLoader } = await import('three/examples/jsm/loaders/GLTFLoader.js');
    const { OBJLoader }  = await import('three/examples/jsm/loaders/OBJLoader.js');

    const el = container.value;
    const w = el.clientWidth  || 400;
    const h = el.clientHeight || 300;

    // ── Scene ────────────────────────────────────────────────────────────────
    const scene = new THREE.Scene();
    scene.background = new THREE.Color(0x161616);

    const camera = new THREE.PerspectiveCamera(45, w / h, 0.001, 2000);
    camera.position.set(0, 0, 4);

    renderer = new THREE.WebGLRenderer({ antialias: true });
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    renderer.setSize(w, h);
    renderer.outputColorSpace = THREE.SRGBColorSpace;
    el.appendChild(renderer.domElement);

    // ── Lights ───────────────────────────────────────────────────────────────
    scene.add(new THREE.AmbientLight(0xffffff, 0.9));
    const sun = new THREE.DirectionalLight(0xffffff, 1.4);
    sun.position.set(5, 10, 7);
    scene.add(sun);
    const fill = new THREE.DirectionalLight(0x8899bb, 0.5);
    fill.position.set(-5, -5, -5);
    scene.add(fill);

    // ── Load model ───────────────────────────────────────────────────────────
    const src = convertFileSrc(props.path);
    const ext = props.path.split('.').pop().toLowerCase();
    let root;

    if (ext === 'glb' || ext === 'gltf') {
      const loader = new GLTFLoader();
      const gltf = await new Promise((res, rej) => loader.load(src, res, undefined, rej));
      root = gltf.scene;
    } else if (ext === 'obj') {
      const loader = new OBJLoader();
      root = await new Promise((res, rej) => loader.load(src, res, undefined, rej));
      // Apply a default material since .mtl may not be co-located
      root.traverse(child => {
        if (child.isMesh) {
          child.material = new THREE.MeshStandardMaterial({
            color: 0xb0b0b0,
            roughness: 0.6,
            metalness: 0.1,
          });
        }
      });
    }

    if (!root) throw new Error('Could not load model');

    // ── Center and scale to fit ───────────────────────────────────────────────
    const box    = new THREE.Box3().setFromObject(root);
    const center = box.getCenter(new THREE.Vector3());
    const size   = box.getSize(new THREE.Vector3());
    const maxDim = Math.max(size.x, size.y, size.z) || 1;
    const scale  = 2.2 / maxDim;
    root.scale.setScalar(scale);
    root.position.sub(center.multiplyScalar(scale));
    scene.add(root);

    state.value = 'ready';

    // ── Auto-rotate ───────────────────────────────────────────────────────────
    function animate() {
      animId = requestAnimationFrame(animate);
      root.rotation.y += 0.005;
      renderer.render(scene, camera);
    }
    animate();

    // ── Resize ────────────────────────────────────────────────────────────────
    ro = new ResizeObserver(() => {
      const nw = el.clientWidth;
      const nh = el.clientHeight;
      if (!nw || !nh) return;
      camera.aspect = nw / nh;
      camera.updateProjectionMatrix();
      renderer.setSize(nw, nh);
    });
    ro.observe(el);

  } catch (e) {
    state.value = 'error';
    errorMsg.value = String(e);
  }
});

onBeforeUnmount(() => {
  if (animId) cancelAnimationFrame(animId);
  ro?.disconnect();
  renderer?.dispose();
});
</script>

<style scoped>
.mv-wrap {
  width: 100%;
  height: 100%;
  position: relative;
  background: #161616;
  overflow: hidden;
}

.mv-overlay {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
  font-size: 12px;
  color: #555;
  pointer-events: none;
}

.mv-overlay--err {
  color: #ff6b6b;
  padding: 16px;
  text-align: center;
}
</style>
