import { test, assert, chromium, startHarnessServer } from './peerjs-resync-harness.js';

test('journal recovery preserves accepted actions, while rematches start a fresh journal', async t => {
  const { vite, baseUrl } = await startHarnessServer(1);
  t.after(() => vite.close());
  const browser = await chromium.launch();
  t.after(() => browser.close());
  const page = await browser.newPage();
  await page.route('**/journal-test', route => route.fulfill({ contentType: 'text/html', body: '<title>Journal regression</title>' }));
  await page.goto(`${baseUrl}/journal-test`);
  const results = await page.evaluate(async () => {
    const { initializeRelayMatch, appendRelayAction, relayCheckpoint } = await import('/src/lib/relay/session.js');
    const results = [];
    for (const securityMode of ['trusted', 'verified']) {
      const lobbyId = `journal-${securityMode}`;
      const session = { lobbyId, securityMode, role: 'host', players: [], matchStarted: true };
      const first = { lobbyId, auditMatchId: lobbyId, seed: 123, securityMode,
        ...(securityMode === 'verified' ? { genesis: { payloadHash: 'first-genesis' } } : {}) };
      await initializeRelayMatch(lobbyId, { match: first, session, actions: [] });
      await appendRelayAction(lobbyId, first, session, { seq: 1, command: { type: 'priority_action' } });
      const recovered = await initializeRelayMatch(lobbyId, { match: first, session, actions: [] });
      const preserved = (await relayCheckpoint(lobbyId)).actions.length;
      const rematch = { ...first, seed: securityMode === 'verified' ? first.seed : 456,
        ...(securityMode === 'verified' ? { genesis: { payloadHash: 'second-genesis' } } : {}) };
      await initializeRelayMatch(lobbyId, { match: rematch, session, actions: [] });
      const reset = (await relayCheckpoint(lobbyId)).actions.length;
      let staleRejected = false;
      try { await appendRelayAction(lobbyId, first, session, { seq: 1 }); }
      catch (error) { staleRejected = error.message === 'Match journal is not initialized'; }
      await appendRelayAction(lobbyId, rematch, session, { seq: 1 });
      const appended = (await relayCheckpoint(lobbyId)).actions.length;
      results.push({ securityMode, recovered, preserved, reset, staleRejected, appended });
    }
    return results;
  });
  for (const result of results) {
    assert.equal(result.recovered, 1, result.securityMode);
    assert.equal(result.preserved, 1, result.securityMode);
    assert.equal(result.reset, 0, result.securityMode);
    assert.equal(result.staleRejected, true, result.securityMode);
    assert.equal(result.appended, 1, result.securityMode);
  }
});
