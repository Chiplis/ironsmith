import { fileURLToPath } from 'node:url';
import test from 'node:test';
import assert from 'node:assert/strict';
import { createServer } from 'vite';
import { chromium } from 'playwright';
import { startRelay } from '../../relay/tests/runtime.mjs';

// Two browser contexts talking through the real local relay, driving the
// transport classes directly (no lobby protocol).
async function setup(t, port) {
  const base = `http://127.0.0.1:${port}`;
  const relay = await startRelay(base); t.after(() => relay.dispose());
  const url = String(await relay.ready).replace(/\/$/, '');
  const server = await createServer({ root: fileURLToPath(new URL('..', import.meta.url)),
    server: { host: '127.0.0.1', port, strictPort: true }, logLevel: 'error' });
  await server.listen(); t.after(() => server.close());
  const browser = await chromium.launch({ headless: true }); t.after(() => browser.close());
  const pages = [];
  for (let i = 0; i < 2; i++) {
    const page = await (await browser.newContext()).newPage();
    page.on('pageerror', e => console.error('Browser error:', e.message));
    page.on('console', m => { if (/^\[(transport|conn)/.test(m.text())) console.error(`p${i}`, m.text()); });
    await page.addInitScript(() => {
      const NativeWebSocket = window.WebSocket; window.testSockets = [];
      window.WebSocket = class extends NativeWebSocket { constructor(...args) { super(...args); window.testSockets.push(this); } };
      const NativeRTC = window.RTCPeerConnection; window.testPeerConnections = [];
      window.RTCPeerConnection = class extends NativeRTC {
        constructor(...args) { super(...args); window.testPeerConnections.push(this); }
        addIceCandidate(...args) { return window.dropIceCandidates ? Promise.resolve() : super.addIceCandidate(...args); }
      };
    });
    const frames = { sent: [] };
    page.on('websocket', socket => socket.on('framesent', ({ payload }) => {
      if (typeof payload !== 'string' || payload === 'ping') return;
      try { frames.sent.push(JSON.parse(payload)); } catch { /* not JSON */ }
    }));
    page.frames = frames;
    await page.goto(`${base}/tests/fixtures/peer-lobby-harness.html`);
    await page.waitForFunction(() => window.__peerHarness?.ready);
    pages.push(page);
  }
  return { pages, url };
}

async function connectPair(host, guest, url, { hostOptions = {}, guestOptions = {} } = {}) {
  await host.evaluate(async ({ url, options }) => {
    const { WebSocketPeer } = await import('/src/lib/relay/websocket-peer.js');
    window.messages = [];
    window.peer = new WebSocketPeer('', { url, format: 'modern', desiredPlayers: 2, ...options });
    window.peer.on('connection', c => { window.connection = c; c.on('transport', e => console.log('[transport host]', JSON.stringify(e))); c.on('error', e => console.log('[conn error host]', e.message)); c.on('data', d => window.messages.push(d)); c.on('close', () => { window.connClosed = true; }); });
    await new Promise(r => window.peer.on('open', r));
  }, { url, options: hostOptions });
  const peerId = await host.evaluate(() => window.peer.id);
  await guest.evaluate(async ({ url, peerId, options }) => {
    const { WebSocketPeer } = await import('/src/lib/relay/websocket-peer.js');
    window.messages = [];
    window.peer = new WebSocketPeer('', { url, room: peerId.split('-')[1], ...options });
    await new Promise(r => window.peer.on('open', r));
    window.connection = window.peer.connect(peerId);
    window.connection.on('transport', e => console.log('[transport guest]', JSON.stringify(e)));
    window.connection.on('error', e => console.log('[conn error guest]', e.message));
    window.connection.on('data', d => window.messages.push(d));
    window.connection.on('close', () => { window.connClosed = true; });
    await new Promise(r => window.connection.on('open', r));
  }, { url, peerId, options: guestOptions });
  return peerId;
}

const dataFramesAfter = (page, index) => page.frames.sent.slice(index).filter(frame => frame.type === 'data').length;

test('connections move onto a direct channel without reordering and survive a relay drop', { timeout: 60000 }, async t => {
  const { pages: [host, guest], url } = await setup(t, 5191);
  await connectPair(host, guest, url);
  // Keep sending across the switch; order must hold on both paths.
  await guest.evaluate(() => new Promise(resolve => {
    let n = 0;
    const timer = setInterval(() => {
      window.connection.send({ n: n++, pad: n % 25 === 0 ? 'x'.repeat(40000) : '' });
      if (n === 150) { clearInterval(timer); resolve(); }
    }, 10);
  }));
  await host.waitForFunction(() => window.messages.length === 150);
  assert.deepEqual(await host.evaluate(() => window.messages.map(m => m.n)), [...Array(150).keys()]);
  await host.waitForFunction(() => window.connection.transport === 'direct');
  await guest.waitForFunction(() => window.connection.transport === 'direct');
  assert.ok(host.frames.sent.some(frame => frame.type === 'rtc' && frame.signal?.kind === 'switch'));
  const hostMark = host.frames.sent.length, guestMark = guest.frames.sent.length;
  await host.evaluate(() => { for (let i = 0; i < 20; i++) window.connection.send({ back: i }); });
  await guest.waitForFunction(() => window.messages.filter(m => 'back' in m).length === 20);
  assert.equal(dataFramesAfter(host, hostMark) + dataFramesAfter(guest, guestMark), 0, 'no game data rides the relay once direct');
  // The relay socket can drop while the direct channel keeps the game going.
  // Only the room socket: closing Vite's HMR socket reloads the page.
  await host.evaluate(() => window.testSockets.filter(socket => socket.url.includes('/rooms/')).forEach(socket => socket.close()));
  await new Promise(r => setTimeout(r, 500));
  await guest.evaluate(() => window.connection.send({ afterDrop: true }));
  await host.waitForFunction(() => window.messages.some(m => m.afterDrop));
  assert.equal(await host.evaluate(() => Boolean(window.connClosed)), false);
});

test('relay-only peers never create a WebRTC connection', { timeout: 60000 }, async t => {
  const { pages: [host, guest], url } = await setup(t, 5192);
  // PeerJS probes WebRTC support at import time; count only what connecting adds.
  const baseline = await Promise.all([host, guest].map(page => page.evaluate(() => window.testPeerConnections.length)));
  await connectPair(host, guest, url, { guestOptions: { relayOnly: true } });
  await guest.evaluate(() => window.connection.send({ hello: 1 }));
  await host.waitForFunction(() => window.messages.some(m => m.hello));
  await new Promise(r => setTimeout(r, 1000));
  for (const [index, page] of [host, guest].entries()) {
    assert.equal(await page.evaluate(() => window.testPeerConnections.length), baseline[index]);
    assert.equal(page.frames.sent.some(frame => frame.type === 'rtc'), false);
    assert.equal(await page.evaluate(() => window.connection.transport), 'relay');
  }
});

test('a direct channel that never connects leaves the connection on the relay', { timeout: 60000 }, async t => {
  const { pages: [host, guest], url } = await setup(t, 5193);
  for (const page of [host, guest]) await page.evaluate(() => { window.dropIceCandidates = true; });
  await connectPair(host, guest, url, { hostOptions: { directTimeoutMs: 1500 }, guestOptions: { directTimeoutMs: 1500 } });
  await guest.waitForFunction(() => window.peer.noDirect.size === 1, null, { timeout: 10000 });
  await guest.evaluate(() => window.connection.send({ stillRelay: true }));
  await host.waitForFunction(() => window.messages.some(m => m.stillRelay));
  assert.equal(await guest.evaluate(() => window.connection.transport), 'relay');
  assert.equal(await host.evaluate(() => Boolean(window.connClosed)), false);
});

test('losing a direct channel that carried traffic closes the connection and reconnects relay-only', { timeout: 60000 }, async t => {
  const { pages: [host, guest], url } = await setup(t, 5194);
  const baseline = await guest.evaluate(() => window.testPeerConnections.length);
  const hostId = await connectPair(host, guest, url);
  await guest.waitForFunction(() => window.connection.transport === 'direct');
  await host.waitForFunction(() => window.connection.transport === 'direct');
  await guest.evaluate(baseline => window.testPeerConnections.slice(baseline).forEach(pc => pc.close()), baseline);
  await guest.waitForFunction(() => window.connClosed === true);
  await host.waitForFunction(() => window.connClosed === true);
  await guest.evaluate(async hostId => {
    window.connClosed = false;
    window.connection = window.peer.connect(hostId);
    await new Promise(r => window.connection.on('open', r));
    window.connection.send({ again: true });
  }, hostId);
  await host.waitForFunction(() => window.messages.some(m => m.again));
  await new Promise(r => setTimeout(r, 1000));
  assert.equal(await guest.evaluate(() => window.connection.transport), 'relay');
  assert.equal(await guest.evaluate(() => window.testPeerConnections.length), baseline + 1, 'no second direct attempt to a peer that failed');
});
