#!/usr/bin/env node
import { createServer as createHttpServer } from 'node:http';
import { appendFile, mkdir } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { StreamableHTTPServerTransport } from '@modelcontextprotocol/sdk/server/streamableHttp.js';
import { z } from 'zod';
import { LocalTable, REPO_ROOT } from './local-table.mjs';

const VERSION = '0.1.0';
const playerId = z.string().min(1).describe('Player identifier returned by open_player');
const textResult = value => ({ content: [{ type: 'text', text: JSON.stringify(value) }] });

export function createRuntime({ evidenceDir, browser, catalog } = {}) {
  const dir = path.resolve(evidenceDir || path.join(REPO_ROOT, 'reports/player-mcp', `${new Date().toISOString().replaceAll(':', '-')}-${process.pid}`));
  const table = new LocalTable();
  let browserPromise = browser ? Promise.resolve(browser) : null;
  let journal = Promise.resolve();
  return {
    table, evidenceDir: dir,
    async catalog() { return catalog || import('./decks.mjs'); },
    async browser() {
      if (!browserPromise) browserPromise = import('./browser.mjs').then(async ({ PlayerBrowser }) => {
        await mkdir(dir, { recursive: true });
        return new PlayerBrowser({ evidenceDir: dir });
      }).catch(error => { browserPromise = null; throw error; });
      return browserPromise;
    },
    record(entry) {
      journal = journal.then(async () => {
        await mkdir(dir, { recursive: true });
        await appendFile(path.join(dir, 'tools.jsonl'), `${JSON.stringify({ time: new Date().toISOString(), ...entry })}\n`);
      }).catch(error => console.error(`[evidence] ${error.message}`));
      return journal;
    },
    async close() {
      await (await browserPromise?.catch(() => null))?.close();
      await table.stop();
      await journal;
    },
  };
}

export function createPlayerServer(runtime) {
  const server = new McpServer({ name: 'ironsmith-player', version: VERSION }, {
    instructions: 'Play only through visible controls. Observe before acting; act requires a current observationId and a ref from it. Each player has an isolated browser context. Local tables use the normal app and real multiplayer crypto. Evidence contains only these tool inputs, visible observations, screenshots and normal UI downloads.',
  });
  const register = (name, description, inputSchema, run, { image = false, readOnly = false } = {}) => {
    server.registerTool(name, { description, inputSchema, annotations: { readOnlyHint: readOnly } }, async args => {
      const callId = crypto.randomUUID();
      await runtime.record({ callId, tool: name, input: args });
      try {
        const result = await run(args);
        await runtime.record({ callId, tool: name, result: image ? { mimeType: result.mimeType, bytes: result.data.length } : result });
        return image ? { content: [{ type: 'image', mimeType: result.mimeType, data: Buffer.from(result.data).toString('base64') }] } : textResult(result);
      } catch (error) {
        const failure = { error: error.message || String(error), ...(error.code ? { code: error.code } : {}), ...(error.observation ? { observation: error.observation } : {}) };
        await runtime.record({ callId, tool: name, error: failure });
        return { ...textResult(failure), isError: true };
      }
    });
  };
  register('open_player', 'Open an isolated visible browser player. Optionally preload a deck through the normal UI. URL defaults to the running local table.', {
    name: z.string().min(1).max(100), url: z.url().optional(), deckId: z.string().min(1).optional(), deckText: z.string().max(100000).optional(), commanderText: z.string().max(10000).optional(), headless: z.boolean().default(false),
  }, async args => {
    const url = args.url || runtime.table.current?.info.url;
    if (!url) throw new Error('Pass a URL or call start_local_table first');
    if (args.deckId && (args.deckText !== undefined || args.commanderText !== undefined)) throw new Error('Choose deckId or deckText/commanderText, not both');
    const deck = args.deckId ? await (await runtime.catalog()).getDeck({ id: args.deckId }) : args;
    return (await runtime.browser()).openPlayer({ ...args, url, deckText: deck.deckText, commanderText: deck.commanderText });
  });
  register('observe', 'Read this player’s visible UI and fresh control references.', { playerId }, async args => (await runtime.browser()).observe(args), { readOnly: true });
  register('act', 'Use a visible control from the latest observation. pointer_click clicks a viewport position inside the referenced control; drag moves the referenced control to a viewport position. Both require position. A stale observation returns a fresh observation without acting.', {
    playerId, observationId: z.string().min(1), ref: z.string().min(1), action: z.enum(['click', 'fill', 'select', 'press', 'hover', 'pointer_click', 'drag']), value: z.string().optional(),
    position: z.object({ x: z.number().nonnegative(), y: z.number().nonnegative() }).optional(),
  }, async args => (await runtime.browser()).act(args));
  register('join_lobby', 'Join a lobby through the visible app UI.', { playerId, lobby: z.string().min(1) }, async args => (await runtime.browser()).joinLobby(args));
  register('screenshot', 'Capture only this player’s visible browser viewport.', { playerId }, async args => (await runtime.browser()).screenshot(args), { image: true, readOnly: true });
  register('close_player', 'Close one isolated player browser.', { playerId }, async args => (await runtime.browser()).closePlayer(args));
  register('start_local_table', 'Start the real app on loopback with local PeerJS signaling and actual WASM crypto. Idempotent while running.', {
    port: z.number().int().min(0).max(65535).default(0),
  }, async args => ({ ...await runtime.table.start(args), evidenceDir: runtime.evidenceDir }));
  register('stop_local_table', 'Stop the local UI and signaling servers. Close players first after finishing the game.', {}, () => runtime.table.stop());
  register('list_decks', 'List reusable playable decks from the local catalog.', {
    format: z.string().optional(), query: z.string().optional(), limit: z.number().int().min(1).max(100).optional(),
  }, async args => (await runtime.catalog()).listDecks(args), { readOnly: true });
  register('get_deck', 'Load a listed deck with its source metadata and deck text.', { id: z.string().min(1) }, async args => (await runtime.catalog()).getDeck(args), { readOnly: true });
  return server;
}

