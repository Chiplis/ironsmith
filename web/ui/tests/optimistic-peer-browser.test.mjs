import {
  assert, assertNoPageErrors, chromium, freePort, startPeerServer, closePeerServer,
  startHarnessServer, openHarness, waitForSnapshot, HOST_DECK, GUEST_DECK, test,
} from './peerjs-resync-harness.js';

async function setup(t) {
  const peerPort = await freePort();
  const peerServer = await startPeerServer(peerPort);
  t.after(() => closePeerServer(peerServer));
  const { vite, baseUrl } = await startHarnessServer(peerPort);
  t.after(() => vite.close());
  const browser = await chromium.launch();
  t.after(() => browser.close());
  const hostContext = await browser.newContext(), guestContext = await browser.newContext();
  const host = await openHarness(hostContext, baseUrl, 'optimistic host');
  const guest = await openHarness(guestContext, baseUrl, 'optimistic guest');
  await host.evaluate(deckText => window.__peerHarness.createLobby({ name: 'Host', desiredPlayers: 2,
    securityMode: 'verified', startingLife: 20, deckText }), HOST_DECK);
  const lobby = await waitForSnapshot(host, snap => snap.multiplayer.mode === 'lobby' && snap.multiplayer.lobbyId, 'host lobby');
  await guest.evaluate(({ lobbyId, deckText }) => window.__peerHarness.joinLobby({ name: 'Guest', lobbyId, deckText }),
    { lobbyId: lobby.multiplayer.lobbyId, deckText: GUEST_DECK });
  await waitForSnapshot(host, snap => snap.canStartHostedMatch, 'both peers ready');
  await host.evaluate(() => window.__peerHarness.startHostedMatch());
  await waitForSnapshot(host, snap => snap.multiplayer.matchStarted, 'host started');
  await waitForSnapshot(guest, snap => snap.multiplayer.matchStarted, 'guest started');
  return { host, guest };
}

test('Verified peers calculate dependent actions while both verification queues are delayed', { timeout: 60000 }, async t => {
  const { host, guest } = await setup(t);
  for (const page of [host, guest]) await page.evaluate(() => {
    window.__peerHarness.enableOptimisticRuntime();
    window.__peerHarness.setApplyDelay(2000);
  });
  const submitted = await host.evaluate(async () => {
    const start = performance.now();
    const result = await window.__peerHarness.submitMultiplayerCommand({ type: 'priority_action',
      action_ref: { kind: 'test_priority_action', actor: 0, sequence: 0 } });
    return { result, duration: performance.now() - start };
  });
  assert.equal(submitted.result?.provisional, true, JSON.stringify(await host.evaluate(() => window.__peerHarness.snapshot())));
  assert.ok(submitted.duration < 1500, `click returned after ${submitted.duration}ms`);
  const guestProvisional = await waitForSnapshot(guest, snap => snap.visibleState?.decision?.player === 1
    && snap.multiplayer.pendingVerification > 0, 'guest sees the next choice before verification');
  assert.equal(guestProvisional.multiplayer.lastAppliedSequence, 0);
  await guest.evaluate(() => window.__peerHarness.submitMultiplayerCommand({ type: 'priority_action',
    action_ref: { kind: 'test_priority_action', actor: 1, sequence: 1 } }));
  const hostDependent = await waitForSnapshot(host, snap => snap.visibleState?.snapshot_id === 2
    && snap.multiplayer.pendingVerification > 0, 'host calculates guest response on the provisional state');
  assert.ok(hostDependent.multiplayer.lastAppliedSequence < 2);
  for (const page of [host, guest]) {
    const settled = await waitForSnapshot(page, snap => snap.multiplayer.lastAppliedSequence === 2
      && snap.multiplayer.pendingVerification === 0, 'both actions verified in order');
    assert.equal(settled.visibleState.snapshot_id, 2);
    assert.ok(!settled.statusEvents.some(event => /invalid action sequence|Sync failed|Cheat detected/.test(event.message)));
  }
  assertNoPageErrors(host, guest);
});


