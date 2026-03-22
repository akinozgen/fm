import { createApp } from 'vue';
import QuickLookWindow from './QuickLookWindow.vue';

createApp(QuickLookWindow).mount('#app');

window.addEventListener('contextmenu', (event) => {
  event.preventDefault();
});
