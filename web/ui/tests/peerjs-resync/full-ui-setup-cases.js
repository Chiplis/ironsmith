import {
  FULL_UI_RECONNECT_DISCONNECT_MS,
  activateLocalButton,
  assert,
  assertNoFullUiSyncFailures,
  assertNoFullUiSyncFailuresWithDebug,
  assertNoPageErrors,
  assertNoSyncFailureText,
  buttonDebugText,
  clickAnyFullUiProgressAction,
  clickEnabledButton,
  clickLocalButton,
  clickLocalDecisionButton,
  clickUnselectedEnabledButton,
  chromium,
  closePeerServer,
  deckUrlParam,
  driveDemonicConsultationCast,
  freePort,
  fullUiPageDebug,
  fullUiSnapshot,
  hasLocalButton,
  openFullUiPage,
  resolveDemonicConsultationWithMissingName,
  sleep,
  snapshotBattlefieldCardCount,
  startFullUiPeerMatch,
  startHarnessServer,
  startPeerServer,
  test,
  visibleBodyText,
  waitAndClickLocalButton,
  waitForFullUiPair,
  waitForFullUiSequenceAdvance,
  waitForFullUiSnapshot,
  waitForFullUiSync,
  waitForLocalButton,
  waitForNamedVisibleHand,
  waitForVisibleBodyText,
  withTimeout,
} from "../peerjs-resync-harness.js";

