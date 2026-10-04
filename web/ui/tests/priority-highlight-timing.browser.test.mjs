import { priorityHighlightTimingPlugin } from './priority-highlight-timing.js';
import { writeFileSync } from 'node:fs';
import {
  assert, chromium, freePort, startPeerServer, closePeerServer, startHarnessServer,
  startFullUiPeerMatch, fullUiSnapshot, waitForFullUiPair, assertNoPageErrors,
  assertNoFullUiSyncFailuresWithDebug, test,
} from './peerjs-resync-harness.js';

for (const fixture of [
  { name: 'mixed Warp and Badgermole decks',
    host: '48 Mountain\n4 Nova Hellkite\n4 Magmatic Hellkite\n4 Sunbillow Verge',
    guest: '48 Forest\n4 Badgermole Cub\n4 Icetill Explorer\n4 Llanowar Elves',
    lands: ['Mountain', 'Sunbillow Verge', 'Forest'] },
]) test(`measure Verified multiplayer priority highlighting: ${fixture.name}`, { timeout: 240000 }, async t => {
  const peerPort = await freePort();
  const peerServer = await startPeerServer(peerPort);
  t.after(() => closePeerServer(peerServer));
  const { vite, baseUrl } = await startHarnessServer(peerPort, {}, [priorityHighlightTimingPlugin()]);
  t.after(() => vite.close());
  const browser = await chromium.launch();
  t.after(() => browser.close());
  const hostContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  const guestContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  for (const context of [hostContext, guestContext]) await context.route('**/lobbies', route =>
    route.fulfill({ contentType: 'application/json', body: '{"lobbies":[]}',
      headers: { 'Access-Control-Allow-Origin': '*' } }));
  const timings = [];
  for (const [seat, context] of [hostContext, guestContext].entries()) context.on('console', message => {
    const text = message.text();
    if (text.startsWith('__PRIORITY_TIMING__')) timings.push({seat, ...JSON.parse(text.slice('__PRIORITY_TIMING__'.length))});
  });
  t.after(() => writeFileSync(process.env.PRIORITY_TIMING_OUTPUT || '/tmp/priority-highlight-timing.json', JSON.stringify(timings, null, 2)));
  const { hostPage: host, guestPage: guest } = await startFullUiPeerMatch({
    baseUrl, hostContext, guestContext, securityMode: 'verified',
    hostDeckText: fixture.host, guestDeckText: fixture.guest,
  });
  const played = new Set();
  for (let turnAction = 0; turnAction < 40 && played.size < 2; turnAction++) {
    const settled = await waitForFullUiPair(host, guest, (a, b) => {
      const actor = a.state.decision?.player;
      const current = actor === 0 ? a : b;
      return a.multiplayer.lastAppliedSequence === b.multiplayer.lastAppliedSequence
        && !a.multiplayer.pendingVerification && !b.multiplayer.pendingVerification
        && (['attackers', 'blockers'].includes(current.state.decision?.kind)
          || (current.state.decision?.kind === 'priority'
            && current.state.priority_analysis_complete === true));
    }, 'the current player receives a completed action menu', 20000);
    const actor = settled.host.state.decision.player;
    const page = actor === 0 ? host : guest;
    const current = actor === 0 ? settled.host : settled.guest;
    const actions = current.state.decisionActions || [];
    const land = actions.find(action => action.action_ref.kind === 'play_land');
    if (land) {
      assert.equal(played.has(actor), false, 'a second land play must not be offered this turn');
      assert.equal(current.state.phase, 'first main phase');
      assert.ok(Number.isInteger(current.state.priority_revision));
      const card = page.locator(`[data-hand-object-id="${land.object_id}"] [data-object-id="${land.object_id}"]`).first();
      assert.match(await card.getAttribute('aria-label'), /playable/i, 'the land is marked playable in the rendered hand');
      assert.match(await card.getAttribute('class'), /\bglow-land\b/, 'the land has its playable highlight');
    }
    const action = land || actions.find(entry =>
      ['keep_opening_hand', 'continue_pregame', 'begin_game', 'pass_priority'].includes(entry.action_ref.kind));
    const command = current.state.decision.kind === 'attackers'
      ? { type: 'declare_attackers', declarations: [] }
      : current.state.decision.kind === 'blockers'
        ? { type: 'declare_blockers', declarations: [] }
        : action && { type: 'priority_action', action_ref: action.action_ref };
    assert.ok(command, JSON.stringify(current.state));
    const previousSequence = current.multiplayer.lastAppliedSequence;
    await page.evaluate(command => window.__ironsmithE2E.submitMultiplayerCommand(command),
      command);
    await waitForFullUiPair(host, guest, (a, b) =>
      a.multiplayer.lastAppliedSequence > previousSequence
      && a.multiplayer.lastAppliedSequence === b.multiplayer.lastAppliedSequence
      && !a.multiplayer.pendingVerification && !b.multiplayer.pendingVerification
      && (!land || (a.state.players[actor].battlefield.length === 1
        && b.state.players[actor].battlefield.length === 1)),
    'the action is verified and its board update is visible to both peers', 30000);
    if (land) {
      played.add(actor);
      for (const peer of [host, guest]) {
        const state = (await fullUiSnapshot(peer)).state;
        assert.equal(state.players[actor].battlefield.length, 1);
        assert.ok(fixture.lands.includes(state.players[actor].battlefield[0].name));
      }
    }
  }
  assert.equal(played.size, 2);
  const [a, b] = await Promise.all([host, guest].map(page => page.evaluate(() => window.__ironsmithE2E.auditTranscript())));
  assert.equal(a.finalStateHash, b.finalStateHash);
  assert.equal(a.finalPublicCheckpointHash, b.finalPublicCheckpointHash);
  await assertNoFullUiSyncFailuresWithDebug('playing lands must not cause sync failures', host, guest);
  assertNoPageErrors(host, guest);
});
