#!/usr/bin/env node
// An MCP client, not an alternate game path. Every setup step uses the same
// observable browser controls available to an interactive player agent.
import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { StreamableHTTPClientTransport } from '@modelcontextprotocol/sdk/client/streamableHttp.js';

const args = process.argv.slice(2);
const option = (name, fallback) => {
  const index = args.indexOf(name);
  if (index < 0) return fallback;
  if (!args[index + 1] || args[index + 1].startsWith('--')) throw new Error(`${name} needs a value`);
  return args[index + 1];
};
const endpoint = option('--url', 'http://127.0.0.1:3847/mcp');
const deckId = option('--deck', 'mtgtop8-90965-890825');
const client = new Client({ name: 'ironsmith-verified-table-setup', version: '0.1.0' });

async function call(name, input = {}) {
  const response = await client.callTool({ name, arguments: input }, undefined, { timeout: 120000 });
  const value = JSON.parse(response.content.find(item => item.type === 'text').text);
  if (response.isError) {
    const error = new Error(`${name}: ${value.error || 'MCP tool failed'}`);
    Object.assign(error, value);
    throw error;
  }
  return value;
}

async function control(playerId, predicate) {
  const deadline = Date.now() + 90000;
  while (Date.now() < deadline) {
    const observation = await call('observe', { playerId });
    if (observation.syncFailures.length) throw new Error(observation.syncFailures.join('\n'));
    const target = observation.controls.find(item => !item.blocked && !item.disabled && predicate(item));
    if (target) return { observation, target };
    await new Promise(resolve => setTimeout(resolve, 500));
  }
  throw new Error(`Timed out waiting for a visible control on ${playerId}`);
}

async function click(playerId, predicate) {
  // A stale observation is safe to re-read during setup. Never retry a sent
  // click after a generic failure: its outcome may already have taken effect.
  for (let attempt = 0; attempt < 4; attempt++) {
    const { observation, target } = await control(playerId, predicate);
    try {
      return await call('act', { playerId, observationId: observation.observationId,
        ref: target.ref, action: 'click' });
    } catch (error) { if (error.code !== 'STALE_OBSERVATION') throw error; }
  }
  throw new Error('The lobby kept changing before the selected setup action');
}

try {
  await client.connect(new StreamableHTTPClientTransport(new URL(endpoint)));
  const table = await call('start_local_table');
  const deck = await call('get_deck', { id: deckId });
  const opened = [];
  for (const name of ['Alice', 'Bob']) {
    opened.push(await call('open_player', { name, deckId, headless: args.includes('--headless') }));
  }
  const [alice, bob] = opened.map(player => player.playerId);
  await click(alice, item => item.tag === 'button' && item.name === 'Create Lobby');
  await click(alice, item => item.tag === 'label' && item.type === 'radio' && item.name.startsWith('Verified'));
  await control(alice, item => item.tag === 'input' && item.value === 'verified' && item.checked);
  await click(alice, item => item.tag === 'button' && item.name === 'Create Lobby');
  const invite = await control(alice, item => item.tag === 'input' && item.name === 'Invite Link' && item.value);
  const lobby = invite.target.value;
  await call('join_lobby', { playerId: bob, lobby });
  await click(alice, item => item.tag === 'button' && item.name === 'Start game');
  console.log(JSON.stringify({ phase: 'starting', securityMode: 'verified', table,
    lobby, players: { Alice: alice, Bob: bob }, deck: { id: deckId, name: deck.name,
      counts: deck.counts, sourceMetadata: deck.sourceMetadata } }, null, 2));
} finally {
  // Disconnecting this client leaves the shared HTTP server and players alive.
  await client.close();
}
