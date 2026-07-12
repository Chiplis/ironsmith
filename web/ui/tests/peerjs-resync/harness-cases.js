import {
  FOUR_PLAYER_DECKS,
  GUEST_DECK,
  HOST_DECK,
  HOST_ZIFFLE_OPENED_LAND_DECK,
  HOST_ZIFFLE_PUBLIC_OPEN_DECK,
  assert,
  assertNoPageErrors,
  checkpointImportEvents,
  chromium,
  closePeerServer,
  compactSnapshotForFailure,
  freePort,
  openHarness,
  sleep,
  snapshot,
  startHarnessServer,
  startPeerServer,
  syncedCommandEvents,
  test,
  waitForSnapshot,
  withTimeout,
} from "../peerjs-resync-harness.js";

test("PeerJS remote apply gates auto-pass until sequence append completes", { timeout: 60000 }, async () => {
  const peerPort = await freePort();
  const peerServer = await startPeerServer(peerPort);
  const { vite, baseUrl } = await startHarnessServer(peerPort);
  const browser = await chromium.launch();
  const hostContext = await browser.newContext();
  const guestContext = await browser.newContext();
  let hostPage = null;
  let guestPage = null;

  try {
    hostPage = await openHarness(hostContext, baseUrl, "host");
    guestPage = await openHarness(guestContext, baseUrl, "guest");

    await hostPage.evaluate((deckText) => {
      window.__peerHarness.createLobby({
        name: "Host",
        desiredPlayers: 2,
        startingLife: 20,
        deckText,
      });
    }, HOST_DECK);

    const hostLobby = await waitForSnapshot(
      hostPage,
      (snap) => snap.multiplayer.mode === "lobby" && snap.multiplayer.lobbyId,
      "host creates a lobby for auto-pass sequence test",
    );

    await guestPage.evaluate(({ lobbyId, deckText }) => {
      window.__peerHarness.joinLobby({
        name: "Guest",
        lobbyId,
        deckText,
      });
    }, { lobbyId: hostLobby.multiplayer.lobbyId, deckText: GUEST_DECK });

    await waitForSnapshot(
      hostPage,
      (snap) => snap.canStartHostedMatch
        && snap.multiplayer.players.length === 2
        && snap.multiplayer.players.every((player) => player.connected !== false),
      "both peers join and are ready for auto-pass sequence test",
    );

    await hostPage.evaluate(() => window.__peerHarness.startHostedMatch());
    await waitForSnapshot(hostPage, (snap) => snap.multiplayer.matchStarted, "host starts auto-pass test match");
    await waitForSnapshot(guestPage, (snap) => snap.multiplayer.matchStarted, "guest receives auto-pass test match");
    await guestPage.evaluate(() => window.__peerHarness.setAutoPass(true));

    await hostPage.evaluate(() => {
      window.__peerHarness.submitMultiplayerCommand({
        type: "priority_action",
        action_ref: {
          kind: "test_priority_action",
          actor: 0,
          sequence: 0,
        },
      }, "host action before guest auto-pass");
    });

    const hostAfterAutoPass = await waitForSnapshot(
      hostPage,
      (snap) => snap.multiplayer.lastAppliedSequence === 2
        && snap.visibleState?.decision?.player === 0,
      "host receives guest auto-pass as sequence 2",
    );
    const guestAfterAutoPass = await waitForSnapshot(
      guestPage,
      (snap) => snap.multiplayer.lastAppliedSequence === 2
        && snap.visibleState?.decision?.player === 0,
      "guest records its auto-pass as sequence 2",
    );

    assert.deepEqual(
      syncedCommandEvents(hostAfterAutoPass).map((event) => event.syncContext?.sequence),
      [1, 2],
      "host should apply host action and guest auto-pass in distinct sequence slots",
    );
    assert.deepEqual(
      syncedCommandEvents(guestAfterAutoPass).map((event) => event.syncContext?.sequence),
      [1, 2],
      "guest should not reuse the remote action sequence for its immediate auto-pass",
    );
    assertNoPageErrors(hostPage, guestPage);
  } finally {
    await hostPage?.close().catch(() => {});
    await guestPage?.close().catch(() => {});
    await hostContext.close().catch(() => {});
    await guestContext.close().catch(() => {});
    await browser.close().catch(() => {});
    await withTimeout(vite.close(), 10000);
    await closePeerServer(peerServer);
  }
});

test("PeerJS click during the final sync window is submitted after the previous action clears", { timeout: 60000 }, async () => {
  const peerPort = await freePort();
  const peerServer = await startPeerServer(peerPort);
  const { vite, baseUrl } = await startHarnessServer(peerPort);
  const browser = await chromium.launch();
  const hostContext = await browser.newContext();
  const guestContext = await browser.newContext();
  let hostPage = null;
  let guestPage = null;

  try {
    hostPage = await openHarness(hostContext, baseUrl, "host");
    guestPage = await openHarness(guestContext, baseUrl, "guest");

    await hostPage.evaluate((deckText) => {
      window.__peerHarness.createLobby({
        name: "Host",
        desiredPlayers: 2,
        startingLife: 20,
        deckText,
      });
    }, HOST_DECK);

    const hostLobby = await waitForSnapshot(
      hostPage,
      (snap) => snap.multiplayer.mode === "lobby" && snap.multiplayer.lobbyId,
      "host creates a lobby for submit-idle wait test",
    );

    await guestPage.evaluate(({ lobbyId, deckText }) => {
      window.__peerHarness.joinLobby({
        name: "Guest",
        lobbyId,
        deckText,
      });
    }, { lobbyId: hostLobby.multiplayer.lobbyId, deckText: GUEST_DECK });

    await waitForSnapshot(
      hostPage,
      (snap) => snap.canStartHostedMatch
        && snap.multiplayer.players.length === 2
        && snap.multiplayer.players.every((player) => player.connected !== false),
      "both peers join and are ready for submit-idle wait test",
    );

    await hostPage.evaluate(() => window.__peerHarness.startHostedMatch());
    await waitForSnapshot(hostPage, (snap) => snap.multiplayer.matchStarted, "host starts submit-idle wait match");
    await waitForSnapshot(guestPage, (snap) => snap.multiplayer.matchStarted, "guest receives submit-idle wait match");
    await guestPage.evaluate(() => window.__peerHarness.setApplyDelay(800));

    await hostPage.evaluate(() => {
      window.__peerHarness.submitMultiplayerCommand({
        type: "priority_action",
        action_ref: {
          kind: "test_priority_action",
          actor: 0,
          sequence: 0,
        },
      }, "host action before guest quick click");
    });

    await waitForSnapshot(
      guestPage,
      (snap) => snap.multiplayer.submittingAction
        && snap.visibleState?.snapshot_id === 1
        && Number(snap.visibleState?.decision?.player) === Number(snap.multiplayer.localPlayerIndex),
      "guest renders its next decision while previous action is still syncing",
    );

    await guestPage.evaluate(async () => {
      const snap = await window.__peerHarness.snapshot();
      const action = snap.visibleState?.decision?.actions?.[0];
      if (!action?.action_ref) {
        throw new Error("guest has no local action to submit during sync");
      }
      return window.__peerHarness.submitMultiplayerCommand({
        type: "priority_action",
        action_ref: action.action_ref,
      }, "guest click while sync finishing");
    });

    const hostAfterGuestClick = await waitForSnapshot(
      hostPage,
      (snap) => snap.multiplayer.lastAppliedSequence === 2,
      "host receives guest click after previous sync clears",
    );
    const guestAfterGuestClick = await waitForSnapshot(
      guestPage,
      (snap) => snap.multiplayer.lastAppliedSequence === 2,
      "guest applies click submitted during previous sync",
    );

    assert.deepEqual(
      syncedCommandEvents(guestAfterGuestClick).map((event) => event.syncContext?.sequence),
      [1, 2],
      "guest should apply the remote action and the queued local click in order",
    );
    assert.equal(hostAfterGuestClick.multiplayer.lastAppliedSequence, 2);
    assertNoPageErrors(hostPage, guestPage);
  } finally {
    await hostPage?.close().catch(() => {});
    await guestPage?.close().catch(() => {});
    await hostContext.close().catch(() => {});
    await guestContext.close().catch(() => {});
    await browser.close().catch(() => {});
    await withTimeout(vite.close(), 10000);
    await closePeerServer(peerServer);
  }
});

