import { fileURLToPath } from 'node:url';
import test from 'node:test';
import assert from 'node:assert/strict';
import { createServer } from 'vite';
import { chromium } from 'playwright';
import { startRelay } from '../../relay/tests/runtime.mjs';

async function setup(t, seats) {
  const port = Number(process.env.RELAY_TEST_PORT || 5189);
  const base = `http://127.0.0.1:${port}`;
  const relay = await startRelay(base); t.after(() => relay.dispose());
  const url = String(await relay.ready).replace(/\/$/, '');
  const server = await createServer({ root: fileURLToPath(new URL('..', import.meta.url)),
    define: { 'import.meta.env.VITE_LOBBY_RELAY_URL': JSON.stringify(url) },
    server: { host: '127.0.0.1', port, strictPort: true }, logLevel: 'error' });
  await server.listen(); t.after(() => server.close());
  const browser = await chromium.launch({ headless: true }); t.after(() => browser.close());
  const fixture = `${base}/tests/fixtures/peer-lobby-harness.html`;
  const open = async context => {
    const page = await context.newPage();
    page.on('pageerror', e => console.error('Browser error:', e.message));
    await page.goto(fixture);
    await page.waitForFunction(() => window.__peerHarness?.ready);
    return page;
  };
  const pages = [];
  for (let i = 0; i < seats; i++) {
    const context = await browser.newContext();
    // Relay-only keeps the host's liveness on the room socket alone.
    await context.addInitScript(() => localStorage.setItem('ironsmith-relay-only-v1', 'true'));
    pages.push(await open(context));
  }
  return { pages, open };
}
const multiplayer = page => page.evaluate(async () => (await window.__peerHarness.lobbyState()).multiplayer);
async function wait(page, label, predicate, timeoutMs = 20000) {
  const end = Date.now() + timeoutMs;
  let last;
  while (Date.now() < end) {
    last = await multiplayer(page);
    if (predicate(last)) return last;
    await new Promise(r => setTimeout(r, 100));
  }
  const state = await page.evaluate(async () => (await window.__peerHarness.lobbyState()));
  console.error(label, JSON.stringify({ role: last.role, mode: last.mode, hostPeerId: last.hostPeerId, seq: last.lastAppliedSequence,
    players: last.players.map(p => [p.index, p.peerId, p.connected]), statuses: state.statusEvents.slice(-8) }));
  assert.fail(`Timed out waiting for ${label}`);
}
const act = (page, actor, sequence) => page.evaluate(([actor, sequence]) => window.__peerHarness.submitMultiplayerCommand(
  { type: 'priority_action', action_ref: { kind: 'test_priority_action', actor, sequence } }, 'Relay action'), [actor, sequence]);

async function startMatch(pages, format, decks) {
  const [host, ...guests] = pages;
  await host.evaluate(([format, deck, desiredPlayers]) => window.__peerHarness.createLobby({ name: 'Host', desiredPlayers,
    format, transport: 'websocket', advertise: false, ...deck }), [format, decks[0], pages.length]);
  const session = await wait(host, 'lobby', s => s.mode === 'lobby');
  for (const [index, guest] of guests.entries()) {
    await guest.evaluate(([lobbyId, name, deck]) => window.__peerHarness.joinLobby({ name, lobbyId, ...deck }),
      [session.lobbyId, `Guest ${index + 1}`, decks[index + 1]]);
    await wait(host, `guest ${index + 1} seated`, s => s.players.length === index + 2);
  }
  await wait(host, 'all ready', s => s.players.every(p => p.ready));
  await host.evaluate(() => window.__peerHarness.startHostedMatch());
  for (const page of pages) await wait(page, 'match start', s => s.matchStarted);
  return session;
}

test('a relay guest takes over a two-player match when the host leaves, and the host returns as a guest', { timeout: 180000 }, async t => {
  const { pages, open } = await setup(t, 2);
  const [host, guest] = pages;
  const session = await startMatch(pages, 'modern', [{ deckText: '60 Plains' }, { deckText: '60 Island' }]);
  await act(host, 0, 0);
  for (const page of pages) await wait(page, 'first action', s => s.lastAppliedSequence === 1);
  const guestId = (await multiplayer(guest)).localPeerId;
  const hostContext = host.context();
  await host.close();

  const promoted = await wait(guest, 'guest promoted', s => s.role === 'host', 60000);
  assert.equal(promoted.hostPeerId, guestId);
  assert.equal(promoted.lobbyId, session.lobbyId);
  assert.equal(promoted.lastAppliedSequence, 1);
  // The match continues without the original host.
  await act(guest, 1, 1);
  await wait(guest, 'solo action', s => s.lastAppliedSequence === 2);

  const returning = await open(hostContext);
  await returning.evaluate(lobbyId => window.__peerHarness.joinLobby({ name: 'Host', lobbyId }), session.lobbyId);
  const rejoined = await wait(returning, 'original host rejoined as guest', s => s.role === 'client' && s.lastAppliedSequence === 2, 60000);
  assert.equal(rejoined.hostPeerId, guestId);
  assert.equal(rejoined.localPlayerIndex, 0);
  await wait(guest, 'host seat reconnected', s => s.players.every(p => p.connected !== false));
  await act(returning, 0, 2);
  for (const page of [guest, returning]) await wait(page, 'action after return', s => s.lastAppliedSequence === 3);

  // The promoted host survives its own refresh.
  await guest.reload();
  await guest.waitForFunction(() => window.__peerHarness?.ready);
  await guest.evaluate(lobbyId => window.__peerHarness.joinLobby({ name: 'Guest 1', lobbyId }), session.lobbyId);
  await wait(guest, 'promoted host restored', s => s.role === 'host' && s.lastAppliedSequence === 3, 60000);
  await wait(guest, 'seats back', s => s.players.every(p => p.connected !== false), 60000);
  await act(guest, 1, 3);
  for (const page of [guest, returning]) await wait(page, 'action after promoted refresh', s => s.lastAppliedSequence === 4);
});

