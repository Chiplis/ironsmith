import test from "node:test";
import assert from "node:assert/strict";
import { chromium } from "playwright";
import { createServer } from "vite";

const CARDS = [
  { route: "grizzly-bears", name: "Grizzly Bears", types: ["Creature"], pips: [[{ Generic: 1 }], ["Green"]] },
  { route: "sol-ring", name: "Sol Ring", types: ["Artifact"], pips: [[{ Generic: 1 }]] },
  { route: "wall-of-roots", name: "Wall of Roots", types: ["Creature"], pips: [[{ Generic: 1 }], ["Green"]] },
  { route: "llanowar-elves", name: "Llanowar Elves", types: ["Creature"], pips: [["Green"]] },
  { route: "lightning-bolt", name: "Lightning Bolt", types: ["Instant"], pips: [["Red"]] },
  { route: "giant-growth", name: "Giant Growth", types: ["Instant"], pips: [["Green"]] },
  { route: "explore", name: "Explore", types: ["Sorcery"], pips: [[{ Generic: 1 }], ["Green"]] },
  { route: "some-plane", name: "Some Plane", types: ["Plane"], pips: [] },
  // Promised by default, and colourless-costed so the filters below would
  // never have sampled it into a green table.
  { route: "omniscience", name: "Omniscience", types: ["Enchantment"], pips: [[{ Generic: 10 }]] },
];

const asset = (card) => ({
  canonicalName: card.name,
  group: { name: card.name, score: 1 },
  artifacts: [{
    payload: {
      definition: {
        card: {
          card_types: card.types,
          supertypes: [],
          subtypes: [],
          mana_cost: card.pips.length ? { pips: card.pips } : null,
          is_token: false,
          linked_face_layout: "None",
        },
      },
    },
  }],
});

async function harness() {
  const vite = await createServer({ server: { host: "127.0.0.1", port: 0 }, logLevel: "silent" });
  await vite.listen();
  const browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: 1000, height: 900 }, reducedMotion: "reduce" });
  const errors = [];
  page.on("pageerror", (error) => errors.push(String(error?.message || error)));
  // Serve a small synthetic catalogue in place of the real card assets. One
  // handler covers both: a later route would otherwise shadow an earlier one.
  await page.route("**/cards/*.json", (route) => {
    const file = route.request().url().split("/cards/")[1].replace(/\.json.*$/, "");
    if (file === "index") {
      return route.fulfill({
        json: {
          version: 1,
          cardCount: CARDS.length,
          cards: CARDS.map((card) => ({ name: card.name, route: card.route, score: 1 })),
        },
      });
    }
    const card = CARDS.find((candidate) => candidate.route === file);
    return card ? route.fulfill({ json: asset(card) }) : route.fulfill({ status: 404, json: {} });
  });
  await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/random-game-sheet.html`);
  await page.locator("[data-random-game-trigger]").click();
  await page.locator(".random-game-sheet").waitFor();
  return {
    page,
    errors,
    generated: () => page.evaluate(() => window.__generated),
    statuses: () => page.evaluate(() => window.__statuses),
    generate: page.locator(".random-game-submit"),
    zoneRow: (zone) => page.locator(`.random-game-sheet-body >> text=${zone}`),
    setNumber: async (label, zone, value) => {
      const row = page.locator(".random-game-sheet-body .grid-cols-\\[1fr_auto_auto\\]", { hasText: zone });
      await row.locator(`label:has-text("${label}") input`).fill(String(value));
    },
    toggle: (name) => page.locator(`label:has-text("${name}") button[role="checkbox"]`).first(),
    close: async () => { await browser.close(); await vite.close(); },
  };
}

test("the sheet generates a table whose zones only hold legal cards", { timeout: 120000 }, async () => {
  const { page, generate, generated, setNumber, toggle, errors, close } = await harness();
  try {
    // Keep it small and green so the expected outcome is exact.
    await setNumber("Cards", "Battlefield", 4);
    await setNumber("Basics", "Battlefield", 1);
    await setNumber("Cards", "Hand", 3);
    await setNumber("Basics", "Hand", 0);
    await setNumber("Cards", "Library", 6);
    await setNumber("Basics", "Library", 4);
    await setNumber("Cards", "Graveyard", 2);
    await setNumber("Basics", "Graveyard", 0);
    for (const color of ["White", "Blue", "Black", "Red"]) await toggle(color).click();
    // The synthetic catalogue holds only a handful of green cards, so let the
    // zones repeat them rather than report a shortfall.
    await toggle("Allow duplicates").click();
    await page.locator('label:has-text("Seed") input').fill("browser-seed");

    await generate.click();
    await page.waitForFunction(() => window.__generated.length > 0, null, { timeout: 30000 });
    const [{ payload, message }] = await generated();
    assert.match(message, /seed browser-seed/);
    assert.equal(payload.version, 1);
    assert.equal(payload.players.length, 2);

    const permanents = new Set(["Grizzly Bears", "Sol Ring", "Wall of Roots", "Llanowar Elves", "Omniscience"]);
    assert.ok(payload.players[0].zones.battlefield.includes("Omniscience"), "promised to our battlefield");
    assert.ok(!payload.players[1].zones.battlefield.includes("Omniscience"), "and not to theirs");
    for (const player of payload.players) {
      assert.equal(player.zones.battlefield.length, 4);
      const basicsOnBoard = player.zones.battlefield.filter((name) => name === "Forest");
      assert.equal(basicsOnBoard.length, 1, "the requested basic, in the one chosen colour");
      for (const name of player.zones.battlefield) {
        if (name === "Forest") continue;
        assert.ok(permanents.has(name), `${name} may sit on the battlefield`);
      }
      assert.equal(player.zones.library.filter((name) => name === "Forest").length, 4);
      assert.equal(player.zones.library.length, 6);
      assert.equal(player.zones.hand.length, 3);
      assert.equal(player.zones.graveyard.length, 2);
      assert.equal(player.zones.command.length, 0);
      assert.equal(player.zones.ante, undefined);
      // A plane is not a card any zone may hold.
      for (const zone of Object.values(player.zones)) {
        assert.ok(!zone.includes("Some Plane"));
      }
    }
    // The sheet closes once the table has been handed over.
    await page.locator(".random-game-sheet").waitFor({ state: "detached", timeout: 5000 });
    assert.deepEqual(errors, []);
  } finally {
    await close();
  }
});

test("a configuration that cannot be served explains itself instead of generating", { timeout: 120000 }, async () => {
  const { page, generate, generated, toggle, errors, close } = await harness();
  try {
    for (const type of ["Creature", "Artifact", "Enchantment", "Planeswalker", "Land", "Instant", "Sorcery"]) {
      await toggle(type).click();
    }
    await page.locator("text=Pick at least one card type.").waitFor();
    assert.equal(await generate.isDisabled(), true);

    // Turning one permanent type back on is enough to fill a battlefield.
    await toggle("Creature").click();
    assert.equal(await generate.isDisabled(), false);

    // Spells alone cannot fill a battlefield that asked for non-basic cards.
    await toggle("Creature").click();
    await toggle("Instant").click();
    await page.locator("text=The battlefield can only hold permanents").waitFor();
    assert.equal(await generate.isDisabled(), true);
    assert.deepEqual(await generated(), []);
    assert.deepEqual(errors, []);
  } finally {
    await close();
  }
});