test("PeerJS client actions can collect host ziffle shuffle steps", { timeout: 60000 }, async () => {
  const peerPort = await freePort();
  const peerServer = await startPeerServer(peerPort);
  const { vite, baseUrl } = await startHarnessServer(peerPort);
  const browser = await chromium.launch();
  const hostContext = await browser.newContext();
  const guestContext = await browser.newContext();
  let hostPage = null;
  let guestPage = null;

  try {
    hostPage = await openHarness(hostContext, baseUrl, "host");
    guestPage = await openHarness(guestContext, baseUrl, "guest");

    await hostPage.evaluate((deckText) => {
      window.__peerHarness.createLobby({
        name: "Host",
        desiredPlayers: 2,
        startingLife: 20,
        securityMode: "verified",
        deckText,
      });
    }, HOST_DECK);

    const hostLobby = await waitForSnapshot(
      hostPage,
      (snap) => snap.multiplayer.mode === "lobby" && snap.multiplayer.lobbyId,
      "host creates a lobby for client ziffle shuffle test",
    );

    await guestPage.evaluate(({ lobbyId, deckText }) => {
      window.__peerHarness.joinLobby({
        name: "Guest",
        lobbyId,
        deckText,
      });
    }, { lobbyId: hostLobby.multiplayer.lobbyId, deckText: GUEST_DECK });

    await waitForSnapshot(
      hostPage,
      (snap) => snap.canStartHostedMatch
        && snap.multiplayer.players.length === 2
        && snap.multiplayer.players.every((player) => player.connected !== false),
      "both peers join and are ready for client ziffle shuffle test",
    );

    await hostPage.evaluate(() => window.__peerHarness.startHostedMatch());
    await waitForSnapshot(hostPage, (snap) => snap.multiplayer.matchStarted, "host starts client ziffle shuffle match");
    await waitForSnapshot(guestPage, (snap) => snap.multiplayer.matchStarted, "guest receives client ziffle shuffle match");

    await hostPage.evaluate(() => window.__peerHarness.submitMultiplayerCommand({
      type: "priority_action",
      action_ref: {
        kind: "test_priority_action",
        actor: 0,
        sequence: 0,
      },
    }, "host action before guest ziffle shuffle"));

    await waitForSnapshot(
      guestPage,
      (snap) => snap.multiplayer.lastAppliedSequence === 1
        && Number(snap.visibleState?.decision?.player) === Number(snap.multiplayer.localPlayerIndex),
      "guest owns the next action before ziffle shuffle",
    );

    await guestPage.evaluate(async () => {
      const snap = await window.__peerHarness.snapshot();
      const action = (snap.visibleState?.decision?.actions || []).find((entry) =>
        entry.action_ref?.kind === "ziffle_shuffle_action"
      );
      if (!action?.action_ref) {
        throw new Error("guest has no ziffle shuffle action to submit");
      }
      return window.__peerHarness.submitMultiplayerCommand({
        type: "priority_action",
        action_ref: action.action_ref,
      }, "guest ziffle shuffle action");
    });

    const hostAfterShuffle = await waitForSnapshot(
      hostPage,
      (snap) => snap.multiplayer.lastAppliedSequence === 2,
      "host applies guest ziffle shuffle action",
    );
    const guestAfterShuffle = await waitForSnapshot(
      guestPage,
      (snap) => snap.multiplayer.lastAppliedSequence === 2,
      "guest applies its ziffle shuffle action",
    );

    const statusText = [
      ...hostAfterShuffle.statusEvents,
      ...guestAfterShuffle.statusEvents,
    ].map((event) => event.message).join("\n");
    assert.match(
      statusText,
      /Waiting for cryptographic shuffle material from Host/,
      "client should request the host's live ziffle shuffle step",
    );
    assert.doesNotMatch(statusText, /Timed out waiting for ziffle shuffle step|Sync failed|Ziffle setup failed/i);
    assertNoPageErrors(hostPage, guestPage);
  } finally {
    await hostPage?.close().catch(() => {});
    await guestPage?.close().catch(() => {});
    await hostContext.close().catch(() => {});
    await guestContext.close().catch(() => {});
    await browser.close().catch(() => {});
    await withTimeout(vite.close(), 10000);
    await closePeerServer(peerServer);
  }
});

test("PeerJS remote public openings can prove ziffle positions against original deck slots", { timeout: 60000 }, async () => {
  const peerPort = await freePort();
  const peerServer = await startPeerServer(peerPort);
  const { vite, baseUrl } = await startHarnessServer(peerPort);
  const browser = await chromium.launch();
  const hostContext = await browser.newContext();
  const guestContext = await browser.newContext();
  let hostPage = null;
  let guestPage = null;

  try {
    hostPage = await openHarness(hostContext, baseUrl, "host");
    guestPage = await openHarness(guestContext, baseUrl, "guest");

    await hostPage.evaluate((deckText) => {
      window.__peerHarness.createLobby({
        name: "Host",
        desiredPlayers: 2,
        startingLife: 20,
        securityMode: "verified",
        deckText,
      });
    }, HOST_ZIFFLE_PUBLIC_OPEN_DECK);

    const hostLobby = await waitForSnapshot(
      hostPage,
      (snap) => snap.multiplayer.mode === "lobby" && snap.multiplayer.lobbyId,
      "host creates a lobby for ziffle public-open test",
    );

    await guestPage.evaluate(({ lobbyId, deckText }) => {
      window.__peerHarness.joinLobby({
        name: "Guest",
        lobbyId,
        deckText,
      });
    }, { lobbyId: hostLobby.multiplayer.lobbyId, deckText: GUEST_DECK });

    await waitForSnapshot(
      hostPage,
      (snap) => snap.canStartHostedMatch
        && snap.multiplayer.players.length === 2
        && snap.multiplayer.players.every((player) => player.connected !== false),
      "both peers join and are ready for ziffle public-open test",
    );

    await hostPage.evaluate(() => window.__peerHarness.startHostedMatch());
    await waitForSnapshot(hostPage, (snap) => snap.multiplayer.matchStarted, "host starts ziffle public-open match");
    await waitForSnapshot(guestPage, (snap) => snap.multiplayer.matchStarted, "guest receives ziffle public-open match");

    await hostPage.evaluate(() => window.__peerHarness.submitMultiplayerCommand({
      type: "priority_action",
      action_ref: {
        kind: "test_priority_action",
        actor: 0,
        sequence: 0,
      },
    }, "host action before guest public open"));

    await waitForSnapshot(
      guestPage,
      (snap) => snap.multiplayer.lastAppliedSequence === 1
        && Number(snap.visibleState?.decision?.player) === Number(snap.multiplayer.localPlayerIndex),
      "guest owns the next action before ziffle public open",
    );

    const revealStartedAt = Date.now();
    await guestPage.evaluate(async () => {
      const snap = await window.__peerHarness.snapshot();
      const action = (snap.visibleState?.decision?.actions || []).find((entry) =>
        entry.action_ref?.kind === "ziffle_public_open_action"
      );
      if (!action?.action_ref) {
        throw new Error("guest has no ziffle public-open action to submit");
      }
      return window.__peerHarness.submitMultiplayerCommand({
        type: "priority_action",
        action_ref: action.action_ref,
      }, "guest ziffle public-open action");
    });

    const hostAfterOpen = await waitForSnapshot(
      hostPage,
      (snap) => snap.multiplayer.lastAppliedSequence === 2,
      "host applies guest ziffle public-open action",
    );
    const guestAfterOpen = await waitForSnapshot(
      guestPage,
      (snap) => snap.multiplayer.lastAppliedSequence === 2,
      "guest applies its ziffle public-open action",
    );
    const revealElapsedMs = Date.now() - revealStartedAt;

    const statusText = [
      ...hostAfterOpen.statusEvents,
      ...guestAfterOpen.statusEvents,
    ].map((event) => event.message).join("\n");
	    const signedOpenings = [
	      ...(hostAfterOpen.auditTranscript?.actions?.at(-1)?.audit?.openings || []),
	      ...(guestAfterOpen.auditTranscript?.actions?.at(-1)?.audit?.openings || []),
	    ];
	    assert.ok(
	      signedOpenings.some((opening) =>
	        Number(opening.owner) === 0
	        && String(opening.card || "") === "Mystical Tutor"
	        && opening.position != null
	        && opening.positionCommitment
	      ),
	      `expected signed audit opening to carry ziffle position metadata: ${JSON.stringify(signedOpenings)}`,
	    );
	    assert.doesNotMatch(
	      statusText,
	      /Waiting for cryptographic reveal material from Guest/,
	      "authenticated object-order shuffles should not need a separate reveal-token request",
	    );
    assert.doesNotMatch(
      statusText,
      /does not match slot|hidden card commitment does not match reveal|Sync failed|Timed out waiting for ziffle reveal|Missing ziffle ceremony/i,
    );
    assert.ok(
      revealElapsedMs < 9000,
      `pending action ziffle reveal should not wait for the 10s visible-state timeout; elapsed=${revealElapsedMs}ms`,
    );
    assertNoPageErrors(hostPage, guestPage);
  } finally {
    await hostPage?.close().catch(() => {});
    await guestPage?.close().catch(() => {});
    await hostContext.close().catch(() => {});
    await guestContext.close().catch(() => {});
    await browser.close().catch(() => {});
    await withTimeout(vite.close(), 10000);
    await closePeerServer(peerServer);
  }
});

