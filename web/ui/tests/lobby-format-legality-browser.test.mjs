import test from "node:test";
import assert from "node:assert/strict";
import { createServer } from "vite";
import { chromium } from "playwright";
import { fileURLToPath } from "node:url";

// A public-format lobby checks every seat's deck, on any transport: the host
// shares each seat's problems with the table and only starts once all decks
// are legal. A seat can rename itself without leaving.

async function waitFor(page, predicate, arg) {
  const deadline = Date.now() + 30_000;
  while (Date.now() < deadline) {
    if (await page.evaluate(predicate, arg)) return;
    await new Promise((resolve) => setTimeout(resolve, 50));
  }
  assert.fail(JSON.stringify(await page.evaluate(() => window.__peerHarness.lobbyState())));
}

const root = fileURLToPath(new URL("..", import.meta.url));

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

const canStart = () => window.__peerHarness.snapshot().then((snapshot) => snapshot.canStartHostedMatch);
const seat = (name) => window.__peerHarness.lobbyState().multiplayer.players.find((player) => player.name === name);

for (const securityMode of ["trusted", "verified"]) {
  test(`${securityMode} Modern lobby blocks an illegal deck, tells every seat, and takes renames`, { timeout: 120_000 }, async (t) => {
    const [host, guest] = await setup(t, securityMode === "trusted");
    await host.evaluate((securityMode) => window.__peerHarness.createLobby({
      name: "Host", desiredPlayers: 2, startingLife: 20, format: "modern", deckText: "60 Island", securityMode,
    }), securityMode);
    await waitFor(host, () => window.__peerHarness.lobbyState().multiplayer.mode === "lobby");
    const lobbyId = await host.evaluate(() => window.__peerHarness.lobbyState().multiplayer.lobbyId);
    await guest.evaluate((lobbyId) => window.__peerHarness.joinLobby({
      name: "Guest", lobbyId, deckText: "56 Mountain\n4 Sol Ring",
    }), lobbyId);

    // Both seats learn that the guest's deck breaks the format.
    for (const page of [host, guest]) {
      await waitFor(page, (seat) => (window.__peerHarness.lobbyState().multiplayer.players
        .find((player) => player.name === seat)?.formatErrors || []).some((error) => /Sol Ring/.test(error)), "Guest");
    }
    assert.equal(await host.evaluate(canStart), false);

    await guest.evaluate(() => window.__peerHarness.updateLobbyDeck({ deckText: "60 Mountain" }));
    await waitFor(host, canStart);
    assert.deepEqual(await guest.evaluate(seat, "Guest").then((player) => player.formatErrors), []);

    await guest.evaluate(() => window.__peerHarness.updateLobbyDeck({ name: "Renamed Guest" }));
    await waitFor(host, (name) => window.__peerHarness.lobbyState().multiplayer.players.some((player) => player.name === name), "Renamed Guest");
    await host.evaluate(() => window.__peerHarness.updateLobbyDeck({ name: "Renamed Host" }));
    await waitFor(guest, (name) => window.__peerHarness.lobbyState().multiplayer.players.some((player) => player.name === name), "Renamed Host");
    await waitFor(host, canStart);

    await host.evaluate(() => window.__peerHarness.startHostedMatch());
    for (const page of [host, guest]) await waitFor(page, () => window.__peerHarness.lobbyState().multiplayer.matchStarted);
    const names = await guest.evaluate(() => window.__peerHarness.lobbyState().multiplayer.players.map((player) => player.name).sort());
    assert.deepEqual(names, ["Renamed Guest", "Renamed Host"]);
  });
}

test('Community room shares its pool, rejects excluded cards and starts with custom cards', { timeout: 120_000 }, async t => {
  const [host, guest] = await setup(t, false);
  const communityFormat = { name: 'Community night', includedSets: [{ code: 'tst', name: 'Test set', cards: ['Island', 'Sol Ring'] }],
    excludedCards: ['Sol Ring'], customCards: [{ id: 'mage', draft: { layout: 'single', faces: [{ name: 'Community Mage', cardTypes: ['Creature'], power: '2', toughness: '2', oracleText: '' }] } }] };
  await host.evaluate(communityFormat => window.__peerHarness.createLobby({ name: 'Host', desiredPlayers: 2, startingLife: 20,
    format: 'normal', securityMode: 'trusted', deckText: '59 Island\n1 Community Mage', communityFormat }), communityFormat);
  await waitFor(host, () => window.__peerHarness.lobbyState().multiplayer.mode === 'lobby');
  const lobbyId = await host.evaluate(() => window.__peerHarness.lobbyState().multiplayer.lobbyId);
  await guest.evaluate(lobbyId => window.__peerHarness.joinLobby({ name: 'Guest', lobbyId, deckText: '59 Island\n1 Sol Ring' }), lobbyId);
  for (const page of [host, guest]) await waitFor(page, () => window.__peerHarness.lobbyState().multiplayer.players.some(player => player.formatErrors?.some(error => error.includes('Sol Ring'))));
  assert.equal(await host.evaluate(canStart), false);
  assert.deepEqual(await guest.evaluate(() => window.__peerHarness.lobbyState().multiplayer.communityFormat.excludedCards), ['Sol Ring']);
  await guest.evaluate(() => window.__peerHarness.updateLobbyDeck({ deckText: '59 Island\n1 Community Mage' }));
  await waitFor(host, canStart);
  await host.evaluate(() => window.__peerHarness.startHostedMatch());
  for (const page of [host, guest]) await waitFor(page, () => window.__peerHarness.lobbyState().multiplayer.matchStarted);
});
