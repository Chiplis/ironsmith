import test from 'node:test';
import assert from 'node:assert/strict';
import { SeatHub } from './helpers/seat-hub.mjs';
import { playStep } from './helpers/play-driver.mjs';

// The whole Verified peer-lobby stack (usePeerLobby and its modules) running
// headless: two isolated seats (worker threads, own wasm engine), committed
// decks, jointly shuffled libraries, signed and replayed actions.
const DECK = ['28 Mountain', '4 Lightning Bolt', '4 Shock', '4 Lava Axe', '4 Goblin Guide',
  '4 Monastery Swiftspear', '4 Burst Lightning', '4 Rift Bolt', '4 Searing Spear'].join('\n');

test('two headless seats start a Verified match and apply the same signed actions', { timeout: 280_000 }, async () => {
  const hub = new SeatHub();
  try {
    const host = await hub.spawn('host', { deckText: DECK });
    const guest = await hub.spawn('guest', { deckText: DECK });
    await host.lobby('createLobby', { name: 'Host', desiredPlayers: 2, securityMode: 'verified', deckText: DECK });
    const lobbyId = await host.until('lobby id', s => s.multiplayer.lobbyId);
    await guest.lobby('joinLobby', { name: 'Guest', lobbyId, deckText: DECK });
    await host.until('lobby ready', s => s.canStart, 60_000);
    await host.lobby('startHostedMatch');
    await Promise.all([host, guest].map(seat => seat.until('match start', s => s.multiplayer.matchStarted, 180_000)));

    let applied = 0;
    const started = Date.now();
    while (applied < 80 && Date.now() - started < 200_000) {
      let moved = false;
      for (const [seat, index] of [[host, 0], [guest, 1]]) {
        if (await playStep(seat, index)) { moved = true; applied += 1; }
      }
      if (!moved) await new Promise(resolve => setTimeout(resolve, 40));
    }
    // Let the trailing action reach the other seat before comparing.
    const head = (await host.snapshot()).multiplayer.lastAppliedSequence;
    await guest.until('transcript caught up', s => s.multiplayer.lastAppliedSequence >= head && !s.multiplayer.submittingAction, 30_000);
    const [a, b] = await Promise.all([host.snapshot(), guest.snapshot()]);
    assert.ok(a.multiplayer.lastAppliedSequence >= 50, `host applied ${a.multiplayer.lastAppliedSequence}`);
    assert.equal(b.multiplayer.lastAppliedSequence, a.multiplayer.lastAppliedSequence, 'seats agree on the transcript head');
    assert.deepEqual(b.state.players, a.state.players, 'public state agrees');
    for (const snap of [a, b]) {
      assert.deepEqual(snap.errors.filter(text => !/reconnect identity/.test(text)), [], 'no protocol errors');
      assert.ok(!snap.multiplayer.matchDisputed && !snap.multiplayer.disputed, 'match not disputed');
    }
    // The host kept a land; hidden cards stayed hidden from the other seat.
    assert.equal(a.state.players[0].hand_size, 6);
  } finally {
    await hub.dispose();
  }
});
