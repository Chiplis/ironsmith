import test from "node:test";
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import net from "node:net";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { createServer as createViteServer } from "vite";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, "..");
const HARNESS_PATH = "/tests/fixtures/peer-lobby-harness.html";
const HOST_DECK = "60 Island";
const GUEST_DECK = "60 Mountain";
const FOUR_PLAYER_DECKS = [
  "60 Island",
  "60 Mountain",
  "60 Forest",
  "60 Plains",
];

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function withTimeout(promise, ms) {
  let timer = null;
  try {
    return await Promise.race([
      promise,
      new Promise((resolve) => {
        timer = setTimeout(resolve, ms);
      }),
    ]);
  } finally {
    if (timer) clearTimeout(timer);
  }
}

async function freePort() {
  return new Promise((resolve, reject) => {
    const server = net.createServer();
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      const port = typeof address === "object" && address ? address.port : 0;
      server.close(() => resolve(port));
    });
  });
}

async function startPeerServer(port) {
  const child = spawn(
    process.execPath,
    [
      "--input-type=module",
      "-e",
      `
        import { PeerServer } from "peer";

        let httpServer = null;
        PeerServer({
          host: "127.0.0.1",
          port: Number(process.env.PEER_PORT),
          path: "/peerjs",
          key: "peerjs",
          allow_discovery: false,
        }, (server) => {
          httpServer = server;
          console.log("PEER_READY");
        }).once("error", (error) => {
          console.error(error?.stack || error);
          process.exit(1);
        });

        function shutdown() {
          if (!httpServer) {
            process.exit(0);
            return;
          }
          httpServer.closeAllConnections?.();
          httpServer.close(() => process.exit(0));
          setTimeout(() => process.exit(0), 1000).unref();
        }

        process.on("SIGTERM", shutdown);
        process.on("SIGINT", shutdown);
      `,
    ],
    {
      cwd: UI_ROOT,
      env: {
        ...process.env,
        PEER_PORT: String(port),
      },
      stdio: ["ignore", "pipe", "pipe"],
    }
  );
  let stderr = "";
  child.stderr.on("data", (chunk) => {
    stderr += String(chunk);
  });
  await new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      reject(new Error(`PeerJS server did not start\n${stderr}`));
    }, 10000);
    child.stdout.on("data", (chunk) => {
      if (!String(chunk).includes("PEER_READY")) return;
      clearTimeout(timer);
      resolve();
    });
    child.once("exit", (code) => {
      clearTimeout(timer);
      reject(new Error(`PeerJS server exited with code ${code}\n${stderr}`));
    });
  });
  return child;
}

async function closePeerServer(child) {
  if (!child || child.exitCode !== null) return;
  child.kill("SIGTERM");
  const exited = await Promise.race([
    new Promise((resolve) => child.once("exit", () => resolve(true))),
    sleep(3000).then(() => false),
  ]);
  if (exited) return;
  child.kill("SIGKILL");
  await Promise.race([
    new Promise((resolve) => child.once("exit", resolve)),
    sleep(3000),
  ]);
}

async function startHarnessServer(peerPort) {
  const vitePort = await freePort();
  process.env.VITE_PEER_HOST = "127.0.0.1";
  process.env.VITE_PEER_PORT = String(peerPort);
  process.env.VITE_PEER_PATH = "/peerjs";
  process.env.VITE_PEER_KEY = "peerjs";
  process.env.VITE_PEER_SECURE = "false";
  process.env.VITE_PEER_HEARTBEAT_INTERVAL_MS = "500";
  process.env.VITE_PEER_HEARTBEAT_TIMEOUT_MS = "2000";

  const vite = await createViteServer({
    root: UI_ROOT,
    configFile: path.join(UI_ROOT, "vite.config.js"),
    clearScreen: false,
    logLevel: "silent",
    server: {
      host: "127.0.0.1",
      port: vitePort,
      strictPort: true,
      hmr: false,
      watch: null,
    },
  });
  await vite.listen();
  return {
    vite,
    baseUrl: `http://127.0.0.1:${vitePort}`,
  };
}

