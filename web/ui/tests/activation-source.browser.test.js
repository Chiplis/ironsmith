import test from "node:test";
import assert from "node:assert/strict";
import { chromium } from "playwright";
import { createServer } from "vite";

test("activation source stays blue during a choice and clears on cancellation", async () => {
  const vite = await createServer({server: {host: "127.0.0.1", port: 0}, logLevel: "silent"});
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({viewport: {width: 1200, height: 900}});
    const errors = [];
    page.on("pageerror", error => errors.push(error.message));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/battlefield-stable-slots.html`);
    await page.getByRole("button", {name: "Activate source"}).click();
    const source = page.locator('.game-card[data-object-id="1"]');
    await page.waitForFunction(() => document.querySelector('.game-card[data-object-id="1"]')?.classList.contains("activation-source"));
    assert.equal(await page.locator('.game-card.activation-source').count(), 1);
    assert.equal(await source.evaluate(el => getComputedStyle(el).outlineColor), "rgb(88, 182, 255)");
    await page.getByRole("button", {name: "Cancel activation"}).click();
    await page.waitForFunction(() => !document.querySelector('.game-card.activation-source'));
    assert.deepEqual(errors, []);
  } finally {
    await browser.close();
    await vite.close();
  }
});
