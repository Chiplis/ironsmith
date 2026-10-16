import test from "node:test";
import assert from "node:assert/strict";
import { chromium } from "playwright";
import { createServer } from "vite";

test("keyboard casting browses legal targets with frames and places permanents in grid cells", { timeout: 120000 }, async () => {
  const vite = await createServer({ server: { host: "127.0.0.1", port: 0 }, logLevel: "silent" });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1400, height: 900 }, reducedMotion: "reduce" });
    const errors = [];
    page.on("pageerror", error => errors.push(error.message));
    await page.route("**/api.scryfall.com/**", route => route.abort());
    await page.route("**/cards.scryfall.io/**", route => route.abort());
    const open = async query => {
      await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/hand-cast-release.html?${query}`);
      await page.locator('.hand-card[data-object-id="5"]').waitFor();
      await page.evaluate(async () => {
        const { installHandToggleShortcut } = await import("/src/lib/hand-open.js");
        installHandToggleShortcut(document);
      });
      await page.keyboard.press(".");
      await page.keyboard.press("ArrowRight");
      await page.keyboard.press("Space");
    };
    await open("keyboardTargets");
    await page.locator('[data-keyboard-target]').first().waitFor();
    await page.keyboard.press("ArrowRight");
    assert.match(await page.locator('[data-keyboard-target]:focus').textContent(), /Grizzly Bears/);
    await page.waitForFunction(() => document.querySelector('.floating-card-preview[data-preview-object-id="20"][data-visible="true"] [data-frame-presentation="inspector"][data-render-ready="true"]'));
    const arrow = await page.evaluate(() => window.__getArrow());
    const target = await page.locator('.game-card.field-card[data-object-id="20"]').boundingBox();
    assert.ok(Math.abs(arrow.x - (target.x + target.width / 2)) < 2);
    assert.ok(Math.abs(arrow.y - (target.y + target.height / 2)) < 2);
    assert.equal(await page.locator('.hand-reveal-shell[data-open="true"]').count(), 0);
    await page.keyboard.press("ArrowDown");
    assert.match(await page.locator('[data-keyboard-target]:focus').textContent(), /Bob/);
    await page.keyboard.press("ArrowLeft");
    await page.keyboard.press("Space");
    await page.waitForFunction(() => document.querySelector('[data-keyboard-target][aria-pressed="true"]'));
    assert.equal(await page.evaluate(() => window.__dispatched.filter(command => command.type === "select_targets").length), 0);

    await open("permanent");
    const active = page.locator('[data-battlefield-drop-grid="true"] [data-battlefield-drop-slot][data-active="true"]');
    await page.keyboard.press("ArrowRight");
    await active.waitFor();
    const first = await active.evaluate(node => ({ row: Number(node.dataset.row), column: Number(node.dataset.column) }));
    await page.keyboard.press("ArrowRight");
    await page.waitForFunction(column => Number(document.querySelector('[data-battlefield-drop-slot][data-active="true"]')?.dataset.column) > column, first.column);
    await page.keyboard.press("ArrowLeft");
    await page.waitForFunction(column => Number(document.querySelector('[data-battlefield-drop-slot][data-active="true"]')?.dataset.column) === column, first.column);
    await page.keyboard.press("Space");
    await page.waitForFunction(() => window.__dispatched.some(command => command.type === "priority_action"));
    assert.equal(await page.evaluate(() => window.__drag), null);
    const committed = await page.evaluate(() => window.__placementSlots["object:5"]);
    assert.deepEqual({ row: committed.row, column: committed.column }, first);
    assert.deepEqual(errors, []);
  } finally {
    await browser.close();
    await vite.close();
  }
});
