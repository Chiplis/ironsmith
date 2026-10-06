import {
  assert, chromium, freePort, startPeerServer, closePeerServer, startHarnessServer,
  fullUiSnapshot, waitForFullUiPair, assertNoPageErrors, test,
  deckUrlParam, openFullUiPage, waitForVisibleBodyText, waitForFullUiSnapshot, sleep,
} from './peerjs-resync-harness.js';

async function startSurrenderMatch({
  baseUrl,
  hostContext,
  guestContext,
  hostDeckText = "60 Mountain",
  guestDeckText = "60 Mountain",
  hostName = "Chiplis",
  guestName = "Alice",
  hostLabel = "host-ui",
  guestLabel = "guest-ui",
  securityMode = "",
}) {
  const hostDeck = deckUrlParam(hostDeckText);
  const guestDeck = deckUrlParam(guestDeckText);
  const securityQuery = securityMode ? `&securityMode=${encodeURIComponent(securityMode)}` : "";
  const hostPage = await openFullUiPage(
    hostContext,
    `${baseUrl}/?name=${encodeURIComponent(hostName)}&deck=${hostDeck}${securityQuery}`,
    hostLabel
  );
  await waitForVisibleBodyText(hostPage, /LOBBY/i, "host shows lobby control", 30000);
  await hostPage.getByRole("button").filter({ hasText: /^(?:CREATE )?LOBBY$/i }).first().click();
  await waitForVisibleBodyText(hostPage, /Host or join/i, "host shows lobby chooser", 120000);
  await hostPage.getByRole("button").filter({ hasText: /CREATE LOBBY/i }).last().click();
  const lobby = await waitForFullUiSnapshot(hostPage,
    snap => snap.multiplayer.mode === 'lobby' && snap.multiplayer.lobbyId,
    "host creates shareable lobby", 120000);
  const lobbyCode = lobby.multiplayer.lobbyId;
  assert.ok(lobbyCode, "expected the full UI to create a lobby code");

  const guestPage = await openFullUiPage(
    guestContext,
    `${baseUrl}/?lobby=${encodeURIComponent(lobbyCode)}&name=${encodeURIComponent(guestName)}&deck=${guestDeck}`,
    guestLabel,
  );
  await Promise.all([
    waitForFullUiSnapshot(
      hostPage,
      (snap) => snap.canStartHostedMatch
        && snap.multiplayer.mode === "lobby"
        && snap.multiplayer.players.length === 2
        && snap.multiplayer.players.every((player) => player.connected !== false && player.ready),
      "host can start full UI match",
      60000,
    ),
    waitForFullUiSnapshot(
      guestPage,
      (snap) => snap.multiplayer.mode === "lobby"
        && snap.multiplayer.localPlayerIndex === 1
        && snap.multiplayer.players.length === 2
        && snap.multiplayer.players.every((player) => player.connected !== false && player.ready),
      "guest is ready in full UI lobby",
      60000,
    ),
  ]);

  await hostPage.getByRole("button").filter({ hasText: /START GAME/i }).click();
  await hostPage.getByRole("button").filter({ hasText: /START GAME/i }).waitFor({
    state: "detached",
    timeout: 60000,
  }).catch(() => {});
  await sleep(8000);
  await Promise.all([
    hostPage.keyboard.press("Escape").catch(() => {}),
    guestPage.keyboard.press("Escape").catch(() => {}),
  ]);

  await Promise.all([
    waitForFullUiSnapshot(
      hostPage,
      (snap) => snap.multiplayer.matchStarted && snap.multiplayer.localPlayerIndex === 0,
      "host starts full UI match",
      60000,
    ),
    waitForFullUiSnapshot(
      guestPage,
      (snap) => snap.multiplayer.matchStarted && snap.multiplayer.localPlayerIndex === 1,
      "guest receives full UI match",
      60000,
    ),
  ]);

  return {
    hostPage,
    guestPage,
    lobbyCode,
    hostDeck,
    guestDeck,
  };
}


for (const securityMode of ['trusted', 'verified']) {
  test(`${securityMode}: surrender during the opponent's decision ends both games`, { timeout: 180000 }, async t => {
    const peerPort = await freePort();
    const peerServer = await startPeerServer(peerPort); t.after(() => closePeerServer(peerServer));
    const { vite, baseUrl } = await startHarnessServer(peerPort); t.after(() => vite.close());
    const browser = await chromium.launch(); t.after(() => browser.close());
    const hostContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
    const guestContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
    for (const context of [hostContext, guestContext]) await context.route('**/lobbies', route =>
      route.fulfill({ contentType: 'application/json', body: '{"lobbies":[]}', headers: { 'Access-Control-Allow-Origin': '*' } }));
    const { hostPage: host, guestPage: guest } = await startSurrenderMatch({ baseUrl, hostContext, guestContext, securityMode });
    const before = await fullUiSnapshot(host);
    assert.equal(before.multiplayer.securityMode, securityMode);
    const actor = before.state.decision.player;
    const surrenderer = actor === 0 ? guest : host;
    assert.equal(await surrenderer.getByRole('button', { name: /Add Card|Compile Card/ }).count(), 0);
    await surrenderer.getByRole('button', { name: 'Surrender', exact: true }).first().click();
    await surrenderer.getByRole('button', { name: 'Yes', exact: true }).first().click();
    await waitForFullUiPair(host, guest, (a, b) => a.state.game_over && b.state.game_over
      && a.multiplayer.lastAppliedSequence === b.multiplayer.lastAppliedSequence,
      'surrender reaches both peers', 30000);
    assertNoPageErrors(host); assertNoPageErrors(guest);
  });
}
