import test from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';
import { request as httpRequest } from 'node:http';
import { mkdtemp, readFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { fileURLToPath } from 'node:url';
import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { StreamableHTTPClientTransport } from '@modelcontextprotocol/sdk/client/streamableHttp.js';
import { StdioClientTransport } from '@modelcontextprotocol/sdk/client/stdio.js';
import { createRuntime, startHttpServer } from './server.mjs';
import { LocalTable } from './local-table.mjs';

const directory = path.dirname(fileURLToPath(import.meta.url));
const expectedTools = ['open_player', 'observe', 'act', 'join_lobby', 'screenshot', 'close_player', 'start_local_table', 'stop_local_table', 'list_decks', 'get_deck'];
const value = result => JSON.parse(result.content.find(item => item.type === 'text').text);

async function connect(url) {
  const client = new Client({ name: 'player-transport-test', version: '0.1.0' });
  await client.connect(new StreamableHTTPClientTransport(new URL(url)));
  return client;
}

test('HTTP SDK initialization, shared runtime, screenshots, errors and loopback guards', async t => {
  const evidenceDir = await mkdtemp(path.join(tmpdir(), 'player-mcp-transport-'));
  const observation = { playerId: 'player-one', observationId: 'observation-2', text: 'Visible game screen' };
  let closed = false;
  let observed = 0;
  const actions = [];
  // This adapter double tests transport preservation only; no gameplay,
  // game state, cryptography, or real player session is simulated here.
  const browser = {
    observe: async () => ({ ...observation, count: ++observed }),
    act: async args => {
      actions.push(args);
      if (args.observationId === 'old') throw Object.assign(new Error('Observation is stale'), { code: 'STALE_OBSERVATION', observation });
      return observation;
    },
    screenshot: async () => ({ mimeType: 'image/png', data: Buffer.from('png-bytes') }),
    close: async () => { closed = true; },
  };
  const service = await startHttpServer({ port: 0, runtime: createRuntime({ evidenceDir, browser }) });
  t.after(() => service.close());
  const first = await connect(service.url);
  t.after(() => first.close());
  const second = await connect(service.url);
  t.after(() => second.close());
  assert.deepEqual((await first.listTools()).tools.map(tool => tool.name), expectedTools);
  assert.equal(value(await first.callTool({ name: 'observe', arguments: { playerId: 'player-one' } })).count, 1);
  assert.equal(value(await second.callTool({ name: 'observe', arguments: { playerId: 'player-one' } })).count, 2);
  const stale = await second.callTool({ name: 'act', arguments: { playerId: 'player-one', observationId: 'old', ref: 'button-1', action: 'click' } });
  assert.equal(stale.isError, true);
  assert.equal(value(stale).code, 'STALE_OBSERVATION');
  assert.deepEqual(value(stale).observation, observation);
  for (const action of ['pointer_click', 'drag']) {
    const arguments_ = { playerId: 'player-one', observationId: observation.observationId, ref: 'surface-1', action, position: { x: 123.5, y: 456 } };
    assert.deepEqual(value(await first.callTool({ name: 'act', arguments: arguments_ })), observation);
    assert.deepEqual(actions.at(-1), arguments_, 'pointer coordinates and observation guards reach the adapter intact');
  }
  const actionCount = actions.length;
  for (const position of [{ x: '123', y: 456 }, { x: -1, y: 456 }, { x: 123 }]) {
    const invalidPosition = await first.callTool({ name: 'act', arguments: { playerId: 'player-one', observationId: observation.observationId, ref: 'surface-1', action: 'drag', position } });
    assert.equal(invalidPosition.isError, true);
  }
  assert.equal(actions.length, actionCount, 'invalid pointer coordinates never reach the browser');
  const image = await first.callTool({ name: 'screenshot', arguments: { playerId: 'player-one' } });
  assert.deepEqual(image.content, [{ type: 'image', mimeType: 'image/png', data: Buffer.from('png-bytes').toString('base64') }]);
  const invalid = await first.callTool({ name: 'act', arguments: { playerId: 'player-one', observationId: 'old', ref: 'button-1', action: 'evaluate' } });
  assert.equal(invalid.isError, true, 'arbitrary browser evaluation is not a supported action');
  const hostStatus = await new Promise((resolve, reject) => {
    const request = httpRequest(service.url, { headers: { Host: 'evil.example' } }, response => { response.resume(); resolve(response.statusCode); });
    request.once('error', reject);
    request.end();
  });
  assert.equal(hostStatus, 403);
  assert.equal((await fetch(service.url, { headers: { Origin: 'https://evil.example' } })).status, 403);
  const records = (await readFile(path.join(evidenceDir, 'tools.jsonl'), 'utf8')).trim().split('\n').map(line => JSON.parse(line));
  assert.ok(records.some(record => record.error?.code === 'STALE_OBSERVATION'));
  assert.ok(records.some(record => record.tool === 'screenshot' && record.result?.bytes === 9));
  assert.equal(closed, false, 'closing a request does not close the shared player runtime');
});

test('stdio SDK handshake and tool calls keep stdout valid JSON-RPC', async t => {
  const client = new Client({ name: 'player-stdio-test', version: '0.1.0' });
  const transport = new StdioClientTransport({ command: process.execPath, args: [path.join(directory, 'server.mjs'), '--stdio'], stderr: 'pipe' });
  t.after(() => client.close());
  await client.connect(transport);
  assert.deepEqual((await client.listTools()).tools.map(tool => tool.name), expectedTools);
  const result = await client.callTool({ name: 'open_player', arguments: { name: 'No table yet' } });
  assert.equal(result.isError, true);
  assert.match(value(result).error, /URL or call start_local_table/);
});

test('catalog and manual Commander sections reach open_player without mixing sources', async t => {
  const evidenceDir = await mkdtemp(path.join(tmpdir(), 'player-mcp-commander-'));
  const deck = { deckText: 'Deck\n99 Plains', commanderText: '1 Éowyn, Shieldmaiden' };
  const opened = [];
  const catalog = {
    getDeck: async ({ id }) => { assert.equal(id, 'commander-deck'); return deck; },
  };
  const browser = {
    openPlayer: async args => { opened.push(args); return { playerId: 'commander-player' }; },
    close: async () => {},
  };
  const service = await startHttpServer({ port: 0, runtime: createRuntime({ evidenceDir, browser, catalog }) });
  t.after(() => service.close());
  const client = await connect(service.url);
  t.after(() => client.close());
  const base = { name: 'Commander player', url: 'http://127.0.0.1:5173/' };
  const imported = await client.callTool({ name: 'open_player', arguments: { ...base, deckId: 'commander-deck' } });
  assert.equal(imported.isError, undefined);
  assert.equal(opened[0].deckText, deck.deckText);
  assert.equal(opened[0].commanderText, deck.commanderText);
  const manual = await client.callTool({ name: 'open_player', arguments: { ...base, ...deck } });
  assert.equal(manual.isError, undefined);
  assert.equal(opened[1].commanderText, deck.commanderText);
  const conflict = await client.callTool({ name: 'open_player', arguments: { ...base, deckId: 'commander-deck', commanderText: '1 A different commander' } });
  assert.equal(conflict.isError, true);
  assert.match(value(conflict).error, /Choose deckId or deckText\/commanderText/);
  assert.equal(opened.length, 2, 'conflicting sections cannot open a player');
});

test('local table serves the normal application with real local PeerJS signaling', { timeout: 30000 }, async t => {
  const keepAlive = setInterval(() => {}, 1000);
  t.after(() => clearInterval(keepAlive));
  const table = new LocalTable({ log() {} });
  t.after(() => table.stop());
  const info = await table.start();
  assert.equal((await table.start()).url, info.url);
  const html = await (await fetch(info.url)).text();
  assert.match(html, /src="\/src\/main\.jsx"/);
  assert.doesNotMatch(html, /peer-lobby-harness|fixtures/);
  const source = await (await fetch(new URL('/src/hooks/peer-lobby/shared.js', info.url))).text();
  assert.match(source, /"VITE_PEER_HOST"\s*:\s*"127\.0\.0\.1"/);
  assert.match(source, new RegExp(`"VITE_PEER_PORT"\\s*:\\s*"${info.peer.port}"`));
  assert.doesNotMatch(source, /"VITE_E2E_TEST"\s*:\s*"true"/);
  const peerId = await (await fetch(`http://127.0.0.1:${info.peer.port}/peerjs/peerjs/id`)).text();
  assert.match(peerId, /^[\w-]+$/);
  // Stop immediately after the first module response, while Vite may still
  // be crawling/prebundling its imports. Only the public stop API may wait.
  const startedStopping = performance.now();
  const stopping = table.stop();
  assert.equal((await stopping).stopped, true);
  assert.ok(performance.now() - startedStopping < 6000, 'shutdown must remain bounded during initial transforms');
  await assert.rejects(fetch(info.url));
  await assert.rejects(fetch(`http://127.0.0.1:${info.peer.port}/peerjs/peerjs/id`));
  const restarted = await table.start();
  assert.equal((await fetch(restarted.url)).status, 200, 'a stopped initial load must allow a fresh table');
  assert.equal((await table.stop()).stopped, true);
  assert.equal((await table.stop()).stopped, false);
});
