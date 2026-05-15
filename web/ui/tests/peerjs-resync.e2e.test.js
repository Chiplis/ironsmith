import test from "node:test";
import assert from "node:assert/strict";
import { Buffer } from "node:buffer";
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
const HOST_ZIFFLE_PUBLIC_OPEN_DECK = "7 Island\n1 Mystical Tutor\n52 Mountain";
const HOST_ZIFFLE_OPENED_LAND_DECK = "7 Mountain\n1 Island\n5 Mountain\n1 Mystical Tutor\n46 Mountain";
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
  const pageConsole = [];
  page.on("pageerror", (error) => pageErrors.push(String(error?.stack || error)));
  page.on("console", async (message) => {
    let argsText = "";
    try {
      const args = await Promise.all(message.args().map((arg) => arg.jsonValue().catch(() => null)));
      argsText = ` ${JSON.stringify(args)}`;
    } catch {
      argsText = "";
    }
    pageConsole.push(`${message.type()}: ${message.text()}${argsText}`);
    if (pageConsole.length > 200) pageConsole.shift();
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

function deckUrlParam(deckText) {
  return Buffer.from(String(deckText || ""), "utf8").toString("base64url");
}

async function openFullUiPage(context, url, label) {
  const page = await context.newPage();
  const pageErrors = [];
  const pageConsole = [];
  page.on("pageerror", (error) => pageErrors.push(String(error?.stack || error)));
  page.on("console", async (message) => {
    let argsText = "";
    try {
      const args = await Promise.all(message.args().map((arg) => arg.jsonValue().catch(() => null)));
      argsText = ` ${JSON.stringify(args)}`;
    } catch {
      argsText = "";
    }
    pageConsole.push(`${message.type()}: ${message.text()}${argsText}`);
    if (pageConsole.length > 200) pageConsole.shift();
    if (message.type() === "error") {
      pageErrors.push(message.text());
    }
  });
  await page.route("https://api.scryfall.com/**", (route) =>
    route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        object: "card",
        name: "Test Card",
        image_uris: {},
        card_faces: [],
      }),
    })
  );
  await page.goto(url);
  page.__peerHarnessErrors = pageErrors;
  page.__peerHarnessConsole = pageConsole;
  page.__peerHarnessLabel = label;
  return page;
}

async function clickLocalButton(page, label, textPattern = null) {
  let button = page.locator('button[data-local-action="true"]:enabled:not([aria-disabled="true"])');
  if (textPattern) {
    button = button.filter({ hasText: textPattern });
  }
  button = button.first();
  if ((await button.count()) === 0) return null;
  try {
    const text = (await button.innerText({ timeout: 1000 })).replace(/\s+/g, " ").trim();
    await button.press("Enter", { timeout: 3000 });
    return { label, text };
  } catch (err) {
    const message = String(err?.message || err || "");
    if (!message.includes("Timeout") && !message.includes("detached")) {
      throw err;
    }
    return null;
  }
}

async function activateLocalButton(page, label, textPattern) {
  const button = page.locator('button[data-local-action="true"]:enabled:not([aria-disabled="true"])').filter({ hasText: textPattern }).first();
  if ((await button.count()) === 0) return null;
  const text = (await button.innerText({ timeout: 1000 })).replace(/\s+/g, " ").trim();
  await activateButtonNode(button);
  return { label, text };
}

async function activateButtonNode(button) {
  await button.evaluate((node) => {
    node.dispatchEvent(new PointerEvent("pointerdown", {
      bubbles: true,
      cancelable: true,
      button: 0,
      buttons: 1,
      pointerId: 1,
      pointerType: "mouse",
    }));
    node.dispatchEvent(new PointerEvent("pointerup", {
      bubbles: true,
      cancelable: true,
      button: 0,
      buttons: 0,
      pointerId: 1,
      pointerType: "mouse",
    }));
    node.dispatchEvent(new MouseEvent("click", {
      bubbles: true,
      cancelable: true,
      button: 0,
      detail: 1,
    }));
  });
}

async function clickEnabledButton(page, label, textPattern) {
  const button = page.locator("button:enabled").filter({ hasText: textPattern }).first();
  if ((await button.count()) === 0) return null;
  try {
    const text = (await button.innerText({ timeout: 1000 })).replace(/\s+/g, " ").trim();
    await activateButtonNode(button);
    return { label, text };
  } catch (err) {
    const message = String(err?.message || err || "");
    if (!message.includes("Timeout") && !message.includes("detached")) {
      throw err;
    }
    return null;
  }
}

async function clickLastEnabledButton(page, label, textPattern) {
  const button = page.locator("button:enabled").filter({ hasText: textPattern }).last();
  if ((await button.count()) === 0) return null;
  try {
    const text = (await button.innerText({ timeout: 1000 })).replace(/\s+/g, " ").trim();
    await activateButtonNode(button);
    return { label, text };
  } catch (err) {
    const message = String(err?.message || err || "");
    if (!message.includes("Timeout") && !message.includes("detached")) {
      throw err;
    }
    return null;
  }
}