function json(response, status, value) {
  response.writeHead(status, { 'content-type': 'application/json' });
  response.end(JSON.stringify(value));
}

export async function startHttpServer({ port = 3847, runtime = createRuntime() } = {}) {
  const connections = new Set();
  const http = createHttpServer(async (request, response) => {
    try {
      const host = new URL(`http://${request.headers.host || ''}`);
      if (!['127.0.0.1', 'localhost'].includes(host.hostname)) return json(response, 403, { error: 'Loopback Host header required' });
      if (request.headers.origin && new URL(request.headers.origin).host !== host.host) return json(response, 403, { error: 'Cross-origin requests are disabled' });
      if (request.url === '/health' && request.method === 'GET') return json(response, 200, { name: 'ironsmith-player', version: VERSION, evidenceDir: runtime.evidenceDir });
      if (request.url !== '/mcp') return json(response, 404, { error: 'Use /mcp' });
      const server = createPlayerServer(runtime);
      const transport = new StreamableHTTPServerTransport({ sessionIdGenerator: undefined, enableJsonResponse: true });
      connections.add(server);
      response.on('close', () => { connections.delete(server); void server.close(); });
      await server.connect(transport);
      await transport.handleRequest(request, response);
    } catch (error) {
      console.error(`[http] ${error.message}`);
      if (!response.headersSent) json(response, 500, { error: error.message });
      else response.end();
    }
  });
  await new Promise((resolve, reject) => {
    http.once('error', reject);
    http.listen(port, '127.0.0.1', resolve);
  });
  return {
    url: `http://127.0.0.1:${http.address().port}/mcp`, runtime,
    async close() {
      await Promise.all([...connections].map(server => server.close()));
      await runtime.close();
      await new Promise(resolve => http.close(resolve));
    },
  };
}

async function main() {
  const args = process.argv.slice(2);
  const portIndex = args.indexOf('--port');
  const evidenceIndex = args.indexOf('--evidence-dir');
  const runtime = createRuntime({ evidenceDir: evidenceIndex >= 0 ? args[evidenceIndex + 1] : undefined });
  let close;
  if (args.includes('--http')) {
    const port = Number(portIndex >= 0 ? args[portIndex + 1] : process.env.PLAYER_MCP_PORT || 3847);
    if (!Number.isInteger(port) || port < 0 || port > 65535) throw new Error('Invalid HTTP port');
    const http = await startHttpServer({ port, runtime });
    console.error(`[player-mcp] ${http.url}`);
    console.error(`[player-mcp] evidence: ${runtime.evidenceDir}`);
    close = () => http.close();
  } else {
    const server = createPlayerServer(runtime);
    await server.connect(new StdioServerTransport());
    close = async () => { await runtime.close(); await server.close(); };
    process.stdin.once('end', () => { void close().finally(() => process.exit(0)); });
  }
  for (const signal of ['SIGINT', 'SIGTERM']) process.once(signal, () => { void close().finally(() => process.exit(0)); });
}
if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main().catch(error => { console.error(error.stack || error); process.exitCode = 1; });
}
