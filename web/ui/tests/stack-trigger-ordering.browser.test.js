import test from "node:test";
import assert from "node:assert/strict";
import { chromium } from "playwright";
import { createServer } from "vite";

// Pending triggers wait above the live stack while a player orders them. Each
// one previews and inspects the object it came from, and its arrows reorder
// it without also inspecting it.
test("pending trigger tiles preview and inspect their source object", async () => {
  const vite = await createServer({ server: { host: "127.0.0.1", port: 0 }, logLevel: "silent" });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1100, height: 800 } });
    const errors = [];
    page.on("pageerror", (e) => errors.push(e.message));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/stack-trigger-ordering.html`);
    const pending = page.locator('.stack-card[data-pending-trigger="true"]');
    await pending.nth(2).waitFor();
    assert.equal(await pending.count(), 3, "three triggers wait to be ordered");
    assert.deepEqual(
      await page.locator(".stack-timeline-entry .stack-card-title").allTextContents(),
      ["Blood Artist", "Blood Artist", "Zulaport Cutthroat", "Lightning Bolt", "Zulaport Cutthroat"],
      "pending triggers sit above the live stack",
    );
    assert.equal(await page.locator(".stack-timeline-entry .stack-card-position").first().textContent(), "Top");
    assert.equal(await page.locator('.stack-panel-count [aria-hidden="true"]').textContent(), "5");
    assert.match(await page.locator(".stack-panel-count").textContent(), /Stack entries: 5/, "the full count label stays in the text");

    const previewFor = async (tile) => {
      await tile.hover();
      await page.waitForFunction(() => document.querySelector('[data-card-hover-preview][data-visible="true"]'));
      return page.evaluate(() => document.querySelector('[data-card-hover-preview][data-visible="true"]').dataset.previewObjectId);
    };
    assert.equal(await previewFor(pending.nth(0)), "40", "a Blood Artist trigger previews Blood Artist");
    // Blood Artist has died: the graveyard opens on its own and the card it
    // holds for the trigger glows and grows by a third, animated.
    const sourceRow = page.locator('.zone-pile-menu .zone-pile-card-row[data-object-id="40"]');
    await sourceRow.waitFor({ timeout: 5000 });
    assert.equal(await sourceRow.getAttribute("data-hover-source"), "true");
    assert.equal(await page.locator('.zone-pile-menu .zone-pile-card-row[data-object-id="42"]').getAttribute("data-hover-source"), null, "its neighbour is left alone");
    assert.match(await sourceRow.evaluate((el) => getComputedStyle(el).boxShadow), /rgba\(255, 255, 255/, "white glow");
    const sourceSlot = page.locator('.zone-pile-menu .zone-pile-card-slot[data-hover-source="true"]');
    assert.match(await sourceSlot.evaluate((el) => getComputedStyle(el).transitionProperty), /width/);
    await page.waitForFunction(() => {
      const grown = document.querySelector('.zone-pile-menu .zone-pile-card-slot[data-hover-source="true"]');
      const plain = document.querySelector('.zone-pile-menu .zone-pile-card-slot:not([data-hover-source="true"])');
      return grown && plain && Math.abs((grown.getBoundingClientRect().width / plain.getBoundingClientRect().width) - 1.3) < 0.03;
    }, null, { timeout: 5000 });
    await page.mouse.move(900, 700);
    await page.waitForFunction(() => !document.querySelector('[data-card-hover-preview][data-visible="true"]'));
    assert.equal(await previewFor(pending.nth(2)), "41", "the Cutthroat trigger previews Zulaport Cutthroat");

    await pending.nth(2).click();
    assert.deepEqual(await page.evaluate(() => window.__inspections), [{ id: "41", entryId: "trigger-order-2" }]);

    await pending.nth(0).locator(".stack-card-reorder-button-down").click();
    assert.deepEqual(
      (await page.locator(".stack-timeline-entry .stack-card-title").allTextContents()).slice(0, 3),
      ["Blood Artist", "Blood Artist", "Zulaport Cutthroat"],
    );
    assert.deepEqual(
      await page.locator('.stack-card[data-pending-trigger="true"]').evaluateAll((nodes) => nodes.map((node) => node.dataset.objectId)),
      ["trigger-order-1", "trigger-order-0", "trigger-order-2"],
      "the down arrow moves the first trigger one step toward the bottom",
    );
    assert.equal((await page.evaluate(() => window.__inspections)).length, 1, "an arrow click does not inspect");
    assert.equal(await pending.nth(0).locator(".stack-card-reorder-button-up").isDisabled(), true, "the top trigger cannot move up");
    assert.equal(await pending.nth(2).locator(".stack-card-reorder-button-down").isDisabled(), true, "the last trigger cannot move down");
    assert.deepEqual(errors, []);
  } finally {
    await browser.close();
    await vite.close();
  }
});
