import test from "node:test";
import assert from "node:assert/strict";
import { chromium } from "playwright";
import { createServer } from "vite";

test("a fresh targeting arrow points at dead space until the mouse moves", { timeout: 120000 }, async () => {
  const vite = await createServer({ server: { host: "127.0.0.1", port: 0 }, logLevel: "silent" });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1400, height: 900 }, reducedMotion: "reduce" });
    const errors = [];
    page.on("pageerror", (error) => errors.push(String(error?.message || error)));
    await page.route("**/api.scryfall.com/**", (route) => route.abort());
    await page.route("**/cards.scryfall.io/**", (route) => route.abort());
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/target-arrow-dead-zone.html`);

    const source = page.locator('.game-card[data-object-id="10"]').first();
    await source.waitFor({ timeout: 30000 });
    const sourceBox = await source.boundingBox();
    await page.waitForFunction(() => window.__dragArrowHistory.length > 0, null, { timeout: 30000 });

    // The tip the arrow opened with, before any mouse movement could claim it.
    const arrow = await page.evaluate(() => window.__dragArrowHistory[0]);
    assert.equal(Number(arrow.fromId), 10, "the arrow leaves the spell's source");
    const onSource = arrow.x >= sourceBox.x && arrow.x <= sourceBox.x + sourceBox.width
      && arrow.y >= sourceBox.y && arrow.y <= sourceBox.y + sourceBox.height;
    assert.equal(onSource, false, "it is not collapsed onto its own source");
    const resting = await page.evaluate(
      async ([x, y]) => {
        const { aimPointIsOccupied } = await import("/src/lib/aim-dead-zone.js");
        return {
          onCard: Boolean(document.elementFromPoint(x, y)?.closest?.(".game-card")),
          occupied: aimPointIsOccupied(x, y),
        };
      },
      [arrow.x, arrow.y],
    );
    assert.deepEqual(resting, { onCard: false, occupied: false }, "aimed at nothing a click would pick");

    // Moving the mouse hands the arrow over, target or not.
    const target = page.locator('.game-card[data-object-id="20"]').first();
    const targetBox = await target.boundingBox();
    const overTarget = [targetBox.x + (targetBox.width / 2), targetBox.y + (targetBox.height / 2)];
    await page.mouse.move(overTarget[0], overTarget[1]);
    await page.waitForFunction(
      (point) => window.__dragArrow
        && Math.abs(window.__dragArrow.x - point[0]) < 2
        && Math.abs(window.__dragArrow.y - point[1]) < 2,
      overTarget,
      { timeout: 5000 },
    );
    assert.deepEqual(errors, []);
  } finally {
    await browser.close();
    await vite.close();
  }
});

test("clicking dead space abandons the cast, clicking anything else does not", { timeout: 120000 }, async () => {
  const vite = await createServer({ server: { host: "127.0.0.1", port: 0 }, logLevel: "silent" });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1400, height: 900 }, reducedMotion: "reduce" });
    const errors = [];
    page.on("pageerror", (error) => errors.push(String(error?.message || error)));
    await page.route("**/api.scryfall.com/**", (route) => route.abort());
    await page.route("**/cards.scryfall.io/**", (route) => route.abort());
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/target-arrow-dead-zone.html`);
    await page.locator('.game-card[data-object-id="10"]').first().waitFor({ timeout: 30000 });
    await page.waitForFunction(() => window.__dragArrowHistory.length > 0, null, { timeout: 30000 });
    const cancelled = () => page.evaluate(() => window.__cancelled);

    // A card is never dead space: clicking one is how a target is picked, and
    // clicking a card that is not a target still must not throw the cast away.
    for (const objectId of [20, 10]) {
      await page.locator(`.game-card[data-object-id="${objectId}"]`).first().click();
      await page.waitForTimeout(120);
      assert.equal(await cancelled(), 0, `clicking card ${objectId}`);
    }

    // Neither are the controls around the table.
    const utilityButton = page.locator(".table-persistent-utility-strip button").first();
    if (await utilityButton.count()) {
      await utilityButton.click({ force: true });
      await page.waitForTimeout(120);
      assert.equal(await cancelled(), 0, "clicking a table control");
    }

    // Dead space is anywhere on the table with nothing to pick under it.
    const dead = await page.evaluate(() => {
      const table = document.querySelector("[data-drop-zone]");
      const rect = table.getBoundingClientRect();
      const chrome = ".game-card, [data-zone-pile], [data-player-target], [data-player-drop-target],"
        + " button, a, input, select, textarea, label, [role='button'], [role='dialog'],"
        + " [data-action-popover], .interactive-card-frame-stage, .table-persistent-utility-strip";
      for (let y = Math.ceil(rect.top) + 16; y < rect.bottom - 16; y += 10) {
        for (let x = Math.ceil(rect.left) + 16; x < rect.right - 16; x += 10) {
          const el = document.elementFromPoint(x, y);
          if (!el || !table.contains(el) || el.closest(chrome)) continue;
          return { x, y };
        }
      }
      return null;
    });
    assert.ok(dead, "the table has somewhere dead to click");
    await page.mouse.click(dead.x, dead.y);
    await page.waitForFunction(() => window.__cancelled === 1, null, { timeout: 5000 });
    assert.deepEqual(errors, []);
  } finally {
    await browser.close();
    await vite.close();
  }
});

