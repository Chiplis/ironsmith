import test from 'node:test';
import assert from 'node:assert/strict';
import { createServer } from 'vite';
import { chromium } from 'playwright';
import { fileURLToPath } from 'node:url';
import { startRelay } from '../../relay/tests/runtime.mjs';

async function setup(t) {
  // Fixed test-only origin permits exercising the real CORS and WebSocket checks.
  const port = Number(process.env.RELAY_TEST_PORT || 5189);
  const base = `http://127.0.0.1:${port}`;
  const relay = await startRelay(base); t.after(() => relay.dispose());
  const url = String(await relay.ready).replace(/\/$/, '');
  const server = await createServer({ root: fileURLToPath(new URL('..', import.meta.url)),
    define: { 'import.meta.env.VITE_LOBBY_RELAY_URL': JSON.stringify(url) },
    server: { host: '127.0.0.1', port, strictPort: true }, logLevel: 'error' });
  await server.listen(); t.after(() => server.close());
  const browser = await chromium.launch({ headless: true }); t.after(() => browser.close());
  const pages = [];
  for (let i = 0; i < 2; i++) {
    const context = await browser.newContext();
    const page = await context.newPage();
    page.on('pageerror', e => console.error('Browser error:', e.message));
    await page.addInitScript(() => {
      const NativeWebSocket = window.WebSocket; window.testSockets = [];
      window.WebSocket = class extends NativeWebSocket { constructor(...args) { super(...args); window.testSockets.push(this); } };
    });
    await page.goto(`${base}/tests/fixtures/peer-lobby-harness.html`);
    await page.waitForFunction(() => window.__peerHarness?.ready);
    pages.push(page);
  }
  return { pages, url, base, browser };
}
async function wait(page, predicate, arg) {
  const end = Date.now() + 20000;
  while (Date.now() < end) {
    if (await page.evaluate(predicate, arg)) return;
    await new Promise(r => setTimeout(r, 50));
  }
  console.error(await page.evaluate(async () => { const s = await window.__peerHarness.lobbyState(); return { mode: s.multiplayer.mode, statuses: s.statusEvents, notices: s.noticeEvents }; }));
  assert.fail('Timed out waiting for lobby state');
}
test('WebSocket lobby validates decks and resumes guest and host after socket drops and refreshes', { timeout: 90000 }, async t => {
  const { pages: [host, guest], url } = await setup(t);
  guest.on('websocket', socket => { socket.on('framereceived', ({payload}) => { if (typeof payload === 'string' && /error|reject/.test(payload)) console.error('RELAY FRAME', payload.slice(0, 1800)); }); });
  await host.evaluate(() => window.__peerHarness.createLobby({ name: 'Modern table', desiredPlayers: 4,
    startingLife: 40, format: 'modern', transport: 'websocket', deckText: '60 Plains' }));
  await wait(host, async () => (await window.__peerHarness.lobbyState()).multiplayer.mode === 'lobby');
  const session = await host.evaluate(async () => (await window.__peerHarness.lobbyState()).multiplayer);
  assert.equal(session.startingLife, 20); assert.equal(session.desiredPlayers, 2);
  await wait(host, async url => (await (await fetch(`${url}/lobbies`)).json()).lobbies.length === 1, url);
  await guest.evaluate(lobbyId => window.__peerHarness.joinLobby({ name: 'Guest', lobbyId, deckText: '59 Island\n1 Sol Ring' }), session.lobbyId);
  await wait(host, async () => (await window.__peerHarness.lobbyState()).multiplayer.players.length === 2);
  await new Promise(r => setTimeout(r, 500));
  const afterJoin = await host.evaluate(() => window.__peerHarness.lobbyState());
  if (afterJoin.multiplayer.players.length !== 2) console.error('JOIN SNAPSHOTS', JSON.stringify(afterJoin), JSON.stringify(await guest.evaluate(() => window.__peerHarness.lobbyState())));
  assert.equal(afterJoin.multiplayer.players[1]?.ready, false);
  await guest.evaluate(() => window.__peerHarness.updateLobbyDeck({ deckText: '61 Island' }));
  await wait(host, async () => (await window.__peerHarness.lobbyState()).multiplayer.players.every(p => p.ready));
  await host.evaluate(() => window.__peerHarness.startHostedMatch());
  for (const page of [host, guest]) await wait(page, async () => (await window.__peerHarness.lobbyState()).multiplayer.matchStarted);
  await host.evaluate(() => window.__peerHarness.submitMultiplayerCommand({ type: 'priority_action', action_ref: { kind: 'test_priority_action', actor: 0, sequence: 0 } }, 'Relay action'));
  for (const page of [host, guest]) await wait(page, async () => (await window.__peerHarness.lobbyState()).multiplayer.lastAppliedSequence >= 1);
  await guest.evaluate(() => window.testSockets.filter(s => s.url.includes('/rooms/')).forEach(s => s.close()));
  await wait(host, async () => (await window.__peerHarness.lobbyState()).multiplayer.players.some(p => p.connected === false));
  try { await wait(host, async () => (await window.__peerHarness.lobbyState()).multiplayer.players.every(p => p.connected !== false)); } catch(e) { console.error('GUEST STATUS', await guest.evaluate(async () => (await window.__peerHarness.lobbyState()).statusEvents)); throw e; }
  await guest.evaluate(() => window.__peerHarness.submitMultiplayerCommand({ type: 'priority_action', action_ref: { kind: 'test_priority_action', actor: 1, sequence: 1 } }, 'After reconnect'));
  for (const page of [host, guest]) await wait(page, async () => (await window.__peerHarness.lobbyState()).multiplayer.lastAppliedSequence >= 2);
  await host.evaluate(() => window.testSockets.filter(s => s.url.includes('/rooms/')).forEach(s => s.close()));
  await wait(guest, async () => (await window.__peerHarness.lobbyState()).multiplayer.players.some(p => p.connected === false));
  try { await wait(guest, async () => (await window.__peerHarness.lobbyState()).multiplayer.players.every(p => p.connected !== false)); } catch(e) { console.error('HOST STATUS', await host.evaluate(() => window.__peerHarness.lobbyState().statusEvents)); throw e; }
  await host.evaluate(() => window.__peerHarness.submitMultiplayerCommand({ type: 'priority_action', action_ref: { kind: 'test_priority_action', actor: 0, sequence: 2 } }, 'After host reconnect'));
  for (const page of [host, guest]) await wait(page, async () => (await window.__peerHarness.lobbyState()).multiplayer.lastAppliedSequence >= 3);
  await wait(host, async url => (await (await fetch(`${url}/lobbies`)).json()).lobbies.length === 0, url);
  const originalGuestId = await guest.evaluate(() => window.__peerHarness.lobbyState().multiplayer.localPeerId);
  await guest.reload();
  await guest.waitForFunction(() => window.__peerHarness?.ready);
  await guest.evaluate(lobbyId => window.__peerHarness.joinLobby({ name: 'Reopened', lobbyId }), session.lobbyId);
  await wait(guest, () => window.__peerHarness.lobbyState().multiplayer.lastAppliedSequence === 3);
  assert.equal(await guest.evaluate(() => window.__peerHarness.lobbyState().multiplayer.localPeerId), originalGuestId);
  assert.equal(await guest.evaluate(() => window.__peerHarness.lobbyState().multiplayer.localPlayerIndex), 1);
  await guest.evaluate(() => window.__peerHarness.submitMultiplayerCommand({ type: 'priority_action', action_ref: { kind: 'test_priority_action', actor: 1, sequence: 3 } }, 'After guest refresh'));
  for (const page of [host, guest]) await wait(page, () => window.__peerHarness.lobbyState().multiplayer.lastAppliedSequence === 4);
  // The host restores its persisted checkpoint before accepting returning guests.
  await host.reload();
  await host.waitForFunction(() => window.__peerHarness?.ready);
  await host.evaluate(lobbyId => window.__peerHarness.joinLobby({ name: 'Reopened host', lobbyId }), session.lobbyId);
  await wait(host, () => window.__peerHarness.lobbyState().multiplayer.lastAppliedSequence === 4);
  await wait(host, () => window.__peerHarness.lobbyState().multiplayer.players.every(p => p.connected));
  assert.equal(await host.evaluate(() => window.__peerHarness.lobbyState().multiplayer.role), 'host');
  await host.evaluate(() => window.__peerHarness.submitMultiplayerCommand({ type: 'priority_action', action_ref: { kind: 'test_priority_action', actor: 0, sequence: 4 } }, 'After host refresh'));
  for (const page of [host, guest]) await wait(page, () => window.__peerHarness.lobbyState().multiplayer.lastAppliedSequence === 5);
  const hostContext = host.context(), guestContext = guest.context();
  const fixtureUrl = host.url();
  await guest.close();
  // An unrelated browser with the link cannot take the disconnected seat.
  const outsider = await hostContext.browser().newPage();
  await outsider.goto(fixtureUrl);
  await outsider.waitForFunction(() => window.__peerHarness?.ready);
  await outsider.evaluate(lobbyId => window.__peerHarness.joinLobby({ name: 'Intruder', lobbyId }), session.lobbyId);
  await wait(outsider, () => window.__peerHarness.lobbyState().statusEvents.some(e => JSON.stringify(e).includes('No disconnected player slots')));
  assert.equal(await host.evaluate(() => window.__peerHarness.lobbyState().multiplayer.players[1].peerId), originalGuestId);
  await outsider.close();
  await host.close();
  const returningGuest = await guestContext.newPage();
  await returningGuest.goto(fixtureUrl);
  await returningGuest.waitForFunction(() => window.__peerHarness?.ready);
  await returningGuest.evaluate(lobbyId => window.__peerHarness.joinLobby({ name: 'Guest', lobbyId }), session.lobbyId);
  const returningHost = await hostContext.newPage();
  await returningHost.goto(fixtureUrl);
  await returningHost.waitForFunction(() => window.__peerHarness?.ready);
  await returningHost.evaluate(lobbyId => window.__peerHarness.joinLobby({ name: 'Host', lobbyId }), session.lobbyId);
  for (const page of [returningHost, returningGuest]) await wait(page, () => window.__peerHarness.lobbyState().multiplayer.lastAppliedSequence === 5);
  await wait(returningHost, () => window.__peerHarness.lobbyState().multiplayer.players.every(p => p.connected));
  await returningGuest.evaluate(() => window.__peerHarness.submitMultiplayerCommand({ type: 'priority_action', action_ref: { kind: 'test_priority_action', actor: 1, sequence: 5 } }, 'After both tabs closed'));
  for (const page of [returningHost, returningGuest]) await wait(page, () => window.__peerHarness.lobbyState().multiplayer.lastAppliedSequence === 6);


});
test('transport preserves large Unicode message order and reconnect identity', { timeout: 60000 }, async t => {
  const { pages: [host, guest], url } = await setup(t);
  await host.evaluate(async url => {
    const { WebSocketPeer } = await import('/src/lib/relay/websocket-peer.js');
    window.messages = [];
    window.peer = new WebSocketPeer('', { url, format: 'modern', desiredPlayers: 2 });
    window.peer.on('connection', c => c.on('data', d => window.messages.push(d)));
    await new Promise(r => window.peer.on('open', r));
  }, url);
  const peerId = await host.evaluate(() => window.peer.id);
  await guest.evaluate(async ({ url, peerId }) => {
    const { WebSocketPeer } = await import('/src/lib/relay/websocket-peer.js');
    window.peer = new WebSocketPeer('', { url, room: peerId.split('-')[1] });
    await new Promise(r => window.peer.on('open', r));
    window.connection = window.peer.connect(peerId);
    await new Promise(r => window.connection.on('open', r));
    window.connection.send({ text: '🌳abc'.repeat(300000) });
    window.connection.send({ after: true });
  }, { url, peerId });
  await host.waitForFunction(() => window.messages.length === 2);
  assert.deepEqual(await host.evaluate(() => [window.messages[0].text === '🌳abc'.repeat(300000), window.messages[1].after]), [true, true]);
  const guestId = await guest.evaluate(() => window.peer.id);
  await guest.evaluate(async () => {
    await new Promise(r => { window.peer.on('disconnected', r); window.peer.socket.close(); });
    window.peer.reconnect(); await new Promise(r => window.peer.on('open', r));
  });
  assert.equal(await guest.evaluate(() => window.peer.id), guestId);
  await guest.evaluate(async peerId => {
    const c = window.peer.connect(peerId); await new Promise(r => c.on('open', r)); c.send({ resumed: true });
  }, peerId);
  await host.waitForFunction(() => window.messages.length === 3);
  const replacement = await guest.context().newPage();
  await replacement.goto(guest.url());
  await replacement.evaluate(async ({ url, peerId }) => {
    const { WebSocketPeer } = await import('/src/lib/relay/websocket-peer.js');
    window.peer = new WebSocketPeer('', { url, room: peerId.split('-')[1] });
    await new Promise(r => window.peer.on('open', r));
    const c = window.peer.connect(peerId);
    await new Promise(r => c.on('open', r));
    c.send({ reopenedInNewTab: true });
  }, { url, peerId });
  await guest.waitForFunction(() => window.peer.destroyed);
  assert.equal(await replacement.evaluate(() => window.peer.id), guestId);
  await host.waitForFunction(() => window.messages.length === 4);

});
test('lobby UI selects public formats, constrains settings, searches and selects advertised tables', { timeout: 60000 }, async t => {
  const { pages: [host, page], base, url } = await setup(t);
  await host.evaluate(() => window.__peerHarness.createLobby({ name: 'Modern table', desiredPlayers: 2,
    format: 'modern', transport: 'websocket', deckText: '60 Plains' }));
  await wait(host, async () => (await window.__peerHarness.lobbyState()).multiplayer.mode === 'lobby');
  await wait(host, async url => (await (await fetch(`${url}/lobbies`)).json()).lobbies.length === 1, url);
  const lobbyId = await host.evaluate(async () => (await window.__peerHarness.lobbyState()).multiplayer.lobbyId);
  await page.goto(`${base}/tests/fixtures/relay-lobby.html`);
  await page.waitForTimeout(2000);
  await page.screenshot({ path: '/private/tmp/ironsmith-relay-create.png', fullPage: true });
  await page.getByLabel('Connection', { exact: true }).selectOption('websocket');
  assert.equal(await page.getByLabel('Format', { exact: true }).inputValue(), 'modern');
  assert.equal(await page.getByLabel('Starting Life', { exact: true }).isDisabled(), true);
  assert.equal(await page.getByLabel('Players', { exact: true }).isDisabled(), true);
  await page.getByLabel('Format', { exact: true }).selectOption('commander');
  assert.equal(await page.getByLabel('Starting Life', { exact: true }).inputValue(), '40');
  assert.equal(await page.getByLabel('Players', { exact: true }).isDisabled(), false);
  await page.getByLabel('Players', { exact: true }).selectOption('4');
  await page.getByLabel('Advertise in public lobby search').uncheck();
  await page.getByRole('button', { name: 'Create Lobby', exact: true }).click();
  const created = await page.evaluate(() => window.createdLobby);
  assert.equal(created.transport, 'websocket'); assert.equal(created.format, 'commander');
  assert.equal(created.advertise, false); assert.equal(created.desiredPlayers, 4);
  await page.getByRole('button', { name: 'Join', exact: true }).click();
  await page.getByRole('button', { name: /Modern table.*Select/ }).waitFor();
  await page.getByLabel('Filter public lobbies by format').selectOption('vintage');
  assert.equal(await page.getByRole('button', { name: /Modern table.*Select/ }).count(), 0);
  await page.getByLabel('Filter public lobbies by format').selectOption('modern');
  await page.getByRole('button', { name: /Modern table.*Select/ }).click();
  assert.equal(await page.getByLabel('Lobby Code', { exact: true }).inputValue(), lobbyId);
  await page.setViewportSize({ width: 1100, height: 950 });
  await page.screenshot({ path: '/private/tmp/ironsmith-public-lobby.png', fullPage: true });
});
