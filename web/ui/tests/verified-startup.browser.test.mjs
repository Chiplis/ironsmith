import {
  assert, test, chromium, freePort, startPeerServer, closePeerServer,
  startHarnessServer, openFullUiPage, waitForFullUiSnapshot,
  deckUrlParam, waitForNamedVisibleHand, assertNoFullUiSyncFailures,
  waitAndClickLocalButton, fullUiSnapshot,
} from './peerjs-resync-harness.js';

const cases = [
  ...[2, 3, 4].map(playerCount => ({ playerCount, securityMode: 'verified', quotaFailure: false })),
  ...['trusted', 'verified'].map(securityMode => ({ playerCount: 2, securityMode, quotaFailure: true })),
];
for (const { playerCount, securityMode, quotaFailure } of cases) test(`${playerCount}-player casual ${securityMode} P2P starts without the witness relay${quotaFailure ? ' despite reconnect storage quota failure' : ''}`, { timeout: 120000 }, async t => {
  const peerPort = await freePort();
  const peerServer = await startPeerServer(peerPort);
  t.after(() => closePeerServer(peerServer));
  const { vite, baseUrl } = await startHarnessServer(peerPort);
  t.after(() => vite.close());
  const browser = await chromium.launch();
  t.after(() => browser.close());
  const contexts = await Promise.all(Array.from({ length: playerCount }, () => browser.newContext()));
  if (quotaFailure) for (const context of contexts) await context.addInitScript(() => {
    const setItem = Storage.prototype.setItem;
    Storage.prototype.setItem = function (key, value) {
      if (String(key).startsWith('ironsmith-peerjs-resume-v1:')) {
        window.__reconnectQuotaFailures = (window.__reconnectQuotaFailures || 0) + 1;
        throw new DOMException('Setting the value exceeded the quota.', 'QuotaExceededError');
      }
      return setItem.call(this, key, value);
    };
  });
  for (const context of contexts) await context.route('**/ironsmith-lobby-relay.*/**', route => {
    return route.abort();
  });
  const deck = deckUrlParam('60 Mountain');
  const host = await openFullUiPage(contexts[0], `${baseUrl}/?name=Host&deck=${deck}&securityMode=${securityMode}`, 'quota-host');
  await host.getByRole('button', { name: 'Lobby', exact: true }).click();
  await host.getByText('Host or join').waitFor();
  await host.getByRole('combobox', { name: 'Players', exact: true }).selectOption(String(playerCount));
  await host.getByRole('button').filter({ hasText: /CREATE LOBBY/i }).last().click();
  const lobby = await waitForFullUiSnapshot(host, s => s.multiplayer.mode === 'lobby' && s.multiplayer.lobbyId, 'host creates Verified lobby');
  assert.equal(lobby.multiplayer.securityMode, securityMode);
  if (quotaFailure) assert.ok(await host.evaluate(() => window.__reconnectQuotaFailures > 0));
  assert.equal(lobby.multiplayer.desiredPlayers, playerCount);
  const guests = [];
  for (let seat = 1; seat < playerCount; seat++) {
    guests.push(await openFullUiPage(contexts[seat], `${baseUrl}/?name=Guest${seat}&deck=${deck}&lobby=${lobby.multiplayer.lobbyId}`, `verified-guest-${seat}`));
  }
  await waitForFullUiSnapshot(host, s => s.canStartHostedMatch, 'Verified seats ready');
  await host.getByRole('button').filter({ hasText: /START GAME/i }).click();
  await Promise.all([host, ...guests].map(async page => {
    const state = await waitForFullUiSnapshot(page, s => s.multiplayer.matchStarted && s.multiplayer.mode === 'in_match', 'Verified P2P starts', 45000);
    assert.equal(state.multiplayer.securityMode, securityMode);
    assert.equal(state.multiplayer.tournament ?? null, null);
    if (quotaFailure) assert.ok(await page.evaluate(() => window.__reconnectQuotaFailures > 0));
    await page.keyboard.press('Escape');
    const hand = await waitForNamedVisibleHand(page, 'Verified opening hand reveals without witness', 30000);
    assert.deepEqual(hand, Array(7).fill('Mountain'));
  }));
  await assertNoFullUiSyncFailures(host, ...guests);
  const pages = [host, ...guests];
  for (let action = 0; action < playerCount; action++) {
    const snapshot = await fullUiSnapshot(host);
    const seat = Number(snapshot.state.decision.player);
    await waitAndClickLocalButton(pages[seat], `seat ${seat} keeps opening hand`, /KEEP HAND/i, 15000);
    await Promise.all(pages.map(page => waitForFullUiSnapshot(page,
      s => s.multiplayer.lastAppliedSequence >= action + 1,
      `Keep Hand from seat ${seat} reaches every peer`, 20000)));
    await assertNoFullUiSyncFailures(...pages);
  }
});
