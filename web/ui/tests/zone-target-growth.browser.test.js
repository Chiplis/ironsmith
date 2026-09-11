import test from "node:test";
import assert from "node:assert/strict";
import { chromium } from "playwright";
import { createServer } from "vite";

const slotScale = async (page) => page.evaluate(() => {
  const slot = document.querySelector(".zone-pile-slot");
  return slot.getBoundingClientRect().width / slot.offsetWidth;
});

test("a zone holding a legal target grows by half, smoothly, and hands over when opened", { timeout: 120000 }, async () => {
  const vite = await createServer({ server: { host: "127.0.0.1", port: 0 }, logLevel: "silent" });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1000, height: 800 } });
    const errors = [];
    page.on("pageerror", (error) => errors.push(String(error?.message || error)));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/zone-piles.html`);
    const graveyard = page.locator('[data-zone-pile="graveyard"]');
    const exile = page.locator('[data-zone-pile="exile"]');
    const slot = page.locator(".zone-pile-slot").first();
    await graveyard.waitFor();
    await page.waitForTimeout(300);

    // At priority there is nothing to pick, so the piles sit at their own size.
    assert.equal(await graveyard.getAttribute("data-has-targets"), null);
    assert.ok(Math.abs(await slotScale(page) - 1) < 0.02, "unscaled at rest");
    const resting = await slot.boundingBox();
    const restingExile = await exile.boundingBox();
    // The growth is animated rather than snapped.
    assert.match(await slot.evaluate((el) => getComputedStyle(el).transitionProperty), /transform/);
    assert.match(await slot.evaluate((el) => getComputedStyle(el).transitionDuration), /0\.2\d*s/);

    await page.getByRole("button", { name: "Toggle targeting" }).click();
    assert.equal(await graveyard.getAttribute("data-has-targets"), "true");
    await page.waitForFunction(() => {
      const el = document.querySelector(".zone-pile-slot");
      return Math.abs((el.getBoundingClientRect().width / el.offsetWidth) - 1.5) < 0.02;
    }, null, { timeout: 5000 });

    const grown = await slot.boundingBox();
    assert.ok(Math.abs(grown.width / resting.width - 1.5) < 0.02, JSON.stringify({ resting, grown }));
    assert.ok(Math.abs(grown.height / resting.height - 1.5) < 0.02, "and taller in proportion");
    // It grows away from the corner it is anchored in, so the pile beside it
    // keeps its place.
    assert.ok(Math.abs((grown.x + grown.width) - (resting.x + resting.width)) < 1.5, "right edge held");
    assert.ok(Math.abs((grown.y + grown.height) - (resting.y + resting.height)) < 1.5, "bottom edge held");
    const exileNow = await exile.boundingBox();
    assert.equal(await exile.getAttribute("data-has-targets"), null, "nothing to pick in exile");
    assert.ok(Math.abs(exileNow.width - restingExile.width) < 1, "so exile stays its own size");
    assert.ok(Math.abs(exileNow.y - restingExile.y) < 1, "and stays put");

    // Opening the zone hands over to the strip, which is placed against the
    // pile's own box, so the pile returns to size.
    await graveyard.hover();
    await page.locator(".zone-pile-menu").waitFor();
    await page.waitForFunction(() => {
      const el = document.querySelector(".zone-pile-slot");
      return Math.abs((el.getBoundingClientRect().width / el.offsetWidth) - 1) < 0.02;
    }, null, { timeout: 5000 });

    // Leaving it alone again brings the growth back.
    await page.mouse.move(900, 740);
    await page.locator(".zone-pile-menu").waitFor({ state: "hidden" });
    await page.waitForFunction(() => {
      const el = document.querySelector(".zone-pile-slot");
      return Math.abs((el.getBoundingClientRect().width / el.offsetWidth) - 1.5) < 0.02;
    }, null, { timeout: 5000 });
    assert.deepEqual(errors, []);
  } finally {
    await browser.close();
    await vite.close();
  }
});