test("PeerJS opened ziffle hand cards keep original slots when played", { timeout: 60000 }, async () => {
  const peerPort = await freePort();
  const peerServer = await startPeerServer(peerPort);
  const { vite, baseUrl } = await startHarnessServer(peerPort);
  const browser = await chromium.launch();
  const hostContext = await browser.newContext();
  const guestContext = await browser.newContext();
  let hostPage = null;
  let guestPage = null;

  try {
    hostPage = await openHarness(hostContext, baseUrl, "host");
    guestPage = await openHarness(guestContext, baseUrl, "guest");

    await hostPage.evaluate((deckText) => {
      window.__peerHarness.createLobby({
        name: "Host",
        desiredPlayers: 2,
        startingLife: 20,
        securityMode: "verified",
        deckText,
      });
    }, HOST_ZIFFLE_OPENED_LAND_DECK);

    const hostLobby = await waitForSnapshot(
      hostPage,
      (snap) => snap.multiplayer.mode === "lobby" && snap.multiplayer.lobbyId,
      "host creates a lobby for opened ziffle land test",
    );

    await guestPage.evaluate(({ lobbyId, deckText }) => {
      window.__peerHarness.joinLobby({
        name: "Guest",
        lobbyId,
        deckText,
      });
    }, { lobbyId: hostLobby.multiplayer.lobbyId, deckText: GUEST_DECK });

    await waitForSnapshot(
      hostPage,
      (snap) => snap.canStartHostedMatch
        && snap.multiplayer.players.length === 2
        && snap.multiplayer.players.every((player) => player.connected !== false),
      "both peers join and are ready for opened ziffle land test",
    );

    await hostPage.evaluate(() => window.__peerHarness.startHostedMatch());
    await waitForSnapshot(hostPage, (snap) => snap.multiplayer.matchStarted, "host starts opened ziffle land match");
    await waitForSnapshot(guestPage, (snap) => snap.multiplayer.matchStarted, "guest receives opened ziffle land match");

    await hostPage.evaluate(async () => {
      const snap = await window.__peerHarness.snapshot();
      const action = (snap.visibleState?.decision?.actions || []).find((entry) =>
        Number(entry.action_ref?.land_id) === 4343
      );
      if (!action?.action_ref) {
        throw new Error("host has no opened ziffle land action to submit");
      }
      return window.__peerHarness.submitMultiplayerCommand({
        type: "priority_action",
        action_ref: action.action_ref,
      }, "host opened ziffle land action");
    });

    const hostAfterLand = await waitForSnapshot(
      hostPage,
      (snap) => snap.multiplayer.lastAppliedSequence === 1,
      "host applies opened ziffle land action",
    );
    const guestAfterLand = await waitForSnapshot(
      guestPage,
      (snap) => snap.multiplayer.lastAppliedSequence === 1,
      "guest applies opened ziffle land action",
    );
    const statusText = [
      ...hostAfterLand.statusEvents,
      ...guestAfterLand.statusEvents,
      ...hostAfterLand.noticeEvents,
      ...guestAfterLand.noticeEvents,
    ].map((event) => `${event?.title || ""} ${event?.body || event?.message || ""}`).join("\n");
    assert.doesNotMatch(
      statusText,
      /Private deck opening does not match slot|hidden card commitment does not match reveal|Sync failed/i,
    );
    assertNoPageErrors(hostPage, guestPage);
  } finally {
    await hostPage?.close().catch(() => {});
    await guestPage?.close().catch(() => {});
    await hostContext.close().catch(() => {});
    await guestContext.close().catch(() => {});
    await browser.close().catch(() => {});
    await withTimeout(vite.close(), 10000);
    await closePeerServer(peerServer);
  }
});

test("PeerJS opened ziffle hand cards resolve positions when object export is gone", { timeout: 60000 }, async () => {
  const peerPort = await freePort();
  const peerServer = await startPeerServer(peerPort);
  const { vite, baseUrl } = await startHarnessServer(peerPort);
  const browser = await chromium.launch();
  const hostContext = await browser.newContext();
  const guestContext = await browser.newContext();
  let hostPage = null;
  let guestPage = null;

  try {
    hostPage = await openHarness(hostContext, baseUrl, "host");
    guestPage = await openHarness(guestContext, baseUrl, "guest");

    await hostPage.evaluate((deckText) => {
      window.__peerHarness.createLobby({
        name: "Host",
        desiredPlayers: 2,
        startingLife: 20,
        securityMode: "verified",
        deckText,
      });
    }, HOST_ZIFFLE_OPENED_LAND_DECK);

    const hostLobby = await waitForSnapshot(
      hostPage,
      (snap) => snap.multiplayer.mode === "lobby" && snap.multiplayer.lobbyId,
      "host creates a lobby for fallback ziffle land test",
    );

    await guestPage.evaluate(({ lobbyId, deckText }) => {
      window.__peerHarness.joinLobby({
        name: "Guest",
        lobbyId,
        deckText,
      });
    }, { lobbyId: hostLobby.multiplayer.lobbyId, deckText: GUEST_DECK });

    await waitForSnapshot(
      hostPage,
      (snap) => snap.canStartHostedMatch
        && snap.multiplayer.players.length === 2
        && snap.multiplayer.players.every((player) => player.connected !== false),
      "both peers join and are ready for fallback ziffle land test",
    );

    await hostPage.evaluate(() => window.__peerHarness.setIncludeOpenedLandInCheckpointHand(true));
    await hostPage.evaluate(() => window.__peerHarness.startHostedMatch());
    await waitForSnapshot(hostPage, (snap) => snap.multiplayer.matchStarted, "host starts fallback ziffle land match");
    await waitForSnapshot(guestPage, (snap) => snap.multiplayer.matchStarted, "guest receives fallback ziffle land match");
    await hostPage.evaluate(() => window.__peerHarness.setFailOpenedLandExport(true));

    await hostPage.evaluate(async () => {
      const snap = await window.__peerHarness.snapshot();
      const action = (snap.visibleState?.decision?.actions || []).find((entry) =>
        Number(entry.action_ref?.land_id) === 4343
      );
      if (!action?.action_ref) {
        throw new Error("host has no opened ziffle land action to submit");
      }
      return window.__peerHarness.submitMultiplayerCommand({
        type: "priority_action",
        action_ref: action.action_ref,
      }, "host fallback ziffle land action");
    });

    const hostAfterLand = await waitForSnapshot(
      hostPage,
      (snap) => snap.multiplayer.lastAppliedSequence === 1,
      "host applies fallback ziffle land action",
    );
    const guestAfterLand = await waitForSnapshot(
      guestPage,
      (snap) => snap.multiplayer.lastAppliedSequence === 1,
      "guest applies fallback ziffle land action",
    );
    const signedOpenings = hostAfterLand.auditTranscript?.actions?.at(-1)?.audit?.openings || [];
    assert.ok(
      signedOpenings.some((opening) =>
        Number(opening.objectId) === 4343
        && Number(opening.owner) === 0
        && Number(opening.slot) === 7
        && opening.position != null
      ),
      `expected fallback opened-land audit opening to retain its ziffle position: ${JSON.stringify(signedOpenings)}`,
    );
    const noticeText = [
      ...hostAfterLand.statusEvents,
      ...guestAfterLand.statusEvents,
      ...hostAfterLand.noticeEvents,
      ...guestAfterLand.noticeEvents,
    ].map((event) => `${event?.title || ""} ${event?.body || event?.message || ""}`).join("\n");
    assert.doesNotMatch(
      noticeText,
      /Private deck opening does not match slot|hidden card commitment does not match reveal|Sync failed/i,
    );
    assertNoPageErrors(hostPage, guestPage);
  } finally {
    await hostPage?.close().catch(() => {});
    await guestPage?.close().catch(() => {});
    await hostContext.close().catch(() => {});
    await guestContext.close().catch(() => {});
    await browser.close().catch(() => {});
    await withTimeout(vite.close(), 10000);
    await closePeerServer(peerServer);
  }
});

