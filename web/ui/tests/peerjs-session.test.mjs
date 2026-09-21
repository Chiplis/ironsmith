import test from 'node:test';
import assert from 'node:assert/strict';
import { canPersistMatch, readPeerSession, saveRelayLobby } from '../src/lib/relay/session.js';

test('trusted PeerJS recovery preserves seat identity and ignores incomplete or verified records', () => {
  const old = globalThis.localStorage;
  const records = new Map();
  globalThis.localStorage = { getItem: key => records.get(key) ?? null, setItem: (key, value) => records.set(key, value) };
  try {
    const session = { lobbyId: 'host-id', localPeerId: 'guest-id', role: 'client', securityMode: 'trusted', localPlayerIndex: 1,
      localName: 'Guest', localDeckText: '60 Mountain', players: [], matchStarted: true };
    saveRelayLobby(session);
    assert.equal(readPeerSession('host-id').peerId, 'guest-id');
    assert.equal(readPeerSession('host-id').session.localPlayerIndex, 1);
    assert.equal(readPeerSession('another-host'), null);
    assert.equal(canPersistMatch({ ...session, securityMode: 'verified' }), false);
    saveRelayLobby({ ...session, lobbyId: 'verified-host', securityMode: 'verified' });
    assert.equal(readPeerSession('verified-host'), null);
    records.set('ironsmith-peerjs-resume-v1:host-id', '{broken');
    assert.equal(readPeerSession('host-id'), null);
    records.set('ironsmith-peerjs-resume-v1:host-id', JSON.stringify({ peerId: 'wrong', session }));
    assert.equal(readPeerSession('host-id'), null);
    globalThis.localStorage.setItem = () => { throw new Error('Storage disabled'); };
    assert.throws(() => saveRelayLobby(session), /Storage disabled/);
  } finally { globalThis.localStorage = old; }
});
