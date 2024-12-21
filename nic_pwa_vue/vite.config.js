import { defineConfig } from 'vite';
import vue from '@vitejs/plugin-vue';
import { VitePWA } from 'vite-plugin-pwa';

export default defineConfig({
  plugins: [
    vue(),
    VitePWA({
      manifest: {
        name: 'New Irrigation Controller',
        short_name: 'NIC',
        start_url: '/',
        display: 'standalone',
        icons: [{ src: 'icon.png', sizes: '192x192', type: 'image/png' }],
      },
    }),
  ],
});