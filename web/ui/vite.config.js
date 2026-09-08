import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import path from 'path'
import { readFileSync } from 'node:fs'
import process from 'node:process'
import { lanLobbyPlugin } from './scripts/lan/service.mjs'

export default defineConfig(({ mode }) => {
  let https;
  if (mode === 'lan' && (process.env.LAN_TLS_CERT || process.env.LAN_TLS_KEY)) {
    if (!process.env.LAN_TLS_CERT || !process.env.LAN_TLS_KEY) throw new Error('Set both LAN_TLS_CERT and LAN_TLS_KEY');
    https = { cert: readFileSync(process.env.LAN_TLS_CERT), key: readFileSync(process.env.LAN_TLS_KEY) };
  }
  return {
    base: './',
    plugins: [react(), tailwindcss(), ...(mode === 'lan' ? [lanLobbyPlugin()] : [])],
    define: { 'import.meta.env.VITE_LAN_LOBBY': JSON.stringify(mode === 'lan' ? 'true' : 'false') },
    resolve: {
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
