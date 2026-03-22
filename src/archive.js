import { createApp } from 'vue';
import ArchiveWindow from './ArchiveWindow.vue';

createApp(ArchiveWindow).mount('#app');

window.addEventListener('contextmenu', (e) => e.preventDefault());
