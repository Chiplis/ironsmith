import test from "node:test";
import assert from "node:assert/strict";
import { chromium } from "playwright";
import { createServer } from "vite";

const ritual = { id: 9, name: "Dark Ritual", card_types: ["instant"], type_line: "Instant" };
const elves = { id: 21, name: "Llanowar Elves", card_types: ["creature"], type_line: "Creature — Elf Druid" };

const state = {
  perspective: 0,
  players: [{
    id: 0,
    index: 0,
    name: "You",
    can_view_hand: true,
    hand_cards: [ritual],
    battlefield: [elves],
  }],
  decision: {
    kind: "priority",
    player: 0,
    actions: [
      { index: 0, kind: "cast_spell", object_id: 9, label: "Cast Dark Ritual", action_ref: { kind: "cast_spell", spell_id: 9 } },
      { index: 1, kind: "cast_spell", object_id: 9, label: "Cast Dark Ritual with kicker", action_ref: { kind: "cast_spell", spell_id: 9, casting_method: { kind: "kicker" } } },
      { index: 2, kind: "activate_ability", object_id: 21, label: "Activate Llanowar Elves: Add G", action_ref: { kind: "activate_ability", source_id: 21 } },
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
  await page.addInitScript((fixture) => { window.__actionPopoverHoverFixture = fixture; }, {
    state,
    actions: state.decision.actions,
    anchorRect: { left: 520, top: 180, right: 640, bottom: 320, width: 120, height: 140 },
  });
  await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/action-popover-hand-preview.html`);
  await page.locator('[data-action-popover] [data-action-row]').first().waitFor();
  await page.locator('[data-hand-case] .game-card.hand-card').first().waitFor();
  const rows = page.locator('[data-action-popover] [data-action-row]');
  return {
    page,
    errors,
    hover: async (index) => {
      await rows.nth(index).hover();
      await page.waitForTimeout(120);
    },
    away: async () => {
      await page.mouse.move(20, 20);
      await page.waitForTimeout(120);
    },
    previewObjectId: () => page.evaluate(() => window.__hoveredObjectId ?? null),
    liftedInHand: () => page.evaluate(
      () => Array.from(document.querySelectorAll('[data-hand-case] .game-card.inspected'))
        .map((card) => card.getAttribute("data-object-id")),
    ),
    close: async () => { await browser.close(); await vite.close(); },
  };
}

test("a menu row previews its card once: in hand if the hand is showing it, otherwise in the frame", { timeout: 120000 }, async () => {
  const { hover, away, previewObjectId, liftedInHand, errors, close } = await harness();
  try {
    // A card the hand is holding answers the hover by lifting out of the fan.
    // Opening the frame preview too would say the same thing twice.
    await hover(0);
    assert.deepEqual(await liftedInHand(), ["9"]);
    assert.equal(await previewObjectId(), null, "no frame preview for a card the hand is showing");

    await hover(1);
    assert.deepEqual(await liftedInHand(), ["9"], "every way to play it lifts the same card");
    assert.equal(await previewObjectId(), null);

    // Nothing in the hand can show a battlefield permanent, so its row falls
    // back to the frame preview.
    await hover(2);
    assert.deepEqual(await liftedInHand(), [], "the hand card drops back into the fan");
    assert.equal(await previewObjectId(), "21");

    // Walking back onto a hand row closes the frame preview it left open.
    await hover(0);
    assert.deepEqual(await liftedInHand(), ["9"]);
    assert.equal(await previewObjectId(), null);

    await away();
    assert.deepEqual(await liftedInHand(), []);
    assert.equal(await previewObjectId(), null);
  } finally {
    assert.deepEqual(errors, []);
    await close();
  }
});
