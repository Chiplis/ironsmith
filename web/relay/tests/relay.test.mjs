import test from 'node:test';
import assert from 'node:assert/strict';
import { startRelay } from './runtime.mjs';
import { randomBytes } from 'node:crypto';
const id = () => randomBytes(16).toString('hex');
const origin = 'http://localhost:5173';
function inbox(socket) {
  const queue = []; const waits = [];
  socket.addEventListener('message', event => { const value = JSON.parse(event.data); const waiter = waits.shift(); if (waiter) waiter(value); else queue.push(value); });
  return () => queue.length ? Promise.resolve(queue.shift()) : new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error('Timed out waiting for relay')), 5000);
    waits.push(value => { clearTimeout(timer); resolve(value); });
  });
}
test('Durable Object authenticates peers, relays only within rooms, lists and removes advertisements', { timeout: 30000 }, async t => {
  const mf = await startRelay(); t.after(() => mf.dispose());
  const room = id(); const host = `ws-${room}-${id()}`; const guest = `ws-${room}-${id()}`;
  async function connect(peer, token, format) {
    const response = await mf.dispatchFetch(`http://localhost/rooms/${room}/socket?peer=${peer}`, { headers: { Origin: origin, Upgrade: 'websocket' } });
    assert.equal(response.status, 101);
    const socket = response.webSocket; socket.accept(); const next = inbox(socket);
    socket.send(JSON.stringify({ type: 'auth', token, format, desiredPlayers: 2 }));
    return { socket, next, first: await next() };
  }
  assert.equal((await mf.dispatchFetch('http://localhost/lobbies', { headers: { Origin: 'https://evil.example' } })).status, 403);
  const token = id(); const h = await connect(host, token, 'modern'); assert.equal(h.first.type, 'open');
  const g = await connect(guest, id()); assert.equal(g.first.type, 'open');
  h.socket.send(JSON.stringify({ type: 'advertise', lobby: { name: 'Modern table', format: 'vintage', available: true, playerCount: 1 } }));
  let listing;
  for (let i = 0; i < 30; i++) {
    listing = await (await mf.dispatchFetch('http://localhost/lobbies', { headers: { Origin: origin } })).json();
    if (listing.lobbies.length) break;
    await new Promise(r => setTimeout(r, 30));
  }
  assert.equal(listing.lobbies[0].format, 'modern'); assert.equal(listing.lobbies[0].id, host);
  const connectionId = id();
  g.socket.send(JSON.stringify({ type: 'data', to: host, from: 'spoofed', connectionId, data: '.{"value":42}' }));
  assert.deepEqual(await h.next(), { type: 'data', from: guest, connectionId, data: '.{"value":42}' });
  g.socket.send(JSON.stringify({ type: 'data', to: `ws-${id()}-${id()}`, connectionId, data: '.' }));
  assert.equal((await g.next()).type, 'unavailable');
  const thief = await connect(host, id()); assert.equal(thief.first.type, 'error');
  const reconnect = await connect(host, token); assert.equal(reconnect.first.type, 'open');
  reconnect.socket.close(1000, 'leave');
  for (let i = 0; i < 30; i++) {
    listing = await (await mf.dispatchFetch('http://localhost/lobbies', { headers: { Origin: origin } })).json();
    if (!listing.lobbies.length) break;
    await new Promise(r => setTimeout(r, 30));
  }
  assert.equal(listing.lobbies.length, 0);
});

test('an expired resume credential cannot create a replacement room', async t => {
  const mf = await startRelay(); t.after(() => mf.dispose());
  const room = id(), peer = `ws-${room}-${id()}`;
  const response = await mf.dispatchFetch(`http://localhost/rooms/${room}/socket?peer=${peer}`, { headers: { Origin: origin, Upgrade: 'websocket' } });
  const socket = response.webSocket; socket.accept(); const next = inbox(socket);
  socket.send(JSON.stringify({ type: 'auth', token: id(), resume: true, format: 'modern', desiredPlayers: 2 }));
  assert.match((await next()).message, /expired/);
});