test('rejected parent rolls both peers back and cancels the dependent local command', { timeout: 60000 }, async t => {
  const { host, guest } = await setup(t);
  for (const page of [host, guest]) await page.evaluate(() => {
    window.__peerHarness.enableOptimisticRuntime();
    window.__peerHarness.setApplyDelay(2500);
  });
  await host.evaluate(() => window.__peerHarness.rejectNextVerifiedDispatch());
  await host.evaluate(() => window.__peerHarness.submitMultiplayerCommand({ type: 'priority_action',
    action_ref: { kind: 'test_priority_action', actor: 0, sequence: 0 } }));
  await waitForSnapshot(guest, snap => snap.visibleState?.decision?.player === 1
    && snap.multiplayer.pendingVerification > 0, 'guest can respond provisionally');
  await guest.evaluate(() => window.__peerHarness.submitMultiplayerCommand({ type: 'priority_action',
    action_ref: { kind: 'test_priority_action', actor: 1, sequence: 1 } }));
  await waitForSnapshot(host, snap => snap.visibleState?.snapshot_id === 2, 'dependent calculation visible');
  for (const page of [host, guest]) {
    const rolledBack = await waitForSnapshot(page, snap => snap.visibleState?.snapshot_id === 0
      && snap.multiplayer.pendingVerification === 0, 'both peers return to the verified base');
    assert.equal(rolledBack.multiplayer.lastAppliedSequence, 0);
    assert.equal(rolledBack.auditTranscript.actions.length, 0);
  }
  await new Promise(resolve => setTimeout(resolve, 300));
  for (const page of [host, guest]) {
    const settled = await page.evaluate(() => window.__peerHarness.snapshot());
    assert.equal(settled.multiplayer.lastAppliedSequence, 0);
    assert.equal(settled.visibleState.snapshot_id, 0);
  }
  assertNoPageErrors(host, guest);
});

test('a material-blocked click releases as soon as its prepared calculation is available', { timeout: 60000 }, async t => {
  const { host, guest } = await setup(t);
  for (const page of [host, guest]) await page.evaluate(() => {
    window.__peerHarness.enableOptimisticRuntime();
    window.__peerHarness.setApplyDelay(2000);
  });
  await host.evaluate(() => window.__peerHarness.blockNextOptimisticCalculation());
  await host.evaluate(() => {
    const sign = crypto.subtle.sign.bind(crypto.subtle);
    crypto.subtle.sign = async (...args) => {
      await new Promise(resolve => setTimeout(resolve, 1500));
      return sign(...args);
    };
  });
  const result = await host.evaluate(() => window.__peerHarness.submitMultiplayerCommand({ type: 'priority_action',
    action_ref: { kind: 'test_priority_action', actor: 0, sequence: 0 } }));
  assert.equal(result?.provisional, true, JSON.stringify(await host.evaluate(() => window.__peerHarness.snapshot())));
  const beforeVerification = await host.evaluate(() => window.__peerHarness.snapshot());
  assert.equal(beforeVerification.multiplayer.lastAppliedSequence, 0);
  assert.equal(beforeVerification.multiplayer.submittingAction, false);
  assert.equal(beforeVerification.multiplayer.pendingVerification, 1);
  await waitForSnapshot(guest, snap => snap.visibleState?.decision?.player === 1
    && snap.multiplayer.pendingVerification > 0, 'peer sees the prepared provisional calculation');
  await guest.evaluate(() => window.__peerHarness.submitMultiplayerCommand({ type: 'priority_action',
    action_ref: { kind: 'test_priority_action', actor: 1, sequence: 1 } }));
  for (const page of [host, guest]) await waitForSnapshot(page,
    snap => snap.multiplayer.lastAppliedSequence === 2 && snap.multiplayer.pendingVerification === 0,
    'prepared action and dependent response verify');
  assertNoPageErrors(host, guest);
});
