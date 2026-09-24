import test from "node:test";
import assert from "node:assert/strict";
import { createServer } from "vite";
import { chromium } from "playwright";
import { fileURLToPath } from "node:url";

// "Play again" after a lobby game: every seat can bring a different deck, the
// way it did when joining the lobby, and only the host starts the next game,
// once every deck is ready.

async function waitFor(page, predicate, arg) {
  const deadline = Date.now() + 30_000;
  while (Date.now() < deadline) {
    if (await page.evaluate(predicate, arg)) return;
    await new Promise((resolve) => setTimeout(resolve, 50));
  }
  assert.fail(JSON.stringify(await page.evaluate(() => window.__peerHarness.lobbyState())));
}

const root = fileURLToPath(new URL("..", import.meta.url));
const lobbyState = () => window.__peerHarness.lobbyState();

async function setup(t, insecure) {
  const server = await createServer({ root, mode: "lan", server: { host: "127.0.0.1", port: 0, allowedHosts: ["ironsmith.test"] }, logLevel: "error" });
  await server.listen();
  t.after(() => server.close());
  const browser = await chromium.launch({ headless: true, args: ["--host-resolver-rules=MAP ironsmith.test 127.0.0.1", "--no-proxy-server"] });
  t.after(() => browser.close());
  const base = `http://127.0.0.1:${server.httpServer.address().port}`;
  const browserBase = insecure ? base.replace("127.0.0.1", "ironsmith.test") : base;
  const pages = [];
  for (let index = 0; index < 2; index++) {
    const context = await browser.newContext();
    if (insecure) await context.addInitScript(() => {
      Object.defineProperty(globalThis.crypto, "subtle", { value: undefined });
    });
    const page = await context.newPage();
    await page.goto(`${browserBase}/tests/fixtures/peer-lobby-harness.html`);
    await page.waitForFunction(() => window.__peerHarness?.ready);
    pages.push(page);
  }
  return pages;
}

const startCount = (page) => page.evaluate(async () => (await window.__peerHarness.snapshot()).instrumentation.startMatch);

for (const securityMode of ["trusted", "verified"]) {
  test(`${securityMode} play again: new decks, host-started next game`, { timeout: 120_000 }, async (t) => {
    const [host, guest] = await setup(t, securityMode === "trusted");
    await host.evaluate((securityMode) => window.__peerHarness.createLobby({ name: "Host", desiredPlayers: 2, startingLife: 20, deckText: "60 Island", securityMode }), securityMode);
    await waitFor(host, () => window.__peerHarness.lobbyState().multiplayer.mode === "lobby");
    const lobbyId = await host.evaluate(() => window.__peerHarness.lobbyState().multiplayer.lobbyId);
    await guest.evaluate((lobbyId) => window.__peerHarness.joinLobby({ name: "Guest", lobbyId, deckText: "60 Mountain" }), lobbyId);
    await waitFor(host, async () => (await window.__peerHarness.snapshot()).canStartHostedMatch);
    await host.evaluate(() => window.__peerHarness.startHostedMatch());
    for (const page of [host, guest]) await waitFor(page, () => window.__peerHarness.lobbyState().multiplayer.matchStarted);
    assert.equal(await startCount(host), 1);

    // A guest's Play again asks the host, who opens deck selection for both.
    await guest.evaluate(() => window.__peerHarness.startRematchSideboarding());
    for (const page of [host, guest]) {
      await waitFor(page, () => window.__peerHarness.lobbyState().multiplayer.rematch?.phase === "sideboarding");
    }
    // Each seat's editor opens on the deck it just played.
    assert.match((await guest.evaluate(lobbyState)).multiplayer.rematch.localDeckText, /60 Mountain/);
    assert.match((await host.evaluate(lobbyState)).multiplayer.rematch.localDeckText, /60 Island/);

    // A deck the lobby would refuse is refused here too.
    await host.evaluate(() => window.__peerHarness.updateRematchDeck({ deckText: "10 Island" }));
    await host.evaluate(() => window.__peerHarness.readyForRematch());
    const refused = await host.evaluate(lobbyState);
    assert.equal(refused.multiplayer.rematch.localReady, false);
    assert.ok(refused.statusEvents.some((event) => event.isError && /60 main-deck cards/.test(event.message)), JSON.stringify(refused.statusEvents.slice(-3)));

    // The guest brings a different deck and marks it ready.
    await guest.evaluate(() => window.__peerHarness.updateRematchDeck({ deckText: "60 Forest" }));
    await guest.evaluate(() => window.__peerHarness.readyForRematch());
    await waitFor(host, () => window.__peerHarness.lobbyState().multiplayer.rematch.players.find((p) => p.name === "Guest")?.ready);

    // Editing after readying withdraws the ready mark on the host.
    await guest.evaluate(() => window.__peerHarness.updateRematchDeck({ deckText: "60 Swamp" }));
    await waitFor(host, () => window.__peerHarness.lobbyState().multiplayer.rematch.players.find((p) => p.name === "Guest")?.ready === false);
    await guest.evaluate(() => window.__peerHarness.readyForRematch());
    await waitFor(host, () => window.__peerHarness.lobbyState().multiplayer.rematch.players.find((p) => p.name === "Guest")?.ready);

    await host.evaluate(() => window.__peerHarness.updateRematchDeck({ deckText: "60 Plains" }));
    await host.evaluate(() => window.__peerHarness.readyForRematch());
    await waitFor(guest, () => window.__peerHarness.lobbyState().multiplayer.rematch.players.every((p) => p.ready));

    // Everyone is ready, but nothing starts until the host says so; a guest
    // cannot start it.
    await new Promise((resolve) => setTimeout(resolve, 1500));
    assert.equal(await startCount(host), 1, "all-ready must not auto-start the next game");
    assert.equal((await host.evaluate(lobbyState)).multiplayer.rematch.phase, "sideboarding");
    await guest.evaluate(() => window.__peerHarness.startRematch());
    assert.ok((await guest.evaluate(lobbyState)).statusEvents.some((event) => event.isError && /Only the host/.test(event.message)));
    await new Promise((resolve) => setTimeout(resolve, 500));
    assert.equal(await startCount(host), 1);

    await host.evaluate(() => window.__peerHarness.startRematch());
    for (const page of [host, guest]) {
      await waitFor(page, async () => (await window.__peerHarness.snapshot()).instrumentation.startMatch === 2);
      await waitFor(page, () => window.__peerHarness.lobbyState().multiplayer.rematch == null);
    }
    // The next game is played with the decks chosen for it.
    for (const page of [host, guest]) {
      const players = (await page.evaluate(lobbyState)).multiplayer.players;
      const deckOf = (name) => players.find((player) => player.name === name)?.deck || [];
      assert.deepEqual([...new Set(deckOf("Host"))], ["Plains"]);
      assert.deepEqual([...new Set(deckOf("Guest"))], ["Swamp"]);
      assert.equal(deckOf("Guest").length, 60);
    }
    // A later rematch opens on the deck the last game used.
    await host.evaluate(() => window.__peerHarness.startRematchSideboarding());
    await waitFor(guest, () => window.__peerHarness.lobbyState().multiplayer.rematch?.phase === "sideboarding");
    assert.match((await guest.evaluate(lobbyState)).multiplayer.rematch.localDeckText, /60 Swamp/);
  });
}
