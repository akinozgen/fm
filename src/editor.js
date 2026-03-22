import { createApp } from 'vue';
import EditorWindow from './EditorWindow.vue';

createApp(EditorWindow).mount('#app');

window.addEventListener('contextmenu', (event) => {
  event.preventDefault();
});
