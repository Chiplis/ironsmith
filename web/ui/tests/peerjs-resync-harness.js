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
export { assert, chromium, fs, path, test };

export const __dirname = path.dirname(fileURLToPath(import.meta.url));
export const UI_ROOT = path.resolve(__dirname, "..");
export const HARNESS_PATH = "/tests/fixtures/peer-lobby-harness.html";
export const HOST_DECK = "60 Island";
export const GUEST_DECK = "60 Mountain";
export const FULL_UI_RECONNECT_DISCONNECT_MS = 15250;
export const HOST_ZIFFLE_PUBLIC_OPEN_DECK = "7 Island\n1 Mystical Tutor\n52 Mountain";
export const HOST_ZIFFLE_OPENED_LAND_DECK = "7 Mountain\n1 Island\n5 Mountain\n1 Mystical Tutor\n46 Mountain";
export const FOUR_PLAYER_DECKS = [
  "60 Island",
  "60 Mountain",
  "60 Forest",
  "60 Plains",
];

export const nodeTestKeepAlive = setInterval(() => {}, 1000);
test.after(() => {
  clearInterval(nodeTestKeepAlive);
});

export const testKeepAlives = new WeakMap();
test.beforeEach((context) => {
  const keepAlive = setInterval(() => {}, 1000);
  testKeepAlives.set(context, keepAlive);
});
test.afterEach((context) => {
  const keepAlive = testKeepAlives.get(context);
  if (keepAlive) {
    clearInterval(keepAlive);
  }
  testKeepAlives.delete(context);
});

export function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