test("PeerJS receivers infer ziffle positions for opened hand cards when audit openings omit them", { timeout: 60000 }, async () => {
  const peerPort = await freePort();
  const peerServer = await startPeerServer(peerPort);
  const { vite, baseUrl } = await startHarnessServer(peerPort);
  const browser = await chromium.launch();
  const hostContext = await browser.newContext();
  const guestContext = await browser.newContext();
  let hostPage = null;
  let guestPage = null;

  try {
    hostPage = await openHarness(hostContext, baseUrl, "host");
    guestPage = await openHarness(guestContext, baseUrl, "guest");

    await hostPage.evaluate((deckText) => {
      window.__peerHarness.createLobby({
        name: "Host",
        desiredPlayers: 2,
        startingLife: 20,
        securityMode: "verified",
        deckText,
      });
    }, HOST_ZIFFLE_OPENED_LAND_DECK);

    const hostLobby = await waitForSnapshot(
      hostPage,
      (snap) => snap.multiplayer.mode === "lobby" && snap.multiplayer.lobbyId,
      "host creates a lobby for inferred ziffle land test",
    );

    await guestPage.evaluate(({ lobbyId, deckText }) => {
      window.__peerHarness.joinLobby({
        name: "Guest",
        lobbyId,
        deckText,
      });
    }, { lobbyId: hostLobby.multiplayer.lobbyId, deckText: GUEST_DECK });

    await waitForSnapshot(
      hostPage,
      (snap) => snap.canStartHostedMatch
        && snap.multiplayer.players.length === 2
        && snap.multiplayer.players.every((player) => player.connected !== false),
      "both peers join and are ready for inferred ziffle land test",
    );

    await hostPage.evaluate(() => window.__peerHarness.startHostedMatch());
    await waitForSnapshot(hostPage, (snap) => snap.multiplayer.matchStarted, "host starts inferred ziffle land match");
    await waitForSnapshot(guestPage, (snap) => snap.multiplayer.matchStarted, "guest receives inferred ziffle land match");
    await hostPage.evaluate(() => window.__peerHarness.setOmitOwnerOpenedLandPosition(true));

    await hostPage.evaluate(async () => {
      const snap = await window.__peerHarness.snapshot();
      const action = (snap.visibleState?.decision?.actions || []).find((entry) =>
        Number(entry.action_ref?.land_id) === 4343
      );
      if (!action?.action_ref) {
        throw new Error("host has no opened ziffle land action to submit");
      }
      return window.__peerHarness.submitMultiplayerCommand({
        type: "priority_action",
        action_ref: action.action_ref,
      }, "host opened ziffle land without position metadata");
    });

    const hostAfterLand = await waitForSnapshot(
      hostPage,
      (snap) => snap.multiplayer.lastAppliedSequence === 1,
      "host applies inferred ziffle land action",
    );
    const guestAfterLand = await waitForSnapshot(
      guestPage,
      (snap) => snap.multiplayer.lastAppliedSequence === 1,
      "guest applies inferred ziffle land action",
    );
    const noticeText = [
      ...hostAfterLand.statusEvents,
      ...guestAfterLand.statusEvents,
      ...hostAfterLand.noticeEvents,
      ...guestAfterLand.noticeEvents,
    ].map((event) => `${event?.title || ""} ${event?.body || event?.message || ""}`).join("\n");
    assert.doesNotMatch(
      noticeText,
      /hidden card commitment does not match reveal|hidden ziffle position commitment does not match reveal|Sync failed/i,
    );
    assertNoPageErrors(hostPage, guestPage);
  } finally {
    await hostPage?.close().catch(() => {});
    await guestPage?.close().catch(() => {});
    await hostContext.close().catch(() => {});
    await guestContext.close().catch(() => {});
    await browser.close().catch(() => {});
    await withTimeout(vite.close(), 10000);
    await closePeerServer(peerServer);
  }
});

test("PeerJS post-timed remote openings are revealed after dispatch", { timeout: 60000 }, async () => {
  const peerPort = await freePort();
  const peerServer = await startPeerServer(peerPort);
  const { vite, baseUrl } = await startHarnessServer(peerPort);
  const browser = await chromium.launch();
  const hostContext = await browser.newContext();
  const guestContext = await browser.newContext();
  let hostPage = null;
  let guestPage = null;

  try {
    hostPage = await openHarness(hostContext, baseUrl, "host");
    guestPage = await openHarness(guestContext, baseUrl, "guest");

    await hostPage.evaluate((deckText) => {
      window.__peerHarness.createLobby({
        name: "Host",
        desiredPlayers: 2,
        startingLife: 20,
        securityMode: "verified",
        deckText,
      });
    }, HOST_ZIFFLE_OPENED_LAND_DECK);

    const hostLobby = await waitForSnapshot(
      hostPage,
      (snap) => snap.multiplayer.mode === "lobby" && snap.multiplayer.lobbyId,
      "host creates a lobby for post-timed opening test",
    );

    await guestPage.evaluate(({ lobbyId, deckText }) => {
      window.__peerHarness.joinLobby({
        name: "Guest",
        lobbyId,
        deckText,
      });
    }, { lobbyId: hostLobby.multiplayer.lobbyId, deckText: GUEST_DECK });

    await waitForSnapshot(
      hostPage,
      (snap) => snap.canStartHostedMatch
        && snap.multiplayer.players.length === 2
        && snap.multiplayer.players.every((player) => player.connected !== false),
      "both peers join and are ready for post-timed opening test",
    );

    await hostPage.evaluate(() => window.__peerHarness.startHostedMatch());
    await waitForSnapshot(hostPage, (snap) => snap.multiplayer.matchStarted, "host starts post-timed opening match");
    await waitForSnapshot(guestPage, (snap) => snap.multiplayer.matchStarted, "guest receives post-timed opening match");

    await hostPage.evaluate(() => window.__peerHarness.submitMultiplayerCommand({
      type: "priority_action",
      action_ref: {
        kind: "test_priority_action",
        actor: 0,
        sequence: 0,
      },
    }, "host action before guest post-timed opening"));

    await waitForSnapshot(
      guestPage,
      (snap) => snap.multiplayer.lastAppliedSequence === 1
        && Number(snap.visibleState?.decision?.player) === Number(snap.multiplayer.localPlayerIndex),
      "guest owns the next action before post-timed opening",
    );

    await guestPage.evaluate(async () => {
      const snap = await window.__peerHarness.snapshot();
      const action = (snap.visibleState?.decision?.actions || []).find((entry) =>
        entry.action_ref?.kind === "post_public_open_action"
      );
      if (!action?.action_ref) {
        throw new Error("guest has no post-timed public-open action to submit");
      }
      return window.__peerHarness.submitMultiplayerCommand({
        type: "priority_action",
        action_ref: action.action_ref,
      }, "guest post-timed public-open action");
    });

    const hostAfterOpen = await waitForSnapshot(
      hostPage,
      (snap) => snap.multiplayer.lastAppliedSequence === 2,
      "host applies guest post-timed public-open action",
    );
    const guestAfterOpen = await waitForSnapshot(
      guestPage,
      (snap) => snap.multiplayer.lastAppliedSequence === 2,
      "guest applies its post-timed public-open action",
    );
    const noticeText = [
      ...hostAfterOpen.statusEvents,
      ...guestAfterOpen.statusEvents,
      ...hostAfterOpen.noticeEvents,
      ...guestAfterOpen.noticeEvents,
    ].map((event) => `${event?.title || ""} ${event?.body || event?.message || ""}`).join("\n");
    assert.doesNotMatch(
      noticeText,
      /hidden card commitment does not match reveal|Resync checkpoint hash mismatch|Sync failed/i,
    );
    assertNoPageErrors(hostPage, guestPage);
  } finally {
    await hostPage?.close().catch(() => {});
    await guestPage?.close().catch(() => {});
    await hostContext.close().catch(() => {});
    await guestContext.close().catch(() => {});
    await browser.close().catch(() => {});
    await withTimeout(vite.close(), 10000);
    await closePeerServer(peerServer);
  }
});

