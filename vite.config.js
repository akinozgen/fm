import path from 'path';
import { defineConfig } from 'vite';
import vue from '@vitejs/plugin-vue';

export default defineConfig({
  resolve: {
    alias: { '@': path.resolve(__dirname, 'src') }
  },
  plugins: [vue()],
  server: {
    port: 1420,
    strictPort: true
  },
  clearScreen: false
});
