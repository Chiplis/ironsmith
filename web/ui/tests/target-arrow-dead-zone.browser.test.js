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