test("PeerJS local post-timed public openings are revealed before signing", { timeout: 60000 }, async () => {
  const peerPort = await freePort();
  const peerServer = await startPeerServer(peerPort);
  const { vite, baseUrl } = await startHarnessServer(peerPort);
  const browser = await chromium.launch();
  const hostContext = await browser.newContext();
  const guestContext = await browser.newContext();
  let hostPage = null;
  let guestPage = null;

  try {
    hostPage = await openHarness(hostContext, baseUrl, "host");
    guestPage = await openHarness(guestContext, baseUrl, "guest");

    await hostPage.evaluate((deckText) => {
      window.__peerHarness.createLobby({
        name: "Host",
        desiredPlayers: 2,
        startingLife: 20,
        securityMode: "verified",
        deckText,
      });
    }, HOST_ZIFFLE_OPENED_LAND_DECK);

    const hostLobby = await waitForSnapshot(
      hostPage,
      (snap) => snap.multiplayer.mode === "lobby" && snap.multiplayer.lobbyId,
      "host creates a lobby for local post-timed opening test",
    );

    await guestPage.evaluate(({ lobbyId, deckText }) => {
      window.__peerHarness.joinLobby({
        name: "Guest",
        lobbyId,
        deckText,
      });
    }, { lobbyId: hostLobby.multiplayer.lobbyId, deckText: GUEST_DECK });

    await waitForSnapshot(
      hostPage,
      (snap) => snap.canStartHostedMatch
        && snap.multiplayer.players.length === 2
        && snap.multiplayer.players.every((player) => player.connected !== false),
      "both peers join and are ready for local post-timed opening test",
    );

    await hostPage.evaluate(() => window.__peerHarness.startHostedMatch());
    await waitForSnapshot(hostPage, (snap) => snap.multiplayer.matchStarted, "host starts local post-timed opening match");
    await waitForSnapshot(guestPage, (snap) => snap.multiplayer.matchStarted, "guest receives local post-timed opening match");

    await hostPage.evaluate(async () => {
      const snap = await window.__peerHarness.snapshot();
      const action = (snap.visibleState?.decision?.actions || []).find((entry) =>
        entry.action_ref?.kind === "post_public_open_action"
      );
      if (!action?.action_ref) {
        throw new Error("host has no post-timed public-open action to submit");
      }
      return window.__peerHarness.submitMultiplayerCommand({
        type: "priority_action",
        action_ref: action.action_ref,
      }, "host local post-timed public-open action");
    });

    const hostAfterOpen = await waitForSnapshot(
      hostPage,
      (snap) => snap.multiplayer.lastAppliedSequence === 1,
      "host applies its local post-timed public-open action",
    );
    const guestAfterOpen = await waitForSnapshot(
      guestPage,
      (snap) => snap.multiplayer.lastAppliedSequence === 1,
      "guest applies host local post-timed public-open action",
    );

    assert.equal(
      hostAfterOpen.instrumentation.postPublicOpenRevealSlot,
      1,
      "submitting peer should apply its own post-timed public opening before signing",
    );
    assert.equal(
      guestAfterOpen.instrumentation.postPublicOpenRevealSlot,
      1,
      "receiving peer should apply the post-timed public opening from the audit",
    );
    assertNoPageErrors(hostPage, guestPage);
  } finally {
    await hostPage?.close().catch(() => {});
    await guestPage?.close().catch(() => {});
    await hostContext.close().catch(() => {});
    await guestContext.close().catch(() => {});
    await browser.close().catch(() => {});
    await withTimeout(vite.close(), 10000);
    await closePeerServer(peerServer);
  }
});

test("PeerJS post-apply public openings are requested before signing", { timeout: 60000 }, async () => {
  const peerPort = await freePort();
  const peerServer = await startPeerServer(peerPort);
  const { vite, baseUrl } = await startHarnessServer(peerPort);
  const browser = await chromium.launch();
  const hostContext = await browser.newContext();
  const guestContext = await browser.newContext();
  let hostPage = null;
  let guestPage = null;

  try {
    hostPage = await openHarness(hostContext, baseUrl, "host");
    guestPage = await openHarness(guestContext, baseUrl, "guest");

    await hostPage.evaluate((deckText) => {
      window.__peerHarness.createLobby({
        name: "Host",
        desiredPlayers: 2,
        startingLife: 20,
        securityMode: "verified",
        deckText,
      });
    }, HOST_ZIFFLE_OPENED_LAND_DECK);

    const hostLobby = await waitForSnapshot(
      hostPage,
      (snap) => snap.multiplayer.mode === "lobby" && snap.multiplayer.lobbyId,
      "host creates a lobby for post-apply public-open test",
    );

    await guestPage.evaluate(({ lobbyId, deckText }) => {
      window.__peerHarness.joinLobby({
        name: "Guest",
        lobbyId,
        deckText,
      });
    }, { lobbyId: hostLobby.multiplayer.lobbyId, deckText: GUEST_DECK });

    await waitForSnapshot(
      hostPage,
      (snap) => snap.canStartHostedMatch
        && snap.multiplayer.players.length === 2
        && snap.multiplayer.players.every((player) => player.connected !== false),
      "both peers join and are ready for post-apply public-open test",
    );

    await hostPage.evaluate(() => window.__peerHarness.startHostedMatch());
    await waitForSnapshot(hostPage, (snap) => snap.multiplayer.matchStarted, "host starts post-apply public-open match");
    await waitForSnapshot(guestPage, (snap) => snap.multiplayer.matchStarted, "guest receives post-apply public-open match");

    await hostPage.evaluate(() => window.__peerHarness.submitMultiplayerCommand({
      type: "priority_action",
      action_ref: {
        kind: "test_priority_action",
        actor: 0,
        sequence: 0,
      },
    }, "host action before guest post-apply opening"));

    await waitForSnapshot(
      guestPage,
      (snap) => snap.multiplayer.lastAppliedSequence === 1
        && Number(snap.visibleState?.decision?.player) === Number(snap.multiplayer.localPlayerIndex),
      "guest owns the next action before post-apply public opening",
    );

    await guestPage.evaluate(async () => {
      const snap = await window.__peerHarness.snapshot();
      const action = (snap.visibleState?.decision?.actions || []).find((entry) =>
        entry.action_ref?.kind === "late_public_open_action"
      );
      if (!action?.action_ref) {
        throw new Error("guest has no late public-open action to submit");
      }
      return window.__peerHarness.submitMultiplayerCommand({
        type: "priority_action",
        action_ref: action.action_ref,
      }, "guest late public-open action");
    });

    const hostAfterOpen = await waitForSnapshot(
      hostPage,
      (snap) => snap.multiplayer.lastAppliedSequence === 2,
      "host applies guest post-apply public-open action",
    );
    const guestAfterOpen = await waitForSnapshot(
      guestPage,
      (snap) => snap.multiplayer.lastAppliedSequence === 2,
      "guest applies its post-apply public-open action",
    );

    assert.equal(
      guestAfterOpen.instrumentation.latePublicOpenRevealSlot,
      1,
      "submitting peer should fetch and reveal remote post-apply public opening before signing",
    );
    assert.equal(
      hostAfterOpen.instrumentation.latePublicOpenRevealSlot,
      1,
      "receiving peer should verify the signed audit contains the post-apply public opening",
    );
    const noticeText = [
      ...hostAfterOpen.statusEvents,
      ...guestAfterOpen.statusEvents,
      ...hostAfterOpen.noticeEvents,
      ...guestAfterOpen.noticeEvents,
    ].map((event) => `${event?.title || ""} ${event?.body || event?.message || ""}`).join("\n");
    assert.doesNotMatch(
      noticeText,
      /Missing public_open audit opening|Sync failed|hidden card commitment does not match reveal/i,
    );
    assertNoPageErrors(hostPage, guestPage);
  } finally {
    await hostPage?.close().catch(() => {});
    await guestPage?.close().catch(() => {});
    await hostContext.close().catch(() => {});
    await guestContext.close().catch(() => {});
    await browser.close().catch(() => {});
    await withTimeout(vite.close(), 10000);
    await closePeerServer(peerServer);
  }
});

test("PeerJS duplicate local click is dropped when the action is stale after sync", { timeout: 60000 }, async () => {
  const peerPort = await freePort();
  const peerServer = await startPeerServer(peerPort);
  const { vite, baseUrl } = await startHarnessServer(peerPort);
  const browser = await chromium.launch();
  const hostContext = await browser.newContext();
  const guestContext = await browser.newContext();
  let hostPage = null;
  let guestPage = null;

  try {
    hostPage = await openHarness(hostContext, baseUrl, "host");
    guestPage = await openHarness(guestContext, baseUrl, "guest");

    await hostPage.evaluate((deckText) => {
      window.__peerHarness.createLobby({
        name: "Host",
        desiredPlayers: 2,
        startingLife: 20,
        securityMode: "verified",
        deckText,
      });
    }, HOST_DECK);

    const hostLobby = await waitForSnapshot(
      hostPage,
      (snap) => snap.multiplayer.mode === "lobby" && snap.multiplayer.lobbyId,
      "host creates a lobby for duplicate local click test",
    );

    await guestPage.evaluate(({ lobbyId, deckText }) => {
      window.__peerHarness.joinLobby({
        name: "Guest",
        lobbyId,
        deckText,
      });
    }, { lobbyId: hostLobby.multiplayer.lobbyId, deckText: GUEST_DECK });

    await waitForSnapshot(
      hostPage,
      (snap) => snap.canStartHostedMatch
        && snap.multiplayer.players.length === 2
        && snap.multiplayer.players.every((player) => player.connected !== false),
      "both peers join and are ready for duplicate local click test",
    );

    await hostPage.evaluate(() => window.__peerHarness.startHostedMatch());
    await waitForSnapshot(hostPage, (snap) => snap.multiplayer.matchStarted, "host starts duplicate local click match");
    await waitForSnapshot(guestPage, (snap) => snap.multiplayer.matchStarted, "guest receives duplicate local click match");

    await hostPage.evaluate(async () => {
      const command = {
        type: "priority_action",
        action_ref: {
          kind: "test_priority_action",
          actor: 0,
          sequence: 0,
        },
      };
      const first = window.__peerHarness.submitMultiplayerCommand(command, "host first click");
      const second = window.__peerHarness.submitMultiplayerCommand(command, "host duplicate click");
      await Promise.allSettled([first, second]);
    });

    const hostAfterClick = await waitForSnapshot(
      hostPage,
      (snap) => snap.multiplayer.lastAppliedSequence === 1,
      "host applies only the first duplicate local click",
    );
    await sleep(1000);
    const hostAfterIdle = await snapshot(hostPage);
    const guestAfterIdle = await snapshot(guestPage);
    assert.equal(hostAfterIdle.multiplayer.lastAppliedSequence, 1);
    assert.equal(guestAfterIdle.multiplayer.lastAppliedSequence, 1);
    assert.equal(
      syncedCommandEvents(hostAfterClick).length,
      1,
      "duplicate stale local click should not append a second action",
    );
    assert.match(
      hostAfterIdle.statusEvents.map((event) => event.message).join("\n"),
      /That action is no longer available/,
    );
    assertNoPageErrors(hostPage, guestPage);
  } finally {
    await hostPage?.close().catch(() => {});
    await guestPage?.close().catch(() => {});
    await hostContext.close().catch(() => {});
    await guestContext.close().catch(() => {});
    await browser.close().catch(() => {});
    await withTimeout(vite.close(), 10000);
    await closePeerServer(peerServer);
  }
});

