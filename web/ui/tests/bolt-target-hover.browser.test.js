import test from "node:test";
import assert from "node:assert/strict";
import { chromium } from "playwright";
import { createServer } from "vite";

test("default Lightning Bolt hover does not remeasure zone piles", { timeout: 120000 }, async (t) => {
  const vite = await createServer({ server: { host: "127.0.0.1", port: 0 }, logLevel: "silent" });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
    const errors = [];
    page.on("pageerror", error => errors.push(error.message));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}`);
    await page.locator(".decision-main-button:visible").first().waitFor({ timeout: 90000 });
    await page.waitForTimeout(5000);
    // Default opening: keep, pregame, draw, first main phase.
    for (let i = 0; i < 4; i++) {
      await page.locator(".decision-main-button:visible").first().click();
      await page.waitForTimeout(700);
    }
    const bolt = page.locator('.hand-card[data-card-name="Lightning Bolt"]').first();
    await bolt.hover();
    await page.waitForTimeout(250);
    const box = await bolt.boundingBox();
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
    await page.mouse.down();
    await page.mouse.move(box.x + box.width / 2, box.y - 90, { steps: 5 });
    await page.waitForTimeout(900);
    const targets = await page.locator(".game-card.field-card.target-legal").evaluateAll(cards => cards
      .filter(card => card.getBoundingClientRect().width > 0).slice(0, 4)
      .map(card => {
        const rect = card.getBoundingClientRect();
        return { id: card.dataset.objectId, x: rect.x + rect.width / 2, y: rect.y + rect.height / 2 };
      }));
    assert.equal(targets.length, 4);
    await page.evaluate(() => {
      window.__pileReads = 0;
      for (const pile of document.querySelectorAll(".player-zone-piles")) {
        const measure = pile.getBoundingClientRect.bind(pile);
        pile.getBoundingClientRect = () => { window.__pileReads++; return measure(); };
      }
      window.__hoverFrames = [];
      let last = performance.now();
      const tick = at => {
        window.__hoverFrames.push(at - last);
        last = at;
        window.__hoverRaf = requestAnimationFrame(tick);
      };
      window.__hoverRaf = requestAnimationFrame(tick);
    });
    for (let i = 0; i < 24; i++) {
      const target = targets[i % targets.length];
      await page.mouse.move(target.x, target.y);
      await page.waitForFunction(id => document.querySelector(`.game-card.target-legal.hovered[data-object-id="${id}"]`), target.id);
    }
    const result = await page.evaluate(() => {
      cancelAnimationFrame(window.__hoverRaf);
      return { reads: window.__pileReads, maxFrame: Math.max(...window.__hoverFrames) };
    });
    assert.equal(result.reads, 0, "target highlights must not trigger zone-pile geometry reads");
    t.diagnostic(`Maximum frame interval across 24 target switches: ${result.maxFrame.toFixed(1)} ms`);
    await page.keyboard.press("Escape");
    await page.mouse.up();
    assert.deepEqual(errors, []);
  } finally {
    await browser.close();
    await vite.close();
  }
});
