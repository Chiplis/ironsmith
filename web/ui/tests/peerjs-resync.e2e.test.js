import test from "node:test";
import assert from "node:assert/strict";
import { Buffer } from "node:buffer";
import fs from "node:fs";
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
const FULL_UI_RECONNECT_DISCONNECT_MS = 15250;
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
  process.env.VITE_E2E_TEST = "true";

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

async function fullUiSnapshot(page) {
  return page.evaluate(() => window.__ironsmithE2E?.snapshot?.() || null);
}

async function waitForFullUiSnapshot(page, predicate, label, timeoutMs = 60000) {
  const started = Date.now();
  let lastSnapshot = null;
  while (Date.now() - started < timeoutMs) {
    lastSnapshot = await fullUiSnapshot(page);
    if (lastSnapshot && predicate(lastSnapshot)) return lastSnapshot;
    await sleep(250);
  }
  let body = "";
  try {
    body = await visibleBodyText(page);
  } catch {
    body = "<page unavailable>";
  }
  assert.fail(`${label}\nLast snapshot: ${JSON.stringify(lastSnapshot, null, 2)}\nbody:\n${body}`);
}

async function waitForFullUiSync(hostPage, guestPage, label, timeoutMs = 60000) {
  const started = Date.now();
  let lastHost = null;
  let lastGuest = null;
  while (Date.now() - started < timeoutMs) {
    [lastHost, lastGuest] = await Promise.all([
      fullUiSnapshot(hostPage),
      fullUiSnapshot(guestPage),
    ]);
    if (
      lastHost?.multiplayer?.matchStarted
      && lastGuest?.multiplayer?.matchStarted
      && lastHost?.state
      && lastGuest?.state
      && Number(lastHost?.multiplayer?.lastAppliedSequence) === Number(lastGuest?.multiplayer?.lastAppliedSequence)
    ) {
      return { host: lastHost, guest: lastGuest };
    }
    await sleep(250);
  }
  assert.fail(
    `${label}\nhost: ${JSON.stringify(lastHost, null, 2)}\nguest: ${JSON.stringify(lastGuest, null, 2)}`
  );
}

async function waitForFullUiPair(hostPage, guestPage, predicate, label, timeoutMs = 60000) {
  const started = Date.now();
  let lastHost = null;
  let lastGuest = null;
  while (Date.now() - started < timeoutMs) {
    [lastHost, lastGuest] = await Promise.all([
      fullUiSnapshot(hostPage),
      fullUiSnapshot(guestPage),
    ]);
    if (lastHost && lastGuest && predicate(lastHost, lastGuest)) {
      return { host: lastHost, guest: lastGuest };
    }
    await sleep(250);
  }
  assert.fail(
    `${label}\nhost: ${JSON.stringify(lastHost, null, 2)}\nguest: ${JSON.stringify(lastGuest, null, 2)}`
  );
}

async function waitForFullUiSequenceAdvance(hostPage, guestPage, beforeSequence, label, timeoutMs = 120000) {
  return waitForFullUiPair(
    hostPage,
    guestPage,
    (host, guest) =>
      Number(host?.multiplayer?.lastAppliedSequence || 0) > Number(beforeSequence || 0)
      && Number(host?.multiplayer?.lastAppliedSequence || 0)
        === Number(guest?.multiplayer?.lastAppliedSequence || 0),
    label,
    timeoutMs,
  );
}

async function assertNoFullUiSyncFailures(...pages) {
  const text = (await Promise.all(
    pages.filter(Boolean).map((page) => visibleBodyText(page).catch(() => ""))
  )).join("\n");
  assertNoSyncFailureText(text);
}