test("PeerJS local pass reuses checkpoint hash and skips unchanged ziffle hand scan", { timeout: 60000 }, async () => {
  const peerPort = await freePort();
  const peerServer = await startPeerServer(peerPort);
  const { vite, baseUrl } = await startHarnessServer(peerPort);
  const browser = await chromium.launch();
  const hostContext = await browser.newContext();
  const guestContext = await browser.newContext();
  let hostPage = null;
  let guestPage = null;

  try {
    hostPage = await openHarness(hostContext, baseUrl, "host");
    guestPage = await openHarness(guestContext, baseUrl, "guest");

    await hostPage.evaluate((deckText) => {
      window.__peerHarness.createLobby({
        name: "Host",
        desiredPlayers: 2,
        startingLife: 20,
        securityMode: "verified",
        deckText,
      });
    }, HOST_DECK);

    const hostLobby = await waitForSnapshot(
      hostPage,
      (snap) => snap.multiplayer.mode === "lobby" && snap.multiplayer.lobbyId,
      "host creates a lobby for checkpoint export count test",
    );

    await guestPage.evaluate(({ lobbyId, deckText }) => {
      window.__peerHarness.joinLobby({
        name: "Guest",
        lobbyId,
        deckText,
      });
    }, { lobbyId: hostLobby.multiplayer.lobbyId, deckText: GUEST_DECK });

    await waitForSnapshot(
      hostPage,
      (snap) => snap.canStartHostedMatch
        && snap.multiplayer.players.length === 2
        && snap.multiplayer.players.every((player) => player.connected !== false),
      "both peers join and are ready for checkpoint export count test",
    );

    await hostPage.evaluate(() => window.__peerHarness.startHostedMatch());
    await waitForSnapshot(hostPage, (snap) => snap.multiplayer.matchStarted, "host starts checkpoint export count match");
    await waitForSnapshot(guestPage, (snap) => snap.multiplayer.matchStarted, "guest receives checkpoint export count match");
    await hostPage.evaluate(() => window.__peerHarness.resetInstrumentation());

    await hostPage.evaluate(() => {
      window.__peerHarness.submitMultiplayerCommand({
        type: "priority_action",
        action_ref: {
          kind: "test_priority_action",
          actor: 0,
          sequence: 0,
        },
      }, "host local pass");
    });

    const hostAfterPass = await waitForSnapshot(
      hostPage,
      (snap) => snap.multiplayer.lastAppliedSequence === 1,
      "host applies local pass for checkpoint export count test",
    );

    assert.equal(
      hostAfterPass.instrumentation.exportPublicAuditCheckpoint,
      1,
      "local pass should export/hash the public audit checkpoint once",
    );
	    assert.equal(
	      hostAfterPass.instrumentation.exportSyncCheckpoint,
	      1,
	      "local pass should only export the validation snapshot checkpoint, not an extra ziffle hand reveal checkpoint",
	    );
    assertNoPageErrors(hostPage, guestPage);
  } finally {
    await hostPage?.close().catch(() => {});
    await guestPage?.close().catch(() => {});
    await hostContext.close().catch(() => {});
    await guestContext.close().catch(() => {});
    await browser.close().catch(() => {});
    await withTimeout(vite.close(), 10000);
    await closePeerServer(peerServer);
  }
});

