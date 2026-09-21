import { assert, chromium, closePeerServer, freePort, openHarness, startHarnessServer, startPeerServer, test, waitForSnapshot } from './peerjs-resync-harness.js';

test('trusted PeerJS host and guest retain their seats and accepted history across refreshes', { timeout: 90000 }, async () => {
  const port = await freePort();
  const peerServer = await startPeerServer(port);
  const { vite, baseUrl } = await startHarnessServer(port);
  const browser = await chromium.launch();
  try {
    const host = await openHarness(await browser.newContext(), baseUrl, 'host');
    const guest = await openHarness(await browser.newContext(), baseUrl, 'guest');
    await host.evaluate(() => window.__peerHarness.createLobby({ name: 'Host', desiredPlayers: 2, startingLife: 20, securityMode: 'trusted', deckText: '60 Island' }));
    const lobby = await waitForSnapshot(host, s => s.multiplayer.mode === 'lobby', 'host lobby');
    const lobbyId = lobby.multiplayer.lobbyId;
    await guest.evaluate(lobbyId => window.__peerHarness.joinLobby({ name: 'Guest', lobbyId, deckText: '60 Mountain' }), lobbyId);
    await waitForSnapshot(host, s => s.canStartHostedMatch, 'ready');
    await host.evaluate(() => window.__peerHarness.startHostedMatch());
    await waitForSnapshot(guest, s => s.multiplayer.matchStarted, 'started');
    const guestId = (await guest.evaluate(() => window.__peerHarness.lobbyState())).multiplayer.localPeerId;
    const act = async (page, actor, sequence) => {
      await page.evaluate(({ actor, sequence }) => window.__peerHarness.submitMultiplayerCommand({ type: 'priority_action', action_ref: { kind: 'test_priority_action', actor, sequence } }, 'Refresh regression'), { actor, sequence });
      for (const p of [host, guest]) await waitForSnapshot(p, s => s.multiplayer.lastAppliedSequence === sequence + 1, 'accepted');
    };
    const refresh = async page => {
      await page.reload();
      await page.waitForFunction(() => window.__peerHarness?.ready);
      await page.evaluate(lobbyId => window.__peerHarness.joinLobby({ name: 'Reopened', lobbyId }), lobbyId);
    };
    await act(host, 0, 0);
    await refresh(host);
    await waitForSnapshot(host, s => s.multiplayer.role === 'host' && s.multiplayer.lastAppliedSequence === 1 && s.multiplayer.players.every(p => p.connected), 'host restored');
    await act(guest, 1, 1);
    await refresh(guest);
    const restored = await waitForSnapshot(guest, s => s.multiplayer.lastAppliedSequence === 2, 'guest restored');
    assert.equal(restored.multiplayer.localPeerId, guestId);
    assert.equal(restored.multiplayer.localPlayerIndex, 1);
    await act(host, 0, 2);
    await refresh(host);
    await waitForSnapshot(host, s => s.multiplayer.lastAppliedSequence === 3 && s.multiplayer.players.every(p => p.connected), 'second host restore');
    await act(guest, 1, 3);
  } finally {
    await browser.close(); await vite.close(); await closePeerServer(peerServer);
  }
});
