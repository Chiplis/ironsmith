import test from "node:test";
import assert from "node:assert/strict";
import { createServer } from "vite";
import { chromium } from "playwright";
import { fileURLToPath } from "node:url";

async function waitFor(page, predicate) {
  const deadline = Date.now() + 20_000;
  while (Date.now() < deadline) {
    if (await page.evaluate(predicate)) return;
    await new Promise((resolve) => setTimeout(resolve, 50));
  }
  assert.fail(JSON.stringify(await page.evaluate(() => window.__peerHarness.snapshot())));
}

const root = fileURLToPath(new URL("..", import.meta.url));

async function setup(t, insecure = false) {
  const server = await createServer({ root, mode: "lan", server: { host: "127.0.0.1", port: 0, allowedHosts: ["ironsmith.test"] }, logLevel: "error" });
  await server.listen();
  t.after(() => server.close());
  const browser = await chromium.launch({ headless: true, args: ["--host-resolver-rules=MAP ironsmith.test 127.0.0.1", "--no-proxy-server"] });
  t.after(() => browser.close());
  const base = `http://127.0.0.1:${server.httpServer.address().port}`;
  const browserBase = insecure ? base.replace("127.0.0.1", "ironsmith.test") : base;
  const pages = [];
  for (let index = 0; index < 3; index++) {
    const context = await browser.newContext();
    if (insecure) await context.addInitScript(() => {
      Object.defineProperty(globalThis.crypto, "subtle", { value: undefined });
    });
    const page = await context.newPage();
    await page.goto(`${browserBase}/tests/fixtures/peer-lobby-harness.html`);
    await page.waitForFunction(() => window.__peerHarness?.ready);
    pages.push(page);
  }
  return { pages, base, browserBase };
}

test("native LAN WebRTC sends large ordered Unicode payloads, meshes peers, and survives signaling reconnect", { timeout: 60_000 }, async (t) => {
  const { pages } = await setup(t);
  for (const page of pages) {
    await page.evaluate(async () => {
      const { NativeLanPeer } = await import("/src/lib/lan/native-peer.js");
      window.received = [];
      window.errors = [];
      window.native = new NativeLanPeer();
      window.native.on("error", (error) => window.errors.push(error.message));
      window.native.on("connection", (connection) => {
        window.incoming = connection;
        connection.on("data", (data) => window.received.push(data));
      });
      await new Promise((resolve) => window.native.on("open", resolve));
    });
  }
  const [host, guest, third] = pages;
  const hostId = await host.evaluate(() => window.native.id);
  const guestId = await guest.evaluate(() => window.native.id);
  for (const [page, target] of [[guest, hostId], [third, guestId]]) {
    await page.evaluate(async (peerId) => {
      const connection = window.native.connect(peerId, { metadata: { channel: "peer-direct" } });
      window.outgoing = connection;
      await new Promise((resolve, reject) => { connection.on("open", resolve); connection.on("error", reject); });
      connection.send({ type: "large", text: "🌳abc".repeat(300_000) });
      connection.send({ type: "after" });
    }, target);
  }
  for (const page of [host, guest]) {
    await page.waitForFunction(() => window.received.length === 2);
    assert.deepEqual(await page.evaluate(() => [window.received[0].text === "🌳abc".repeat(300_000), window.received[1].type, window.incoming.metadata.channel]), [true, "after", "peer-direct"]);
  }
  await guest.evaluate(async () => {
    window.native.events.close();
    window.native.open = false;
    window.native.disconnected = true;
    window.outgoing.send({ type: "without-signaling" });
    window.native.reconnect();
    await new Promise((resolve) => window.native.on("open", resolve));
    window.outgoing.send({ type: "reconnected" });
  });
  await host.waitForFunction(() => window.received.length === 4);
  assert.deepEqual(await host.evaluate(() => window.received.slice(2).map((entry) => entry.type)), ["without-signaling", "reconnected"]);
  await guest.evaluate(() => window.outgoing.close());
  await host.waitForFunction(() => window.incoming.closed);
  for (const page of pages) {
    assert.deepEqual(await page.evaluate(() => window.errors), []);
    await page.evaluate(() => window.native.destroy());
  }
});

for (const securityMode of ["trusted", "verified"]) {
test(`LAN ${securityMode} lobby search, join, match start, and synchronized game action`, { timeout: 60_000 }, async (t) => {
  const { pages: [host, guest, search], base, browserBase } = await setup(t, securityMode === "trusted");
  assert.equal(await host.evaluate(() => window.isSecureContext), securityMode === "verified");
  assert.equal(await host.evaluate(() => Boolean(globalThis.crypto.subtle)), securityMode === "verified");
  await host.evaluate((securityMode) => window.__peerHarness.createLobby({ name: "LAN Host", desiredPlayers: 2, startingLife: 20, deckText: "60 Island", securityMode }), securityMode);
  await waitFor(host, async () => (await window.__peerHarness.snapshot()).multiplayer.mode === "lobby");
  const lobbyId = await host.evaluate(async () => (await window.__peerHarness.snapshot()).multiplayer.lobbyId);
  await waitFor(host, async () => (await (await fetch("/__ironsmith_lan/lobbies")).json()).lobbies.length === 1);
  assert.equal((await (await fetch(`${base}/__ironsmith_lan/lobbies`)).json()).lobbies[0].id, lobbyId);
  await search.goto(`${browserBase}/tests/fixtures/lan-search.html`);
  const lobbyButton = search.getByRole("button", { name: /LAN Host/ });
  await lobbyButton.waitFor();
  await search.getByRole("textbox", { name: "Search local lobbies" }).fill("not a host");
  await search.getByText("No available lobbies found.").waitFor();
  await search.getByRole("textbox", { name: "Search local lobbies" }).fill("LAN Host");
  await lobbyButton.click();
  assert.equal(await search.evaluate(() => window.selectedLobby), lobbyId);
  await guest.evaluate((lobbyId) => window.__peerHarness.joinLobby({ name: "Guest", lobbyId, deckText: "60 Mountain" }), lobbyId);
  await waitFor(host, async () => (await window.__peerHarness.snapshot()).canStartHostedMatch);
  await waitFor(host, async () => (await (await fetch("/__ironsmith_lan/lobbies")).json()).lobbies.length === 0);
  await host.evaluate(() => window.__peerHarness.startHostedMatch());
  for (const page of [host, guest]) await waitFor(page, async () => (await window.__peerHarness.snapshot()).multiplayer.matchStarted);
  await host.evaluate(() => window.__peerHarness.submitMultiplayerCommand({ type: "priority_action", action_ref: { kind: "test_priority_action", actor: 0, sequence: 0 } }, "LAN action"));
  for (const page of [host, guest]) {
    await waitFor(page, async () => (await window.__peerHarness.snapshot()).multiplayer.lastAppliedSequence === 1);
  }
  await guest.evaluate(() => window.__peerHarness.leaveLobby());
  await host.evaluate(() => window.__peerHarness.leaveLobby());
  assert.deepEqual((await (await fetch(`${base}/__ironsmith_lan/lobbies`)).json()).lobbies, []);
});
}