export async function withTimeout(promise, ms) {
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

export async function withRejectingTimeout(promise, ms, label) {
  let timer = null;
  try {
    return await Promise.race([
      promise,
      new Promise((_, reject) => {
        timer = setTimeout(
          () => reject(new Error(`${label} timed out after ${ms}ms`)),
          ms,
        );
      }),
    ]);
  } finally {
    if (timer) clearTimeout(timer);
  }
}

export async function freePort() {
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

export async function startPeerServer(port) {
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

export async function closePeerServer(child) {
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

export async function startHarnessServer(peerPort, envOverrides = {}) {
  const vitePort = await freePort();
  const harnessEnv = {
    VITE_PEER_HOST: "127.0.0.1",
    VITE_PEER_PORT: String(peerPort),
    VITE_PEER_PATH: "/peerjs",
    VITE_PEER_KEY: "peerjs",
    VITE_PEER_SECURE: "false",
    VITE_PEER_HEARTBEAT_INTERVAL_MS: "500",
    VITE_PEER_HEARTBEAT_TIMEOUT_MS: "2000",
    VITE_E2E_TEST: "true",
    ...envOverrides,
  };
  const previousEnv = new Map(
    Object.keys(harnessEnv).map((key) => [key, process.env[key]])
  );
  for (const [key, value] of Object.entries(harnessEnv)) {
    process.env[key] = String(value);
  }

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
  const closeVite = vite.close.bind(vite);
  vite.close = async (...args) => {
    try {
      return await closeVite(...args);
    } finally {
      for (const [key, value] of previousEnv.entries()) {
        if (value == null) {
          delete process.env[key];
        } else {
          process.env[key] = value;
        }
      }
    }
  };
  await vite.listen();
  return {
    vite,
    baseUrl: `http://127.0.0.1:${vitePort}`,
  };
}

export async function openHarness(context, baseUrl, label) {
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
    if (pageConsole.length > 600) pageConsole.shift();
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

export async function snapshot(page) {
  return page.evaluate(() => window.__peerHarness.snapshot());
}

export function compactSnapshotForFailure(snap) {
  if (!snap || typeof snap !== "object") return snap;
  const multiplayer = snap.multiplayer || {};
  const visibleState = snap.visibleState || {};
  const transcript = snap.auditTranscript || snap.liveAuditTranscript || {};
  return {
    multiplayer: {
      role: multiplayer.role,
      mode: multiplayer.mode,
      lobbyId: multiplayer.lobbyId,
      hostPeerId: multiplayer.hostPeerId,
      localPeerId: multiplayer.localPeerId,
      localPlayerIndex: multiplayer.localPlayerIndex,
      matchStarted: multiplayer.matchStarted,
      lastAppliedSequence: multiplayer.lastAppliedSequence,
      submittingAction: multiplayer.submittingAction,
      players: (multiplayer.players || []).map((player) => ({
        index: player.index,
        name: player.name,
        peerId: player.peerId,
        currentPeerId: player.currentPeerId,
        routePeerId: player.routePeerId,
        connected: player.connected,
        ready: player.ready,
      })),
    },
    visibleState: {
      snapshot_id: visibleState.snapshot_id,
      perspective: visibleState.perspective,
      phase: visibleState.phase,
      active_player: visibleState.active_player,
      priority_player: visibleState.priority_player,
      decision: visibleState.decision
        ? {
            type: visibleState.decision.type,
            player: visibleState.decision.player,
            actionCount: Array.isArray(visibleState.decision.actions)
              ? visibleState.decision.actions.length
              : undefined,
          }
        : null,
      players: (visibleState.players || []).map((player) => ({
        index: player.index,
        battlefield: Array.isArray(player.battlefield) ? player.battlefield.length : undefined,
        hand: Array.isArray(player.hand) ? player.hand.length : undefined,
        library: Array.isArray(player.library) ? player.library.length : undefined,
        graveyard: Array.isArray(player.graveyard) ? player.graveyard.length : undefined,
        exile: Array.isArray(player.exile) ? player.exile.length : undefined,
      })),
    },
    statusEvents: (snap.statusEvents || []).slice(-20).map((event) => event.message || event),
    syncEvents: (snap.syncEvents || []).slice(-20).map((event) => ({
      type: event.type,
      sequence: event.syncContext?.sequence ?? event.sequence,
      commandType: event.command?.type,
      message: event.message,
    })),
    transcriptActions: (transcript.actions || []).slice(-5).map((action) => ({
      seq: action.seq,
      actorIndex: action.actorIndex,
      commandType: action.command?.type,
      label: action.label,
    })),
    transcriptOutcome: transcript.outcome || null,
    perfEvents: (snap.perfEvents || []).slice(-50).map((event) => ({
      label: event.label,
      seq: event.payload?.seq,
      actor: event.payload?.actor,
      target_peer_id: event.payload?.target_peer_id,
      target_player_index: event.payload?.target_player_index,
      sent: event.payload?.sent,
      route: event.payload?.route,
      error: event.payload?.error,
    })),
  };
}

export async function waitForSnapshot(page, predicate, label, timeoutMs = 20000) {
  const started = Date.now();
  let lastSnapshot = null;
  while (Date.now() - started < timeoutMs) {
    lastSnapshot = await snapshot(page);
    if (predicate(lastSnapshot)) return lastSnapshot;
    await sleep(100);
  }
  assert.fail(`${label}\nLast snapshot: ${JSON.stringify(compactSnapshotForFailure(lastSnapshot), null, 2)}`);
}

export function checkpointImportEvents(snap) {
  return (snap.syncEvents || []).filter((event) => event.type === "sync_checkpoint_import");
}

export function syncedCommandEvents(snap) {
  return (snap.syncEvents || []).filter((event) => event.type === "synced_command");
}

export function assertNoPageErrors(...pages) {
  for (const page of pages) {
    const errors = page?.__peerHarnessErrors || [];
    assert.deepEqual(errors, [], `${page?.__peerHarnessLabel || "page"} had browser errors`);
  }
}

export function deckUrlParam(deckText) {
  return Buffer.from(String(deckText || ""), "utf8").toString("base64url");
}

export async function openFullUiPage(context, url, label) {
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
    if (pageConsole.length > 600) pageConsole.shift();
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
  await page.goto(url, { waitUntil: "domcontentloaded", timeout: 60000 });
  page.__peerHarnessErrors = pageErrors;
  page.__peerHarnessConsole = pageConsole;
  page.__peerHarnessLabel = label;
  return page;
}

export async function fullUiSnapshot(page) {
  const snapshot = await withTimeout(
    page.evaluate(() => window.__ironsmithE2E?.snapshot?.() || null).catch(() => null),
    5000,
  );
  return snapshot || null;
}

export async function waitForFullUiSnapshot(page, predicate, label, timeoutMs = 60000) {
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
  let dom = null;
  try {
    dom = await page.evaluate(() => ({
      readyState: document.readyState,
      title: document.title,
      hasHarness: Boolean(window.__ironsmithE2E?.snapshot),
      bodyHtml: String(document.body?.innerHTML || "").slice(0, 2000),
      rootHtml: String(document.querySelector("#root")?.innerHTML || "").slice(0, 2000),
    }));
  } catch (err) {
    dom = { error: String(err?.message || err) };
  }
  assert.fail(`${label}\nLast snapshot: ${JSON.stringify(lastSnapshot, null, 2)}\nurl: ${page.url()}\nerrors: ${JSON.stringify(page.__peerHarnessErrors || [], null, 2)}\nconsole: ${JSON.stringify((page.__peerHarnessConsole || []).slice(-80), null, 2)}\ndom: ${JSON.stringify(dom, null, 2)}\nbody:\n${body}`);
}

export async function waitForFullUiSync(hostPage, guestPage, label, timeoutMs = 60000) {
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

export async function waitForFullUiPair(hostPage, guestPage, predicate, label, timeoutMs = 60000) {
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
  const summarize = (snap) => ({
    status: snap?.status,
    role: snap?.multiplayer?.role,
    localPlayerIndex: snap?.multiplayer?.localPlayerIndex,
    lastAppliedSequence: snap?.multiplayer?.lastAppliedSequence,
    matchStarted: snap?.multiplayer?.matchStarted,
    submittingAction: snap?.multiplayer?.submittingAction,
    players: snap?.multiplayer?.players?.map((player) => ({
      index: player.index,
      name: player.name,
      peerId: player.peerId,
      currentPeerId: player.currentPeerId,
      connected: player.connected,
    })),
    decision: snap?.state?.decision,
    handSizes: snap?.state?.players?.map((player) => player.hand_size),
    librarySizes: snap?.state?.players?.map((player) => player.library_size),
    lastPerfEvents: snap?.perfEvents?.slice(-12),
  });
  const checkpointSummary = async (page) => page.evaluate(async () => {
    const checkpoint = await window.__ironsmithE2E?.publicCheckpoint?.();
    const syncCheckpoint = await window.__ironsmithE2E?.checkpoint?.();
    const objectsById = new Map((syncCheckpoint?.objects || []).map((object) => [
      Number(object.id),
      object,
    ]));
    const handEntries = (syncCheckpoint?.players || []).flatMap((player) =>
      (player.hand || []).map((objectId, index) => {
        const object = objectsById.get(Number(objectId)) || {};
        const hidden = object.hiddenCard || object.hidden_card || {};
        return {
          owner: player.id,
          index,
          objectId,
          name: object.name,
          slot: hidden.slot,
          publicSlot: hidden.publicSlot ?? hidden.public_slot,
          commitment: hidden.commitment,
          publicCommitment: hidden.publicCommitment ?? hidden.public_commitment,
        };
      })
    );
    return {
      hiddenZones: (checkpoint?.hidden_zones || checkpoint?.hiddenZones || []).map((zone) => ({
        owner: zone.owner,
        zone: zone.zone,
        count: zone.count,
        commitmentRoot: zone.commitmentRoot ?? zone.commitment_root,
      })),
      battlefield: checkpoint?.battlefield || [],
      publicExile: checkpoint?.publicExile || checkpoint?.public_exile || [],
      stack: checkpoint?.stack || [],
      objects: (checkpoint?.objects || []).map((object) => ({
        id: object.id,
        stableId: object.stableId ?? object.stable_id,
        owner: object.owner,
        controller: object.controller,
        zone: object.zone,
        name: object.identity?.name || null,
      })),
      handEntries,
    };
  }).catch((err) => ({ error: String(err?.message || err) }));
  const [hostCheckpoint, guestCheckpoint] = await Promise.all([
    checkpointSummary(hostPage),
    checkpointSummary(guestPage),
  ]);
  assert.fail(
    `${label}`
    + `\nhost(${hostPage.__peerHarnessLabel || "host"}): ${JSON.stringify(summarize(lastHost), null, 2)}`
    + `\nhostCheckpoint: ${JSON.stringify(hostCheckpoint, null, 2)}`
    + `\nguest(${guestPage.__peerHarnessLabel || "guest"}): ${JSON.stringify(summarize(lastGuest), null, 2)}`
    + `\nguestCheckpoint: ${JSON.stringify(guestCheckpoint, null, 2)}`
  );
}

export async function waitForFullUiSequenceAdvance(hostPage, guestPage, beforeSequence, label, timeoutMs = 120000) {
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

export async function assertNoFullUiSyncFailures(...pages) {
  const text = (await Promise.all(
    pages.filter(Boolean).map((page) => visibleBodyText(page).catch(() => ""))
  )).join("\n");
  assertNoSyncFailureText(text);
}

export async function assertNoFullUiSyncFailuresWithDebug(label, ...pages) {
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

export async function clickLocalButton(page, label, textPattern = null) {
  let button = page.locator('button:visible[data-local-action="true"]:enabled:not([aria-disabled="true"])');
  if (textPattern) {
    button = button.filter({ hasText: textPattern });
  }
  button = button.first();
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

export async function activateLocalButton(page, label, textPattern) {
  const button = page.locator('button:visible[data-local-action="true"]:enabled:not([aria-disabled="true"])').filter({ hasText: textPattern }).first();
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

export async function activateButtonNode(button) {
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

export async function clickEnabledButton(page, label, textPattern) {
  const button = page.locator('button:visible:enabled:not([aria-disabled="true"])').filter({ hasText: textPattern }).first();
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

export async function clickUnselectedEnabledButton(page, label, textPattern) {
  const button = page.locator("button:enabled:not(.is-selected)").filter({ hasText: textPattern }).first();
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

export async function clickLastEnabledButton(page, label, textPattern) {
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

export async function clickLocalDecisionButton(page, label) {
  return clickLocalButton(page, label);
}

export async function visibleBodyText(page) {
  const text = await withTimeout(
    page.locator("body").innerText({ timeout: 10000 }).catch(() => ""),
    15000,
  );
  return String(text || "");
}

export async function buttonDebugText(page) {
  const buttons = await withTimeout(
    page.locator("button").evaluateAll((nodes) =>
      nodes.map((button) => ({
        text: (button.innerText || button.textContent || "").replace(/\s+/g, " ").trim(),
        disabled: button.disabled,
        ariaDisabled: button.getAttribute("aria-disabled"),
        localAction: button.getAttribute("data-local-action"),
      }))
    ).catch(() => []),
    5000,
  );
  return Array.isArray(buttons) ? buttons : [];
}

export async function fullUiPageDebug(page, index = 0) {
  const auditTranscript = await page.evaluate(async () => {
    const transcript = await window.__ironsmithE2E?.auditTranscript?.();
    const actions = Array.isArray(transcript?.actions) ? transcript.actions : [];
    return {
      actionCount: actions.length,
      actions: actions.map((action) => ({
        seq: action.seq,
        actor: action.actor,
        commandType: action.command?.type,
        commandKind: action.command?.kind,
        shuffleProofs: (action.audit?.shuffleProofs || []).map((proof) => ({
          owner: proof.owner,
          deckHash: proof.deckHash,
          beforeLen: (proof.beforeOrder || proof.before_order || []).length,
          afterLen: (proof.afterOrder || proof.after_order || []).length,
          epoch: proof.epoch,
        })),
        openings: (action.audit?.openings || []).map((opening) => ({
          owner: opening.owner,
          slot: opening.slot,
          objectId: opening.objectId ?? opening.object_id ?? null,
          shuffleObjectId: opening.shuffleObjectId ?? opening.shuffle_object_id ?? null,
          card: opening.card,
          position: opening.position ?? null,
          positionCommitment: opening.positionCommitment ?? opening.position_commitment ?? "",
          hasProof: Boolean(opening.ziffleReveal || opening.ziffleProof || opening.positionOpeningProof),
          timing: opening.timing,
        })),
      })),
    };
  }).catch((err) => ({ error: String(err?.message || err) }));
  const publicCheckpoint = await page.evaluate(async () => {
    const checkpoint = await window.__ironsmithE2E?.publicCheckpoint?.();
    const syncCheckpoint = await window.__ironsmithE2E?.checkpoint?.();
    const objectsById = new Map((syncCheckpoint?.objects || []).map((object) => [
      Number(object.id),
      object,
    ]));
    if (!checkpoint) return null;
    return {
      hiddenZones: (checkpoint.hidden_zones || checkpoint.hiddenZones || []).map((zone) => ({
        owner: zone.owner,
        zone: zone.zone,
        count: zone.count,
        commitmentRoot: zone.commitmentRoot ?? zone.commitment_root ?? null,
      })),
      players: (checkpoint.players || []).map((player) => ({
        id: player.id,
        life: player.life,
        handCount: player.hand_count ?? player.handCount,
        libraryCount: player.library_count ?? player.libraryCount,
        graveyard: player.graveyard || [],
      })),
      battlefield: checkpoint.battlefield || [],
      stack: checkpoint.stack || [],
      objects: (checkpoint.objects || []).map((object) => ({
        id: object.id,
        stableId: object.stableId ?? object.stable_id,
        owner: object.owner,
        controller: object.controller,
        zone: object.zone,
        name: object.identity?.name || null,
        tapped: object.tapped,
      })),
      handEntries: (syncCheckpoint?.players || []).flatMap((player) =>
        (player.hand || []).map((objectId, index) => {
          const object = objectsById.get(Number(objectId)) || {};
          const hidden = object.hiddenCard || object.hidden_card || {};
          return {
            owner: player.id,
            index,
            objectId: Number(objectId),
            name: object.name || object.identity?.name || null,
            hiddenSlot: hidden.slot ?? null,
            hiddenCommitment: hidden.commitment || "",
            publicSlot: hidden.publicSlot ?? hidden.public_slot ?? null,
            publicCommitment: hidden.publicCommitment || hidden.public_commitment || "",
          };
        })
      ),
    };
  }).catch((err) => ({ error: String(err?.message || err) }));
  return {
    index,
    label: page?.__peerHarnessLabel || "",
    url: page?.url?.() || "",
    errors: page?.__peerHarnessErrors || [],
    console: (page?.__peerHarnessConsole || []).slice(-30),
    snapshot: await fullUiSnapshot(page).catch((err) => String(err?.message || err)),
    auditTranscript,
    publicCheckpoint,
    buttons: await buttonDebugText(page).catch((err) => String(err?.message || err)),
    body: await visibleBodyText(page).catch((err) => String(err?.message || err)),
  };
}

export async function visibleHandCardNames(page) {
  const names = await withTimeout(
    page.locator(".hand-card[data-card-name]").evaluateAll((cards) =>
      cards
        .map((card) => String(card.getAttribute("data-card-name") || "").trim())
        .filter(Boolean)
    ).catch(() => []),
    5000,
  );
  return Array.isArray(names) ? names : [];
}

export function snapshotBattlefieldCards(snapshot) {
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

export function snapshotBattlefieldCardCount(snapshot, cardName) {
  return snapshotBattlefieldCards(snapshot).filter(
    (card) => String(card?.name || "") === cardName
  ).length;
}

export function snapshotPlayer(snapshot, playerIndex) {
  return (snapshot?.state?.players || []).find(
    (player) => Number(player?.id) === Number(playerIndex)
  ) || null;
}

export function snapshotZoneCardCount(cards) {
  return (cards || []).reduce((total, card) => {
    const count = Number(card?.count);
    return total + (Number.isFinite(count) && count > 0 ? count : 1);
  }, 0);
}

export async function stackCardCount(page, cardName) {
  const count = await withTimeout(
    page.locator(".stack-card[data-card-name]").evaluateAll((cards, name) =>
      cards.filter((card) => String(card.getAttribute("data-card-name") || "") === name).length,
      cardName,
    ).catch(() => 0),
    5000,
  );
  return Number(count) || 0;
}

export async function waitForNamedVisibleHand(page, label, timeoutMs = 60000, snapshotPredicate = null) {
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

export async function waitForLocalButton(page, pattern, label, timeoutMs = 60000) {
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

export async function waitForVisibleBodyText(page, pattern, label, timeoutMs = 60000) {
  const started = Date.now();
  let text = "";
  while (Date.now() - started < timeoutMs) {
    text = await visibleBodyText(page);
    if (pattern.test(text)) return text;
    await sleep(500);
  }
  assert.fail(`${label}\nerrors: ${JSON.stringify(page.__peerHarnessErrors || [], null, 2)}\nconsole: ${JSON.stringify((page.__peerHarnessConsole || []).slice(-80), null, 2)}\nLast body text:\n${text}`);
}

export function assertNoSyncFailureText(text, label = "unexpected sync failure") {
  const failurePattern = /Unknown Ziffle Ceremony|Unknown ziffle ceremony|Private deck opening does not match slot|Ziffle card opening proof reveals a different committed slot|hidden card commitment does not match reveal|No direct ziffle route|Match clock elapsed time exceeds local observation|Sequenced action public checkpoint hash does not match local state|Sync failed|Match start failed|Auto-pass failed|Resync checkpoint hash mismatch|Match disputed|Protocol response timeout|Disconnect timeout policy failed/i;
  const match = String(text || "").match(failurePattern);
  assert.ok(!match, `${label}: ${match?.[0] || ""}`);
}

export async function hasLocalButton(page, textPattern) {
  const buttons = await buttonDebugText(page);
  return buttons.some((button) =>
    button.localAction === "true"
    && !button.disabled
    && button.ariaDisabled !== "true"
    && textPattern.test(button.text)
  );
}

export async function waitAndClickLocalButton(page, label, textPattern, timeoutMs = 90000) {
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

export async function waitAndClickEnabledButton(page, label, textPattern, timeoutMs = 90000) {
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

export async function submitFullUiMultiplayerCommand(page, command, label, options = {}) {
  const attempts = Math.max(1, Number(options.routeRetryAttempts || 3));
  const timeoutMs = Math.max(1000, Number(options.timeoutMs || 120000));
  for (let attempt = 1; attempt <= attempts; attempt += 1) {
    try {
      return await withRejectingTimeout(
        page.evaluate(({ command: nextCommand, label: nextLabel }) => (
          window.__ironsmithE2E.submitMultiplayerCommand(nextCommand, nextLabel)
        ), { command, label }),
        timeoutMs,
        `${label || "multiplayer command"} submit`,
      );
    } catch (err) {
      const message = String(err?.message || err || "");
      const retryable =
        /No direct ziffle route to peer/i.test(message)
        || /Execution context was destroyed|navigation/i.test(message);
      if (
        attempt >= attempts
        || !retryable
      ) {
        const detailedMessage = `${message}
command: ${JSON.stringify(command, null, 2)}
buttons: ${JSON.stringify(await buttonDebugText(page).catch(() => []), null, 2)}
errors: ${JSON.stringify(page.__peerHarnessErrors || [], null, 2)}
console: ${JSON.stringify((page.__peerHarnessConsole || []).slice(-80), null, 2)}
body:
${await visibleBodyText(page).catch(() => "<page unavailable>")}`;
        throw new Error(detailedMessage, { cause: err });
      }
      await page.waitForLoadState("domcontentloaded", { timeout: 10000 }).catch(() => {});
      await page.waitForFunction(
        () => Boolean(window.__ironsmithE2E?.submitMultiplayerCommand),
        null,
        { timeout: 15000 }
      ).catch(() => {});
      await sleep(Math.max(500, Number(options.routeRetryDelayMs || 5000)));
    }
  }
  return null;
}

export async function captureFullUiStep(dir, index, slug, pages) {
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

export async function startFullUiPeerMatch({
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
  await waitForVisibleBodyText(hostPage, /CREATE LOBBY/i, "host shows create lobby", 120000);
  await hostPage.getByRole("button").filter({ hasText: /CREATE LOBBY/i }).first().click();
  await waitForVisibleBodyText(hostPage, /Host or join/i, "host shows lobby chooser", 120000);
  await hostPage.getByRole("button").filter({ hasText: /CREATE LOBBY/i }).last().click();
  await waitForVisibleBodyText(hostPage, /Share this code/i, "host creates shareable lobby", 120000);

  const lobbyCode = (await visibleBodyText(hostPage)).match(
    /[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/i
  )?.[0];
  assert.ok(lobbyCode, "expected the full UI to create a lobby code");

  const guestPage = await openFullUiPage(
    guestContext,
    `${baseUrl}/?lobby=${encodeURIComponent(lobbyCode)}&name=${encodeURIComponent(guestName)}&deck=${guestDeck}`,
    guestLabel,
  );
  await Promise.all([
    waitForVisibleBodyText(hostPage, /All players are ready/i, "host sees all players ready", 120000),
    waitForVisibleBodyText(guestPage, /All players are ready/i, "guest sees all players ready", 120000),
  ]);
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

export async function clickAnyFullUiProgressAction(pages, label, timeoutMs = 90000) {
  const progressPattern = /PLAY MOUNTAIN|KEEP HAND|PREGAME|BEGIN GAME|CONTINUE|UNTAP|UPKEEP|DRAW|MAIN|PASS PRIORITY|RESOLVE/i;
  const started = Date.now();
  while (Date.now() - started < timeoutMs) {
    await assertNoFullUiSyncFailuresWithDebug(`${label}: unexpected sync failure before click`, ...pages);
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


export async function driveDemonicConsultationCast({
  hostPage,
  guestPage,
  actorPage,
  actorLabel,
  landAlreadyPlayed = false,
  maxSteps = 160,
}) {
  let drivingPage = actorPage;
  let playedSwamp = Boolean(landAlreadyPlayed);
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
    const actorText = drivingPage === hostPage ? hostText : guestText;
    const actorSnapshot = drivingPage === hostPage ? hostSnapshot : guestSnapshot;
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
      const result = await clickLocalButton(drivingPage, `${actorLabel}-play-swamp`, /PLAY SWAMP/i);
      if (result) {
        playedSwamp = true;
        await sleep(3500);
        continue;
      }
    }

    if (playedSwamp && !castConsultation && /CAST DEMONIC CONSULTATION/i.test(combinedText)) {
      let result = null;
      const candidatePages = [...new Set([drivingPage, hostPage, guestPage])];
      for (const page of candidatePages) {
        result = await clickLocalButton(
          page,
          `${actorLabel}-cast-consultation`,
          /CAST DEMONIC CONSULTATION/i
        );
        if (result) {
          drivingPage = page;
          break;
        }
      }
      if (!result) {
        for (const page of candidatePages) {
          result = await clickEnabledButton(
            page,
            `${actorLabel}-cast-consultation`,
            /CAST DEMONIC CONSULTATION/i
          );
          if (result) {
            drivingPage = page;
            break;
          }
        }
      }
      if (result) {
        attemptedCastConsultation = true;
        await sleep(2500);
        const [nextHostSnapshot, nextGuestSnapshot] = await Promise.all([
          fullUiSnapshot(hostPage),
          fullUiSnapshot(guestPage),
        ]);
        const nextActorSnapshot = drivingPage === hostPage ? nextHostSnapshot : nextGuestSnapshot;
        const nextActorKind = String(nextActorSnapshot?.state?.decision?.kind || "");
        if (
          /^(choose_option|text_input)$/i.test(nextActorKind)
          || await stackCardCount(hostPage, "Demonic Consultation") > 0
          || await stackCardCount(guestPage, "Demonic Consultation") > 0
        ) {
          castConsultation = true;
        }
        continue;
      }
    }

    if (playedSwamp && !castConsultation && attemptedCastConsultation) {
      for (const page of [...new Set([hostPage, guestPage])]) {
        if (await hasLocalButton(page, /CAST DEMONIC CONSULTATION/i)) {
          drivingPage = page;
          attemptedCastConsultation = false;
          break;
        }
      }
      if (!attemptedCastConsultation) {
        continue;
      }
    }

    if (attemptedCastConsultation || castConsultation) {
      const paymentPending = /Pay [\s\S]*Demonic Consultation|CHOOSE OPTION|remaining|Use\s+from mana pool/i.test(actorText);
      if (paymentPending) {
        castConsultation = true;
        const paymentOption =
          await clickLocalButton(
            drivingPage,
            `${actorLabel}-pay-consultation-with-swamp`,
            /Tap Swamp|SWAMP|ADD|BLACK|Use\s+from mana pool|\{B\}/i
          )
          || await clickEnabledButton(
            drivingPage,
            `${actorLabel}-pay-consultation-with-swamp`,
            /Tap Swamp|SWAMP|ADD|BLACK|Use\s+from mana pool|\{B\}/i
          );
        if (paymentOption) {
          await sleep(3000);
          continue;
        }
        const paymentSubmit =
          await clickLocalButton(drivingPage, `${actorLabel}-submit-consultation-payment`, /^SUBMIT$|^PAY$/i)
          || await clickEnabledButton(drivingPage, `${actorLabel}-submit-consultation-payment`, /^SUBMIT$|^PAY$/i);
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

    const progressToConsultationPattern = playedSwamp || actorPage !== hostPage
      ? /KEEP HAND|PREGAME|BEGIN GAME|UPKEEP|DRAW|MAIN|COMBAT|ATTACKERS|BLOCKERS|NO ATTACKERS|DONE|M2|END|CLEAN|PASS PRIORITY|RESOLVE/i
      : /KEEP HAND|PREGAME|BEGIN GAME|UPKEEP|DRAW|MAIN|PASS PRIORITY|RESOLVE/i;
    const hostProgress = await clickLocalButton(
      hostPage,
      `${actorLabel}-host-progress-to-consultation`,
      progressToConsultationPattern
    );
    if (hostProgress) {
      await sleep(2500);
      continue;
    }
    const guestProgress = await clickLocalButton(
      guestPage,
      `${actorLabel}-guest-progress-to-consultation`,
      progressToConsultationPattern
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

export async function resolveDemonicConsultationWithMissingName({
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
      const hostCaster = snapshotPlayer(lastHostSnapshot, actorIndex);
      const guestCaster = snapshotPlayer(lastGuestSnapshot, actorIndex);
      const hostLibrarySize = Number(hostCaster?.library_size);
      const guestLibrarySize = Number(guestCaster?.library_size);
      const hostExileCount = snapshotZoneCardCount(hostCaster?.exile_cards);
      const guestExileCount = snapshotZoneCardCount(guestCaster?.exile_cards);
      const hostSequence = Number(lastHostSnapshot?.multiplayer?.lastAppliedSequence || 0);
      const guestSequence = Number(lastGuestSnapshot?.multiplayer?.lastAppliedSequence || 0);
      if (
        librarySize === 0
        && exileCount - preExileCount === preLibrarySize
        && hostLibrarySize === 0
        && guestLibrarySize === 0
        && hostExileCount - preExileCount === preLibrarySize
        && guestExileCount - preExileCount === preLibrarySize
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
