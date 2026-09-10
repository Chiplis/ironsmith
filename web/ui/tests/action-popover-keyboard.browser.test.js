import test from "node:test";
import assert from "node:assert/strict";
import { chromium } from "playwright";
import { createServer } from "vite";

const actions = ["Cast Dark Ritual", "Cast Dark Ritual with kicker", "Cast Dark Ritual without paying its mana cost"]
  .map((label, index) => ({
    index,
    kind: "cast_spell",
    object_id: 9,
    label,
    action_ref: { kind: "cast_spell", spell_id: 9, casting_method: { kind: `method-${index}` } },
  }));

test("the picker opens on its first row and walks with the arrow keys", { timeout: 120000 }, async () => {
  const vite = await createServer({ server: { host: "127.0.0.1", port: 0 }, logLevel: "silent" });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 900, height: 700 }, reducedMotion: "reduce" });
    const errors = [];
    page.on("pageerror", (error) => errors.push(String(error?.message || error)));
    await page.addInitScript((fixture) => { window.__popoverFixture = fixture; }, {
      actions,
      anchorRect: { left: 380, top: 420, right: 500, bottom: 560, width: 120, height: 140 },
    });
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/action-popover-keyboard.html`);
      const rows = page.locator('[data-action-popover] [data-action-row]');
    await rows.first().waitFor();
    assert.equal(await rows.count(), 3, "every way to play the card is its own row");

    const focused = () => page.evaluate(() => document.activeElement?.textContent?.trim() || null);
    // The player pressed a key to get here, so the first row is already theirs
    // to confirm without tabbing into the popover.
    await page.waitForFunction(() => document.activeElement?.hasAttribute?.("data-action-row"));
    assert.equal(await focused(), "Cast Dark Ritual");

    await page.keyboard.press("ArrowDown");
    assert.equal(await focused(), "Cast Dark Ritual with kicker");
    await page.keyboard.press("ArrowDown");
    assert.equal(await focused(), "Cast Dark Ritual without paying its mana cost");
    // The walk wraps, so a long list never dead-ends at either edge.
    await page.keyboard.press("ArrowDown");
    assert.equal(await focused(), "Cast Dark Ritual");
    await page.keyboard.press("ArrowUp");
    assert.equal(await focused(), "Cast Dark Ritual without paying its mana cost");
    await page.keyboard.press("Home");
    assert.equal(await focused(), "Cast Dark Ritual");
    await page.keyboard.press("End");
    assert.equal(await focused(), "Cast Dark Ritual without paying its mana cost");

    // Arrow keys must not leak to the board's own card-to-card navigation.
    assert.equal(await page.evaluate(() => window.__chosen.length), 0);

    await page.keyboard.press("ArrowUp");
    await page.waitForTimeout(200); // The open guard ignores the press that opened it.
    await page.keyboard.press(" ");
    assert.deepEqual(await page.evaluate(() => window.__chosen), ["Cast Dark Ritual with kicker"]);
    assert.deepEqual(errors, []);
  } finally {
    await browser.close();
    await vite.close();
  }
});