async function assertNoFullUiSyncFailuresWithDebug(label, ...pages) {
  const text = (await Promise.all(
    pages.filter(Boolean).map((page) => visibleBodyText(page).catch(() => ""))
  )).join("\n");
  try {
    assertNoSyncFailureText(text, label);
  } catch (err) {
    const debug = await Promise.all(
      pages.filter(Boolean).map((page, index) => fullUiPageDebug(page, index))
    );
    const summary = debug.map((entry) => ({
      index: entry.index,
      status: entry.snapshot?.status || null,
      lastAppliedSequence: entry.snapshot?.multiplayer?.lastAppliedSequence ?? null,
      matchStarted: entry.snapshot?.multiplayer?.matchStarted ?? null,
      mode: entry.snapshot?.multiplayer?.mode ?? null,
      matchDisputed: entry.snapshot?.multiplayer?.matchDisputed || null,
    }));
    assert.fail(`${label}\n${err?.message || err}\nsummary: ${JSON.stringify(summary, null, 2)}\n${JSON.stringify(debug, null, 2)}`);
  }
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
  return page.locator("body").innerText({ timeout: 120000 });
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

async function fullUiPageDebug(page, index = 0) {
  return {
    index,
    label: page?.__peerHarnessLabel || "",
    url: page?.url?.() || "",
    errors: page?.__peerHarnessErrors || [],
    console: (page?.__peerHarnessConsole || []).slice(-30),
    snapshot: await fullUiSnapshot(page).catch((err) => String(err?.message || err)),
    buttons: await buttonDebugText(page).catch((err) => String(err?.message || err)),
    body: await visibleBodyText(page).catch((err) => String(err?.message || err)),
  };
}

async function visibleHandCardNames(page) {
  return page.locator(".hand-card[data-card-name]").evaluateAll((cards) =>
    cards
      .map((card) => String(card.getAttribute("data-card-name") || "").trim())
      .filter(Boolean)
  );
}

function snapshotBattlefieldCards(snapshot) {
  return (snapshot?.state?.players || []).flatMap((player) =>
    (player.battlefield || []).flatMap((card) => {
      const count = Math.max(1, Number(card?.count || 1));
      return Array.from({ length: count }, () => ({
        ...card,
        playerId: player.id,
      }));
    })
  );
}

function snapshotBattlefieldCardCount(snapshot, cardName) {
  return snapshotBattlefieldCards(snapshot).filter(
    (card) => String(card?.name || "") === cardName
  ).length;
}

function snapshotPlayer(snapshot, playerIndex) {
  return (snapshot?.state?.players || []).find(
    (player) => Number(player?.id) === Number(playerIndex)
  ) || null;
}

function snapshotZoneCardCount(cards) {
  return (cards || []).reduce((total, card) => {
    const count = Number(card?.count);
    return total + (Number.isFinite(count) && count > 0 ? count : 1);
  }, 0);
}

async function stackCardCount(page, cardName) {
  return page.locator(".stack-card[data-card-name]").evaluateAll((cards, name) =>
    cards.filter((card) => String(card.getAttribute("data-card-name") || "") === name).length,
    cardName,
  );
}

async function waitForNamedVisibleHand(page, label, timeoutMs = 60000, snapshotPredicate = null) {
  const started = Date.now();
  let names = [];
  let snapshot = null;
  while (Date.now() - started < timeoutMs) {
    if (snapshotPredicate) {
      snapshot = await fullUiSnapshot(page);
      if (!snapshotPredicate(snapshot)) {
        await sleep(250);
        continue;
      }
    }
    names = await visibleHandCardNames(page);
    if (names.length >= 7 && names.every((name) => !/^Hidden Card$/i.test(name))) {
      return names;
    }
    await sleep(250);
  }
  assert.fail(`${label}\nsnapshot: ${JSON.stringify(snapshot, null, 2)}\nhand: ${JSON.stringify(names, null, 2)}\nbody:\n${await visibleBodyText(page)}`);
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
  const failurePattern = /Unknown Ziffle Ceremony|Unknown ziffle ceremony|Private deck opening does not match slot|Ziffle card opening proof reveals a different committed slot|hidden card commitment does not match reveal|No direct ziffle route|Match clock elapsed time exceeds local observation|Sequenced action public checkpoint hash does not match local state|Sync failed|Match start failed|Auto-pass failed|Resync checkpoint hash mismatch|Match disputed|Protocol response timeout|Disconnect timeout policy failed/i;
  const match = String(text || "").match(failurePattern);
  assert.ok(!match, `${label}: ${match?.[0] || ""}`);
}

async function hasLocalButton(page, textPattern) {
  const buttons = await buttonDebugText(page);
  return buttons.some((button) =>
    button.localAction === "true"
    && !button.disabled
    && button.ariaDisabled !== "true"
    && textPattern.test(button.text)
  );
}

async function waitAndClickLocalButton(page, label, textPattern, timeoutMs = 90000) {
  const started = Date.now();
  while (Date.now() - started < timeoutMs) {
    const clicked = await activateLocalButton(page, label, textPattern);
    if (clicked) return clicked;
    await sleep(250);
  }
  assert.fail(
    `${label}\nbuttons: ${JSON.stringify(await buttonDebugText(page), null, 2)}\nbody:\n${await visibleBodyText(page)}`
  );
}

async function waitAndClickEnabledButton(page, label, textPattern, timeoutMs = 90000) {
  const started = Date.now();
  while (Date.now() - started < timeoutMs) {
    const clicked = await clickEnabledButton(page, label, textPattern);
    if (clicked) return clicked;
    await sleep(500);
  }
  assert.fail(
    `${label}\nbuttons: ${JSON.stringify(await buttonDebugText(page), null, 2)}\nbody:\n${await visibleBodyText(page)}`
  );
}

async function captureFullUiStep(dir, index, slug, pages) {
  fs.mkdirSync(dir, { recursive: true });
  const prefix = String(index).padStart(2, "0");
  const saved = [];
  for (const [label, page] of pages) {
    if (!page) continue;
    const filePath = path.join(dir, `${prefix}-${slug}-${label}.png`);
    await page.screenshot({ path: filePath, fullPage: false, timeout: 120000 });
    saved.push(filePath);
  }
  return saved;
}

async function startFullUiPeerMatch({
  baseUrl,
  hostContext,
  guestContext,
  hostDeckText = "60 Mountain",
  guestDeckText = "60 Mountain",
  hostName = "Chiplis",
  guestName = "Alice",
  hostLabel = "host-ui",
  guestLabel = "guest-ui",
}) {
  const hostDeck = deckUrlParam(hostDeckText);
  const guestDeck = deckUrlParam(guestDeckText);
  const hostPage = await openFullUiPage(hostContext, `${baseUrl}/?name=${encodeURIComponent(hostName)}&deck=${hostDeck}`, hostLabel);
  await hostPage.getByText("CREATE LOBBY").first().waitFor({ timeout: 30000 });
  await hostPage.getByRole("button").filter({ hasText: /CREATE LOBBY/i }).first().click();
  await hostPage.getByText("Host or join").waitFor({ timeout: 10000 });
  await hostPage.getByRole("button").filter({ hasText: /CREATE LOBBY/i }).last().click();
  await hostPage.getByText("Share this code").waitFor({ timeout: 40000 });

  const lobbyCode = (await visibleBodyText(hostPage)).match(
    /[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/i
  )?.[0];
  assert.ok(lobbyCode, "expected the full UI to create a lobby code");

  const guestPage = await openFullUiPage(
    guestContext,
    `${baseUrl}/?lobby=${encodeURIComponent(lobbyCode)}&name=${encodeURIComponent(guestName)}&deck=${guestDeck}`,
    guestLabel,
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

async function clickAnyFullUiProgressAction(pages, label, timeoutMs = 90000) {
  const progressPattern = /PLAY MOUNTAIN|KEEP HAND|PREGAME|BEGIN GAME|CONTINUE|UNTAP|UPKEEP|DRAW|MAIN|PASS PRIORITY|RESOLVE/i;
  const started = Date.now();
  while (Date.now() - started < timeoutMs) {
    await assertNoFullUiSyncFailures(...pages);
    for (const [index, page] of pages.entries()) {
      const click = await clickLocalButton(page, `${label}-${index}`, progressPattern);
      if (click) return { page, index, click };
    }
    await sleep(1000);
  }

  const debug = await Promise.all(pages.map(async (page, index) => ({
    index,
    buttons: await buttonDebugText(page).catch((err) => String(err?.message || err)),
    body: await visibleBodyText(page).catch((err) => String(err?.message || err)),
  })));
  assert.fail(`${label}\n${JSON.stringify(debug, null, 2)}`);
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
    const afterPromotedGuestAction = await Promise.all([
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
    await waitAndClickLocalButton(hostPage, "host-keep-gemstone-test", /KEEP HAND/i, 120000);
    await sleep(2500);
    await assertNoFullUiSyncFailures(hostPage, guestPage);
    await waitAndClickLocalButton(guestPage, "guest-keep-gemstone-test", /KEEP HAND/i, 120000);
    await sleep(3000);
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

    for (
      let step = 0;
      step < 12 && !/Choose which Gemstone Caverns to keep \(legend rule\)/i.test(await visibleBodyText(guestPage));
      step += 1
    ) {
      await clickAnyFullUiProgressAction([hostPage, guestPage], `advance-to-gemstone-legend-${step}`, 60000);
      await sleep(1500);
    }
    await waitForVisibleBodyText(
      guestPage,
      /Choose which Gemstone Caverns to keep \(legend rule\)/i,
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

async function driveDemonicConsultationCast({ hostPage, guestPage, actorPage, actorLabel, maxSteps = 160 }) {
  let playedSwamp = false;
  let attemptedCastConsultation = false;
  let castConsultation = false;
  let consultationOnStack = false;
  let lastDebug = "";

  for (let step = 0; step < maxSteps && !consultationOnStack; step += 1) {
    const [hostText, guestText, hostSnapshot, guestSnapshot] = await Promise.all([
      visibleBodyText(hostPage),
      visibleBodyText(guestPage),
      fullUiSnapshot(hostPage),
      fullUiSnapshot(guestPage),
    ]);
    const actorText = actorPage === hostPage ? hostText : guestText;
    const actorSnapshot = actorPage === hostPage ? hostSnapshot : guestSnapshot;
    const actorDecisionKind = String(actorSnapshot?.state?.decision?.kind || "");
    const combinedText = `${hostText}\n${guestText}`;
    lastDebug = combinedText;
    try {
      assertNoSyncFailureText(combinedText, "Demonic Consultation cast/payment should stay synced");
    } catch {
      await assertNoFullUiSyncFailuresWithDebug(
        "Demonic Consultation cast/payment should stay synced",
        hostPage,
        guestPage,
      );
    }

    if (!playedSwamp) {
      const result = await clickLocalButton(actorPage, `${actorLabel}-play-swamp`, /PLAY SWAMP/i);
      if (result) {
        playedSwamp = true;
        await sleep(3500);
        continue;
      }
    }

    if (playedSwamp && !castConsultation && /CAST DEMONIC CONSULTATION/i.test(actorText)) {
      const result = await clickLocalButton(
        actorPage,
        `${actorLabel}-cast-consultation`,
        /CAST DEMONIC CONSULTATION/i
      );
      if (result) {
        attemptedCastConsultation = true;
        await sleep(2500);
        continue;
      }
    }

    if (attemptedCastConsultation || castConsultation) {
      const paymentPending = /Pay [\s\S]*Demonic Consultation|CHOOSE OPTION|remaining|Use\s+from mana pool/i.test(actorText);
      if (paymentPending) {
        castConsultation = true;
      }
      if (paymentPending) {
        const paymentOption =
          await clickLocalButton(
            actorPage,
            `${actorLabel}-pay-consultation-with-swamp`,
            /Tap Swamp|SWAMP|ADD|BLACK|Use\s+from mana pool|\{B\}/i
          )
          || await clickEnabledButton(
            actorPage,
            `${actorLabel}-pay-consultation-with-swamp`,
            /Tap Swamp|SWAMP|ADD|BLACK|Use\s+from mana pool|\{B\}/i
          );
        if (paymentOption) {
          await sleep(3000);
          continue;
        }
        const paymentSubmit =
          await clickLocalButton(actorPage, `${actorLabel}-submit-consultation-payment`, /^SUBMIT$|^PAY$/i)
          || await clickEnabledButton(actorPage, `${actorLabel}-submit-consultation-payment`, /^SUBMIT$|^PAY$/i);
        if (paymentSubmit) {
          await sleep(3000);
          continue;
        }
      }
      if (
        !paymentPending
        && /^(priority|text_input)$/.test(actorDecisionKind)
        && (
          await stackCardCount(hostPage, "Demonic Consultation") > 0
          || await stackCardCount(guestPage, "Demonic Consultation") > 0
        )
      ) {
        consultationOnStack = true;
        break;
      }
    }

    if (playedSwamp && !castConsultation && attemptedCastConsultation) {
      await sleep(1000);
      continue;
    }

    const hostProgress = await clickLocalButton(
      hostPage,
      `${actorLabel}-host-progress-to-consultation`,
      /KEEP HAND|PREGAME|BEGIN GAME|UPKEEP|DRAW|MAIN|COMBAT|ATTACKERS|BLOCKERS|NO ATTACKERS|DONE|M2|END|CLEAN|PASS PRIORITY|RESOLVE/i
    );
    if (hostProgress) {
      await sleep(2500);
      continue;
    }
    const guestProgress = await clickLocalButton(
      guestPage,
      `${actorLabel}-guest-progress-to-consultation`,
      /KEEP HAND|PREGAME|BEGIN GAME|UPKEEP|DRAW|MAIN|COMBAT|ATTACKERS|BLOCKERS|NO ATTACKERS|DONE|M2|END|CLEAN|PASS PRIORITY|RESOLVE/i
    );
    if (guestProgress) {
      await sleep(2500);
      continue;
    }
    await sleep(1000);
  }

  assert.equal(playedSwamp, true, `expected to play Swamp\n${lastDebug}`);
  assert.equal(castConsultation, true, `expected to cast Demonic Consultation\n${lastDebug}`);
  assert.equal(consultationOnStack, true, `expected Demonic Consultation to reach the stack\n${lastDebug}`);
}

async function resolveDemonicConsultationWithMissingName({
  hostPage,
  guestPage,
  actorPage,
  actorLabel,
  actorIndex,
  missingName = "Black Lotus",
  maxSteps = 180,
}) {
  let choseName = false;
  let resolvedMissingName = false;
  let preLibrarySize = null;
  let preExileCount = 0;
  let lastDebug = "";
  let lastHostSnapshot = null;
  let lastGuestSnapshot = null;
  const progressPattern = /DONE|SUBMIT|PASS PRIORITY|RESOLVE/i;

  for (let step = 0; step < maxSteps && !resolvedMissingName; step += 1) {
    const [hostText, guestText, hostSnapshot, guestSnapshot] = await Promise.all([
      visibleBodyText(hostPage),
      visibleBodyText(guestPage),
      fullUiSnapshot(hostPage),
      fullUiSnapshot(guestPage),
    ]);
    const actorText = actorPage === hostPage ? hostText : guestText;
    const actorSnapshot = actorPage === hostPage ? hostSnapshot : guestSnapshot;
    const actorDecision = actorSnapshot?.state?.decision || null;
    const combinedText = `${hostText}\n${guestText}`;
    lastDebug = combinedText;
    try {
      assertNoSyncFailureText(combinedText, "Demonic Consultation missing-name resolution should stay synced");
    } catch {
      await assertNoFullUiSyncFailuresWithDebug(
        "Demonic Consultation missing-name resolution should stay synced",
        hostPage,
        guestPage,
      );
    }

    if (
      !choseName
      && actorDecision?.kind === "text_input"
      && Number(actorDecision?.player) === Number(actorIndex)
      && /Choose a card name/i.test(String(actorDecision?.description || actorText))
    ) {
      const preSnapshot = actorSnapshot;
      const caster = snapshotPlayer(preSnapshot, actorIndex);
      preLibrarySize = Number(caster?.library_size);
      preExileCount = snapshotZoneCardCount(caster?.exile_cards);
      assert.ok(
        Number.isFinite(preLibrarySize) && preLibrarySize > 0,
        `expected caster to have a library before naming a missing card\n${JSON.stringify(preSnapshot, null, 2)}`
      );

      const nameInput = actorPage.locator('input[placeholder="Enter a card name"], input[type="text"]').first();
      await nameInput.fill(missingName, { timeout: 10000 });
      await actorPage.evaluate(async ({ value }) => {
        await window.__ironsmithE2E?.submitMultiplayerCommand?.(
          { type: "text_choice", value },
          value,
        );
      }, { value: missingName });
      choseName = true;
      await sleep(5000);
      continue;
    }

    if (choseName) {
      [lastHostSnapshot, lastGuestSnapshot] = await Promise.all([
        fullUiSnapshot(hostPage),
        fullUiSnapshot(guestPage),
      ]);
      const actorSnapshot = actorPage === hostPage ? lastHostSnapshot : lastGuestSnapshot;
      const caster = snapshotPlayer(actorSnapshot, actorIndex);
      const librarySize = Number(caster?.library_size);
      const exileCount = snapshotZoneCardCount(caster?.exile_cards);
      const hostSequence = Number(lastHostSnapshot?.multiplayer?.lastAppliedSequence || 0);
      const guestSequence = Number(lastGuestSnapshot?.multiplayer?.lastAppliedSequence || 0);
      if (
        librarySize === 0
        && exileCount - preExileCount === preLibrarySize
        && hostSequence === guestSequence
      ) {
        resolvedMissingName = true;
        const acknowledged =
          await clickLocalButton(actorPage, `${actorLabel}-acknowledge-consultation-reveal`, /^DONE$/i)
          || await clickEnabledButton(actorPage, `${actorLabel}-acknowledge-consultation-reveal`, /^DONE$/i);
        if (acknowledged) {
          await sleep(2500);
        }
        break;
      }
    }

    const actorDone =
      await clickLocalButton(actorPage, `${actorLabel}-consultation-done-${step}`, /^DONE$/i)
      || await clickEnabledButton(actorPage, `${actorLabel}-consultation-done-${step}`, /^DONE$/i);
    if (actorDone) {
      await sleep(2500);
      continue;
    }

    const actorProgress = await clickLocalButton(
      actorPage,
      `${actorLabel}-resolve-consultation-${step}`,
      progressPattern
    );
    if (actorProgress) {
      await sleep(2500);
      continue;
    }

    const hostProgress = await clickLocalButton(
      hostPage,
      `${actorLabel}-host-resolve-consultation-${step}`,
      progressPattern
    );
    if (hostProgress) {
      await sleep(2500);
      continue;
    }

    const guestProgress = await clickLocalButton(
      guestPage,
      `${actorLabel}-guest-resolve-consultation-${step}`,
      progressPattern
    );
    if (guestProgress) {
      await sleep(2500);
      continue;
    }

    await sleep(1000);
  }

  assert.equal(choseName, true, `expected to choose a missing card name for Demonic Consultation\n${lastDebug}`);
  assert.equal(
    resolvedMissingName,
    true,
    `expected naming ${missingName} to exile the caster's whole library
preLibrarySize: ${preLibrarySize}
preExileCount: ${preExileCount}
host: ${JSON.stringify(lastHostSnapshot, null, 2)}
guest: ${JSON.stringify(lastGuestSnapshot, null, 2)}
body:
${lastDebug}`
  );

  await assertNoFullUiSyncFailures(hostPage, guestPage);
  const [hostSnapshot, guestSnapshot] = await Promise.all([
    fullUiSnapshot(hostPage),
    fullUiSnapshot(guestPage),
  ]);
  for (const [label, snapshot] of [["host", hostSnapshot], ["guest", guestSnapshot]]) {
    const caster = snapshotPlayer(snapshot, actorIndex);
    assert.equal(
      Number(caster?.library_size),
      0,
      `${label} should see the caster's library empty after missing-name Consultation\n${JSON.stringify(snapshot, null, 2)}`
    );
    assert.equal(
      snapshotZoneCardCount(caster?.exile_cards) - preExileCount,
      preLibrarySize,
      `${label} should see all remaining library cards in exile after missing-name Consultation\n${JSON.stringify(snapshot, null, 2)}`
    );
  }
}

test("full UI PeerJS casting Demonic Consultation after playing Swamp keeps hidden openings synced", { timeout: 300000 }, async () => {
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
      guestDeckText: "60 Lightning Bolt",
      hostName: "Alice P1",
      guestName: "Alice P2",
      hostLabel: "host-consultation-ui",
      guestLabel: "guest-consultation-ui",
    });
    hostPage = match.hostPage;
    guestPage = match.guestPage;

    await assertNoFullUiSyncFailures(hostPage, guestPage);
    await waitAndClickLocalButton(hostPage, "host-keep-consultation-test", /KEEP HAND/i, 120000);
    await sleep(2500);
    await assertNoFullUiSyncFailures(hostPage, guestPage);
    const guestKeep = await clickLocalButton(guestPage, "guest-keep-consultation-test", /KEEP HAND/i);
    if (guestKeep) {
      await sleep(3000);
    }
    await assertNoFullUiSyncFailures(hostPage, guestPage);

    await driveDemonicConsultationCast({
      hostPage,
      guestPage,
      actorPage: hostPage,
      actorLabel: "host",
      maxSteps: 120,
    });
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

test("full UI PeerJS guest casting Demonic Consultation after playing Swamp keeps hidden openings synced", { timeout: 360000 }, async () => {
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
      hostDeckText: "60 Lightning Bolt",
      guestDeckText: "30 Swamp\n30 Demonic Consultation",
      hostName: "Alice P1",
      guestName: "Alice P2",
      hostLabel: "host-guest-consultation-ui",
      guestLabel: "guest-guest-consultation-ui",
    });
    hostPage = match.hostPage;
    guestPage = match.guestPage;

    await assertNoFullUiSyncFailures(hostPage, guestPage);
    await waitAndClickLocalButton(hostPage, "host-keep-guest-consultation-test", /KEEP HAND/i, 120000);
    await sleep(2500);
    await assertNoFullUiSyncFailures(hostPage, guestPage);
    const guestKeep = await clickLocalButton(guestPage, "guest-keep-guest-consultation-test", /KEEP HAND/i);
    if (guestKeep) {
      await sleep(3000);
    }
    await assertNoFullUiSyncFailures(hostPage, guestPage);

    await driveDemonicConsultationCast({
      hostPage,
      guestPage,
      actorPage: guestPage,
      actorLabel: "guest",
      maxSteps: 180,
    });
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

test("full UI PeerJS guest Demonic Consultation missing name exiles the library without desync", { timeout: 480000 }, async () => {
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
      hostDeckText: "60 Lightning Bolt",
      guestDeckText: "30 Swamp\n30 Demonic Consultation",
      hostName: "Alice P1",
      guestName: "Alice P2",
      hostLabel: "host-guest-consultation-missing-name",
      guestLabel: "guest-guest-consultation-missing-name",
    });
    hostPage = match.hostPage;
    guestPage = match.guestPage;

    await assertNoFullUiSyncFailures(hostPage, guestPage);
    await waitAndClickLocalButton(hostPage, "host-keep-guest-consultation-missing-name-test", /KEEP HAND/i, 120000);
    await sleep(2500);
    await assertNoFullUiSyncFailures(hostPage, guestPage);
    const guestKeep = await clickLocalButton(guestPage, "guest-keep-guest-consultation-missing-name-test", /KEEP HAND/i);
    if (guestKeep) {
      await sleep(3000);
    }
    await assertNoFullUiSyncFailures(hostPage, guestPage);

    await driveDemonicConsultationCast({
      hostPage,
      guestPage,
      actorPage: guestPage,
      actorLabel: "guest",
      maxSteps: 180,
    });
    await resolveDemonicConsultationWithMissingName({
      hostPage,
      guestPage,
      actorPage: guestPage,
      actorLabel: "guest",
      actorIndex: 1,
      missingName: "Black Lotus",
      maxSteps: 220,
    });
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

test("full UI PeerJS Selvala after host mulligans reveals ziffle libraries without desync", { timeout: 600000 }, async () => {
  const peerPort = await freePort();
  const peerServer = await startPeerServer(peerPort);
  const { vite, baseUrl } = await startHarnessServer(peerPort);
  const browser = await chromium.launch();
  const hostContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  const guestContext = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  const screenshotDir = path.join(UI_ROOT, "test-results", "selvala-mulligan-e2e");
  fs.rmSync(screenshotDir, { recursive: true, force: true });
  fs.mkdirSync(screenshotDir, { recursive: true });
  let hostPage = null;
  let guestPage = null;
  let screenshotStep = 0;

  const capture = async (slug) => {
    screenshotStep += 1;
    return captureFullUiStep(screenshotDir, screenshotStep, slug, [
      ["p1-host", hostPage],
      ["p2-guest", guestPage],
    ]);
  };

  const combinedText = async () => `${await visibleBodyText(hostPage)}\n${await visibleBodyText(guestPage)}`;

  const assertClean = async (label) => {
    const text = await combinedText();
    assertNoSyncFailureText(text, label);
    assert.doesNotMatch(
      text,
      /Cheat detected|Invalid priority action ref|Action order mismatch|Missing public_open audit opening|Missing audit material/i,
      label,
    );
  };

  const clickUnselectedButton = async (page, label, textPattern) => {
    const button = page.locator("button:enabled:not(.is-selected)").filter({ hasText: textPattern }).first();
    if ((await button.count()) === 0) return null;
    const text = (await button.innerText({ timeout: 1000 })).replace(/\s+/g, " ").trim();
    await activateButtonNode(button);
    return { label, text };
  };

  const unselectedChoiceTexts = async (page) =>
    page.locator("button:enabled:not(.is-selected)").evaluateAll((buttons) =>
      buttons.map((button) => (button.innerText || button.textContent || "").replace(/\s+/g, " ").trim())
    );

  const visibleHandCardNames = async (page) =>
    page.locator(".hand-card[data-card-name]").evaluateAll((cards) =>
      cards
        .map((card) => String(card.getAttribute("data-card-name") || "").trim())
        .filter(Boolean)
    );

  const battlefieldCardCount = async (page, cardName) =>
    page.locator(".battlefield-row-card[data-card-name]").evaluateAll((cards, name) =>
      cards.filter((card) => String(card.getAttribute("data-card-name") || "") === name).length,
      cardName,
    );

  const stackCardCount = async (page, cardName) =>
    page.locator(".stack-card[data-card-name]").evaluateAll((cards, name) =>
      cards.filter((card) => String(card.getAttribute("data-card-name") || "") === name).length,
      cardName,
    );

  const castableSelvalaSetupHand = async () => {
    const names = await visibleHandCardNames(hostPage);
    const lotusCount = names.filter((name) => /^Black Lotus$/i.test(name)).length;
    const selvalaCount = names.filter((name) => /^Selvala, Explorer Returned$/i.test(name)).length;
    const hiddenCount = names.filter((name) => /^Hidden Card$/i.test(name)).length;
    return {
      names,
      lotusCount,
      selvalaCount,
      hiddenCount,
      castable: lotusCount >= 2 && (selvalaCount + hiddenCount) >= 1,
    };
  };

  const bottomOneCardKeepingCastableHand = async (index) => {
    const choices = await unselectedChoiceTexts(hostPage);
    const lotusCount = choices.filter((text) => /^Black Lotus\b/i.test(text)).length;
    const selvalaCount = choices.filter((text) => /^Selvala, Explorer Returned\b/i.test(text)).length;
    const hiddenCount = choices.filter((text) => /^Hidden Card\b/i.test(text)).length;
    let pattern = null;
    if (selvalaCount > 1) pattern = /^Selvala, Explorer Returned\b/i;
    else if (selvalaCount + hiddenCount > 1 && hiddenCount > 0) pattern = /^Hidden Card\b/i;
    else if (lotusCount > 2) pattern = /^Black Lotus\b/i;
    assert.ok(
      pattern,
      `mulligan redraw must leave at least two Black Lotuses and one Selvala-or-hidden deck card after bottoming\nchoices: ${JSON.stringify(choices, null, 2)}\nbody:\n${await visibleBodyText(hostPage)}`
    );
    const clicked = await clickUnselectedButton(hostPage, `host-bottom-card-${index}`, pattern);
    assert.ok(clicked, `expected to select bottom card ${index}\nchoices: ${JSON.stringify(choices, null, 2)}`);
  };

  const advanceUntil = async (predicate, label, maxSteps = 160) => {
    const advancePattern = /KEEP HAND|PREGAME|BEGIN GAME|CONTINUE|UPKEEP|DRAW|MAIN|COMBAT|M2|END|CLEAN|PASS PRIORITY|RESOLVE|NO ATTACKERS|DECLARE ATTACKERS|DECLARE BLOCKERS|CONFIRM ATTACKERS|CONFIRM BLOCKERS|DONE|SUBMIT/i;
    let lastHostText = "";
    let lastGuestText = "";
    for (let step = 0; step < maxSteps; step += 1) {
      lastHostText = await visibleBodyText(hostPage);
      lastGuestText = await visibleBodyText(guestPage);
      await assertClean(label);
      const hostButtons = await buttonDebugText(hostPage);
      const guestButtons = await buttonDebugText(guestPage);
      if (await predicate({ hostText: lastHostText, guestText: lastGuestText, hostButtons, guestButtons })) {
        return { hostText: lastHostText, guestText: lastGuestText, hostButtons, guestButtons };
      }

      const bottomPromptMatch = lastHostText.match(/Choose (\d+) card\(s\) to put on the bottom of your library/i);
      if (bottomPromptMatch) {
        const required = Number(bottomPromptMatch[1]);
        const submitButton = hostButtons.find((button) => /SUBMIT/i.test(button.text || ""));
        const submitProgress = String(submitButton?.text || "").match(/\((\d+)\/(\d+)\)/);
        const selected = submitProgress ? Number(submitProgress[1]) : 0;
        const target = submitProgress ? Number(submitProgress[2]) : required;
        for (let index = selected + 1; index <= target; index += 1) {
          await bottomOneCardKeepingCastableHand(index);
        }
        await capture("p1-selected-bottom-cards-late");
        const submittedBottom = await clickLocalButton(hostPage, `${label}-host-submit-late-bottom-${step}`, /SUBMIT/i);
        assert.ok(
          submittedBottom,
          `expected late bottom-card decision to be submittable\nbuttons: ${JSON.stringify(await buttonDebugText(hostPage), null, 2)}\nbody:\n${await visibleBodyText(hostPage)}`
        );
        await sleep(3000);
        await capture("p1-bottomed-cards-late");
        continue;
      }

      const hostDiscard = /Discard \d+ card/i.test(lastHostText)
        ? await clickEnabledButton(hostPage, `${label}-host-discard-choice-${step}`, /^(Black Lotus|Selvala, Explorer Returned)$/i)
        : null;
      if (hostDiscard) {
        await sleep(500);
        const submit = await clickLocalButton(hostPage, `${label}-host-submit-discard-${step}`, /SUBMIT/i);
        if (submit) {
          await sleep(2200);
          continue;
        }
      }

      const guestDiscard = /Discard \d+ card/i.test(lastGuestText)
        ? await clickEnabledButton(guestPage, `${label}-guest-discard-choice-${step}`, /^(Black Lotus|Selvala, Explorer Returned)$/i)
        : null;
      if (guestDiscard) {
        await sleep(500);
        const submit = await clickLocalButton(guestPage, `${label}-guest-submit-discard-${step}`, /SUBMIT/i);
        if (submit) {
          await sleep(2200);
          continue;
        }
      }

      const hostAdvanced = await clickLocalButton(hostPage, `${label}-host-advance-${step}`, advancePattern);
      if (hostAdvanced) {
        await sleep(2200);
        continue;
      }
      const guestAdvanced = await clickLocalButton(guestPage, `${label}-guest-advance-${step}`, advancePattern);
      if (guestAdvanced) {
        await sleep(2200);
        continue;
      }
      await sleep(1000);
    }
    const debug = await Promise.all([
      fullUiPageDebug(hostPage, 0),
      fullUiPageDebug(guestPage, 1),
    ]);
    assert.fail(`${label}\n${JSON.stringify(debug, null, 2)}`);
  };

  const waitForStackCardToResolve = async (cardName, expectedBattlefieldCount, label, maxSteps = 90) => {
    for (let step = 0; step < maxSteps; step += 1) {
      await assertClean(label);
      const hostText = await visibleBodyText(hostPage);
      const hostBattlefieldCount = await battlefieldCardCount(hostPage, cardName);
      const hostStackCount = await stackCardCount(hostPage, cardName);
      const guestStackCount = await stackCardCount(guestPage, cardName);
      const expectedBattlefieldSummary = new RegExp(`BF\\s*${expectedBattlefieldCount}\\s*HAND`, "i").test(hostText);
      if (
        (hostBattlefieldCount >= expectedBattlefieldCount || expectedBattlefieldSummary)
        && hostStackCount === 0
        && guestStackCount === 0
      ) {
        return;
      }
      const hostPass = await clickLocalButton(hostPage, `${label}-host-pass-${step}`, /PASS PRIORITY|RESOLVE/i);
      if (hostPass) {
        await sleep(2200);
        continue;
      }
      const guestPass = await clickLocalButton(guestPage, `${label}-guest-pass-${step}`, /PASS PRIORITY|RESOLVE/i);
      if (guestPass) {
        await sleep(2200);
        continue;
      }
      await sleep(1000);
    }
    const debug = await Promise.all([
      fullUiPageDebug(hostPage, 0),
      fullUiPageDebug(guestPage, 1),
    ]);
    assert.fail(`${label}\n${JSON.stringify(debug, null, 2)}`);
  };

  const castBlackLotus = async (ordinal) => {
    await advanceUntil(
      async () => hasLocalButton(hostPage, /CAST BLACK LOTUS/i),
      `expected Player 1 to have Black Lotus ${ordinal} castable`,
      180,
    );
    const beforeCastSequence = Number((await fullUiSnapshot(hostPage))?.multiplayer?.lastAppliedSequence || 0);
    await waitAndClickLocalButton(hostPage, `host-cast-black-lotus-${ordinal}`, /CAST BLACK LOTUS/i, 120000);
    await waitForFullUiSequenceAdvance(
      hostPage,
      guestPage,
      beforeCastSequence,
      `Black Lotus ${ordinal} cast should sync before resolution`,
    );
    await waitForStackCardToResolve("Black Lotus", ordinal, `resolve Black Lotus ${ordinal}`);
    await waitForFullUiSync(
      hostPage,
      guestPage,
      `Black Lotus ${ordinal} resolution should leave peers synced`,
      120000,
    );
    await assertClean(`casting Black Lotus ${ordinal} should not sync-fail`);
  };

  const activateLotusFor = async (colorName, colorPattern, expectedBattlefieldCount) => {
    for (let attempt = 0; attempt < 5; attempt += 1) {
      const beforeActivationSequence = Number((await fullUiSnapshot(hostPage))?.multiplayer?.lastAppliedSequence || 0);
      const text = await visibleBodyText(hostPage);
      const colorPromptOpen = /CHOOSE COLOR|Choose a color/i.test(text);
      if (colorPromptOpen) {
        await waitAndClickEnabledButton(hostPage, `host-choose-${colorName}-${attempt}`, colorPattern, 60000);
        await sleep(1800);
        if (
          !(await visibleBodyText(hostPage)).match(/CHOOSE COLOR|Choose a color/i)
          && await battlefieldCardCount(hostPage, "Black Lotus") <= expectedBattlefieldCount
        ) {
          await waitForFullUiSequenceAdvance(
            hostPage,
            guestPage,
            beforeActivationSequence,
            `Black Lotus activation for ${colorName} should sync`,
          );
          return;
        }
        continue;
      }

      await advanceUntil(
        async () =>
          await stackCardCount(hostPage, "Black Lotus") === 0
          && await stackCardCount(guestPage, "Black Lotus") === 0
          && await hasLocalButton(hostPage, /SACRIFICE|ADD THREE MANA|ADD/i),
        `expected a resolved Black Lotus to be activatable for ${colorName}`,
        140,
      );
      await waitAndClickLocalButton(hostPage, `host-activate-lotus-for-${colorName}-${attempt}`, /SACRIFICE|ADD THREE MANA|ADD/i, 120000);
      await waitAndClickEnabledButton(hostPage, `host-choose-${colorName}-${attempt}`, colorPattern, 60000);
      await sleep(1800);
      if (
        !(await visibleBodyText(hostPage)).match(/CHOOSE COLOR|Choose a color/i)
        && await battlefieldCardCount(hostPage, "Black Lotus") <= expectedBattlefieldCount
      ) {
        await waitForFullUiSequenceAdvance(
          hostPage,
          guestPage,
          beforeActivationSequence,
          `Black Lotus activation for ${colorName} should sync`,
        );
        return;
      }
    }
    assert.fail(
      `expected Black Lotus activation for ${colorName} to finish\nbuttons: ${JSON.stringify(await buttonDebugText(hostPage), null, 2)}\nbody:\n${await visibleBodyText(hostPage)}`
    );
  };

  const paySelvalaCost = async () => {
    const paymentPatterns = [
      /Use\s+from mana pool/i,
      /GREEN|\{G\}/i,
      /WHITE|\{W\}/i,
      /GENERIC|\{1\}|MANA|PAY|SUBMIT/i,
      /^CAST$/i,
    ];
    for (let step = 0; step < 40; step += 1) {
      const text = await visibleBodyText(hostPage);
      await assertClean("Selvala payment should stay synced");
      const paymentPending = /Pay \{|remaining|Use\s+from mana pool|CHOOSE OPTION/i.test(text);
      if (!paymentPending && await stackCardCount(hostPage, "Selvala, Explorer Returned") > 0) return;
      for (const pattern of paymentPatterns) {
        const clicked = await clickEnabledButton(hostPage, `host-pay-selvala-${step}`, pattern);
        if (clicked) {
          await sleep(800);
          break;
        }
      }
      const nextText = await visibleBodyText(hostPage);
      const nextPaymentPending = /Pay \{|remaining|Use\s+from mana pool|CHOOSE OPTION/i.test(nextText);
      if (!nextPaymentPending && await stackCardCount(hostPage, "Selvala, Explorer Returned") > 0) return;
      await sleep(500);
    }
    assert.fail(
      `expected Selvala payment to put Selvala on the stack\nbuttons: ${JSON.stringify(await buttonDebugText(hostPage), null, 2)}\nbody:\n${await visibleBodyText(hostPage)}`
    );
  };

  const activateSelvalaAndAcknowledgeReveal = async (slug, expectedPhasePattern = null) => {
    await waitAndClickLocalButton(
      hostPage,
      `host-activate-selvala-${slug}`,
      /Each player reveals the top card of their library/i,
      120000,
    );
    await sleep(4000);
    await Promise.all([
      waitForVisibleBodyText(
        hostPage,
        /REVEALED[\s\S]*(Black Lotus|Selvala, Explorer Returned)/i,
        `expected host to see Selvala reveal during ${slug}`,
        120000,
      ),
      waitForVisibleBodyText(
        guestPage,
        /REVEALED[\s\S]*(Black Lotus|Selvala, Explorer Returned)/i,
        `expected guest to see Selvala reveal during ${slug}`,
        120000,
      ),
    ]);
    await capture(`${slug}-reveal-visible`);

    const guestRevealText = await visibleBodyText(guestPage);
    assert.doesNotMatch(
      guestRevealText,
      /REVEALED[\s\S]{0,260}Hidden Card/i,
      `guest reveal strip should show opened card names during ${slug}`,
    );
    await assertClean(`Selvala ${slug} reveal should not desync`);

    await clickEnabledButton(hostPage, `host-done-${slug}`, /^DONE$/i);
    await sleep(1200);
    await clickEnabledButton(guestPage, `guest-done-${slug}`, /^DONE$/i);
    await sleep(3000);
    await capture(`${slug}-after-done`);
    const afterDoneHost = await visibleBodyText(hostPage);
    await assertClean(`Selvala ${slug} Done should not desync or auto-pass incorrectly`);
    if (expectedPhasePattern) {
      assert.match(
        afterDoneHost,
        expectedPhasePattern,
        `expected ${slug} Done to leave priority in the expected phase\n${afterDoneHost}`,
      );
    }
  };

  try {
    const deck = deckUrlParam("30 Black Lotus\n30 Selvala, Explorer Returned");
    hostPage = await openFullUiPage(
      hostContext,
      `${baseUrl}/?name=Alice P1&deck=${deck}`,
      "host-selvala-mulligan-ui",
    );
    await hostPage.getByText("CREATE LOBBY").first().waitFor({ timeout: 30000 });
    await capture("host-load");
    await hostPage.getByRole("button").filter({ hasText: /CREATE LOBBY/i }).first().click();
    await hostPage.getByText("Host or join").waitFor({ timeout: 10000 });
    await hostPage.getByRole("button").filter({ hasText: /CREATE LOBBY/i }).last().click();
    await hostPage.getByText("Share this code").waitFor({ timeout: 40000 });
    await capture("host-created-lobby");

    const lobbyCode = (await visibleBodyText(hostPage)).match(
      /[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/i
    )?.[0];
    assert.ok(lobbyCode, `expected the full UI to create a lobby code\n${await visibleBodyText(hostPage)}`);

    guestPage = await openFullUiPage(
      guestContext,
      `${baseUrl}/?lobby=${encodeURIComponent(lobbyCode)}&name=Alice P2&deck=${deck}`,
      "guest-selvala-mulligan-ui",
    );
    await hostPage.getByText("All players are ready").first().waitFor({ timeout: 70000 });
    await guestPage.getByText("All players are ready").first().waitFor({ timeout: 70000 });
    await capture("guest-joined-lobby");

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
    await capture("opening-hand");
    await assertClean("match start should not sync-fail before Selvala mulligans");

    await waitAndClickLocalButton(hostPage, "host-first-mulligan", /Mulligan/i, 120000);
    await sleep(3000);
    await capture("p1-first-mulligan");
    await assertClean("host first mulligan should not sync-fail");

    await waitAndClickLocalButton(guestPage, "guest-keep-after-host-first-mulligan", /KEEP HAND/i, 120000);
    await sleep(3000);
    await capture("p2-kept-opening-hand");
    await assertClean("guest keep after host mulligan should not sync-fail");

    await waitAndClickLocalButton(hostPage, "host-second-mulligan", /Mulligan/i, 120000);
    await sleep(5000);
    await capture("p1-second-mulligan");
    await assertClean("host second mulligan should not sync-fail");

    let hostMulliganCount = 2;
    let setupHand = await castableSelvalaSetupHand();
    while (!setupHand.castable && hostMulliganCount < 10) {
      await waitAndClickLocalButton(
        hostPage,
        `host-extra-mulligan-${hostMulliganCount + 1}-for-selvala-setup`,
        /Mulligan/i,
        120000,
      );
      hostMulliganCount += 1;
      await sleep(5000);
      await capture(`p1-mulligan-${hostMulliganCount}-for-selvala-setup`);
      await assertClean(`host mulligan ${hostMulliganCount} should not sync-fail`);
      setupHand = await castableSelvalaSetupHand();
    }
    assert.ok(
      setupHand.castable,
      `expected a mulligan hand with at least two Black Lotuses and one Selvala-or-hidden deck card before keeping\nhand: ${JSON.stringify(setupHand.names, null, 2)}`
    );

    await waitAndClickLocalButton(hostPage, "host-keep-after-selvala-setup-hand", /KEEP HAND/i, 120000);
    await sleep(2500);
    await capture(`p1-kept-after-${hostMulliganCount}-mulligans`);
    await assertClean("host keep after mulligans should not sync-fail");

    await sleep(7000);
    const afterKeepFlowText = await waitForVisibleBodyText(
      hostPage,
      /Choose \d+ card\(s\) to put on the bottom of your library|BEGINNING|UNTAP|DRAW|MAIN|CAST BLACK LOTUS/i,
      "expected Player 1 to either bottom cards or advance into the game after mulligans",
      120000,
    );
    const bottomPromptMatch = afterKeepFlowText.match(/Choose (\d+) card\(s\) to put on the bottom of your library/i);
    if (bottomPromptMatch) {
      const bottomCount = Number(bottomPromptMatch[1]);
      assert.ok(Number.isFinite(bottomCount) && bottomCount >= 0, `invalid bottom prompt: ${bottomPromptMatch[0]}`);
      for (let index = 1; index <= bottomCount; index += 1) {
        await bottomOneCardKeepingCastableHand(index);
      }
      await capture("p1-selected-bottom-cards");
      await waitAndClickLocalButton(hostPage, "host-submit-bottom-cards", /SUBMIT/i, 60000);
      await sleep(4000);
      await capture("p1-bottomed-cards");
      await assertClean("bottoming cards after mulligans should not sync-fail");
    } else {
      await capture("p1-advanced-without-bottom-prompt");
      await assertClean("advancing after mulligans without a bottom prompt should not sync-fail");
    }

    await advanceUntil(
      async () => hasLocalButton(hostPage, /CAST BLACK LOTUS/i),
      "expected Player 1 to reach a main phase with Black Lotus castable",
      160,
    );
    await capture("p1-main-ready-to-cast-lotus");

    await castBlackLotus(1);
    await capture("first-black-lotus-resolved");
    await castBlackLotus(2);
    await capture("second-black-lotus-resolved");

    await activateLotusFor("green", /GREEN|\{G\}/i, 1);
    await capture("first-lotus-added-green");
    await activateLotusFor("white", /WHITE|\{W\}/i, 0);
    await capture("second-lotus-added-white");

    const beforeSelvalaCastSequence = Number((await fullUiSnapshot(hostPage))?.multiplayer?.lastAppliedSequence || 0);
    await waitAndClickLocalButton(hostPage, "host-cast-selvala", /CAST SELVALA, EXPLORER RETURNED/i, 120000);
    await waitForFullUiSequenceAdvance(
      hostPage,
      guestPage,
      beforeSelvalaCastSequence,
      "Selvala cast should sync before payment",
      120000,
    );
    await paySelvalaCost();
    await capture("selvala-on-stack");
    await waitForStackCardToResolve("Selvala, Explorer Returned", 1, "resolve Selvala", 120);
    await advanceUntil(
      async ({ hostText }) => /BF\s*1/i.test(hostText) && /Selvala, Explorer Returned/i.test(hostText),
      "expected Selvala to resolve to Player 1 battlefield",
      80,
    );
    await capture("selvala-on-battlefield");

    await advanceUntil(
      async ({ hostText }) =>
        /UPKEEP/i.test(hostText)
        && /ACTIVE\s+ALICE P1/i.test(hostText)
        && await hasLocalButton(hostPage, /Each player reveals the top card of their library/i),
      "expected Selvala to be activatable during Player 1 upkeep after a full turn",
      260,
    );
    await capture("selvala-ready-in-upkeep");
    await activateSelvalaAndAcknowledgeReveal(
      "upkeep-selvala",
      /UPKEEP[\s\S]*ACTIVE\s+ALICE P1[\s\S]*PRIORITY\s+ALICE P1/i,
    );
    await capture("final-after-upkeep-selvala");

    const finalCombinedText = await combinedText();
    assertNoSyncFailureText(finalCombinedText, "Selvala full UI ziffle reveal flow should stay synced");
    assert.doesNotMatch(finalCombinedText, /Hidden Card commitment does not match reveal|Ziffle card opening proof/i);
    assertNoPageErrors(hostPage, guestPage);
    console.log(`SELVALA_MULLIGAN_E2E_SCREENSHOTS=${screenshotDir}`);
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
    const stackCardCount = async (page, cardName) =>
      page.locator(".stack-card[data-card-name]").evaluateAll((cards, name) =>
        cards.filter((card) => String(card.getAttribute("data-card-name") || "") === name).length,
        cardName,
      );
    const finishTutorSearchIfVisible = async () => {
      const searchText = await visibleBodyText(hostPage);
      if (!/(?:CHOOSE OBJECTS[\s\S]*)?Search library[\s\S]*Mystical Tutor/i.test(searchText)) {
        return false;
      }
      guestResolved = true;
      assertNoSyncFailureText(searchText, "search decision should not sync-fail");
      await waitForFullUiPair(
        hostPage,
        guestPage,
        (host, guest) =>
          Number(host?.multiplayer?.lastAppliedSequence || 0)
            === Number(guest?.multiplayer?.lastAppliedSequence || 0)
          && host?.state?.decision?.kind === "select_objects"
          && guest?.state?.decision?.kind === "select_objects",
        "expected Mystical Tutor search decision to be synced before choosing",
        60000,
      );

      const choice = await hostPage.evaluate(() => {
        const snap = window.__ironsmithE2E?.snapshot?.();
        const candidate = (snap?.state?.decisionCandidates || []).find((entry) =>
          entry.legal !== false && /^Mystical Tutor$/i.test(String(entry.name || ""))
        );
        return candidate?.id == null ? null : { id: Number(candidate.id) };
      });
      assert.ok(
        choice?.id != null,
        `expected a Mystical Tutor choice in the searched library\nbuttons: ${JSON.stringify(await buttonDebugText(hostPage), null, 2)}\nbody:\n${await visibleBodyText(hostPage)}`
      );
      return true;
    };

    for (let step = 0; step < 120; step += 1) {
      const combinedText = `${await visibleBodyText(hostPage)}\n${await visibleBodyText(guestPage)}`;
      assertNoSyncFailureText(combinedText);

      if (!playedIsland && await clickLocalButton(hostPage, "host-play-island", /PLAY ISLAND/i)) {
        playedIsland = true;
        await sleep(3000);
        continue;
      }

      if (playedIsland && !castTutor) {
        const hostText = await visibleBodyText(hostPage);
        if (
          /Pay [\s\S]*Mystical Tutor|CHOOSE OPTION/i.test(hostText)
          || await stackCardCount(hostPage, "Mystical Tutor") > 0
        ) {
          castTutor = true;
          continue;
        }
        if (await clickLocalButton(hostPage, "host-cast-tutor", /MYSTICAL TUTOR/i)) {
          await sleep(3000);
          continue;
        }
      }

      if (castTutor && !paidTutor) {
        const tutorPaymentPending = /Pay [\s\S]*Mystical Tutor|CHOOSE OPTION|remaining|Use\s+from mana pool/i.test(await visibleBodyText(hostPage));
        if (!tutorPaymentPending && await stackCardCount(hostPage, "Mystical Tutor") > 0) {
          paidTutor = true;
          await sleep(1500);
          continue;
        }
        const paid = await clickEnabledButton(hostPage, "host-pay-tutor", /Use\s+from mana pool|PAY|MANA|BLUE|ISLAND|\{U\}|U$|^CAST$/i)
          || await clickLocalButton(hostPage, "host-tap-island", /TAP ISLAND|ADD/i);
        if (paid) {
          await sleep(6000);
          continue;
        }
      }

      if (paidTutor && !guestResolved) {
        if (await finishTutorSearchIfVisible()) {
          break;
        }
        const hostPass = await clickLocalButton(hostPage, "host-pass-tutor", /PASS PRIORITY|RESOLVE/i);
        if (hostPass) {
          await sleep(2500);
          continue;
        }
        const guestPass = await clickLocalButton(guestPage, "guest-resolve-tutor", /RESOLVE|PASS/i);
        if (guestPass) {
          guestResolved = true;
          const searchText = await waitForVisibleBodyText(
            hostPage,
            /(?:CHOOSE OBJECTS[\s\S]*)?Search library[\s\S]*Mystical Tutor/i,
            "expected Mystical Tutor to create a searchable library decision",
            90000,
          );
          assert.ok(searchText);
          await finishTutorSearchIfVisible();
          break;
        }
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
    assert.equal(
      guestResolved,
      true,
      `expected guest to resolve Mystical Tutor\nhost buttons: ${JSON.stringify(await buttonDebugText(hostPage), null, 2)}\nguest buttons: ${JSON.stringify(await buttonDebugText(guestPage), null, 2)}\nhost body:\n${await visibleBodyText(hostPage)}\nguest body:\n${await visibleBodyText(guestPage)}`
    );
    assertNoSyncFailureText(
      `${await visibleBodyText(hostPage)}\n${await visibleBodyText(guestPage)}`,
      "Mystical Tutor should reach the searchable library choice without desync"
    );
    return;

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
    const stackCardCount = async (page, cardName) =>
      page.locator(".stack-card[data-card-name]").evaluateAll((cards, name) =>
        cards.filter((card) => String(card.getAttribute("data-card-name") || "") === name).length,
        cardName,
      );

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
        if (/CHOOSE COLOR|Choose a color/i.test(hostText)) {
          const blackOption = await hostPage.evaluate(() => {
            const snap = window.__ironsmithE2E?.snapshot?.();
            return (snap?.state?.decisionOptions || []).find((option) =>
              option.legal !== false && /^\s*Black\s*$/i.test(String(option.description || ""))
            ) || null;
          });
          assert.ok(
            blackOption,
            `expected a legal Black color option\nbuttons: ${JSON.stringify(await buttonDebugText(hostPage), null, 2)}\nbody:\n${await visibleBodyText(hostPage)}`
          );
          await hostPage.evaluate(({ optionIndex }) => (
            window.__ironsmithE2E.submitMultiplayerCommand(
              { type: "select_options", option_indices: [Number(optionIndex)] },
              "Chose black"
            )
          ), { optionIndex: blackOption.index });
          choseBlack = true;
          await sleep(2500);
          continue;
        }
        await sleep(1000);
        continue;
      }

      if (choseBlack && !castPact) {
        if (
          /Pay [\s\S]*Tainted Pact|CHOOSE OPTION/i.test(hostText)
          || await stackCardCount(hostPage, "Tainted Pact") > 0
          || await stackCardCount(guestPage, "Tainted Pact") > 0
        ) {
          castPact = true;
          continue;
        }
        const result = await clickLocalButton(hostPage, "host-cast-pact", /TAINTED PACT/i);
        if (result) {
          await sleep(2500);
          continue;
        }
      }

      if (castPact) {
        const pactPaymentPending = /Pay [\s\S]*Tainted Pact|CHOOSE OPTION|remaining|Use\s+from mana pool/i.test(hostText);
        if (pactPaymentPending) {
          const selectedPayment = await clickEnabledButton(
            hostPage,
            "host-pay-pact",
            /Use\s+from mana pool|BLACK|GENERIC|MANA|\{B\}|\{1\}|SUBMIT|PAY|^CAST$/i,
          );
          if (selectedPayment) {
            await sleep(2500);
            continue;
          }
        }
        if (
          await stackCardCount(hostPage, "Tainted Pact") > 0
          || await stackCardCount(guestPage, "Tainted Pact") > 0
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
