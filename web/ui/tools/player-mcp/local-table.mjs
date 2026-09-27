import { spawn } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createServer as createViteServer } from 'vite';

const modulePath = fileURLToPath(import.meta.url);
export const UI_ROOT = path.resolve(path.dirname(modulePath), '../..');
export const REPO_ROOT = path.resolve(UI_ROOT, '../..');
const SHUTDOWN_GRACE_MS = 3000;

const peerSource = `
import { PeerServer } from 'peer';
const peer = PeerServer({host:'127.0.0.1',port:0,path:'/peerjs',key:'peerjs',allow_discovery:false},
  server => process.send({type:'ready',port:server.address().port}));
peer.on('error', error => { console.error(error.stack || error); process.exit(1); });
process.on('SIGTERM', () => process.exit(0));
process.on('disconnect', () => process.exit(0));
`;

async function stopService(child) {
  if (!child || child.exitCode !== null || child.signalCode !== null) return false;
  return new Promise(resolve => {
    let forced = false;
    const timer = setTimeout(() => { forced = true; child.kill('SIGKILL'); }, SHUTDOWN_GRACE_MS);
    child.once('exit', () => { clearTimeout(timer); resolve(forced); });
    child.kill('SIGTERM');
  });
}

async function startService(args, label, log) {
  const child = spawn(process.execPath, args, { cwd: UI_ROOT, stdio: ['ignore', 'pipe', 'pipe', 'ipc'] });
  child.stdout.on('data', data => log(`[${label}] ${String(data).trim()}`));
  child.stderr.on('data', data => log(`[${label}] ${String(data).trim()}`));
  try {
    const ready = await new Promise((resolve, reject) => {
      const timer = setTimeout(() => fail(new Error(`${label} did not start within 15 seconds`)), 15000);
      const cleanup = () => {
        clearTimeout(timer);
        child.off('error', fail);
        child.off('exit', exited);
        child.off('message', message);
      };
      const fail = error => { cleanup(); reject(error); };
      const exited = code => fail(new Error(`${label} exited during startup (${code})`));
      const message = value => {
        if (value?.type !== 'ready') return;
        cleanup();
        resolve(value);
      };
      child.once('error', fail);
      child.once('exit', exited);
      child.on('message', message);
    });
    return { child, ...ready };
  } catch (error) {
    await stopService(child);
    throw error;
  }
}

async function runViteChild(port, peerPort) {
  const environment = {
    VITE_PEER_HOST: '127.0.0.1', VITE_PEER_PORT: String(peerPort),
    VITE_PEER_PATH: '/peerjs', VITE_PEER_KEY: 'peerjs', VITE_PEER_SECURE: 'false',
  };
  const log = message => console.error(message);
  const logger = { info() {}, warn: log, warnOnce: log, error: log,
    clearScreen() {}, hasWarned: false, hasErrorLogged: () => false };
  const vite = await createViteServer({
    root: UI_ROOT, configFile: path.join(UI_ROOT, 'vite.config.js'), mode: 'development',
    logLevel: 'error', customLogger: logger,
    cacheDir: path.join(UI_ROOT, 'node_modules', `.vite-player-mcp-${process.pid}`),
    define: Object.fromEntries(Object.entries(environment).map(([key, value]) => [`import.meta.env.${key}`, JSON.stringify(value)])),
    server: { host: '127.0.0.1', port, strictPort: port !== 0, hmr: false },
  });
  if (vite.config.env.VITE_E2E_TEST === 'true') {
    throw new Error('Local player tables require the normal app; remove VITE_E2E_TEST from the environment');
  }
  await vite.listen();
  process.send({ type: 'ready', url: `http://127.0.0.1:${vite.httpServer.address().port}/` });
  const close = () => { void vite.close().finally(() => process.exit(0)); };
  process.once('SIGTERM', close);
  process.once('disconnect', () => {
    // If the MCP host itself dies, no parent remains to enforce its deadline.
    setTimeout(() => process.exit(0), SHUTDOWN_GRACE_MS);
    close();
  });
}

export class LocalTable {
  constructor({ log = message => console.error(message) } = {}) {
    this.log = log;
    this.current = null;
    this.starting = null;
    this.stopping = null;
  }

  async start({ port = 0 } = {}) {
    if (this.stopping) await this.stopping;
    if (this.current) return this.current.info;
    if (this.starting) return this.starting;
    this.starting = this.startNew(port).finally(() => { this.starting = null; });
    return this.starting;
  }

  async startNew(port) {
    const peer = await startService(['--input-type=module', '-e', peerSource], 'peer', this.log);
    let vite;
    try {
      // Vite.close can wait indefinitely for an initial transform/dependency
      // crawl. Isolating it lets stop terminate every watcher and compiler
      // resource after a bounded grace period without leaving an old optimizer
      // racing the next table's dependency cache.
      vite = await startService([modulePath, '--vite-child', String(port), String(peer.port)], 'vite', this.log);
      const info = {
        url: vite.url,
        peer: { host: '127.0.0.1', port: peer.port, path: '/peerjs', key: 'peerjs', secure: false },
        app: 'normal', crypto: 'real',
      };
      this.current = { vite, peer, info };
      this.log(`[table] ${info.url}`);
      return info;
    } catch (error) {
      await Promise.all([stopService(vite?.child), stopService(peer.child)]);
      throw error;
    }
  }

  stop() {
    if (this.stopping) return this.stopping;
    this.stopping = this.stopCurrent().finally(() => { this.stopping = null; });
    return this.stopping;
  }

  async stopCurrent() {
    if (this.starting) await this.starting.catch(() => {});
    const current = this.current;
    this.current = null;
    if (!current) return { stopped: false };
    const forced = (await Promise.all([stopService(current.vite.child), stopService(current.peer.child)])).some(Boolean);
    if (forced) this.log('[table] Graceful shutdown timed out; terminated the remaining child process');
    return { stopped: true, url: current.info.url, forced };
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === modulePath && process.argv[2] === '--vite-child') {
  runViteChild(Number(process.argv[3]), Number(process.argv[4])).catch(error => {
    console.error(error.stack || error);
    process.exit(1);
  });
}
