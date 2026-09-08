import { build } from 'esbuild';
import { Miniflare } from 'miniflare';
export async function startRelay(allowed = 'http://localhost:5173') {
  const bundle = await build({ entryPoints: [new URL('../src/worker.js', import.meta.url).pathname], bundle: true, write: false, format: 'esm', platform: 'browser' });
  const mf = new Miniflare({ modules: true, script: bundle.outputFiles[0].text, compatibilityDate: '2026-08-01',
    bindings: { ALLOWED_ORIGINS: allowed }, durableObjects: { ROOMS: { className: 'LobbyRoom', useSQLite: true }, DIRECTORY: { className: 'LobbyDirectory', useSQLite: true } } });
  try { await mf.ready; } catch (error) { await mf.dispose(); throw error; }
  return mf;
}