async function clickLocalDecisionButton(page, label) {
  return clickLocalButton(page, label);
}

async function visibleBodyText(page) {
  return page.locator("body").innerText();
}

async function buttonDebugText(page) {
  return page.locator("button").evaluateAll((buttons) =>
    buttons.map((button) => ({
      text: (button.innerText || button.textContent || "").replace(/\s+/g, " ").trim(),
      disabled: button.disabled,
      ariaDisabled: button.getAttribute("aria-disabled"),
      localAction: button.getAttribute("data-local-action"),
    }))
  );
}

async function waitForLocalButton(page, pattern, label, timeoutMs = 60000) {
  const started = Date.now();
  let buttons = [];
  while (Date.now() - started < timeoutMs) {
    buttons = await buttonDebugText(page);
    if (buttons.some((button) =>
      button.localAction === "true"
      && !button.disabled
      && button.ariaDisabled !== "true"
      && pattern.test(button.text)
    )) {
      return buttons;
    }
    await sleep(250);
  }
  assert.fail(`${label}\nbuttons: ${JSON.stringify(buttons, null, 2)}\nbody:\n${await visibleBodyText(page)}`);
}

async function waitForVisibleBodyText(page, pattern, label, timeoutMs = 60000) {
  const started = Date.now();
  let text = "";
  while (Date.now() - started < timeoutMs) {
    text = await visibleBodyText(page);
    if (pattern.test(text)) return text;
    await sleep(500);
  }
  assert.fail(`${label}\nLast body text:\n${text}`);
}

