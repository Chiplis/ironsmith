import test from 'node:test';
import assert from 'node:assert/strict';
import { canPersistMatch, readPeerSession, readRelaySession, saveRelayIdentity, saveRelayLobby } from '../src/lib/relay/session.js';

function storage(records) {
  return { get length() { return records.size; }, key: index => [...records.keys()][index] ?? null,
    getItem: key => records.get(key) ?? null, setItem: (key, value) => records.set(key, value),
    removeItem: key => records.delete(key) };
}

test('PeerJS recovery preserves trusted and verified seat identities and rejects incomplete records', () => {
  const old = globalThis.localStorage;
  const records = new Map();
  globalThis.localStorage = storage(records);
  try {
    const session = { lobbyId: 'host-id', localPeerId: 'guest-id', role: 'client', securityMode: 'trusted', localPlayerIndex: 1,
      localName: 'Guest', localDeckText: '60 Mountain', players: [], matchStarted: true };
    saveRelayLobby(session);
    assert.equal(readPeerSession('host-id').peerId, 'guest-id');
    assert.equal(readPeerSession('host-id').session.localPlayerIndex, 1);
    assert.equal(readPeerSession('another-host'), null);
    assert.equal(canPersistMatch({ ...session, securityMode: 'verified' }), true);
    saveRelayLobby({ ...session, lobbyId: 'verified-host', securityMode: 'verified' });
    assert.equal(readPeerSession('verified-host').session.localPlayerIndex, 1);
    records.set('ironsmith-peerjs-resume-v1:host-id', '{broken');
    assert.equal(readPeerSession('host-id'), null);
    records.set('ironsmith-peerjs-resume-v1:host-id', JSON.stringify({ peerId: 'wrong', session }));
    assert.equal(readPeerSession('host-id'), null);
    globalThis.localStorage.setItem = () => { throw new Error('Storage disabled'); };
    assert.throws(() => saveRelayLobby(session), /Storage disabled/);
  } finally { globalThis.localStorage = old; }
});

test('only the latest reconnect lobby survives across PeerJS and relay saves', () => {
  const old = globalThis.localStorage;
  const records = new Map([
    ['ironsmith-peerjs-resume-v1:old-peer', 'old'],
    ['ironsmith-relay-session-v1:https://old-relay:old-room', 'old'],
    ['ironsmith-relay-only-v1', 'true'],
    ['ironsmith-custom-card-art-urls', 'art'],
  ]);
  globalThis.localStorage = storage(records);
  try {
    const session = { lobbyId: 'latest-peer', localPeerId: 'guest', securityMode: 'verified' };
    saveRelayLobby(session);
    assert.deepEqual([...records.keys()], ['ironsmith-relay-only-v1', 'ironsmith-custom-card-art-urls', 'ironsmith-peerjs-resume-v1:latest-peer']);
    const room = 'a'.repeat(32), peerId = `ws-${room}-${'b'.repeat(32)}`;
    saveRelayIdentity(room, 'https://relay', { peerId, token: 'c'.repeat(32) });
    assert.equal(readPeerSession(session.lobbyId), null);
    assert.equal(readRelaySession(room, 'https://relay').peerId, peerId);
    saveRelayIdentity(room, 'https://relay', { advertise: false });
    assert.equal(readRelaySession(room, 'https://relay').token, 'c'.repeat(32), 'updates preserve the current identity');
    saveRelayLobby(session);
    assert.equal(readRelaySession(room, 'https://relay'), null);
    assert.equal(readPeerSession(session.lobbyId).peerId, 'guest');
    assert.equal(records.get('ironsmith-custom-card-art-urls'), 'art');
  } finally { globalThis.localStorage = old; }
});

test('obsolete lobby records are removed before a quota-limited write and on unchanged updates', () => {
  const old = globalThis.localStorage;
  const records = new Map([['ironsmith-peerjs-resume-v1:old', 'x'.repeat(1000)]]);
  globalThis.localStorage = storage(records);
  globalThis.localStorage.setItem = (key, value) => {
    const size = value.length + [...records].filter(([storedKey]) => storedKey !== key).reduce((sum, [, storedValue]) => sum + storedValue.length, 0);
    if (size > 500) throw new Error('Quota exceeded');
    records.set(key, value);
  };
  try {
    const session = { lobbyId: 'latest', localPeerId: 'guest', securityMode: 'trusted' };
    saveRelayLobby(session);
    assert.equal(readPeerSession('latest').peerId, 'guest');
    records.set('ironsmith-peerjs-resume-v1:legacy', 'old');
    saveRelayLobby(session, session);
    assert.equal(records.has('ironsmith-peerjs-resume-v1:legacy'), false);
    const saved = records.get('ironsmith-peerjs-resume-v1:latest');
    assert.throws(() => saveRelayLobby({ ...session, localDeckText: 'x'.repeat(1000) }), /Quota exceeded/);
    assert.equal(records.get('ironsmith-peerjs-resume-v1:latest'), saved, 'failed replacement preserves the current record');
  } finally { globalThis.localStorage = old; }
});
