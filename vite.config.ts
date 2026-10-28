/** Configuration de Vite pour Tauri (port fixe, cibles des WebView). */

import { fileURLToPath, URL } from 'node:url';

import tailwindcss from '@tailwindcss/vite';
import react from '@vitejs/plugin-react';
import { defineConfig } from 'vite';

/** Hôte fourni par Tauri pour le développement sur appareil distant. */
const host = process.env.TAURI_DEV_HOST;

// https://v2.tauri.app/start/frontend/vite/
export default defineConfig({
  plugins: [react(), tailwindcss()],
  // `@/…` pointe vers `src/`.
  resolve: {
    alias: { '@': fileURLToPath(new URL('./src', import.meta.url)) },
  },
  // Garde visibles les messages de compilation Rust dans le même terminal.
  clearScreen: false,
  // Tauri attend le serveur sur le port 1420 ; le dossier Rust n'est pas surveillé.
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: 'ws', host, port: 1421 } : undefined,
    watch: { ignored: ['**/src-tauri/**', '**/target/**'] },
  },
  envPrefix: ['VITE_', 'TAURI_ENV_*'],
  // Cible la WebView de chaque système : WebView2 (Chromium) sous Windows, WebKit ailleurs.
  build: {
    target: process.env.TAURI_ENV_PLATFORM === 'windows' ? 'chrome111' : 'safari16',
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
  },
});
