import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import path from 'path'
import { createHash } from 'node:crypto'
import { readFileSync } from 'node:fs'
import process from 'node:process'
import { lanLobbyPlugin } from './scripts/lan/service.mjs'

export default defineConfig(({ mode }) => {
  const runtimeHash = createHash('sha256');
  for (const file of ['engine.js', 'engine_bg.wasm', 'compiler.js', 'compiler_bg.wasm', 'ironsmith.js']) {
    runtimeHash.update(file).update(readFileSync(path.resolve(__dirname, '../wasm_demo/pkg', file)));
  }
  runtimeHash.update(readFileSync(path.resolve(__dirname, 'public/cards/.ironsmith_frontend_cards_checksum')));
  const runtimeVersion = runtimeHash.digest('hex');
  let https;
  if (mode === 'lan' && (process.env.LAN_TLS_CERT || process.env.LAN_TLS_KEY)) {
    if (!process.env.LAN_TLS_CERT || !process.env.LAN_TLS_KEY) throw new Error('Set both LAN_TLS_CERT and LAN_TLS_KEY');
    https = { cert: readFileSync(process.env.LAN_TLS_CERT), key: readFileSync(process.env.LAN_TLS_KEY) };
  }
  return {
    base: './',
    // Concurrent LAN, development, and browser-test servers must not replace
    // each other's optimized React chunks underneath an active module graph.
    cacheDir: path.resolve(__dirname, 'node_modules', process.env.NODE_TEST_CONTEXT
      ? `.vite-test-${process.pid}` : `.vite-${mode}`),
    plugins: [react(), tailwindcss(), ...(mode === 'lan' ? [lanLobbyPlugin()] : [])],
    define: { __IRONSMITH_RUNTIME_VERSION__: JSON.stringify(runtimeVersion), 'import.meta.env.VITE_LAN_LOBBY': JSON.stringify(mode === 'lan' ? 'true' : 'false') },
    resolve: {
      dedupe: ['react', 'react-dom'],
      alias: {
        '@': path.resolve(__dirname, './src'),
      },
    },
    server: {
      https,
      fs: {
        allow: ['..', '../..'],
      },
    },
    preview: { https },
  };
})
