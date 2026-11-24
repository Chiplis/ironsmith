import test from 'node:test';
import assert from 'node:assert/strict';
import { SeatHub } from './helpers/seat-hub.mjs';
import { answerFor } from './helpers/manabrew-driver.mjs';

// Manabrew drives a Verified match through the engine's Manabrew adapter: each
// seat renders its own view, translates prompt answers into engine commands,
// and submits them through the signed transcript. Needs an engine built with
// manabrewTranslate (set IRONSMITH_PKG to a package built after that change).
const DECK = ['28 Mountain', '4 Lightning Bolt', '4 Shock', '4 Lava Axe', '4 Goblin Guide',
  '4 Monastery Swiftspear', '4 Burst Lightning', '4 Rift Bolt', '4 Searing Spear'].join('\n');

test('Manabrew views and prompts over a Verified match stay private and replicate through the transcript', { timeout: 280_000 }, async (t) => {
  const hub = new SeatHub();
  try {
    const host = await hub.spawn('host', { deckText: DECK });
    if (!(await host.hasMethod('manabrewTranslate'))) { t.skip('engine package predates manabrewTranslate'); return; }
    const guest = await hub.spawn('guest', { deckText: DECK });
    await host.lobby('createLobby', { name: 'Host', desiredPlayers: 2, securityMode: 'verified', deckText: DECK });
    const lobbyId = await host.until('lobby id', s => s.multiplayer.lobbyId);
    await guest.lobby('joinLobby', { name: 'Guest', lobbyId, deckText: DECK });
    await host.until('lobby ready', s => s.canStart, 60_000);
    await host.lobby('startHostedMatch');
    await Promise.all([host, guest].map(seat => seat.until('match start', s => s.multiplayer.matchStarted, 180_000)));
    const seats = [[host, 0], [guest, 1]];
    for (const [seat] of seats) await seat.engine('manabrewAttach', 'verified-test', [], true);

    // Privacy: a seat sees its own hand and never the other's.
    for (const [seat, index] of seats) {
      let own = [];
      for (let attempt = 0; attempt < 100 && own.length < 7; attempt += 1) {
        const view = await seat.engine('manabrewView', index);
        const hands = view.state.gameView.zones.filter(zone => zone.zone === 'hand');
        own = hands.find(zone => zone.playerId === `player-${index}` || zone.ownerId === `player-${index}`)?.cards
          ?? hands[index]?.cards ?? [];
        const other = hands.find(zone => zone !== hands[index])?.cards ?? [];
        assert.equal(other.length, 0, `${seat.name} must not see the other seat's hand`);
        if (own.length < 7) await new Promise(resolve => setTimeout(resolve, 50));
      }
      assert.equal(own.length, 7, `${seat.name} sees its own opening hand`);
      assert.ok(own.every(card => card.identity?.name), 'own hand cards are named');
    }

    let submitted = 0;
    const started = Date.now();
    const rejected = [];
    while (submitted < 25 && Date.now() - started < 200_000) {
      let moved = false;
      for (const [seat, index] of seats) {
        const view = await seat.engine('manabrewView', index);
        if (view.forcedCommand) {
          await seat.lobby('submitMultiplayerCommand', view.forcedCommand, 'forced');
          moved = true; submitted += 1;
          continue;
        }
        const answer = answerFor(view.prompt);
        if (!answer) continue;
        const translation = await seat.engine('manabrewTranslate', index, view.prompt.promptId, answer);
        if (translation.kind === 'dispatch') {
          await seat.lobby('submitMultiplayerCommand', translation.command, 'answer');
          moved = true; submitted += 1;
        } else if (translation.kind === 'rejected') rejected.push(translation.view.error?.message);
        else moved = true;
      }
      if (!moved) await new Promise(resolve => setTimeout(resolve, 60));
    }
    assert.deepEqual(rejected, [], 'the engine accepts every answer the driver gives');
    assert.ok(submitted >= 25, `submitted ${submitted} answers`);
    let a; let b;
    for (let attempt = 0; attempt < 300; attempt += 1) {
      [a, b] = await Promise.all([host.snapshot(), guest.snapshot()]);
      if (a.multiplayer.lastAppliedSequence === b.multiplayer.lastAppliedSequence
          && !a.multiplayer.submittingAction && !b.multiplayer.submittingAction) break;
      await new Promise(resolve => setTimeout(resolve, 50));
    }
    assert.equal(b.multiplayer.lastAppliedSequence, a.multiplayer.lastAppliedSequence, 'seats agree on the transcript head');
    assert.deepEqual(b.state.players, a.state.players, 'public state agrees');
    for (const snap of [a, b]) assert.deepEqual(snap.errors.filter(text => !/reconnect identity/.test(text)), []);
  } finally {
    await hub.dispose();
  }
});