test("letting go of a hand gesture over dead space keeps the cast and its arrow", { timeout: 120000 }, async () => {
  const vite = await createServer({ server: { host: "127.0.0.1", port: 0 }, logLevel: "silent" });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1400, height: 900 }, reducedMotion: "reduce" });
    const errors = [];
    page.on("pageerror", (error) => errors.push(String(error?.message || error)));
    await page.route("**/api.scryfall.com/**", (route) => route.abort());
    await page.route("**/cards.scryfall.io/**", (route) => route.abort());
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/target-arrow-dead-zone.html`);
    await page.locator('.game-card[data-object-id="10"]').first().waitFor({ timeout: 30000 });
    const dead = await page.evaluate(() => {
      const table = document.querySelector("[data-drop-zone]");
      const rect = table.getBoundingClientRect();
      const chrome = ".game-card, [data-zone-pile], [data-player-target], [data-player-drop-target],"
        + " button, a, input, select, textarea, label, [role='button'], [role='dialog'],"
        + " [data-action-popover], .interactive-card-frame-stage, .table-persistent-utility-strip";
      for (let y = Math.ceil(rect.top) + 16; y < rect.bottom - 16; y += 10) {
        for (let x = Math.ceil(rect.left) + 16; x < rect.right - 16; x += 10) {
          const el = document.elementFromPoint(x, y);
          if (!el || !table.contains(el) || el.closest(chrome)) continue;
          return { x, y };
        }
      }
      return null;
    });
    assert.ok(dead, "the table has somewhere dead to click");

    // Hold a card, aim at dead space and let go, the way a hand drag does.
    await page.locator("[data-start-cast-gesture]").click();
    await page.waitForFunction(() => Boolean(window.__cancelled === 0));
    await page.mouse.move(dead.x + 40, dead.y + 40);
    await page.mouse.down();
    await page.mouse.move(dead.x, dead.y, { steps: 4 });
    await page.mouse.up();
    await page.waitForTimeout(400);

    // The release is not a decision to cancel: the cast is still waiting for a
    // target and its arrow is following the mouse again.
    assert.equal(await page.evaluate(() => window.__cancelled), 0, "the release must not cancel");
    await page.mouse.move(dead.x + 120, dead.y - 60);
    await page.waitForFunction(
      (point) => window.__dragArrow
        && Math.abs(window.__dragArrow.x - point[0]) < 2
        && Math.abs(window.__dragArrow.y - point[1]) < 2,
      [dead.x + 120, dead.y - 60],
      { timeout: 5000 },
    );

    // A deliberate click on dead space after that does cancel it.
    await page.mouse.click(dead.x, dead.y);
    await page.waitForFunction(() => window.__cancelled === 1, null, { timeout: 5000 });
    assert.deepEqual(errors, []);
  } finally {
    await browser.close();
    await vite.close();
  }
});