test("full UI PeerJS 60-Mountain match lets both players play hidden-deck lands without opening mismatch", { timeout: 240000 }, async () => {
  const peerPort = await freePort();
  const peerServer = await startPeerServer(peerPort);
  const { vite, baseUrl } = await startHarnessServer(peerPort);
  const browser = await chromium.launch();
  const hostContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  const guestContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  let hostPage = null;
  let guestPage = null;

  try {
    const deck = deckUrlParam("60 Mountain");
    hostPage = await openFullUiPage(hostContext, `${baseUrl}/?name=Chiplis&deck=${deck}`, "host-ui");
    await hostPage.getByText("CREATE LOBBY").first().waitFor({ timeout: 30000 });
    await hostPage.getByRole("button").filter({ hasText: /CREATE LOBBY/i }).first().click();
    await hostPage.getByText("Host or join").waitFor({ timeout: 10000 });
    await hostPage.getByRole("button").filter({ hasText: /CREATE LOBBY/i }).last().click();
    await hostPage.getByText("Share this code").waitFor({ timeout: 40000 });

    const lobbyCode = (await visibleBodyText(hostPage)).match(
      /[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/i
    )?.[0];
    assert.ok(lobbyCode, "expected the full UI to create a lobby code");

    guestPage = await openFullUiPage(
      guestContext,
      `${baseUrl}/?lobby=${encodeURIComponent(lobbyCode)}&name=Alice&deck=${deck}`,
      "guest-ui",
    );
    await hostPage.getByText("All players are ready").first().waitFor({ timeout: 70000 });
    await guestPage.getByText("All players are ready").first().waitFor({ timeout: 70000 });

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
    await sleep(500);

    let hostPlayed = false;
    let guestPlayed = false;
    for (let step = 0; step < 120; step += 1) {
      const visibleText = `${await visibleBodyText(hostPage)}\n${await visibleBodyText(guestPage)}`;
      assert.doesNotMatch(
        visibleText,
        /Unknown Ziffle Ceremony|Unknown ziffle ceremony|Private deck opening does not match slot|Sync failed|Match start failed|Auto-pass failed/i
      );

      const hostPlayMountain = !hostPlayed
        ? await clickLocalButton(hostPage, "host-play", /PLAY MOUNTAIN/i)
        : null;
      if (!hostPlayed && hostPlayMountain) {
        hostPlayed = true;
        await sleep(3000);
        continue;
      }

      const guestPlayMountain = hostPlayed && !guestPlayed
        ? await clickLocalButton(guestPage, "guest-play", /PLAY MOUNTAIN/i)
        : null;
      if (!guestPlayed && guestPlayMountain) {
        guestPlayed = true;
        await sleep(6000);
        break;
      }

      const hostClicked = await clickLocalDecisionButton(hostPage, "host");
      if (hostClicked) {
        await sleep(2200);
        continue;
      }
      const guestClicked = await clickLocalDecisionButton(guestPage, "guest");
      if (guestClicked) {
        await sleep(2200);
        continue;
      }
      await sleep(1000);
    }

    assert.equal(hostPlayed, true, "expected host to play a Mountain");
    assert.equal(guestPlayed, true, "expected guest to play a Mountain");

    await waitForFullUiPair(
      hostPage,
      guestPage,
      (host, guest) =>
        snapshotBattlefieldCardCount(host, "Mountain") >= 2
        && snapshotBattlefieldCardCount(guest, "Mountain") >= 2,
      "expected both played Mountains to be public battlefield objects",
      60000,
    );
    const hostText = await visibleBodyText(hostPage);
    const guestText = await visibleBodyText(guestPage);
    assert.doesNotMatch(
      `${hostText}\n${guestText}`,
      /Private deck opening does not match slot|Sync failed|Match start failed/i
    );
    assertNoPageErrors(hostPage, guestPage);
  } finally {
    await withTimeout(Promise.allSettled([
      hostContext.close(),
      guestContext.close(),
      browser.close(),
    ]), 10000);
    await withTimeout(vite.close(), 10000);
    await closePeerServer(peerServer);
  }
});

test("full UI PeerJS real WASM game resumes after 15s reconnects and host takeover", { timeout: 420000 }, async () => {
  const peerPort = await freePort();
  const peerServer = await startPeerServer(peerPort);
  const { vite, baseUrl } = await startHarnessServer(peerPort);
  const browser = await chromium.launch();
  const hostContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  const guestContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  let hostPage = null;
  let guestPage = null;

  try {
    const started = await startFullUiPeerMatch({
      baseUrl,
      hostContext,
      guestContext,
      hostDeckText: "60 Mountain",
      guestDeckText: "60 Mountain",
      hostName: "Host",
      guestName: "Guest",
      hostLabel: "host-real-reconnect-ui",
      guestLabel: "guest-real-reconnect-ui",
    });
    hostPage = started.hostPage;
    guestPage = started.guestPage;
    const { lobbyCode, hostDeck, guestDeck } = started;

    const initialMatch = await waitForFullUiSync(
      hostPage,
      guestPage,
      "clients should be synced before disconnect checks",
      120000,
    );

    await guestPage.close();
    guestPage = null;
    const guestDisconnectedAt = Date.now();
    await sleep(FULL_UI_RECONNECT_DISCONNECT_MS);
    assert.ok(Date.now() - guestDisconnectedAt >= 15000, "guest should remain disconnected for at least 15 seconds");
    await waitForFullUiSnapshot(
      hostPage,
      (snap) => snap.multiplayer.role === "host"
        && snap.multiplayer.players.some((player) => Number(player.index) === 1 && player.connected === false),
      "host should mark the guest offline during the 15 second disconnect",
      30000,
    );

    guestPage = await openFullUiPage(
      guestContext,
      `${baseUrl}/?lobby=${encodeURIComponent(lobbyCode)}&name=Guest&deck=${guestDeck}`,
      "guest-real-reconnect-ui-2",
    );
    await waitForFullUiSnapshot(
      guestPage,
      (snap) => snap.multiplayer.matchStarted
        && snap.multiplayer.role === "client"
        && snap.multiplayer.localPlayerIndex === 1
        && Number(snap.multiplayer.lastAppliedSequence) >= Number(initialMatch.guest.multiplayer.lastAppliedSequence),
      "guest should reconnect after 15 seconds and resume the real WASM match",
      120000,
    );
    const afterGuestReconnect = await waitForFullUiSync(
      hostPage,
      guestPage,
      "guest reconnect should converge with the host",
      120000,
    );

    await hostPage.close();
    hostPage = null;
    const hostDisconnectedAt = Date.now();
    await sleep(FULL_UI_RECONNECT_DISCONNECT_MS);
    assert.ok(Date.now() - hostDisconnectedAt >= 15000, "host should remain disconnected for at least 15 seconds");
    const promotedGuest = await waitForFullUiSnapshot(
      guestPage,
      (snap) => snap.multiplayer.role === "host"
        && snap.multiplayer.localPlayerIndex === 1
        && snap.multiplayer.localPeerId === lobbyCode
        && snap.multiplayer.hostPeerId === lobbyCode,
      "guest should take over as host after the original host disconnects",
      60000,
    );
    assert.equal(promotedGuest.multiplayer.players.find((player) => Number(player.index) === 0)?.connected, false);

    hostPage = await openFullUiPage(
      hostContext,
      `${baseUrl}/?lobby=${encodeURIComponent(lobbyCode)}&name=Host&deck=${hostDeck}`,
      "host-real-reconnect-ui-2",
    );
    await waitForFullUiSnapshot(
      hostPage,
      (snap) => snap.multiplayer.matchStarted
        && snap.multiplayer.role === "client"
        && snap.multiplayer.localPlayerIndex === 0
        && Number(snap.multiplayer.lastAppliedSequence) >= Number(afterGuestReconnect.host.multiplayer.lastAppliedSequence),
      "original host should reconnect to the promoted guest host",
      120000,
    );
    const afterHostReconnect = await waitForFullUiPair(
      hostPage,
      guestPage,
      (host, guest) => host?.multiplayer?.matchStarted
        && guest?.multiplayer?.matchStarted
        && Number(host?.multiplayer?.lastAppliedSequence) === Number(guest?.multiplayer?.lastAppliedSequence)
        && host.multiplayer.role === "client"
        && guest.multiplayer.role === "host"
        && host.multiplayer.players.some((player) =>
          Number(player.index) === 1
          && String(player.routePeerId || player.currentPeerId || player.peerId || "") === lobbyCode
        ),
      "original host reconnect should converge with promoted host routing",
      120000,
    );
    assert.equal(afterHostReconnect.guest.multiplayer.role, "host");
    assert.equal(afterHostReconnect.host.multiplayer.role, "client");
    const hostViewPromotedHost = afterHostReconnect.host.multiplayer.players.find(
      (player) => Number(player.index) === 1
    );
    assert.equal(
      hostViewPromotedHost?.routePeerId || hostViewPromotedHost?.currentPeerId || hostViewPromotedHost?.peerId,
      lobbyCode,
      "reconnected original host should route ziffle requests for the promoted host through the live host peer",
    );
    const reconnectedHostHand = await waitForNamedVisibleHand(
      hostPage,
      "original host should see named hand cards after reconnecting to the promoted host",
      120000,
      (snap) => snap?.multiplayer?.matchStarted
        && snap.multiplayer.role === "client"
        && Number(snap.multiplayer.localPlayerIndex) === 0
        && Number(snap.state?.perspective) === 0,
    );
    assert.equal(
      reconnectedHostHand.filter((name) => /^Hidden Card$/i.test(name)).length,
      0,
      `expected original host hand to be revealed after reconnect: ${JSON.stringify(reconnectedHostHand)}`
    );

    const beforeResumeSequence = Number(afterHostReconnect.host.multiplayer.lastAppliedSequence || 0);
    const beforeResumeSnapshot = Number(afterHostReconnect.host.state.snapshot_id || 0);
    await clickAnyFullUiProgressAction([hostPage, guestPage], "post takeover resume action", 120000);
    const afterResume = await waitForFullUiPair(
      hostPage,
      guestPage,
      (host, guest) => Number(host.multiplayer.lastAppliedSequence) === Number(guest.multiplayer.lastAppliedSequence)
        && (
          Number(host.multiplayer.lastAppliedSequence || 0) > beforeResumeSequence
          || Number(host.state.snapshot_id || 0) > beforeResumeSnapshot
        ),
      "match should accept another real action after host takeover and reconnect",
      120000,
    );
    assert.ok(
      Number(afterResume.host.multiplayer.lastAppliedSequence || 0) > beforeResumeSequence
        || Number(afterResume.host.state.snapshot_id || 0) > beforeResumeSnapshot,
      "expected the real WASM game to advance after host takeover reconnect",
    );

    await assertNoFullUiSyncFailures(hostPage, guestPage);
    assertNoPageErrors(hostPage, guestPage);
  } finally {
    await withTimeout(Promise.allSettled([
      hostContext.close(),
      guestContext.close(),
      browser.close(),
    ]), 10000);
    await withTimeout(vite.close(), 10000);
    await closePeerServer(peerServer);
  }
});

test("full UI PeerJS Mulligan redraw stays synced", { timeout: 300000 }, async () => {
  const peerPort = await freePort();
  const peerServer = await startPeerServer(peerPort);
  const { vite, baseUrl } = await startHarnessServer(peerPort);
  const browser = await chromium.launch();
  const hostContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  const guestContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  let hostPage = null;
  let guestPage = null;

  try {
    const deck = deckUrlParam("60 Mountain");
    hostPage = await openFullUiPage(hostContext, `${baseUrl}/?name=Chiplis&deck=${deck}`, "host-ui");
    await hostPage.getByText("CREATE LOBBY").first().waitFor({ timeout: 30000 });
    await hostPage.getByRole("button").filter({ hasText: /CREATE LOBBY/i }).first().click();
    await hostPage.getByText("Host or join").waitFor({ timeout: 10000 });
    await hostPage.getByRole("button").filter({ hasText: /CREATE LOBBY/i }).last().click();
    await hostPage.getByText("Share this code").waitFor({ timeout: 40000 });

    const lobbyCode = (await visibleBodyText(hostPage)).match(
      /[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/i
    )?.[0];
    assert.ok(lobbyCode, "expected the full UI to create a lobby code");

    guestPage = await openFullUiPage(
      guestContext,
      `${baseUrl}/?lobby=${encodeURIComponent(lobbyCode)}&name=Alice&deck=${deck}`,
      "guest-ui",
    );
    await hostPage.getByText("All players are ready").first().waitFor({ timeout: 70000 });
    await guestPage.getByText("All players are ready").first().waitFor({ timeout: 70000 });

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
    await sleep(500);

    const initialText = `${await visibleBodyText(hostPage)}\n${await visibleBodyText(guestPage)}`;
    assertNoSyncFailureText(initialText, "match start should not sync-fail before mulligan");
    assert.match(initialText, /KEEP HAND[\s\S]*MULLIGAN/i, "expected a visible opening-hand decision");

    await waitForLocalButton(hostPage, /Mulligan/i, "expected Player 0 mulligan button to become local");
    const hostMulligan = await activateLocalButton(hostPage, "host-mulligan", /Mulligan/i);
    assert.ok(
      hostMulligan,
      `expected Player 0 to be able to mulligan\nhost buttons: ${JSON.stringify(await buttonDebugText(hostPage), null, 2)}\nhost body:\n${await visibleBodyText(hostPage)}`
    );
    await sleep(3000);

    const afterHostMulliganText = `${await visibleBodyText(hostPage)}\n${await visibleBodyText(guestPage)}`;
    assertNoSyncFailureText(afterHostMulliganText, "host mulligan click should not sync-fail");
    await sleep(25000);

    try {
      await waitForLocalButton(
        guestPage,
        /KEEP HAND/i,
        "expected Player 1 to receive a local opening-hand decision after Player 0 mulligans",
        120000,
      );
    } catch (err) {
      assert.fail(
        `${err?.message || err}\nhost buttons: ${JSON.stringify(await buttonDebugText(hostPage), null, 2)}\nhost body:\n${await visibleBodyText(hostPage)}\nhost errors: ${JSON.stringify(hostPage.__peerHarnessErrors || [], null, 2)}\nguest errors: ${JSON.stringify(guestPage.__peerHarnessErrors || [], null, 2)}`
      );
    }
    const guestKeep = await activateLocalButton(guestPage, "guest-keep", /KEEP HAND/i);
    assert.ok(
      guestKeep,
      `expected Player 1 to be able to keep after Player 0 keeps\nhost body:\n${await visibleBodyText(hostPage)}\nguest buttons: ${JSON.stringify(await buttonDebugText(guestPage), null, 2)}\nguest body:\n${await visibleBodyText(guestPage)}`
    );
    await sleep(3000);
    const afterGuestKeepCombined = `${await visibleBodyText(hostPage)}\n${await visibleBodyText(guestPage)}`;
    try {
      assertNoSyncFailureText(afterGuestKeepCombined, "Player 1 keep should not sync-fail");
	    } catch (err) {
	      assert.fail(
	        `${err?.message || err}`
	        + `\nhost body:\n${await visibleBodyText(hostPage)}`
	        + `\nguest body:\n${await visibleBodyText(guestPage)}`
	      );
	    }

    await waitForLocalButton(
      hostPage,
      /KEEP HAND/i,
      "expected Player 0 to receive a fresh local keep decision after Player 1 keeps",
      120000,
    );
    const hostRedrawText = await waitForVisibleBodyText(
      hostPage,
      /KEEP HAND[\s\S]*MULLIGAN/i,
      "expected Player 0 to receive a fresh keep/mulligan decision after redraw",
      120000,
    );
    assertNoSyncFailureText(hostRedrawText, "mulligan redraw should not sync-fail");

    const hostKeep = await activateLocalButton(hostPage, "host-keep-redraw", /KEEP HAND/i);
    assert.ok(hostKeep, "expected Player 0 to be able to keep the redraw hand");
    await sleep(3000);
    const afterHostKeepCombined = `${await visibleBodyText(hostPage)}\n${await visibleBodyText(guestPage)}`;
    try {
      assertNoSyncFailureText(afterHostKeepCombined, "Player 0 keep after redraw should not sync-fail");
    } catch (err) {
      assert.fail(
        `${err?.message || err}\nhost console: ${JSON.stringify((hostPage.__peerHarnessConsole || []).slice(-80), null, 2)}\nguest console: ${JSON.stringify((guestPage.__peerHarnessConsole || []).slice(-80), null, 2)}`
      );
    }

    const bottomPromptText = await waitForVisibleBodyText(
      hostPage,
      /Choose 1 card\(s\) to put on the bottom of your library/i,
      "expected Player 0 to bottom one card after keeping a mulligan hand",
      120000,
    );
    assertNoSyncFailureText(bottomPromptText, "mulligan bottom-card prompt should not sync-fail");

    const bottomChoice = await clickEnabledButton(hostPage, "host-bottom-card", /^Mountain$/i);
    assert.ok(bottomChoice, "expected Player 0 to choose a Mountain to bottom");
    const submitBottom = await activateLocalButton(hostPage, "host-submit-bottom-card", /SUBMIT/i);
    assert.ok(submitBottom, "expected Player 0 bottom-card choice to be submittable");

    const finalText = await waitForVisibleBodyText(
      hostPage,
      /PREGAME|BEGIN GAME|CONTINUE|UNTAP|UPKEEP|DRAW/i,
      "expected mulligan flow to advance after bottoming a card",
      120000,
    );
    const combinedFinalText = `${finalText}\n${await visibleBodyText(guestPage)}`;
    assertNoSyncFailureText(combinedFinalText, "full mulligan flow should stay synced");
    assertNoPageErrors(hostPage, guestPage);
  } finally {
    await withTimeout(Promise.allSettled([
      hostContext.close(),
      guestContext.close(),
      browser.close(),
    ]), 10000);
    await withTimeout(vite.close(), 10000);
    await closePeerServer(peerServer);
  }
});

test("full UI PeerJS guest Mulligan redraw stays synced", { timeout: 300000 }, async () => {
  const peerPort = await freePort();
  const peerServer = await startPeerServer(peerPort);
  const { vite, baseUrl } = await startHarnessServer(peerPort);
  const browser = await chromium.launch();
  const hostContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  const guestContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  let hostPage = null;
  let guestPage = null;

  try {
    const deck = deckUrlParam("30 Swamp\n30 Island");
    hostPage = await openFullUiPage(hostContext, `${baseUrl}/?name=Chiplis&deck=${deck}`, "host-guest-mulligan-ui");
    await hostPage.getByText("CREATE LOBBY").first().waitFor({ timeout: 30000 });
    await hostPage.getByRole("button").filter({ hasText: /CREATE LOBBY/i }).first().click();
    await hostPage.getByText("Host or join").waitFor({ timeout: 10000 });
    await hostPage.getByRole("button").filter({ hasText: /CREATE LOBBY/i }).last().click();
    await hostPage.getByText("Share this code").waitFor({ timeout: 40000 });

    const lobbyCode = (await visibleBodyText(hostPage)).match(
      /[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/i
    )?.[0];
    assert.ok(lobbyCode, `expected the full UI to create a lobby code\n${await visibleBodyText(hostPage)}`);

    guestPage = await openFullUiPage(
      guestContext,
      `${baseUrl}/?lobby=${encodeURIComponent(lobbyCode)}&name=Alice&deck=${deck}`,
      "guest-guest-mulligan-ui",
    );
    await hostPage.getByText("All players are ready").first().waitFor({ timeout: 70000 });
    await guestPage.getByText("All players are ready").first().waitFor({ timeout: 70000 });

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
    await sleep(500);

    const initialText = `${await visibleBodyText(hostPage)}\n${await visibleBodyText(guestPage)}`;
    assertNoSyncFailureText(initialText, "match start should not sync-fail before guest mulligan");
    assert.match(initialText, /KEEP HAND[\s\S]*MULLIGAN/i, "expected a visible opening-hand decision");

    await waitForLocalButton(hostPage, /KEEP HAND/i, "expected Player 0 keep button to become local");
    const hostKeep = await activateLocalButton(hostPage, "host-keep-before-guest-mulligan", /KEEP HAND/i);
    assert.ok(hostKeep, `expected Player 0 to be able to keep\n${await visibleBodyText(hostPage)}`);
    await sleep(3000);

    const afterHostKeepText = `${await visibleBodyText(hostPage)}\n${await visibleBodyText(guestPage)}`;
    assertNoSyncFailureText(afterHostKeepText, "host keep before guest mulligan should not sync-fail");

    await waitForLocalButton(guestPage, /Mulligan/i, "expected Player 1 mulligan button to become local", 120000);
    const beforeGuestMulliganSnapshot = await fullUiSnapshot(guestPage);
    const beforeGuestMulliganSequence = Number(
      beforeGuestMulliganSnapshot?.multiplayer?.lastAppliedSequence || 0
    );
    const guestMulligan = await activateLocalButton(guestPage, "guest-mulligan", /Mulligan/i);
    assert.ok(
      guestMulligan,
      `expected Player 1 to be able to mulligan\nhost body:\n${await visibleBodyText(hostPage)}\nguest body:\n${await visibleBodyText(guestPage)}`
    );
    await waitForFullUiPair(
      hostPage,
      guestPage,
      (host, guest) => {
        const hostSeq = Number(host?.multiplayer?.lastAppliedSequence || 0);
        const guestSeq = Number(guest?.multiplayer?.lastAppliedSequence || 0);
        const guestLocal = Number(guest?.multiplayer?.localPlayerIndex);
        return hostSeq > beforeGuestMulliganSequence
          && guestSeq === hostSeq
          && Number(guest?.state?.decision?.player) === guestLocal;
      },
      "expected Player 1 to receive a fresh opening-hand decision after mulligan",
      180000,
    );
    const guestRedrawHand = await waitForNamedVisibleHand(
      guestPage,
      "expected Player 1 redraw hand to be visible after mulligan",
      180000,
      (snap) =>
        Number(snap?.multiplayer?.lastAppliedSequence || 0) > beforeGuestMulliganSequence
        && Number(snap?.state?.decision?.player) === Number(snap?.multiplayer?.localPlayerIndex),
    );
    assert.ok(
      guestRedrawHand.length >= 7,
      `expected Player 1 redraw hand to contain visible cards: ${JSON.stringify(guestRedrawHand)}`
    );
    try {
      await assertNoFullUiSyncFailuresWithDebug(
        "guest mulligan redraw should not sync-fail",
        hostPage,
        guestPage,
      );
    } catch (err) {
      assert.fail(
        `${err?.message || err}\nhost console: ${JSON.stringify((hostPage.__peerHarnessConsole || []).slice(-80), null, 2)}\nguest console: ${JSON.stringify((guestPage.__peerHarnessConsole || []).slice(-80), null, 2)}`
      );
    }

    const guestKeep = await activateLocalButton(guestPage, "guest-keep-redraw", /KEEP HAND/i);
    assert.ok(guestKeep, `expected Player 1 to be able to keep the redraw hand\n${await visibleBodyText(guestPage)}`);
    const bottomPromptText = await waitForVisibleBodyText(
      guestPage,
      /Choose 1 card\(s\) to put on the bottom of your library/i,
      "expected Player 1 to bottom one card after keeping a mulligan hand",
      120000,
    );
    assertNoSyncFailureText(bottomPromptText, "guest keep after redraw should not sync-fail");

    const beforeBottomSnapshot = await fullUiSnapshot(guestPage);
    const beforeBottomSequence = Number(beforeBottomSnapshot?.multiplayer?.lastAppliedSequence || 0);
    const bottomChoice = await clickEnabledButton(guestPage, "guest-bottom-card", /^(Swamp|Island)$/i);
    assert.ok(bottomChoice, "expected Player 1 to choose a card to bottom");
    const submitBottom = await activateLocalButton(guestPage, "guest-submit-bottom-card", /SUBMIT/i);
    assert.ok(submitBottom, "expected Player 1 bottom-card choice to be submittable");

    await waitForFullUiSequenceAdvance(
      hostPage,
      guestPage,
      beforeBottomSequence,
      "expected mulligan flow to advance after Player 1 bottoms a card",
      180000,
    );
    await assertNoFullUiSyncFailuresWithDebug(
      "guest mulligan bottom flow should stay synced",
      hostPage,
      guestPage,
    );
    assertNoPageErrors(hostPage, guestPage);
  } finally {
    await withTimeout(Promise.allSettled([
      hostContext.close(),
      guestContext.close(),
      browser.close(),
    ]), 10000);
    await withTimeout(vite.close(), 10000);
    await closePeerServer(peerServer);
  }
});

test("full UI PeerJS repeated host Mulligans stay synced", { timeout: 300000 }, async () => {
  const peerPort = await freePort();
  const peerServer = await startPeerServer(peerPort);
  const { vite, baseUrl } = await startHarnessServer(peerPort);
  const browser = await chromium.launch();
  const hostContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  const guestContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  let hostPage = null;
  let guestPage = null;

  try {
    const deck = deckUrlParam("30 Swamp\n30 Island");
    hostPage = await openFullUiPage(hostContext, `${baseUrl}/?name=Chiplis&deck=${deck}`, "host-repeated-mulligan-ui");
    await hostPage.getByText("CREATE LOBBY").first().waitFor({ timeout: 30000 });
    await hostPage.getByRole("button").filter({ hasText: /CREATE LOBBY/i }).first().click();
    await hostPage.getByText("Host or join").waitFor({ timeout: 10000 });
    await hostPage.getByRole("button").filter({ hasText: /CREATE LOBBY/i }).last().click();
    await hostPage.getByText("Share this code").waitFor({ timeout: 40000 });

    const lobbyCode = (await visibleBodyText(hostPage)).match(
      /[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/i
    )?.[0];
    assert.ok(lobbyCode, `expected the full UI to create a lobby code\n${await visibleBodyText(hostPage)}`);

    guestPage = await openFullUiPage(
      guestContext,
      `${baseUrl}/?lobby=${encodeURIComponent(lobbyCode)}&name=Alice&deck=${deck}`,
      "guest-repeated-mulligan-ui",
    );
    await hostPage.getByText("All players are ready").first().waitFor({ timeout: 70000 });
    await guestPage.getByText("All players are ready").first().waitFor({ timeout: 70000 });

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
    await sleep(500);

    const initialText = `${await visibleBodyText(hostPage)}\n${await visibleBodyText(guestPage)}`;
    assertNoSyncFailureText(initialText, "match start should not sync-fail before repeated mulligans");
    assert.match(initialText, /KEEP HAND[\s\S]*MULLIGAN/i, "expected a visible opening-hand decision");

    await waitForLocalButton(hostPage, /Mulligan/i, "expected Player 0 first mulligan button to become local");
    const firstHostMulligan = await activateLocalButton(hostPage, "host-first-mulligan", /Mulligan/i);
    assert.ok(firstHostMulligan, `expected Player 0 to be able to mulligan\n${await visibleBodyText(hostPage)}`);
    await sleep(3000);
    assertNoSyncFailureText(
      `${await visibleBodyText(hostPage)}\n${await visibleBodyText(guestPage)}`,
      "first host mulligan should not sync-fail"
    );

    await waitForLocalButton(guestPage, /KEEP HAND/i, "expected Player 1 keep button after first host mulligan", 120000);
    const guestKeep = await activateLocalButton(guestPage, "guest-keep-before-second-host-mulligan", /KEEP HAND/i);
    assert.ok(guestKeep, `expected Player 1 to keep\n${await visibleBodyText(guestPage)}`);
    await sleep(3000);
    assertNoSyncFailureText(
      `${await visibleBodyText(hostPage)}\n${await visibleBodyText(guestPage)}`,
      "guest keep before second host mulligan should not sync-fail"
    );

    for (let mulliganNumber = 2; mulliganNumber <= 4; mulliganNumber += 1) {
      await waitForLocalButton(
        hostPage,
        /Mulligan/i,
        `expected Player 0 mulligan ${mulliganNumber} button to become local`,
        120000,
      );
      const hostMulliganAgain = await activateLocalButton(
        hostPage,
        `host-mulligan-${mulliganNumber}`,
        /Mulligan/i
      );
      assert.ok(
        hostMulliganAgain,
        `expected Player 0 to be able to mulligan ${mulliganNumber} times\n${await visibleBodyText(hostPage)}`
      );
      await sleep(mulliganNumber === 4 ? 20000 : 5000);

      const afterMulliganText = `${await visibleBodyText(hostPage)}\n${await visibleBodyText(guestPage)}`;
      try {
        assertNoSyncFailureText(afterMulliganText, `host mulligan ${mulliganNumber} should not sync-fail`);
      } catch (err) {
        assert.fail(
          `${err?.message || err}\nhost console: ${JSON.stringify((hostPage.__peerHarnessConsole || []).slice(-80), null, 2)}\nguest console: ${JSON.stringify((guestPage.__peerHarnessConsole || []).slice(-80), null, 2)}`
        );
      }
    }

    await waitForLocalButton(hostPage, /KEEP HAND/i, "expected Player 0 keep button after four mulligans", 120000);
    const hostKeepAfterFour = await activateLocalButton(hostPage, "host-keep-after-four-mulligans", /KEEP HAND/i);
    assert.ok(hostKeepAfterFour, `expected Player 0 to keep after four mulligans\n${await visibleBodyText(hostPage)}`);
    await sleep(5000);
    assertNoSyncFailureText(
      `${await visibleBodyText(hostPage)}\n${await visibleBodyText(guestPage)}`,
      "host keep after four mulligans should not sync-fail"
    );
    assertNoPageErrors(hostPage, guestPage);
  } finally {
    await withTimeout(Promise.allSettled([
      hostContext.close(),
      guestContext.close(),
      browser.close(),
    ]), 10000);
    await withTimeout(vite.close(), 10000);
    await closePeerServer(peerServer);
  }
});

test("full UI PeerJS both players Mulligan then host remulligans stays synced", { timeout: 300000 }, async () => {
  const peerPort = await freePort();
  const peerServer = await startPeerServer(peerPort);
  const { vite, baseUrl } = await startHarnessServer(peerPort);
  const browser = await chromium.launch();
  const hostContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  const guestContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  let hostPage = null;
  let guestPage = null;

  try {
    const deck = deckUrlParam("30 Swamp\n30 Island");
    hostPage = await openFullUiPage(hostContext, `${baseUrl}/?name=Chiplis&deck=${deck}`, "host-both-mulligan-ui");
    await hostPage.getByText("CREATE LOBBY").first().waitFor({ timeout: 30000 });
    await hostPage.getByRole("button").filter({ hasText: /CREATE LOBBY/i }).first().click();
    await hostPage.getByText("Host or join").waitFor({ timeout: 10000 });
    await hostPage.getByRole("button").filter({ hasText: /CREATE LOBBY/i }).last().click();
    await hostPage.getByText("Share this code").waitFor({ timeout: 40000 });

    const lobbyCode = (await visibleBodyText(hostPage)).match(
      /[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/i
    )?.[0];
    assert.ok(lobbyCode, `expected the full UI to create a lobby code\n${await visibleBodyText(hostPage)}`);

    guestPage = await openFullUiPage(
      guestContext,
      `${baseUrl}/?lobby=${encodeURIComponent(lobbyCode)}&name=Alice&deck=${deck}`,
      "guest-both-mulligan-ui",
    );
    await hostPage.getByText("All players are ready").first().waitFor({ timeout: 70000 });
    await guestPage.getByText("All players are ready").first().waitFor({ timeout: 70000 });

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
    await sleep(500);

    const initialText = `${await visibleBodyText(hostPage)}\n${await visibleBodyText(guestPage)}`;
    assertNoSyncFailureText(initialText, "match start should not sync-fail before both players mulligan");
    assert.match(initialText, /KEEP HAND[\s\S]*MULLIGAN/i, "expected a visible opening-hand decision");

    await waitForLocalButton(hostPage, /Mulligan/i, "expected Player 0 mulligan button to become local");
    const hostMulligan = await activateLocalButton(hostPage, "host-both-round-mulligan", /Mulligan/i);
    assert.ok(hostMulligan, `expected Player 0 to be able to mulligan\n${await visibleBodyText(hostPage)}`);
    await sleep(3000);
    assertNoSyncFailureText(
      `${await visibleBodyText(hostPage)}\n${await visibleBodyText(guestPage)}`,
      "host mulligan before guest mulligan should not sync-fail"
    );

    await waitForLocalButton(guestPage, /Mulligan/i, "expected Player 1 mulligan button to become local", 120000);
    const guestMulligan = await activateLocalButton(guestPage, "guest-both-round-mulligan", /Mulligan/i);
    assert.ok(guestMulligan, `expected Player 1 to be able to mulligan\n${await visibleBodyText(guestPage)}`);
    await sleep(5000);

    const afterBothMulliganText = `${await visibleBodyText(hostPage)}\n${await visibleBodyText(guestPage)}`;
    try {
      assertNoSyncFailureText(afterBothMulliganText, "both-player mulligan round should not sync-fail");
    } catch (err) {
      const hostDebug = await fullUiPageDebug(hostPage, 0);
      const guestDebug = await fullUiPageDebug(guestPage, 1);
      assert.fail(
        `${err?.message || err}`
        + `\nhost body:\n${await visibleBodyText(hostPage)}`
        + `\nguest body:\n${await visibleBodyText(guestPage)}`
        + `\nhost debug: ${JSON.stringify(hostDebug, null, 2)}`
        + `\nguest debug: ${JSON.stringify(guestDebug, null, 2)}`
      );
    }

    await waitForLocalButton(hostPage, /Mulligan/i, "expected Player 0 remulligan button after both players redraw", 120000);
    const hostSecondMulligan = await activateLocalButton(hostPage, "host-remulligan-after-both", /Mulligan/i);
    assert.ok(hostSecondMulligan, `expected Player 0 to be able to mulligan again\n${await visibleBodyText(hostPage)}`);
    await sleep(5000);

    const afterHostRemulliganText = `${await visibleBodyText(hostPage)}\n${await visibleBodyText(guestPage)}`;
    try {
      assertNoSyncFailureText(afterHostRemulliganText, "host remulligan after both-player round should not sync-fail");
    } catch (err) {
      const hostDebug = await fullUiPageDebug(hostPage, 0);
      const guestDebug = await fullUiPageDebug(guestPage, 1);
      assert.fail(
        `${err?.message || err}`
        + `\nhost body:\n${await visibleBodyText(hostPage)}`
        + `\nguest body:\n${await visibleBodyText(guestPage)}`
        + `\nhost debug: ${JSON.stringify(hostDebug, null, 2)}`
        + `\nguest debug: ${JSON.stringify(guestDebug, null, 2)}`
      );
    }
    assertNoPageErrors(hostPage, guestPage);
  } finally {
    await withTimeout(Promise.allSettled([
      hostContext.close(),
      guestContext.close(),
      browser.close(),
    ]), 10000);
    await withTimeout(vite.close(), 10000);
    await closePeerServer(peerServer);
  }
});

test("full UI PeerJS Gemstone Caverns pregame action stays synced", { timeout: 300000 }, async () => {
  const peerPort = await freePort();
  const peerServer = await startPeerServer(peerPort);
  const { vite, baseUrl } = await startHarnessServer(peerPort);
  const browser = await chromium.launch();
  const hostContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  const guestContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  let hostPage = null;
  let guestPage = null;

  try {
    const match = await startFullUiPeerMatch({
      baseUrl,
      hostContext,
      guestContext,
      hostDeckText: "60 Gemstone Caverns",
      guestDeckText: "60 Gemstone Caverns",
      hostName: "Chiplis",
      guestName: "Alice",
      hostLabel: "host-gemstone-ui",
      guestLabel: "guest-gemstone-ui",
    });
    hostPage = match.hostPage;
    guestPage = match.guestPage;

    const selectGemstoneForCurrentDecision = async (label) => {
      const selection = await guestPage.evaluate(() => {
        const snap = window.__ironsmithE2E?.snapshot?.();
        const candidate = (snap?.state?.decisionCandidates || []).find((entry) =>
          entry.legal !== false && /^Gemstone Caverns$/i.test(String(entry.name || ""))
        );
        if (!candidate) return null;
        return {
          id: candidate.id,
        };
      });
      assert.ok(
        selection?.id != null,
        `${label}\nexpected a Gemstone Caverns candidate\nbuttons: ${JSON.stringify(await buttonDebugText(guestPage), null, 2)}\nbody:\n${await visibleBodyText(guestPage)}`
      );
      const command = {
        type: "select_objects",
        object_ids: [Number(selection.id)],
      };
      await guestPage.evaluate(({ command: submittedCommand }) => (
        window.__ironsmithE2E.submitMultiplayerCommand(
          submittedCommand,
          "Selected 1 object(s)"
        )
      ), { command });
	    };

	    await assertNoFullUiSyncFailures(hostPage, guestPage);
	    const beforeHostKeep = Number((await fullUiSnapshot(hostPage))?.multiplayer?.lastAppliedSequence || 0);
	    await waitAndClickLocalButton(hostPage, "host-keep-gemstone-test", /KEEP HAND/i, 120000);
	    await waitForFullUiSequenceAdvance(
	      hostPage,
	      guestPage,
	      beforeHostKeep,
	      "expected host keep before Gemstone pregame to sync",
	      180000,
	    );
	    await assertNoFullUiSyncFailures(hostPage, guestPage);
	    const beforeGuestKeep = Number((await fullUiSnapshot(guestPage))?.multiplayer?.lastAppliedSequence || 0);
	    await waitAndClickLocalButton(guestPage, "guest-keep-gemstone-test", /KEEP HAND/i, 120000);
	    await waitForFullUiSequenceAdvance(
	      hostPage,
	      guestPage,
	      beforeGuestKeep,
	      "expected guest keep before Gemstone pregame to sync",
	      180000,
	    );
	    await assertNoFullUiSyncFailures(hostPage, guestPage);

    for (let step = 0; step < 12 && !(await hasLocalButton(guestPage, /BEGIN WITH GEMSTONE CAVERNS/i)); step += 1) {
      await clickAnyFullUiProgressAction([hostPage, guestPage], `advance-to-gemstone-pregame-${step}`, 60000);
      await sleep(1500);
    }

    await waitAndClickLocalButton(
      guestPage,
      "guest-begin-with-gemstone",
      /BEGIN WITH GEMSTONE CAVERNS/i,
      120000
    );
    await waitForVisibleBodyText(
      guestPage,
      /Choose 1 card\(s\) from your hand to exile for Gemstone Caverns/i,
      "expected Gemstone Caverns to ask for a hand card to exile",
      120000,
    );
    await sleep(500);
    await selectGemstoneForCurrentDecision("first Gemstone Caverns exile");
    await waitForFullUiPair(
      hostPage,
      guestPage,
      (host, guest) =>
        Number(host?.multiplayer?.lastAppliedSequence || 0) >= 5
        && Number(host?.multiplayer?.lastAppliedSequence || 0)
          === Number(guest?.multiplayer?.lastAppliedSequence || 0),
      "expected Gemstone Caverns exile choice to sync",
      120000,
    );

    await waitAndClickLocalButton(
      guestPage,
      "guest-begin-with-second-gemstone",
      /BEGIN WITH GEMSTONE CAVERNS/i,
      120000
    );
    await waitForVisibleBodyText(
      guestPage,
      /Choose 1 card\(s\) from your hand to exile for Gemstone Caverns/i,
      "expected second Gemstone Caverns to ask for a hand card to exile",
      120000,
    );
    await sleep(500);
    await selectGemstoneForCurrentDecision("second Gemstone Caverns exile");
    await waitForFullUiPair(
      hostPage,
      guestPage,
      (host, guest) =>
        Number(host?.multiplayer?.lastAppliedSequence || 0) >= 7
        && Number(host?.multiplayer?.lastAppliedSequence || 0)
          === Number(guest?.multiplayer?.lastAppliedSequence || 0),
      "expected second Gemstone Caverns exile choice to sync",
      120000,
    );

	    const legendRulePattern = /Choose which Gemstone Caverns to keep \(legend rule\)/i;
	    const progressPattern = /PREGAME|BEGIN GAME|CONTINUE|UNTAP|UPKEEP|DRAW|MAIN|PASS PRIORITY|RESOLVE/i;
	    for (let step = 0; step < 60; step += 1) {
	      if (legendRulePattern.test(await visibleBodyText(guestPage))) break;
	      await assertNoFullUiSyncFailuresWithDebug(
	        `advance-to-gemstone-legend-${step}: unexpected sync failure before click`,
	        hostPage,
	        guestPage,
	      );
	      await clickLocalButton(hostPage, `advance-to-gemstone-legend-host-${step}`, progressPattern);
	      await clickLocalButton(guestPage, `advance-to-gemstone-legend-guest-${step}`, progressPattern);
	      await sleep(1500);
	    }
	    await waitForVisibleBodyText(
	      guestPage,
	      legendRulePattern,
      "expected two pregame Gemstone Caverns to trigger the legend rule",
      120000,
    );
    await sleep(500);
    await selectGemstoneForCurrentDecision("Gemstone Caverns legend rule");
    await waitForFullUiPair(
      hostPage,
      guestPage,
      (host, guest) =>
        Number(host?.multiplayer?.lastAppliedSequence || 0) >= 9
        && Number(host?.multiplayer?.lastAppliedSequence || 0)
          === Number(guest?.multiplayer?.lastAppliedSequence || 0),
      "expected Gemstone Caverns legend rule choice to sync",
      120000,
    );
    await assertNoFullUiSyncFailures(hostPage, guestPage);
    assertNoPageErrors(hostPage, guestPage);
  } finally {
    await withTimeout(Promise.allSettled([
      hostContext.close(),
      guestContext.close(),
      browser.close(),
    ]), 10000);
    await withTimeout(vite.close(), 10000);
    await closePeerServer(peerServer);
  }
});

test("full UI PeerJS Gemstone Caverns after guest mulligans publishes its opening", { timeout: 360000 }, async () => {
  const peerPort = await freePort();
  const peerServer = await startPeerServer(peerPort);
  const { vite, baseUrl } = await startHarnessServer(peerPort);
  const browser = await chromium.launch();
  const hostContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  const guestContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  let hostPage = null;
  let guestPage = null;

  try {
    const match = await startFullUiPeerMatch({
      baseUrl,
      hostContext,
      guestContext,
      hostDeckText: "60 Gemstone Caverns",
      guestDeckText: "60 Gemstone Caverns",
      hostName: "Chiplis",
      guestName: "Alice",
      hostLabel: "host-gemstone-mulligan-ui",
      guestLabel: "guest-gemstone-mulligan-ui",
    });
    hostPage = match.hostPage;
    guestPage = match.guestPage;

    await assertNoFullUiSyncFailures(hostPage, guestPage);
    const beforeHostKeep = Number((await fullUiSnapshot(hostPage))?.multiplayer?.lastAppliedSequence || 0);
    await waitAndClickLocalButton(hostPage, "host-keep-before-guest-gemstone-mulligans", /KEEP HAND/i, 120000);
    await waitForFullUiSequenceAdvance(
      hostPage,
      guestPage,
      beforeHostKeep,
      "expected host keep before guest Gemstone mulligans to sync",
      180000,
    );
    await assertNoFullUiSyncFailuresWithDebug(
      "host keep before guest Gemstone mulligans should stay synced",
      hostPage,
      guestPage,
    );
    if (!(await hasLocalButton(guestPage, /Mulligan/i))) {
      await waitForFullUiPair(
        hostPage,
        guestPage,
        () => false,
        "expected guest Gemstone mulligan button after host keep",
        1000,
      );
    }

    for (let mulligan = 1; mulligan <= 2; mulligan += 1) {
      const beforeMulligan = Number((await fullUiSnapshot(guestPage))?.multiplayer?.lastAppliedSequence || 0);
      await waitAndClickLocalButton(guestPage, `guest-gemstone-mulligan-${mulligan}`, /Mulligan/i, 120000);
      await waitForFullUiSequenceAdvance(
        hostPage,
        guestPage,
        beforeMulligan,
        `expected guest Gemstone mulligan ${mulligan} to sync`,
        180000,
      );
      await waitForNamedVisibleHand(
        guestPage,
        `expected visible guest hand after Gemstone mulligan ${mulligan}`,
        180000,
        (snap) => Number(snap?.multiplayer?.lastAppliedSequence || 0) > beforeMulligan,
      );
      await assertNoFullUiSyncFailuresWithDebug(
        `guest Gemstone mulligan ${mulligan} should stay synced`,
        hostPage,
        guestPage,
      );
    }

    await waitAndClickLocalButton(guestPage, "guest-keep-after-two-gemstone-mulligans", /KEEP HAND/i, 120000);
    await waitForVisibleBodyText(
      guestPage,
      /Choose 2 card\(s\) to put on the bottom of your library/i,
      "expected guest to bottom two cards after two mulligans",
      120000,
    );
    const beforeBottom = Number((await fullUiSnapshot(guestPage))?.multiplayer?.lastAppliedSequence || 0);
    for (let index = 1; index <= 2; index += 1) {
      const clicked = await clickUnselectedEnabledButton(
        guestPage,
        `guest-bottom-gemstone-after-mulligans-${index}`,
        /^Gemstone Caverns$/i,
      );
      assert.ok(clicked, `expected guest to choose bottom card ${index}\n${await visibleBodyText(guestPage)}`);
    }
    await waitAndClickLocalButton(guestPage, "guest-submit-bottom-after-gemstone-mulligans", /SUBMIT/i, 60000);
    await waitForFullUiSequenceAdvance(
      hostPage,
      guestPage,
      beforeBottom,
      "expected guest bottom cards after Gemstone mulligans to sync",
      180000,
    );
    await assertNoFullUiSyncFailuresWithDebug(
      "guest bottom cards after Gemstone mulligans should stay synced",
      hostPage,
      guestPage,
    );
    if (await hasLocalButton(guestPage, /SUBMIT ORDER/i)) {
      const beforeBottomOrder = Number((await fullUiSnapshot(guestPage))?.multiplayer?.lastAppliedSequence || 0);
      await waitAndClickLocalButton(
        guestPage,
        "guest-submit-bottom-order-after-gemstone-mulligans",
        /SUBMIT ORDER/i,
        60000,
      );
      await waitForFullUiSequenceAdvance(
        hostPage,
        guestPage,
        beforeBottomOrder,
        "expected guest bottom-card order after Gemstone mulligans to sync",
        180000,
      );
      await assertNoFullUiSyncFailuresWithDebug(
        "guest bottom-card order after Gemstone mulligans should stay synced",
        hostPage,
        guestPage,
      );
    }

    for (
      let step = 0;
      step < 12 && !(await hasLocalButton(guestPage, /BEGIN WITH GEMSTONE CAVERNS/i));
      step += 1
    ) {
      await clickAnyFullUiProgressAction([hostPage, guestPage], `advance-to-mulliganed-gemstone-pregame-${step}`, 60000);
      await sleep(1500);
    }

    await waitAndClickLocalButton(
      guestPage,
      "guest-begin-with-gemstone-after-mulligans",
      /BEGIN WITH GEMSTONE CAVERNS/i,
      120000,
    );
    await waitForVisibleBodyText(
      guestPage,
      /Choose 1 card\(s\) from your hand to exile for Gemstone Caverns/i,
      "expected Gemstone Caverns to ask the mulliganed guest for a hand card to exile",
      120000,
    );
    const beforeGemstoneExile = Number((await fullUiSnapshot(guestPage))?.multiplayer?.lastAppliedSequence || 0);
    const selectedExile = await clickUnselectedEnabledButton(
      guestPage,
      "guest-exile-gemstone-after-mulligans",
      /^Gemstone Caverns$/i,
    );
    assert.ok(selectedExile, `expected guest to select a Gemstone Caverns to exile\n${await visibleBodyText(guestPage)}`);
    await waitAndClickLocalButton(guestPage, "guest-submit-gemstone-after-mulligans", /SUBMIT/i, 60000);
    await waitForFullUiSequenceAdvance(
      hostPage,
      guestPage,
      beforeGemstoneExile,
      "expected Gemstone Caverns exile after guest mulligans to sync",
      180000,
    );
    await assertNoFullUiSyncFailuresWithDebug(
      "Gemstone Caverns exile after guest mulligans should publish a public opening",
      hostPage,
      guestPage,
    );
    assertNoPageErrors(hostPage, guestPage);
  } finally {
    await withTimeout(Promise.allSettled([
      hostContext.close(),
      guestContext.close(),
      browser.close(),
    ]), 10000);
    await withTimeout(vite.close(), 10000);
    await closePeerServer(peerServer);
  }
});

test("full UI PeerJS Gemstone Caverns after both players mulligan remaps pregame source", { timeout: 480000 }, async () => {
  const peerPort = await freePort();
  const peerServer = await startPeerServer(peerPort);
  const { vite, baseUrl } = await startHarnessServer(peerPort);
  const browser = await chromium.launch();
  const hostContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  const guestContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  let hostPage = null;
  let guestPage = null;

  try {
    const match = await startFullUiPeerMatch({
      baseUrl,
      hostContext,
      guestContext,
      hostDeckText: "30 Swamp\n30 Demonic Consultation",
      guestDeckText: "60 Gemstone Caverns",
      hostName: "Alice",
      guestName: "Chiplis",
      hostLabel: "host-both-mulligan-gemstone-ui",
      guestLabel: "guest-both-mulligan-gemstone-ui",
    });
    hostPage = match.hostPage;
    guestPage = match.guestPage;

    await assertNoFullUiSyncFailures(hostPage, guestPage);
    const beforeHostMulligan = Number((await fullUiSnapshot(hostPage))?.multiplayer?.lastAppliedSequence || 0);
    await waitAndClickLocalButton(hostPage, "host-mulligan-before-guest-gemstone", /Mulligan/i, 120000);
    await waitForFullUiSequenceAdvance(
      hostPage,
      guestPage,
      beforeHostMulligan,
      "expected host mulligan before guest Gemstone to sync",
      180000,
    );
    await assertNoFullUiSyncFailuresWithDebug(
      "host mulligan before guest Gemstone should stay synced",
      hostPage,
      guestPage,
    );

    const beforeGuestMulligan = Number((await fullUiSnapshot(guestPage))?.multiplayer?.lastAppliedSequence || 0);
    await waitAndClickLocalButton(guestPage, "guest-mulligan-before-gemstone", /Mulligan/i, 120000);
    await waitForFullUiSequenceAdvance(
      hostPage,
      guestPage,
      beforeGuestMulligan,
      "expected guest mulligan before Gemstone to sync",
      180000,
    );
    await assertNoFullUiSyncFailuresWithDebug(
      "guest mulligan before Gemstone should stay synced",
      hostPage,
      guestPage,
    );

    const mulliganBottoms = [
      {
        page: hostPage,
        label: "host",
        bottomPattern: /^(Swamp|Demonic Consultation)$/i,
      },
      {
        page: guestPage,
        label: "guest",
        bottomPattern: /^Gemstone Caverns$/i,
      },
    ];
    for (let keepIndex = 0; keepIndex < 2; keepIndex += 1) {
      const startedKeep = Date.now();
      let kept = false;
      while (!kept && Date.now() - startedKeep < 120000) {
        await assertNoFullUiSyncFailuresWithDebug(
          `waiting for keep decision ${keepIndex + 1} after both mulligans`,
          hostPage,
          guestPage,
        );
        for (const entry of mulliganBottoms) {
          const beforeKeep = Number((await fullUiSnapshot(entry.page))?.multiplayer?.lastAppliedSequence || 0);
          const keep = await activateLocalButton(
            entry.page,
            `${entry.label}-keep-after-both-mulligan-${keepIndex + 1}`,
            /KEEP HAND/i,
          );
          if (!keep) continue;
          await waitForFullUiSequenceAdvance(
            hostPage,
            guestPage,
            beforeKeep,
            `expected ${entry.label} keep after both mulligans to sync`,
            180000,
          );
          kept = true;
          break;
        }
        if (!kept) await sleep(250);
      }
      assert.ok(
        kept,
        `expected a local keep decision ${keepIndex + 1} after both mulligans\nhost:\n${await visibleBodyText(hostPage)}\nguest:\n${await visibleBodyText(guestPage)}`,
      );
    }

    for (let bottomIndex = 0; bottomIndex < 2; bottomIndex += 1) {
      const startedBottom = Date.now();
      let bottomed = false;
      while (!bottomed && Date.now() - startedBottom < 120000) {
        await assertNoFullUiSyncFailuresWithDebug(
          `waiting for mulligan bottom decision ${bottomIndex + 1}`,
          hostPage,
          guestPage,
        );
        for (const entry of mulliganBottoms) {
          const body = await visibleBodyText(entry.page);
          if (!/Choose 1 card\(s\) to put on the bottom of your library/i.test(body)) {
            continue;
          }
          const buttons = await buttonDebugText(entry.page);
          if (!buttons.some((button) =>
            button.localAction === "true" && /SUBMIT/i.test(button.text)
          )) {
            continue;
          }
          await waitForVisibleBodyText(
            entry.page,
            /Choose 1 card\(s\) to put on the bottom of your library/i,
            `expected ${entry.label} to bottom one card after mulligan before guest Gemstone`,
            120000,
          );
          const beforeBottom = Number((await fullUiSnapshot(entry.page))?.multiplayer?.lastAppliedSequence || 0);
          const selectedBottom = await clickUnselectedEnabledButton(
            entry.page,
            `${entry.label}-bottom-after-both-mulligan-${bottomIndex + 1}`,
            entry.bottomPattern,
          );
          assert.ok(
            selectedBottom,
            `expected ${entry.label} to choose a bottom card\n${await visibleBodyText(entry.page)}`,
          );
          await waitAndClickLocalButton(
            entry.page,
            `${entry.label}-submit-bottom-before-guest-gemstone-${bottomIndex + 1}`,
            /SUBMIT/i,
            60000,
          );
          await waitForFullUiSequenceAdvance(
            hostPage,
            guestPage,
            beforeBottom,
            `expected ${entry.label} bottom before guest Gemstone to sync`,
            180000,
          );
          bottomed = true;
          break;
        }
        if (!bottomed) await sleep(250);
      }
      assert.ok(
        bottomed,
        `expected a local keep/bottom decision ${bottomIndex + 1} after both mulligans\nhost:\n${await visibleBodyText(hostPage)}\nguest:\n${await visibleBodyText(guestPage)}`,
      );
    }
    await assertNoFullUiSyncFailuresWithDebug(
      "bottoming after both mulligans before Gemstone should stay synced",
      hostPage,
      guestPage,
    );

    for (
      let step = 0;
      step < 12 && !(await hasLocalButton(guestPage, /BEGIN WITH GEMSTONE CAVERNS/i));
      step += 1
    ) {
      await clickAnyFullUiProgressAction([hostPage, guestPage], `advance-to-both-mulligan-gemstone-${step}`, 60000);
      await sleep(1500);
    }

    const beforeGemstone = Number((await fullUiSnapshot(guestPage))?.multiplayer?.lastAppliedSequence || 0);
    await waitAndClickLocalButton(
      guestPage,
      "guest-begin-with-gemstone-after-both-mulligan",
      /BEGIN WITH GEMSTONE CAVERNS/i,
      120000,
    );
    await waitForFullUiSequenceAdvance(
      hostPage,
      guestPage,
      beforeGemstone,
      "expected guest Gemstone pregame source after both mulligans to remap and sync",
      180000,
    );
    await assertNoFullUiSyncFailuresWithDebug(
      "guest Gemstone pregame source after both mulligans should stay synced",
      hostPage,
      guestPage,
    );

    await waitForVisibleBodyText(
      guestPage,
      /Choose 1 card\(s\) from your hand to exile for Gemstone Caverns/i,
      "expected Gemstone Caverns to ask the guest for an exile after both mulligans",
      120000,
    );
    const beforeGemstoneExile = Number((await fullUiSnapshot(guestPage))?.multiplayer?.lastAppliedSequence || 0);
    const selectedGemstoneExile = await clickUnselectedEnabledButton(
      guestPage,
      "guest-exile-gemstone-after-both-mulligan",
      /^Gemstone Caverns$/i,
    );
    assert.ok(
      selectedGemstoneExile,
      `expected guest to select a Gemstone Caverns to exile after both mulligans\n${await visibleBodyText(guestPage)}`
    );
    await waitAndClickLocalButton(
      guestPage,
      "guest-submit-gemstone-exile-after-both-mulligan",
      /SUBMIT/i,
      60000,
    );
    await waitForFullUiSequenceAdvance(
      hostPage,
      guestPage,
      beforeGemstoneExile,
      "expected guest Gemstone exile after both mulligans to sync",
      180000,
    );
    await assertNoFullUiSyncFailuresWithDebug(
      "guest Gemstone exile after both mulligans should stay synced",
      hostPage,
      guestPage,
    );

    await driveDemonicConsultationCast({
      hostPage,
      guestPage,
      actorPage: hostPage,
      actorLabel: "host-after-both-mulligan-gemstone",
      maxSteps: 260,
    });
    const { host: hostAfterSwamp, guest: guestAfterSwamp } = await waitForFullUiSync(
      hostPage,
      guestPage,
      "expected peers to agree after host Swamp after Gemstone",
      120000,
    );
    assert.ok(
      snapshotBattlefieldCardCount(hostAfterSwamp, "Swamp") >= 1,
      `host should see a Swamp on the battlefield\n${JSON.stringify(hostAfterSwamp?.state?.players, null, 2)}`
    );
    assert.ok(
      snapshotBattlefieldCardCount(guestAfterSwamp, "Swamp") >= 1,
      `guest should see a Swamp on the battlefield\n${JSON.stringify(guestAfterSwamp?.state?.players, null, 2)}`
    );
    await resolveDemonicConsultationWithMissingName({
      hostPage,
      guestPage,
      actorPage: hostPage,
      actorLabel: "host-after-both-mulligan-gemstone",
      actorIndex: 0,
      missingName: "Black Lotus",
      maxSteps: 260,
    });
    assertNoPageErrors(hostPage, guestPage);
  } finally {
    await withTimeout(Promise.allSettled([
      hostContext.close(),
      guestContext.close(),
      browser.close(),
    ]), 10000);
    await withTimeout(vite.close(), 10000);
    await closePeerServer(peerServer);
  }
});