const commanderDeck = { deckText: '99 Plains', commanderText: '1 Isamaru, Hound of Konda' };

test('a four-player relay match keeps going as one host after another leaves', { timeout: 240000 }, async t => {
  const { pages } = await setup(t, 4);
  const [host, second, third, fourth] = pages;
  const session = await startMatch(pages, 'commander', pages.map(() => commanderDeck));
  await act(host, 0, 0);
  for (const page of pages) await wait(page, 'first action', s => s.lastAppliedSequence === 1);
  const secondId = (await multiplayer(second)).localPeerId;
  const thirdId = (await multiplayer(third)).localPeerId;

  // The host leaves: the lowest remaining seat hosts and the others follow it.
  await host.close();
  await wait(second, 'second seat promoted', s => s.role === 'host', 60000);
  for (const page of [third, fourth]) {
    const moved = await wait(page, 'seat follows new host', s => s.hostPeerId === secondId && s.role === 'client', 60000);
    assert.equal(moved.lobbyId, session.lobbyId);
  }
  await wait(second, 'remaining seats reconnected', s => s.players.filter(p => p.connected !== false).length === 3, 60000);
  // The harness game passes priority in seat order.
  await act(second, 1, 1);
  for (const page of [second, third, fourth]) await wait(page, 'new host action', s => s.lastAppliedSequence === 2);
  await act(third, 2, 2);
  for (const page of [third, second, fourth]) await wait(page, 'guest action under second host', s => s.lastAppliedSequence === 3);

  // The new host leaves too: the match moves again.
  await second.close();
  await wait(third, 'third seat promoted', s => s.role === 'host', 90000);
  await wait(fourth, 'last seat follows', s => s.hostPeerId === thirdId, 60000);
  await wait(third, 'last seat reconnected', s => s.players.filter(p => p.connected !== false).length === 2, 60000);
  await act(fourth, 3, 3);
  for (const page of [fourth, third]) await wait(page, 'action under third host', s => s.lastAppliedSequence === 4);
});

test('a later seat takes over when the host and the next seat leave together', { timeout: 240000 }, async t => {
  const { pages } = await setup(t, 3);
  const [host, second, third] = pages;
  await startMatch(pages, 'commander', pages.map(() => commanderDeck));
  await act(host, 0, 0);
  for (const page of pages) await wait(page, 'first action', s => s.lastAppliedSequence === 1);
  await Promise.all([host.close(), second.close()]);
  const promoted = await wait(third, 'third seat promoted', s => s.role === 'host', 120000);
  assert.equal(promoted.lastAppliedSequence, 1);
  assert.equal(promoted.hostPeerId, promoted.localPeerId);
});

test('a guest that refreshes while the host is away restores the match from its own copy and hosts it', { timeout: 240000 }, async t => {
  const { pages, open } = await setup(t, 2);
  const [host, guest] = pages;
  const session = await startMatch(pages, 'modern', [{ deckText: '60 Plains' }, { deckText: '60 Island' }]);
  await act(host, 0, 0);
  for (const page of pages) await wait(page, 'first action', s => s.lastAppliedSequence === 1);
  await act(guest, 1, 1);
  for (const page of pages) await wait(page, 'second action', s => s.lastAppliedSequence === 2);
  const guestId = (await multiplayer(guest)).localPeerId;
  const hostContext = host.context();
  await host.close();

  // The guest has no live game after the refresh, only its saved transcript.
  await guest.reload();
  await guest.waitForFunction(() => window.__peerHarness?.ready);
  await guest.evaluate(lobbyId => window.__peerHarness.joinLobby({ name: 'Guest 1', lobbyId }), session.lobbyId);
  const restored = await wait(guest, 'refreshed guest restored and hosting', s => s.role === 'host' && s.matchStarted, 120000);
  assert.equal(restored.lastAppliedSequence, 2);
  assert.equal(restored.localPeerId, guestId);
  assert.equal(restored.localPlayerIndex, 1);

  // The original host returns to the match the guest kept alive.
  const returning = await open(hostContext);
  await returning.evaluate(lobbyId => window.__peerHarness.joinLobby({ name: 'Host', lobbyId }), session.lobbyId);
  await wait(returning, 'original host rejoined', s => s.role === 'client' && s.lastAppliedSequence === 2, 60000);
  await wait(guest, 'host seat reconnected', s => s.players.every(p => p.connected !== false), 60000);
  await act(returning, 0, 2);
  for (const page of [guest, returning]) await wait(page, 'action after restore', s => s.lastAppliedSequence === 3);
});
