import { build } from 'esbuild';
import { Miniflare } from 'miniflare';
import { fileURLToPath } from 'node:url';
export async function startRelay(allowed = 'http://localhost:5173') {
  const entryPoint = fileURLToPath(new URL('../src/worker.js', import.meta.url));
  const bundle = await build({ entryPoints: [entryPoint], bundle: true, write: false, format: 'esm', platform: 'browser' });
  const mf = new Miniflare({ modules: true, script: bundle.outputFiles[0].text, compatibilityDate: '2026-08-01',
    bindings: { ALLOWED_ORIGINS: allowed }, durableObjects: { ROOMS: { className: 'LobbyRoom', useSQLite: true }, DIRECTORY: { className: 'LobbyDirectory', useSQLite: true } } });
  try { await mf.ready; } catch (error) { await mf.dispose(); throw error; }
  return mf;
}