test("PeerJS peers resync after guest reconnect and after host takeover reconnect", { timeout: 90000 }, async () => {
  const peerPort = await freePort();
  const peerServer = await startPeerServer(peerPort);
  const { vite, baseUrl } = await startHarnessServer(peerPort);
  const browser = await chromium.launch();
  const hostContext = await browser.newContext();
  const guestContext = await browser.newContext();
  let hostPage = null;
  let guestPage = null;

  try {
    hostPage = await openHarness(hostContext, baseUrl, "host");
    guestPage = await openHarness(guestContext, baseUrl, "guest");

    await hostPage.evaluate((deckText) => {
      window.__peerHarness.createLobby({
        name: "Host",
        desiredPlayers: 2,
        startingLife: 20,
        securityMode: "verified",
        deckText,
      });
    }, HOST_DECK);

    const hostLobby = await waitForSnapshot(
      hostPage,
      (snap) => snap.multiplayer.mode === "lobby" && snap.multiplayer.lobbyId,
      "host creates a lobby",
    );
    const lobbyId = hostLobby.multiplayer.lobbyId;

    await guestPage.evaluate(({ lobbyId: targetLobby, deckText }) => {
      window.__peerHarness.joinLobby({
        name: "Guest",
        lobbyId: targetLobby,
        deckText,
      });
    }, { lobbyId, deckText: GUEST_DECK });

    await waitForSnapshot(
      hostPage,
      (snap) => snap.canStartHostedMatch
        && snap.multiplayer.players.length === 2
        && snap.multiplayer.players.every((player) => player.connected !== false),
      "both peers join and are ready",
    );
    await waitForSnapshot(
      guestPage,
      (snap) => snap.multiplayer.localPlayerIndex === 1
        && snap.multiplayer.mode === "lobby",
      "guest is assigned player 2",
    );

    await hostPage.evaluate(() => window.__peerHarness.startHostedMatch());
    const hostStart = await waitForSnapshot(hostPage, (snap) => snap.multiplayer.matchStarted, "host starts match");
    const guestStart = await waitForSnapshot(guestPage, (snap) => snap.multiplayer.matchStarted, "guest receives match start");
    assert.equal(hostStart.visibleState.perspective, hostStart.multiplayer.localPlayerIndex);
    assert.equal(guestStart.visibleState.perspective, guestStart.multiplayer.localPlayerIndex);
    assert.equal(hostStart.visibleState.decision.player, guestStart.visibleState.decision.player);
    assert.equal(hostStart.visibleState.decision.player, 0);

    await hostPage.evaluate(() => {
      window.__peerHarness.submitMultiplayerCommand({
        type: "priority_action",
        action_ref: {
          kind: "test_priority_action",
          actor: 0,
          sequence: 0,
        },
      }, "host action");
    });

    await waitForSnapshot(hostPage, (snap) => snap.multiplayer.lastAppliedSequence === 1, "host applies action 1");
    const guestAfterAction = await waitForSnapshot(
      guestPage,
      (snap) => snap.multiplayer.lastAppliedSequence === 1
        && syncedCommandEvents(snap).length === 1,
      "guest replays normal apply_action through local engine",
    );
    assert.equal(
      checkpointImportEvents(guestAfterAction).length,
      0,
      "normal apply_action should not import a host checkpoint",
    );

    await guestPage.close();
    guestPage = null;
    await waitForSnapshot(
      hostPage,
      (snap) => snap.multiplayer.players.some((player) => Number(player.index) === 1 && player.connected === false),
      "host marks disconnected guest offline",
    );

    guestPage = await openHarness(guestContext, baseUrl, "guest-reconnect");
    await guestPage.evaluate(({ lobbyId: targetLobby, deckText }) => {
      window.__peerHarness.joinLobby({
        name: "Guest",
        lobbyId: targetLobby,
        deckText,
      });
    }, { lobbyId, deckText: GUEST_DECK });

    const guestResync = await waitForSnapshot(
      guestPage,
      (snap) => snap.multiplayer.matchStarted
        && snap.multiplayer.localPlayerIndex === 1
        && snap.multiplayer.lastAppliedSequence === 1
        && syncedCommandEvents(snap).length >= 1
        && snap.statusEvents.some((event) => event.message.includes("Resynced with host at action 1")),
      "guest reconnect receives state_resync",
    );
    assert.equal(guestResync.visibleState.snapshot_id, 1);
    assert.equal(guestResync.visibleState.perspective, 1);
    assert.equal(guestResync.visibleState.players[0].battlefield.length, 1);
    assert.equal(
      checkpointImportEvents(guestResync).length,
      0,
      "resync should replay signed actions instead of importing a host checkpoint",
    );

    await hostPage.close();
    hostPage = null;
    const promotedGuest = await waitForSnapshot(
      guestPage,
      (snap) => snap.multiplayer.role === "host"
        && snap.multiplayer.hostPeerId === lobbyId
        && snap.multiplayer.localPeerId === lobbyId,
      "guest takes over as host after original host disconnects",
      30000,
    );
    assert.equal(promotedGuest.multiplayer.localPlayerIndex, 1);

    await sleep(2500);
    hostPage = await openHarness(hostContext, baseUrl, "host-reconnect");
    await hostPage.evaluate(({ lobbyId: targetLobby, deckText }) => {
      window.__peerHarness.joinLobby({
        name: "Host",
        lobbyId: targetLobby,
        deckText,
      });
    }, { lobbyId, deckText: HOST_DECK });

    const hostResync = await waitForSnapshot(
      hostPage,
      (snap) => snap.multiplayer.matchStarted
        && snap.multiplayer.role === "client"
        && snap.multiplayer.localPlayerIndex === 0
        && snap.multiplayer.lastAppliedSequence === 1
        && syncedCommandEvents(snap).length >= 1
        && snap.statusEvents.some((event) => event.message.includes("Resynced with host at action 1")),
      "original host reconnects to promoted host and receives state_resync",
      30000,
    );
    assert.equal(hostResync.visibleState.snapshot_id, 1);
    assert.equal(hostResync.visibleState.perspective, 0);
    assert.equal(hostResync.visibleState.players[0].battlefield.length, 1);
    assert.equal(
      checkpointImportEvents(hostResync).length,
      0,
      "host takeover resync should replay signed actions instead of importing a host checkpoint",
    );

    await waitForSnapshot(
      guestPage,
      (snap) => snap.multiplayer.players.some((player) => Number(player.index) === 0 && player.connected !== false),
      "promoted host marks original host reconnected",
    );

    await guestPage.evaluate(async () => {
      const snap = await window.__peerHarness.snapshot();
      const action = snap.visibleState?.decision?.actions?.[0];
      if (!action?.action_ref) {
        throw new Error("promoted guest has no action after original host reconnect");
      }
      return window.__peerHarness.submitMultiplayerCommand({
        type: "priority_action",
        action_ref: action.action_ref,
      }, "promoted guest action after host reconnect");
    });
    let afterPromotedGuestAction = null;
    try {
      afterPromotedGuestAction = await Promise.all([
        waitForSnapshot(
          hostPage,
          (snap) => snap.multiplayer.lastAppliedSequence === 2
            && syncedCommandEvents(snap).length >= 2,
          "original host accepts promoted host action with elapsed clock",
          30000,
        ),
        waitForSnapshot(
          guestPage,
          (snap) => snap.multiplayer.lastAppliedSequence === 2
            && syncedCommandEvents(snap).length >= 2,
          "promoted host applies its delayed action",
          30000,
        ),
      ]);
    } catch (err) {
      const [hostDebug, guestDebug] = await Promise.all([
        snapshot(hostPage).catch((snapshotErr) => ({ error: String(snapshotErr?.message || snapshotErr) })),
        snapshot(guestPage).catch((snapshotErr) => ({ error: String(snapshotErr?.message || snapshotErr) })),
      ]);
      assert.fail(
        `${err?.message || err}\n`
        + `host after promoted action: ${JSON.stringify(compactSnapshotForFailure(hostDebug), null, 2)}\n`
        + `guest after promoted action: ${JSON.stringify(compactSnapshotForFailure(guestDebug), null, 2)}`
      );
    }
    assert.equal(afterPromotedGuestAction[0].visibleState.snapshot_id, 2);
    assert.equal(afterPromotedGuestAction[0].visibleState.players[0].battlefield.length, 2);

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

test("PeerJS Trusted peers import host checkpoints after guest reconnect and host takeover", { timeout: 90000 }, async () => {
  const peerPort = await freePort();
  const peerServer = await startPeerServer(peerPort);
  const { vite, baseUrl } = await startHarnessServer(peerPort);
  const browser = await chromium.launch();
  const hostContext = await browser.newContext();
  const guestContext = await browser.newContext();
  let hostPage = null;
  let guestPage = null;

  try {
    hostPage = await openHarness(hostContext, baseUrl, "trusted-host");
    guestPage = await openHarness(guestContext, baseUrl, "trusted-guest");

    await hostPage.evaluate((deckText) => {
      window.__peerHarness.createLobby({
        name: "Host",
        desiredPlayers: 2,
        startingLife: 20,
        securityMode: "trusted",
        deckText,
      });
    }, HOST_DECK);

    const hostLobby = await waitForSnapshot(
      hostPage,
      (snap) => snap.multiplayer.mode === "lobby"
        && snap.multiplayer.lobbyId
        && snap.multiplayer.securityMode === "trusted",
      "trusted host creates a lobby",
    );
    const lobbyId = hostLobby.multiplayer.lobbyId;

    await guestPage.evaluate(({ lobbyId: targetLobby, deckText }) => {
      window.__peerHarness.joinLobby({
        name: "Guest",
        lobbyId: targetLobby,
        deckText,
      });
    }, { lobbyId, deckText: GUEST_DECK });

    await waitForSnapshot(
      hostPage,
      (snap) => snap.canStartHostedMatch
        && snap.multiplayer.players.length === 2
        && snap.multiplayer.players.every((player) => player.connected !== false),
      "trusted peers join and are ready",
    );
    await waitForSnapshot(
      guestPage,
      (snap) => snap.multiplayer.localPlayerIndex === 1
        && snap.multiplayer.mode === "lobby"
        && snap.multiplayer.securityMode === "trusted",
      "trusted guest is assigned player 2",
    );

    await hostPage.evaluate(() => window.__peerHarness.startHostedMatch());
    const hostStart = await waitForSnapshot(
      hostPage,
      (snap) => snap.multiplayer.matchStarted
        && snap.multiplayer.securityMode === "trusted",
      "trusted host starts match",
    );
    const guestStart = await waitForSnapshot(
      guestPage,
      (snap) => snap.multiplayer.matchStarted
        && snap.multiplayer.securityMode === "trusted",
      "trusted guest receives match start",
    );
    assert.equal(hostStart.visibleState.perspective, hostStart.multiplayer.localPlayerIndex);
    assert.equal(guestStart.visibleState.perspective, guestStart.multiplayer.localPlayerIndex);
    assert.equal(hostStart.visibleState.decision.player, guestStart.visibleState.decision.player);
    assert.equal(hostStart.visibleState.decision.player, 0);

    await hostPage.evaluate(() => {
      window.__peerHarness.submitMultiplayerCommand({
        type: "priority_action",
        action_ref: {
          kind: "test_priority_action",
          actor: 0,
          sequence: 0,
        },
      }, "trusted host action");
    });

    await waitForSnapshot(
      hostPage,
      (snap) => snap.multiplayer.lastAppliedSequence === 1,
      "trusted host applies action 1",
    );
    const guestAfterAction = await waitForSnapshot(
      guestPage,
      (snap) => snap.multiplayer.lastAppliedSequence === 1
        && syncedCommandEvents(snap).length === 1,
      "trusted guest applies normal apply_action locally",
    );
    assert.equal(
      checkpointImportEvents(guestAfterAction).length,
      0,
      "normal trusted apply_action should not import a host checkpoint",
    );

    await guestPage.close();
    guestPage = null;
    await waitForSnapshot(
      hostPage,
      (snap) => snap.multiplayer.players.some((player) => Number(player.index) === 1 && player.connected === false),
      "trusted host marks disconnected guest offline",
    );

    guestPage = await openHarness(guestContext, baseUrl, "trusted-guest-reconnect");
    await guestPage.evaluate(({ lobbyId: targetLobby, deckText }) => {
      window.__peerHarness.joinLobby({
        name: "Guest",
        lobbyId: targetLobby,
        deckText,
      });
    }, { lobbyId, deckText: GUEST_DECK });

    const guestResync = await waitForSnapshot(
      guestPage,
      (snap) => snap.multiplayer.matchStarted
        && snap.multiplayer.securityMode === "trusted"
        && snap.multiplayer.localPlayerIndex === 1
        && snap.multiplayer.lastAppliedSequence === 1
        && checkpointImportEvents(snap).length >= 1
        && snap.statusEvents.some((event) => event.message.includes("Resynced with trusted host at action 1")),
      "trusted guest reconnect imports host checkpoint",
    );
    assert.equal(guestResync.visibleState.snapshot_id, 1);
    assert.equal(guestResync.visibleState.perspective, 1);
    assert.equal(guestResync.visibleState.players[0].battlefield.length, 1);
    assert.ok(
      checkpointImportEvents(guestResync).length >= 1,
      "trusted reconnect should import the host checkpoint instead of replaying signed actions",
    );
    const hostAfterGuestResync = await snapshot(hostPage);
    assert.equal(
      guestResync.multiplayer.matchClock?.clockHash,
      hostAfterGuestResync.multiplayer.matchClock?.clockHash,
      "trusted checkpoint import should adopt the host clock hash head",
    );
    assert.equal(
      guestResync.multiplayer.matchClock?.lastSequence,
      hostAfterGuestResync.multiplayer.matchClock?.lastSequence,
      "trusted checkpoint import should adopt the host clock sequence",
    );

    await hostPage.close();
    hostPage = null;
    const promotedGuest = await waitForSnapshot(
      guestPage,
      (snap) => snap.multiplayer.role === "host"
        && snap.multiplayer.securityMode === "trusted"
        && snap.multiplayer.hostPeerId === lobbyId
        && snap.multiplayer.localPeerId === lobbyId,
      "trusted guest takes over as host after original host disconnects",
      30000,
    );
    assert.equal(promotedGuest.multiplayer.localPlayerIndex, 1);

    await sleep(2500);
    hostPage = await openHarness(hostContext, baseUrl, "trusted-host-reconnect");
    await hostPage.evaluate(({ lobbyId: targetLobby, deckText }) => {
      window.__peerHarness.joinLobby({
        name: "Host",
        lobbyId: targetLobby,
        deckText,
      });
    }, { lobbyId, deckText: HOST_DECK });

    const hostResync = await waitForSnapshot(
      hostPage,
      (snap) => snap.multiplayer.matchStarted
        && snap.multiplayer.securityMode === "trusted"
        && snap.multiplayer.role === "client"
        && snap.multiplayer.localPlayerIndex === 0
        && snap.multiplayer.lastAppliedSequence === 1
        && checkpointImportEvents(snap).length >= 1
        && snap.statusEvents.some((event) => event.message.includes("Resynced with trusted host at action 1")),
      "original host reconnects to trusted promoted host and imports checkpoint",
      30000,
    );
    assert.equal(hostResync.visibleState.snapshot_id, 1);
    assert.equal(hostResync.visibleState.perspective, 0);
    assert.equal(hostResync.visibleState.players[0].battlefield.length, 1);
    assert.ok(
      checkpointImportEvents(hostResync).length >= 1,
      "trusted host takeover resync should import the promoted host checkpoint",
    );
    const promotedGuestAfterHostResync = await snapshot(guestPage);
    assert.equal(
      hostResync.multiplayer.matchClock?.clockHash,
      promotedGuestAfterHostResync.multiplayer.matchClock?.clockHash,
      "trusted host-takeover resync should adopt the promoted host clock hash head",
    );
    assert.equal(
      hostResync.multiplayer.matchClock?.lastSequence,
      promotedGuestAfterHostResync.multiplayer.matchClock?.lastSequence,
      "trusted host-takeover resync should adopt the promoted host clock sequence",
    );

    await waitForSnapshot(
      guestPage,
      (snap) => snap.multiplayer.players.some((player) => Number(player.index) === 0 && player.connected !== false),
      "trusted promoted host marks original host reconnected",
    );

    await guestPage.evaluate(async () => {
      const snap = await window.__peerHarness.snapshot();
      const action = snap.visibleState?.decision?.actions?.[0];
      if (!action?.action_ref) {
        throw new Error("trusted promoted guest has no action after original host reconnect");
      }
      return window.__peerHarness.submitMultiplayerCommand({
        type: "priority_action",
        action_ref: action.action_ref,
      }, "trusted promoted guest action after host reconnect");
    });

    const afterPromotedGuestAction = await Promise.all([
      waitForSnapshot(
        hostPage,
        (snap) => snap.multiplayer.lastAppliedSequence === 2
          && snap.visibleState?.snapshot_id === 2,
        "trusted original host accepts promoted host action",
        30000,
      ),
      waitForSnapshot(
        guestPage,
        (snap) => snap.multiplayer.lastAppliedSequence === 2
          && snap.visibleState?.snapshot_id === 2,
        "trusted promoted host applies its action",
        30000,
      ),
    ]);
    assert.equal(afterPromotedGuestAction[0].visibleState.players[0].battlefield.length, 2);

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

test("PeerJS four browser peers join, start, relay actions, and flag a silent add-card cheat", { timeout: 120000 }, async () => {
  const peerPort = await freePort();
  const peerServer = await startPeerServer(peerPort);
  const { vite, baseUrl } = await startHarnessServer(peerPort);
  const browser = await chromium.launch();
  const contexts = [];
  const pages = [];

  try {
    for (let index = 0; index < 4; index += 1) {
      const context = await browser.newContext();
      const page = await openHarness(context, baseUrl, `p${index + 1}`);
      contexts.push(context);
      pages.push(page);
    }

    await pages[0].evaluate((deckText) => {
      window.__peerHarness.createLobby({
        name: "Player 1",
        desiredPlayers: 4,
        startingLife: 20,
        securityMode: "verified",
        deckText,
      });
    }, FOUR_PLAYER_DECKS[0]);

    const hostLobby = await waitForSnapshot(
      pages[0],
      (snap) => snap.multiplayer.mode === "lobby" && snap.multiplayer.lobbyId,
      "four-player host creates a lobby",
    );
    const lobbyId = hostLobby.multiplayer.lobbyId;

    await Promise.all([1, 2, 3].map((seat) =>
      pages[seat].evaluate(({ lobbyId: targetLobby, deckText, name }) => {
        window.__peerHarness.joinLobby({
          name,
          lobbyId: targetLobby,
          deckText,
        });
      }, {
        lobbyId,
        deckText: FOUR_PLAYER_DECKS[seat],
        name: `Player ${seat + 1}`,
      })
    ));

    await waitForSnapshot(
      pages[0],
      (snap) => snap.canStartHostedMatch
        && snap.multiplayer.players.length === 4
        && snap.multiplayer.players.every((player) => player.connected !== false),
      "all four peers join and are ready",
      30000,
    );
    const guestLobbySnapshots = await Promise.all([1, 2, 3].map((seat) =>
      waitForSnapshot(
        pages[seat],
        (snap) => Number.isInteger(snap.multiplayer.localPlayerIndex)
          && snap.multiplayer.mode === "lobby",
        `player ${seat + 1} is assigned a seat`,
        30000,
      )
    ));
    assert.deepEqual(
      guestLobbySnapshots
        .map((snap) => Number(snap.multiplayer.localPlayerIndex))
        .sort((left, right) => left - right),
      [1, 2, 3],
    );

    await pages[0].evaluate(() => window.__peerHarness.startHostedMatch());
    await Promise.all(pages.map((page, index) =>
      waitForSnapshot(page, (snap) => snap.multiplayer.matchStarted, `player ${index + 1} receives match start`, 30000)
    ));

    await pages[0].evaluate(() => {
      window.__peerHarness.submitMultiplayerCommand({
        type: "priority_action",
        action_ref: {
          kind: "test_priority_action",
          actor: 0,
          sequence: 0,
        },
      }, "four-player host action");
    });

    const afterAction = await Promise.all(pages.map((page, index) =>
      waitForSnapshot(
        page,
        (snap) => snap.multiplayer.lastAppliedSequence === 1,
        `player ${index + 1} receives the signed action`,
        30000,
      )
    ));
    assert.deepEqual(
      afterAction.map((snap) => snap.multiplayer.lastAppliedSequence),
      [1, 1, 1, 1],
    );

    const cheatingPageIndex = afterAction.findIndex(
      (snap) => Number(snap.multiplayer.localPlayerIndex) === 1
    );
    assert.notEqual(cheatingPageIndex, -1, "expected one browser to own player 2");
    await pages[cheatingPageIndex].evaluate(async () => {
      const playerIndex = (await window.__peerHarness.snapshot()).multiplayer.localPlayerIndex;
      const state = await window.__peerHarness.silentlyAddCard({
        playerIndex,
        cardName: "Black Lotus",
        zone: "hand",
      });
      const forgedAction = state.decision.actions.find((action) =>
        action.kind === "cast_spell"
        && action.action_ref?.kind === "cast_spell"
        && action.action_ref?.from_zone === "hand"
      );
      if (!forgedAction) {
        throw new Error("silent add-card cheat did not create a cast action");
      }
      await window.__peerHarness.submitMultiplayerCommand({
        type: "priority_action",
        action_ref: forgedAction.action_ref,
      }, "silent add-card cheat");
    });
    const cheatDetected = await waitForSnapshot(
      pages[0],
      (snap) => snap.statusEvents.some((event) =>
        event.isError && /Cheat detected from Player/.test(event.message)
      ),
      "host detects silent local add-card cheat when the forged card is played",
      30000,
    );
    assert.equal(cheatDetected.multiplayer.lastAppliedSequence, 1);

    assertNoPageErrors(...pages);
  } finally {
    await withTimeout(Promise.allSettled([
      ...contexts.map((context) => context.close()),
      browser.close(),
    ]), 10000);
    await withTimeout(vite.close(), 10000);
    await closePeerServer(peerServer);
  }
});
