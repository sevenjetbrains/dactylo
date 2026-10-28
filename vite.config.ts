/** Configuration de Vite pour Tauri (port fixe, cibles des WebView). */

import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath, URL } from 'node:url';

import tailwindcss from '@tailwindcss/vite';
import react from '@vitejs/plugin-react';
import { defineConfig, type Plugin } from 'vite';

/** Hôte fourni par Tauri pour le développement sur appareil distant. */
const host = process.env.TAURI_DEV_HOST;

/** Dossier de pdfjs-dist qui contient ses ressources annexes. */
const PDFJS_ROOT = fileURLToPath(new URL('./node_modules/pdfjs-dist/', import.meta.url));
/** Ressources de pdf.js : polices standard, CMaps (textes asiatiques) et décodeurs wasm. */
const PDFJS_DIRS = ['standard_fonts', 'cmaps', 'wasm'];

/**
 * Sert sous `/pdfjs/` les ressources annexes de pdf.js (ADR 0011) : en développement
 * par le serveur Vite, en production en les copiant dans le paquet. Sans elles, pdf.js
 * ne peut pas dessiner les PDF qui utilisent les polices standard (Helvetica, Times…).
 * Elles sont embarquées : l'import PDF marche hors ligne (NF-03).
 */
function pdfjsAssets(): Plugin {
  return {
    name: 'dactylo-pdfjs-assets',
    configureServer(server) {
      server.middlewares.use('/pdfjs', (req, res, next) => {
        // Chemin demandé, ramené dans le dossier de pdfjs-dist (pas de « ../ » qui en sorte).
        const file = path.normalize(
          path.join(PDFJS_ROOT, decodeURIComponent((req.url ?? '').split('?')[0] ?? '')),
        );
        const allowed = PDFJS_DIRS.some((dir) => file.startsWith(path.join(PDFJS_ROOT, dir)));
        if (!allowed || !existsSync(file) || !statSync(file).isFile()) return next();
        res.end(readFileSync(file));
      });
    },
    generateBundle() {
      // Chaque fichier des trois dossiers devient un fichier du paquet, sous `pdfjs/`.
      for (const dir of PDFJS_DIRS) {
        for (const name of readdirSync(path.join(PDFJS_ROOT, dir))) {
          this.emitFile({
            type: 'asset',
            fileName: `pdfjs/${dir}/${name}`,
            source: readFileSync(path.join(PDFJS_ROOT, dir, name)),
          });
        }
      }
    },
  };
}

// https://v2.tauri.app/start/frontend/vite/
export default defineConfig({
  plugins: [react(), tailwindcss(), pdfjsAssets()],
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