function assertNoSyncFailureText(text, label = "unexpected sync failure") {
  assert.doesNotMatch(
    text,
    /Unknown Ziffle Ceremony|Unknown ziffle ceremony|Private deck opening does not match slot|hidden card commitment does not match reveal|Sync failed|Match start failed|Auto-pass failed|Resync checkpoint hash mismatch/i,
    label
  );
}

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

    await guestPage.evaluate(() => {
      const snap = window.__peerHarness.snapshot();
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

    await guestPage.evaluate(() => {
      const snap = window.__peerHarness.snapshot();
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
    await guestPage.evaluate(() => {
      const snap = window.__peerHarness.snapshot();
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
    assert.match(
      statusText,
      /Waiting for cryptographic reveal material from Guest/,
      "host should request the guest's reveal token for the ziffle position",
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

    await hostPage.evaluate(() => {
      const snap = window.__peerHarness.snapshot();
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

test("PeerJS opened ziffle hand cards use cached positions when object export is gone", { timeout: 60000 }, async () => {
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
    }, HOST_ZIFFLE_OPENED_LAND_DECK);

    const hostLobby = await waitForSnapshot(
      hostPage,
      (snap) => snap.multiplayer.mode === "lobby" && snap.multiplayer.lobbyId,
      "host creates a lobby for cached ziffle land test",
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
      "both peers join and are ready for cached ziffle land test",
    );

    await hostPage.evaluate(() => window.__peerHarness.setIncludeOpenedLandInCheckpointHand(true));
    await hostPage.evaluate(() => window.__peerHarness.startHostedMatch());
    await waitForSnapshot(hostPage, (snap) => snap.multiplayer.matchStarted, "host starts cached ziffle land match");
    await waitForSnapshot(guestPage, (snap) => snap.multiplayer.matchStarted, "guest receives cached ziffle land match");
    await hostPage.evaluate(() => window.__peerHarness.setFailOpenedLandExport(true));

    await hostPage.evaluate(() => {
      const snap = window.__peerHarness.snapshot();
      const action = (snap.visibleState?.decision?.actions || []).find((entry) =>
        Number(entry.action_ref?.land_id) === 4343
      );
      if (!action?.action_ref) {
        throw new Error("host has no opened ziffle land action to submit");
      }
      return window.__peerHarness.submitMultiplayerCommand({
        type: "priority_action",
        action_ref: action.action_ref,
      }, "host cached ziffle land action");
    });

    const hostAfterLand = await waitForSnapshot(
      hostPage,
      (snap) => snap.multiplayer.lastAppliedSequence === 1,
      "host applies cached ziffle land action",
    );
    const guestAfterLand = await waitForSnapshot(
      guestPage,
      (snap) => snap.multiplayer.lastAppliedSequence === 1,
      "guest applies cached ziffle land action",
    );
    const signedOpenings = hostAfterLand.auditTranscript?.actions?.at(-1)?.audit?.openings || [];
    assert.ok(
      signedOpenings.some((opening) =>
        Number(opening.objectId) === 4343
        && Number(opening.owner) === 0
        && Number(opening.slot) === 7
        && opening.position != null
      ),
      `expected cached opened-land audit opening to retain its ziffle position: ${JSON.stringify(signedOpenings)}`,
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

    await hostPage.evaluate(() => {
      const snap = window.__peerHarness.snapshot();
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

    await guestPage.evaluate(() => {
      const snap = window.__peerHarness.snapshot();
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

    await hostPage.evaluate(() => {
      const snap = window.__peerHarness.snapshot();
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

    await guestPage.evaluate(() => {
      const snap = window.__peerHarness.snapshot();
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
      0,
      "local pass with unchanged hand should not export a sync checkpoint for ziffle hand reveal",
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

    const cheatingPageIndex = afterAction.findIndex(
      (snap) => Number(snap.multiplayer.localPlayerIndex) === 1
    );
    assert.notEqual(cheatingPageIndex, -1, "expected one browser to own player 2");
    await pages[cheatingPageIndex].evaluate(async () => {
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

    const hostText = await visibleBodyText(hostPage);
    const guestText = await visibleBodyText(guestPage);
    assert.match(hostText, /Basic Land - Mountain/);
    assert.match(guestText, /Basic Land - Mountain/);
    assert.ok(
      ((`${hostText}\n${guestText}`).match(/Basic Land - Mountain/g) || []).length >= 2,
      "expected both played lands to be visible"
    );
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
        `${err?.message || err}\nhost console: ${JSON.stringify((hostPage.__peerHarnessConsole || []).slice(-80), null, 2)}\nguest console: ${JSON.stringify((guestPage.__peerHarnessConsole || []).slice(-80), null, 2)}`
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
    const guestMulligan = await activateLocalButton(guestPage, "guest-mulligan", /Mulligan/i);
    assert.ok(
      guestMulligan,
      `expected Player 1 to be able to mulligan\nhost body:\n${await visibleBodyText(hostPage)}\nguest body:\n${await visibleBodyText(guestPage)}`
    );
    await sleep(5000);

    const afterGuestMulliganText = `${await visibleBodyText(hostPage)}\n${await visibleBodyText(guestPage)}`;
    try {
      assertNoSyncFailureText(afterGuestMulliganText, "guest mulligan click should not sync-fail");
    } catch (err) {
      assert.fail(
        `${err?.message || err}\nhost console: ${JSON.stringify((hostPage.__peerHarnessConsole || []).slice(-80), null, 2)}\nguest console: ${JSON.stringify((guestPage.__peerHarnessConsole || []).slice(-80), null, 2)}`
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
      assert.fail(
        `${err?.message || err}\nhost console: ${JSON.stringify((hostPage.__peerHarnessConsole || []).slice(-80), null, 2)}\nguest console: ${JSON.stringify((guestPage.__peerHarnessConsole || []).slice(-80), null, 2)}`
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
      assert.fail(
        `${err?.message || err}\nhost console: ${JSON.stringify((hostPage.__peerHarnessConsole || []).slice(-80), null, 2)}\nguest console: ${JSON.stringify((guestPage.__peerHarnessConsole || []).slice(-80), null, 2)}`
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

test("full UI PeerJS Mystical Tutor resolves into a searchable hidden library choice", { timeout: 300000 }, async () => {
  const peerPort = await freePort();
  const peerServer = await startPeerServer(peerPort);
  const { vite, baseUrl } = await startHarnessServer(peerPort);
  const browser = await chromium.launch();
  const hostContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  const guestContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  let hostPage = null;
  let guestPage = null;

  try {
    const hostDeck = deckUrlParam("30 Island\n30 Mystical Tutor");
    const guestDeck = deckUrlParam("60 Mountain");
    hostPage = await openFullUiPage(hostContext, `${baseUrl}/?name=Chiplis&deck=${hostDeck}`, "host-ui");
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
      `${baseUrl}/?lobby=${encodeURIComponent(lobbyCode)}&name=Alice&deck=${guestDeck}`,
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

    let playedIsland = false;
    let castTutor = false;
    let paidTutor = false;
    let guestResolved = false;

    for (let step = 0; step < 120; step += 1) {
      const combinedText = `${await visibleBodyText(hostPage)}\n${await visibleBodyText(guestPage)}`;
      assertNoSyncFailureText(combinedText);

      if (!playedIsland && await clickLocalButton(hostPage, "host-play-island", /PLAY ISLAND/i)) {
        playedIsland = true;
        await sleep(3000);
        continue;
      }

      if (playedIsland && !castTutor && await clickLocalButton(hostPage, "host-cast-tutor", /MYSTICAL TUTOR/i)) {
        castTutor = true;
        await sleep(3000);
        continue;
      }

      if (castTutor && !paidTutor) {
        const paid = await clickLocalButton(hostPage, "host-pay-tutor", /PAY|MANA|BLUE|ISLAND|\{U\}|U$/i)
          || await clickEnabledButton(hostPage, "host-tap-island", /TAP ISLAND|ADD/i);
        if (paid) {
          paidTutor = true;
          await sleep(6000);
          continue;
        }
      }

      if (paidTutor && !guestResolved && await clickLocalButton(guestPage, "guest-resolve-tutor", /RESOLVE|PASS/i)) {
        guestResolved = true;
        const searchText = await waitForVisibleBodyText(
          hostPage,
          /CHOOSE OBJECTS[\s\S]*Search library[\s\S]*Mystical Tutor/i,
          "expected Mystical Tutor to create a searchable library decision",
          90000,
        );
        assertNoSyncFailureText(searchText, "search decision should not sync-fail");

        await clickEnabledButton(hostPage, "host-finish-viewing-library", /^DONE$/i);
        const choseTutor = await clickLastEnabledButton(hostPage, "host-choose-mystical-tutor", /^Mystical Tutor$/i);
        assert.ok(choseTutor, "expected a Mystical Tutor choice in the searched library");
        const submittedChoice = await clickLocalButton(hostPage, "host-submit-tutor-choice", /SUBMIT|DONE/i);
        assert.ok(submittedChoice, "expected the searched card choice to be submittable");
        break;
      }

      if (!playedIsland) {
        const hostAdvanced = await clickLocalButton(hostPage, "host-setup", /KEEP HAND|PREGAME|UPKEEP|DRAW|PASS PRIORITY|RESOLVE/i);
        if (hostAdvanced) {
          await sleep(4500);
          continue;
        }
        const guestAdvanced = await clickLocalButton(guestPage, "guest-setup", /KEEP HAND|PREGAME|UPKEEP|DRAW|PASS PRIORITY|RESOLVE/i);
        if (guestAdvanced) {
          await sleep(4500);
          continue;
        }
      }

      await sleep(1000);
    }

    assert.equal(playedIsland, true, "expected host to play Island");
    assert.equal(castTutor, true, "expected host to cast Mystical Tutor");
    assert.equal(paidTutor, true, "expected host to pay for Mystical Tutor");
    assert.equal(guestResolved, true, "expected guest to resolve Mystical Tutor");

    const resolvedText = await waitForVisibleBodyText(
      hostPage,
      /GY\s*1[\s\S]*Mystical Tutor[\s\S]*Instant[\s\S]*Graveyard/i,
      "expected Mystical Tutor to finish resolving into its owner's graveyard",
      90000,
    );
    const guestResolvedText = await waitForVisibleBodyText(
      guestPage,
      /CHIPLIS[\s\S]*GY\s*1/i,
      "expected guest peer to apply the resolved Mystical Tutor",
      90000,
    );
    const finalCombinedText = `${resolvedText}\n${guestResolvedText}`;
    assertNoSyncFailureText(finalCombinedText, "Mystical Tutor full flow should stay synced");
    assert.doesNotMatch(finalCombinedText, /1 stack entry|STACK\s*1/i);

    let drewToppedCard = false;
    let lastAdvanceText = "";
    for (let step = 0; step < 180; step += 1) {
      const combinedText = `${await visibleBodyText(hostPage)}\n${await visibleBodyText(guestPage)}`;
      lastAdvanceText = combinedText;
      assertNoSyncFailureText(combinedText, "drawing the Mystical Tutor target should stay synced");
      if (/CHIPLIS[\s\S]*HAND\s*6[\s\S]*GY\s*1[\s\S]*DECK\s*52/i.test(combinedText)) {
        drewToppedCard = true;
        break;
      }
      if (/DISCARD[\s\S]*Discard 1 card/i.test(combinedText)) {
        const hostDiscardChoice = await clickEnabledButton(hostPage, "host-discard-choice", /^(Island|Mountain|Mystical Tutor)$/i);
        if (hostDiscardChoice) {
          const hostDiscardSubmit = await clickLocalButton(hostPage, "host-submit-discard", /SUBMIT/i);
          if (hostDiscardSubmit) {
            await sleep(2200);
            continue;
          }
        }
        const guestDiscardChoice = await clickEnabledButton(guestPage, "guest-discard-choice", /^(Island|Mountain|Mystical Tutor)$/i);
        if (guestDiscardChoice) {
          const guestDiscardSubmit = await clickLocalButton(guestPage, "guest-submit-discard", /SUBMIT/i);
          if (guestDiscardSubmit) {
            await sleep(2200);
            continue;
          }
        }
      }

      const hostClicked = await clickLocalDecisionButton(hostPage, "host-advance-after-tutor");
      if (hostClicked) {
        await sleep(2200);
        continue;
      }
      const guestClicked = await clickLocalDecisionButton(guestPage, "guest-advance-after-tutor");
      if (guestClicked) {
        await sleep(2200);
        continue;
      }
      await sleep(1000);
    }
    assert.equal(
      drewToppedCard,
      true,
      `expected Player 0 to draw the card Mystical Tutor put on top\n${lastAdvanceText}`
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

test("full UI PeerJS Gitaxian Probe shows the targeted player's hand to the caster", { timeout: 240000 }, async () => {
  const peerPort = await freePort();
  const peerServer = await startPeerServer(peerPort);
  const { vite, baseUrl } = await startHarnessServer(peerPort);
  const browser = await chromium.launch();
  const hostContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  const guestContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  let hostPage = null;
  let guestPage = null;

  try {
    const hostDeck = deckUrlParam("60 Gitaxian Probe");
    const guestDeck = deckUrlParam("60 Lightning Bolt");
    hostPage = await openFullUiPage(hostContext, `${baseUrl}/?name=Chiplis&deck=${hostDeck}`, "host-ui");
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
      `${baseUrl}/?lobby=${encodeURIComponent(lobbyCode)}&name=Alice&deck=${guestDeck}`,
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

    let castProbe = false;
    let targetedGuest = false;
    let paidProbe = false;
    let guestResolved = false;
    let lastCombinedText = "";

    for (let step = 0; step < 70 && !guestResolved; step += 1) {
      const hostText = await visibleBodyText(hostPage);
      const guestText = await visibleBodyText(guestPage);
      const combinedText = `${hostText}\n${guestText}`;
      lastCombinedText = combinedText;
      assertNoSyncFailureText(combinedText);

      if (
        castProbe
        && targetedGuest
        && !paidProbe
        && /PRIORITY\s+ALICE/i.test(hostText)
        && /STACK[\s\S]*Gitaxian Probe/i.test(hostText)
      ) {
        paidProbe = true;
      }

      if (!castProbe && await clickLocalButton(hostPage, "host-cast-probe", /GITAXIAN PROBE/i)) {
        castProbe = true;
        await sleep(2500);
        continue;
      }

      if (castProbe && !targetedGuest) {
        await hostPage.evaluate(() => {
          window.dispatchEvent(new CustomEvent("ironsmith:target-choice", {
            detail: { target: { kind: "player", player: 1 } },
          }));
        });
        await sleep(250);
        const submittedTargets = await clickLocalButton(
          hostPage,
          "host-submit-probe-target",
          /SUBMIT TARGETS|SUBMIT/i,
        );
        if (submittedTargets) {
          targetedGuest = true;
          await sleep(2500);
          continue;
        }
      }

      if (castProbe && !paidProbe) {
        const selectedPayment = await clickEnabledButton(
          hostPage,
          "host-select-probe-payment",
          /2 life|PHYREXIAN|\{U\/P\}/i,
        );
        if (selectedPayment) {
          await sleep(250);
        }
        const submittedPayment = await clickLocalButton(
          hostPage,
          "host-submit-probe-payment",
          /SUBMIT|PAY/i,
        );
        if (submittedPayment) {
          paidProbe = true;
          await sleep(3000);
          continue;
        }
      }

      const resolveClick = paidProbe && !guestResolved
        ? await activateLocalButton(guestPage, "guest-resolve-probe", /RESOLVE/i)
        : null;
      if (resolveClick) {
        try {
          await waitForVisibleBodyText(
            hostPage,
            /GY\s*1[\s\S]*DECK\s*52/i,
            "expected Gitaxian Probe to resolve before checking the looked-at hand",
            45000,
          );
        } catch (err) {
          throw new Error(`${err.message}
resolve click: ${JSON.stringify(resolveClick, null, 2)}
host buttons: ${JSON.stringify(await buttonDebugText(hostPage), null, 2)}
guest buttons: ${JSON.stringify(await buttonDebugText(guestPage), null, 2)}
host body:
${await visibleBodyText(hostPage)}
guest body:
${await visibleBodyText(guestPage)}
host console: ${JSON.stringify((hostPage.__peerHarnessConsole || []).slice(-120), null, 2)}
guest console: ${JSON.stringify((guestPage.__peerHarnessConsole || []).slice(-120), null, 2)}`);
        }
        guestResolved = true;
        let revealText = "";
        try {
          revealText = await waitForVisibleBodyText(
            hostPage,
            /LOOK[\s\S]*Look at target player's hand[\s\S]*Lightning Bolt/i,
            "expected Gitaxian Probe to show the targeted player's hand to its caster",
            30000,
          );
        } catch (err) {
          err.message = `${err.message}\nhost console: ${JSON.stringify((hostPage.__peerHarnessConsole || []).slice(-120), null, 2)}\nguest console: ${JSON.stringify((guestPage.__peerHarnessConsole || []).slice(-120), null, 2)}`;
          throw err;
        }
        assertNoSyncFailureText(revealText, "Gitaxian Probe hand view should stay synced");
        break;
      }

      if (!castProbe) {
        const hostAdvanced = await clickLocalButton(hostPage, "host-setup-probe", /KEEP HAND|PREGAME|UPKEEP|DRAW|PASS PRIORITY|RESOLVE/i);
        if (hostAdvanced) {
          await sleep(3500);
          continue;
        }
        const guestAdvanced = await clickLocalButton(guestPage, "guest-setup-probe", /KEEP HAND|PREGAME|UPKEEP|DRAW|PASS PRIORITY|RESOLVE/i);
        if (guestAdvanced) {
          await sleep(3500);
          continue;
        }
      }

      await sleep(1000);
    }

    assert.equal(castProbe, true, `expected host to cast Gitaxian Probe\n${lastCombinedText}`);
    assert.equal(targetedGuest, true, `expected host to target the other player with Gitaxian Probe\n${lastCombinedText}`);
    assert.equal(paidProbe, true, `expected host to pay for Gitaxian Probe\n${lastCombinedText}`);
    assert.equal(guestResolved, true, `expected guest to resolve Gitaxian Probe\n${lastCombinedText}`);
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

test("full UI PeerJS Tainted Pact resolution reveals choices and stays synced", { timeout: 240000 }, async () => {
  const peerPort = await freePort();
  const peerServer = await startPeerServer(peerPort);
  const { vite, baseUrl } = await startHarnessServer(peerPort);
  const browser = await chromium.launch();
  const hostContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  const guestContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  let hostPage = null;
  let guestPage = null;

  try {
    const hostDeck = deckUrlParam("30 Black Lotus\n30 Tainted Pact");
    const guestDeck = deckUrlParam("60 Lightning Bolt");
    hostPage = await openFullUiPage(hostContext, `${baseUrl}/?name=Chiplis&deck=${hostDeck}`, "host-tainted-pact-ui");
    await hostPage.getByText("CREATE LOBBY").first().waitFor({ timeout: 30000 });
    await hostPage.getByRole("button").filter({ hasText: /CREATE LOBBY/i }).first().click();
    await hostPage.getByText("Host or join").waitFor({ timeout: 10000 });
    await hostPage.getByRole("button").filter({ hasText: /CREATE LOBBY/i }).last().click();
    await hostPage.getByText("Share this code").waitFor({ timeout: 40000 });

    const lobbyCode = (await visibleBodyText(hostPage)).match(
      /[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/i
    )?.[0];
    assert.ok(lobbyCode, "expected the full UI to create a Tainted Pact lobby code");

    guestPage = await openFullUiPage(
      guestContext,
      `${baseUrl}/?lobby=${encodeURIComponent(lobbyCode)}&name=Alice&deck=${guestDeck}`,
      "guest-tainted-pact-ui",
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

    let castLotus = false;
    let lotusResolved = false;
    let activatedLotus = false;
    let choseBlack = false;
    let castPact = false;
    let pactOnStack = false;
    let lastDebug = "";

    for (let step = 0; step < 100 && !pactOnStack; step += 1) {
      const hostText = await visibleBodyText(hostPage);
      const guestText = await visibleBodyText(guestPage);
      const combinedText = `${hostText}\n${guestText}`;
      lastDebug = combinedText;
      assertNoSyncFailureText(combinedText, "Tainted Pact cast/payment should stay synced");
      assert.doesNotMatch(
        combinedText,
        /Cheat detected|Invalid priority action ref|Action order mismatch/i,
        `Tainted Pact cast/payment should not trip cheat detection
host console: ${JSON.stringify((hostPage.__peerHarnessConsole || []).filter((line) => /Cheat detected|invalid priority/i.test(line)).slice(-20), null, 2)}
guest console: ${JSON.stringify((guestPage.__peerHarnessConsole || []).filter((line) => /Cheat detected|invalid priority/i.test(line)).slice(-20), null, 2)}`
      );

      if (!castLotus) {
        const result = await clickLocalButton(hostPage, "host-cast-lotus", /BLACK LOTUS/i);
        if (result) {
          castLotus = true;
          await sleep(2500);
          continue;
        }
      }

      if (castLotus && !lotusResolved) {
        if (/BF\s*1/i.test(hostText) && /Black Lotus[\s\S]*Battlefield/i.test(hostText)) {
          lotusResolved = true;
          continue;
        }
        const hostPass = await clickLocalButton(hostPage, "host-pass-lotus", /PASS PRIORITY|RESOLVE/i);
        if (hostPass) {
          await sleep(2500);
          continue;
        }
        const guestPass = await clickLocalButton(guestPage, "guest-resolve-lotus", /PASS PRIORITY|RESOLVE/i);
        if (guestPass) {
          await sleep(2500);
          continue;
        }
      }

      if (lotusResolved && !activatedLotus) {
        const result = await clickLocalButton(hostPage, "host-activate-lotus", /ADD|SACRIFICE/i);
        if (result) {
          activatedLotus = true;
          await sleep(2500);
          continue;
        }
        await sleep(1000);
        continue;
      }

      if (activatedLotus && !choseBlack) {
        const result = await clickEnabledButton(hostPage, "host-choose-black", /BLACK|\{B\}/i);
        if (result) {
          choseBlack = true;
          await sleep(2500);
          continue;
        }
      }

      if (choseBlack && !castPact) {
        const result = await clickLocalButton(hostPage, "host-cast-pact", /TAINTED PACT/i);
        if (result) {
          castPact = true;
          await sleep(2500);
          continue;
        }
      }

      if (castPact) {
        if (/Pay [\s\S]*Tainted Pact/i.test(hostText)) {
          const selectedPayment = await clickEnabledButton(
            hostPage,
            "host-pay-pact",
            /BLACK|GENERIC|MANA|\{B\}|\{1\}|SUBMIT|PAY/i,
          );
          if (selectedPayment) {
            await sleep(2500);
            continue;
          }
        } else if (
          /STACK[\s\S]*Tainted Pact/i.test(hostText)
          || /STACK[\s\S]*Tainted Pact/i.test(guestText)
        ) {
          pactOnStack = true;
          break;
        }
      }

      const hostProgress = await clickLocalButton(hostPage, "host-setup-pact", /KEEP HAND|PREGAME|UPKEEP|DRAW|MAIN|PASS PRIORITY|RESOLVE/i);
      if (hostProgress) {
        await sleep(2500);
        continue;
      }

      const guestProgress = await clickLocalButton(guestPage, "guest-setup-pact", /KEEP HAND|PREGAME|UPKEEP|DRAW|MAIN|PASS PRIORITY|RESOLVE/i);
      if (guestProgress) {
        await sleep(2500);
        continue;
      }

      await sleep(1000);
    }

    assert.equal(castLotus, true, `expected to cast Black Lotus\n${lastDebug}`);
    assert.equal(lotusResolved, true, `expected Black Lotus to resolve\n${lastDebug}`);
    assert.equal(activatedLotus, true, `expected to activate Black Lotus\n${lastDebug}`);
    assert.equal(choseBlack, true, `expected to choose black mana\n${lastDebug}`);
    assert.equal(castPact, true, `expected to cast Tainted Pact\n${lastDebug}`);
    assert.equal(pactOnStack, true, `expected Tainted Pact to reach the stack\n${lastDebug}`);
    assertNoSyncFailureText(await visibleBodyText(hostPage), "host should remain synced after Tainted Pact payment");
    assertNoSyncFailureText(await visibleBodyText(guestPage), "guest should remain synced after Tainted Pact payment");

    let sawTaintedPactPrompt = false;
    for (let step = 0; step < 40 && !sawTaintedPactPrompt; step += 1) {
      const hostText = await visibleBodyText(hostPage);
      const guestText = await visibleBodyText(guestPage);
      const combinedText = `${hostText}\n${guestText}`;
      assertNoSyncFailureText(combinedText, "Tainted Pact resolution should stay synced");
      assert.doesNotMatch(
        combinedText,
        /Cheat detected|Invalid priority action ref|Action order mismatch/i,
        `Tainted Pact resolution should not trip cheat detection
host console: ${JSON.stringify((hostPage.__peerHarnessConsole || []).slice(-40), null, 2)}
guest console: ${JSON.stringify((guestPage.__peerHarnessConsole || []).slice(-40), null, 2)}`
      );
      assert.doesNotMatch(hostText, /put Hidden Card into your hand/i);
      if (/put (Black Lotus|Island|Swamp|Tainted Pact) into your hand/i.test(hostText)) {
        sawTaintedPactPrompt = true;
        break;
      }
      const hostPass = await clickLocalButton(hostPage, "host-pass-pact", /PASS PRIORITY|RESOLVE/i);
      if (hostPass) {
        await sleep(2500);
        continue;
      }
      const guestResolve = await clickLocalButton(guestPage, "guest-resolve-pact", /RESOLVE|PASS PRIORITY/i);
      if (guestResolve) {
        await sleep(2500);
        continue;
      }
      await sleep(1000);
    }
    assert.equal(sawTaintedPactPrompt, true, `expected Tainted Pact to reveal a named card choice\n${await visibleBodyText(hostPage)}`);
    await sleep(4000);
    {
      const hostText = await visibleBodyText(hostPage);
      const guestText = await visibleBodyText(guestPage);
      const combinedText = `${hostText}\n${guestText}`;
      assertNoSyncFailureText(combinedText, "Tainted Pact first choice should not fail while waiting for input");
      assert.doesNotMatch(hostText, /put Hidden Card into your hand/i);
      assert.doesNotMatch(
        combinedText,
        /Missing audit material for public_view_window|Missing public_open audit opening|Cheat detected|Invalid priority action ref|Action order mismatch/i,
        `Tainted Pact first choice should include public-view audit material
host console: ${JSON.stringify((hostPage.__peerHarnessConsole || []).slice(-40), null, 2)}
guest console: ${JSON.stringify((guestPage.__peerHarnessConsole || []).slice(-40), null, 2)}`,
      );
    }

    let sawDuplicateStopReveal = false;
    let declinedPactCards = 0;
    let lastDuplicateStopText = "";
    for (let step = 0; step < 50 && !sawDuplicateStopReveal; step += 1) {
      const hostText = await visibleBodyText(hostPage);
      const guestText = await visibleBodyText(guestPage);
      const combinedText = `${hostText}\n${guestText}`;
      lastDuplicateStopText = hostText;
      assertNoSyncFailureText(combinedText, "Tainted Pact duplicate-stop reveal should stay synced");
      assert.doesNotMatch(
        combinedText,
        /Missing public_open audit opening|Cheat detected|Invalid priority action ref|Action order mismatch/i,
        `Tainted Pact duplicate-stop reveal should not trip audit verification
host console: ${JSON.stringify((hostPage.__peerHarnessConsole || []).slice(-40), null, 2)}
guest console: ${JSON.stringify((guestPage.__peerHarnessConsole || []).slice(-40), null, 2)}`
      );
      if (
        /Reveal exiled library cards/i.test(hostText)
        && /(Island|Swamp|Black Lotus|Tainted Pact)/i.test(hostText)
        && !/Card #\d+/i.test(hostText)
        && !/Reveal exiled library cards[\s\S]{0,240}Hidden Card/i.test(hostText)
      ) {
        sawDuplicateStopReveal = true;
        break;
      }
      if (/Put .* into your hand\?/i.test(hostText) && /(?:^|\n)NO(?:\n|$)/i.test(hostText)) {
        const declinedCard = await clickEnabledButton(
          hostPage,
          `host-decline-pact-card-${declinedPactCards + 1}`,
          /^NO$/i,
        );
        if (declinedCard) {
          declinedPactCards += 1;
          await sleep(2500);
          continue;
        }
      }
      await sleep(1000);
    }
    assert.ok(
      declinedPactCards > 0,
      `expected to decline at least one Tainted Pact card before duplicate stop
${lastDuplicateStopText}`,
    );
    assert.equal(
      sawDuplicateStopReveal,
      true,
      `expected Tainted Pact duplicate-stop card to be revealed instead of Hidden Card
${lastDuplicateStopText}
host console: ${JSON.stringify((hostPage.__peerHarnessConsole || []).slice(-20), null, 2)}
guest console: ${JSON.stringify((guestPage.__peerHarnessConsole || []).slice(-20), null, 2)}`,
    );
    await hostPage.waitForFunction(
      () => /1\s*\/\s*[3-9]/.test(document.body.innerText || ""),
      null,
      { timeout: 5000 },
    );
    assert.match(
      await visibleBodyText(hostPage),
      /1\s*\/\s*[3-9]/i,
      "Tainted Pact reveal inspector should include exiled cards plus the resolving spell",
    );
    assert.match(
      await visibleBodyText(hostPage),
      /Deck\s*->\s*Exile/i,
      "Tainted Pact reveal inspector should use the runtime library-to-exile zone label",
    );
    assert.doesNotMatch(
      await visibleBodyText(hostPage),
      /Hidden\s*->\s*Exile/i,
      "Tainted Pact reveal inspector should not fall back to hidden-to-exile labels",
    );

    const completedDuplicateRevealStep =
      await clickLocalButton(hostPage, "host-complete-duplicate-pact-reveal", /^DONE$/i)
      || await clickEnabledButton(hostPage, "host-complete-duplicate-pact-reveal", /^DONE$/i);
    assert.ok(
      completedDuplicateRevealStep,
      `expected to acknowledge the duplicate-stop Tainted Pact reveal\n${await visibleBodyText(hostPage)}`,
    );

    let sawFinalPactState = false;
    let finalPactText = "";
    for (let step = 0; step < 20 && !sawFinalPactState; step += 1) {
      const hostText = await visibleBodyText(hostPage);
      const guestText = await visibleBodyText(guestPage);
      const combinedText = `${hostText}\n${guestText}`;
      finalPactText = hostText;
      assertNoSyncFailureText(combinedText, "Tainted Pact final state should stay synced");
      assert.doesNotMatch(
        combinedText,
        /Missing public_open audit opening|Cheat detected|Invalid priority action ref|Action order mismatch/i,
        `Tainted Pact final state should not trip audit verification
host console: ${JSON.stringify((hostPage.__peerHarnessConsole || []).slice(-40), null, 2)}
guest console: ${JSON.stringify((guestPage.__peerHarnessConsole || []).slice(-40), null, 2)}`
      );
      assert.doesNotMatch(hostText, /Card #\d+/i, `Tainted Pact should not leave stale viewed-card labels\n${hostText}`);
      if (/EXL\s*[2-9]/i.test(hostText) && /GY\s*[1-9]/i.test(hostText)) {
        sawFinalPactState = true;
        break;
      }
      await sleep(1000);
    }
    assert.equal(
      sawFinalPactState,
      true,
      `expected declined duplicate-stop Tainted Pact flow to leave two cards in exile and Tainted Pact in graveyard\n${finalPactText}`,
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
