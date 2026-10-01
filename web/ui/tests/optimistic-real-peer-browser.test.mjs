import {
  assert, chromium, freePort, startPeerServer, closePeerServer, startHarnessServer,
  startFullUiPeerMatch, fullUiSnapshot, waitForFullUiPair, assertNoPageErrors,
  assertNoFullUiSyncFailuresWithDebug, test,
} from './peerjs-resync-harness.js';

test('real Verified engine calculates peer choices before signature verification completes', { timeout: 240000 }, async t => {
  const peerPort = await freePort();
  const peerServer = await startPeerServer(peerPort);
  t.after(() => closePeerServer(peerServer));
  const { vite, baseUrl } = await startHarnessServer(peerPort);
  t.after(() => vite.close());
  const browser = await chromium.launch();
  t.after(() => browser.close());
  const hostContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  const guestContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  for (const context of [hostContext, guestContext]) await context.route('**/lobbies', route =>
    route.fulfill({ contentType: 'application/json', body: '{"lobbies":[]}',
      headers: { 'Access-Control-Allow-Origin': '*' } }));
  const { hostPage: host, guestPage: guest } = await startFullUiPeerMatch({
    baseUrl, hostContext, guestContext, securityMode: 'verified',
    hostDeckText: '60 Mountain', guestDeckText: '60 Island',
  });
  assert.equal((await fullUiSnapshot(host)).multiplayer.securityMode, 'verified');
  for (const page of [host, guest]) await page.evaluate(() => {
    const verify = crypto.subtle.verify.bind(crypto.subtle);
    crypto.subtle.verify = async (...args) => {
      await new Promise(resolve => setTimeout(resolve, 1000));
      return verify(...args);
    };
  });
  const initial = await fullUiSnapshot(host);
  const actor = initial.state.decision.player;
  const page = actor === 0 ? host : guest;
  const peer = actor === 0 ? guest : host;
  const action = initial.state.decisionActions.find(entry => entry.action_ref.kind === 'keep_opening_hand');
  assert.ok(action, JSON.stringify(initial.state));
  const result = await page.evaluate(command => window.__ironsmithE2E.submitMultiplayerCommand(command),
    { type: 'priority_action', action_ref: action.action_ref });
  assert.equal(result?.provisional, true);
  await peer.waitForFunction(() => {
    const snap = window.__ironsmithE2E.snapshot();
    return snap.multiplayer.pendingVerification > 0
      && snap.state.decision.player === snap.multiplayer.localPlayerIndex;
  }, { timeout: 20000 });
  const dependent = await fullUiSnapshot(peer);
  assert.equal(dependent.multiplayer.lastAppliedSequence, initial.multiplayer.lastAppliedSequence);
  const choice = dependent.state.decisionActions.find(entry => entry.action_ref.kind === 'keep_opening_hand');
  assert.ok(choice, JSON.stringify(dependent.state));
  const second = await peer.evaluate(command => window.__ironsmithE2E.submitMultiplayerCommand(command),
    { type: 'priority_action', action_ref: choice.action_ref });
  assert.equal(second?.provisional, true);
  await waitForFullUiPair(host, guest, (a, b) => a.multiplayer.lastAppliedSequence >= initial.multiplayer.lastAppliedSequence + 2
    && a.multiplayer.lastAppliedSequence === b.multiplayer.lastAppliedSequence
    && a.multiplayer.pendingVerification === 0 && b.multiplayer.pendingVerification === 0,
  'both provisional choices become verified', 90000);
  const [a, b] = await Promise.all([host, guest].map(p => p.evaluate(() => window.__ironsmithE2E.auditTranscript())));
  assert.equal(a.finalStateHash, b.finalStateHash);
  assert.equal(a.finalPublicCheckpointHash, b.finalPublicCheckpointHash);
  await assertNoFullUiSyncFailuresWithDebug('optimistic verification must not report a sync failure', host, guest);
  assertNoPageErrors(host, guest);
});
