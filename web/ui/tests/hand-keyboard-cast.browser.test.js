import test from "node:test";
import assert from "node:assert/strict";
import { chromium } from "playwright";
import { createServer } from "vite";

const bolt = { id: 7, name: "Lightning Bolt", card_types: ["instant"], type_line: "Instant" };
const bears = { id: 8, name: "Grizzly Bears", card_types: ["creature"], type_line: "Creature — Bear" };
const ritual = { id: 9, name: "Dark Ritual", card_types: ["instant"], type_line: "Instant" };

const action = (overrides) => ({
  kind: "cast_spell",
  drag_requires_targets: false,
  drag_requires_modes: false,
  ...overrides,
  action_ref: { kind: "cast_spell", spell_id: overrides.object_id, ...(overrides.action_ref || {}) },
});

const state = {
  perspective: 0,
  players: [{ id: 0, index: 0, name: "You", can_view_hand: true, hand_cards: [bolt, bears, ritual] }],
  decision: {
    kind: "priority",
    player: 0,
    actions: [
      action({ index: 0, object_id: 7, label: "Cast Lightning Bolt", drag_requires_targets: true }),
      action({ index: 1, object_id: 8, label: "Cast Grizzly Bears" }),
      action({ index: 2, object_id: 9, label: "Cast Dark Ritual" }),
      action({ index: 3, object_id: 9, label: "Cast Dark Ritual with kicker" }),
    ],
  },
};

async function harness() {
  const vite = await createServer({ server: { host: "127.0.0.1", port: 0 }, logLevel: "silent" });
  await vite.listen();
  const browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: 1200, height: 700 }, reducedMotion: "reduce" });
  const errors = [];
  page.on("pageerror", (error) => errors.push(String(error?.message || error)));
  await page.route("**/api.scryfall.com/**", (route) => route.abort());
  await page.route("**/cards.scryfall.io/**", (route) => route.abort());
  await page.addInitScript((fixture) => { window.__handKeyboardFixture = fixture; }, { state });
  await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/hand-keyboard-cast.html`);
  await page.locator('[data-hand-case] .game-card.hand-card').first().waitFor();
  return {
    page,
    errors,
    casts: () => page.evaluate(() => window.__keyboardCasts),
    drag: () => page.evaluate(() => window.__dragState),
    press: async (objectId) => {
      await page.locator(`[data-hand-case] .game-card[data-object-id="${objectId}"]`).focus();
      await page.keyboard.press(" ");
      await page.waitForTimeout(120);
    },
    close: async () => { await browser.close(); await vite.close(); },
  };
}

test("the activation key casts a hand card without waiting for a pointer release", { timeout: 120000 }, async () => {
  const { press, casts, drag, errors, close } = await harness();
  try {
    // A targeted spell is cast at once; the engine's targeting arrow, not a
    // held card, is what the mouse then aims.
    await press(7);
    assert.deepEqual(await drag(), null, "nothing is left held");
    let requested = await casts();
    assert.equal(requested.length, 1);
    assert.deepEqual(requested[0].actions, ["Cast Lightning Bolt"]);
    assert.equal(requested[0].objectId, 7);
    assert.ok(requested[0].anchorRect.width > 0, "the picker can be anchored on the card");

    // Several ways to play one card are published as one request, so the
    // picker resolves the choice before anything reaches the engine.
    await press(9);
    requested = await casts();
    assert.equal(requested.length, 2);
    assert.deepEqual(requested[1].actions, ["Cast Dark Ritual", "Cast Dark Ritual with kicker"]);
    assert.deepEqual(await drag(), null);
  } finally {
    assert.deepEqual(errors, []);
    await close();
  }
});

test("a permanent stays held so its battlefield slot follows the mouse", { timeout: 120000 }, async () => {
  const { page, press, casts, drag, errors, close } = await harness();
  try {
    const cardBox = await page.locator('[data-hand-case] .game-card[data-object-id="8"]').boundingBox();
    await press(8);
    assert.deepEqual(await casts(), [], "a permanent is not cast until it is placed");
    const held = await drag();
    assert.equal(held.objectId, 8);
    assert.equal(held.keyboard, true);
    assert.deepEqual(held.actions, ["Cast Grizzly Bears"]);
    const anchor = { x: held.currentX, y: held.currentY };
    const grip = { x: held.startX, y: held.startY };

    // The arrow stands in for the pointer from the moment the card is held,
    // pointing at dead space above the card rather than at the resting mouse.
    await page.locator(".placement-drag-arrow").waitFor({ timeout: 5000 });
    assert.ok(anchor.y < cardBox.y, "lifted clear of the card it came from");
    assert.ok(Math.abs(anchor.x - (cardBox.x + (cardBox.width / 2))) < 1, "straight ahead of it");
    const overCard = await page.evaluate(
      ([x, y]) => Boolean(document.elementFromPoint(x, y)?.closest?.(".game-card")),
      [anchor.x, anchor.y],
    );
    assert.equal(overCard, false, "nothing a click would pick sits under the arrow");

    // The hold tracks the bare pointer: no button is down to drag with.
    await page.mouse.move(880, 220);
    await page.waitForTimeout(80);
    let moved = await drag();
    assert.deepEqual([moved.currentX, moved.currentY], [880, 220]);
    assert.notDeepEqual([moved.currentX, moved.currentY], [anchor.x, anchor.y], "the mouse takes the arrow over");
    assert.deepEqual([moved.startX, moved.startY], [grip.x, grip.y], "it still points from the card");

    await page.mouse.move(300, 480);
    await page.waitForTimeout(80);
    moved = await drag();
    assert.deepEqual([moved.currentX, moved.currentY], [300, 480]);

    // Escape puts the card back; with no button to lift nothing else would.
    await page.keyboard.press("Escape");
    await page.waitForTimeout(80);
    assert.deepEqual(await drag(), null);
    await page.mouse.move(500, 300);
    await page.waitForTimeout(80);
    assert.deepEqual(await drag(), null, "the released hold stops tracking");
  } finally {
    assert.deepEqual(errors, []);
    await close();
  }
});