async function openHarness(context, baseUrl, label) {
  const page = await context.newPage();
  const pageErrors = [];
  page.on("pageerror", (error) => pageErrors.push(String(error?.stack || error)));
  page.on("console", (message) => {
    if (message.type() === "error") {
      pageErrors.push(message.text());
    }
  });
  await page.goto(`${baseUrl}${HARNESS_PATH}`);
  await page.waitForFunction(() => window.__peerHarness?.ready === true);
  page.__peerHarnessErrors = pageErrors;
  page.__peerHarnessLabel = label;
  return page;
}

async function snapshot(page) {
  return page.evaluate(() => window.__peerHarness.snapshot());
}

async function waitForSnapshot(page, predicate, label, timeoutMs = 20000) {
  const started = Date.now();
  let lastSnapshot = null;
  while (Date.now() - started < timeoutMs) {
    lastSnapshot = await snapshot(page);
    if (predicate(lastSnapshot)) return lastSnapshot;
    await sleep(100);
  }
  assert.fail(`${label}\nLast snapshot: ${JSON.stringify(lastSnapshot, null, 2)}`);
}

function checkpointImportEvents(snap) {
  return (snap.syncEvents || []).filter((event) => event.type === "sync_checkpoint_import");
}

function syncedCommandEvents(snap) {
  return (snap.syncEvents || []).filter((event) => event.type === "synced_command");
}

function assertNoPageErrors(...pages) {
  for (const page of pages) {
    const errors = page?.__peerHarnessErrors || [];
    assert.deepEqual(errors, [], `${page?.__peerHarnessLabel || "page"} had browser errors`);
  }
}

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
    await waitForSnapshot(hostPage, (snap) => snap.multiplayer.matchStarted, "host starts match");
    await waitForSnapshot(guestPage, (snap) => snap.multiplayer.matchStarted, "guest receives match start");

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
        && checkpointImportEvents(snap).length >= 1
        && snap.statusEvents.some((event) => event.message.includes("Resynced with host at action 1")),
      "guest reconnect receives state_resync",
    );
    assert.equal(guestResync.visibleState.snapshot_id, 1);
    assert.equal(guestResync.visibleState.perspective, 1);
    assert.equal(guestResync.visibleState.players[0].battlefield.length, 1);
    assert.equal(
      syncedCommandEvents(guestResync).length,
      0,
      "resync should restore a host checkpoint instead of replaying actions",
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
        && checkpointImportEvents(snap).length >= 1
        && snap.statusEvents.some((event) => event.message.includes("Resynced with host at action 1")),
      "original host reconnects to promoted host and receives state_resync",
      30000,
    );
    assert.equal(hostResync.visibleState.snapshot_id, 1);
    assert.equal(hostResync.visibleState.perspective, 0);
    assert.equal(hostResync.visibleState.players[0].battlefield.length, 1);
    assert.equal(
      syncedCommandEvents(hostResync).length,
      0,
      "host takeover resync should restore the promoted host checkpoint",
    );

    await waitForSnapshot(
      guestPage,
      (snap) => snap.multiplayer.players.some((player) => Number(player.index) === 0 && player.connected !== false),
      "promoted host marks original host reconnected",
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

    await pages[1].evaluate(async () => {
      const playerIndex = window.__peerHarness.snapshot().multiplayer.localPlayerIndex;
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
      await window.__peerHarness.submitMultiplayerCommand({
        type: "priority_action",
        action_ref: forgedAction.action_ref,
      }, "silent add-card cheat");
    });
    const cheatDetected = await waitForSnapshot(
      pages[0],
      (snap) => snap.statusEvents.some((event) =>
        event.isError && /Cheat detected from Player 2/.test(event.message)
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
