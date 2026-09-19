import test from "node:test";
import assert from "node:assert/strict";
import { chromium } from "playwright";
import { createServer } from "vite";

test("hovering a stack object opens the pile holding its target and marks that card", { timeout: 90000 }, async () => {
  const vite = await createServer({ server: { host: "127.0.0.1", port: 0 }, logLevel: "silent" });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1000, height: 600 } });
    const errors = [];
    page.on("pageerror", (error) => errors.push(String(error?.message || error)));
    await page.route("**/api.scryfall.com/**", (route) => route.fulfill({ json: { name: "x", image_uris: {} } }));
    await page.route("**/cards/*.json", (route) => route.fulfill({ status: 404, body: "" }));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/stack-target-pile.html`);
    const reanimate = page.locator('.stack-card[data-object-id="200"]');
    const bolt = page.locator('.stack-card[data-object-id="210"]');
    const strip = page.locator(".zone-pile-menu");
    await reanimate.waitFor();
    await page.locator('[data-zone-pile="graveyard"]').waitFor();
    await page.waitForTimeout(200);
    assert.equal(await strip.count(), 0, "piles are shut at rest");

    // Aimed at a graveyard card: the graveyard opens by itself and the card
    // the spell points at is the one lit up and enlarged.
    await reanimate.hover();
    await strip.waitFor({ timeout: 5000 });
    const target = strip.locator('.zone-pile-card-row[data-stack-target="true"]');
    await target.waitFor();
    assert.equal(await target.count(), 1, "exactly one card is marked");
    assert.equal(await target.getAttribute("data-object-id"), "30");
    assert.equal(await strip.locator('.zone-pile-card-row[data-object-id="60"]').count(), 0, "it is the graveyard, not exile");
    await page.waitForFunction(() => {
      const marked = document.querySelector('.zone-pile-card-slot[data-stack-target="true"]');
      const other = document.querySelector('.zone-pile-card-list .zone-pile-card-slot:not([data-stack-target="true"])');
      return marked && other && Math.abs(marked.getBoundingClientRect().width / other.getBoundingClientRect().width - 1.18) < 0.02;
    }, null, { timeout: 5000 });
    const glow = await target.evaluate((node) => getComputedStyle(node).boxShadow);
    assert.match(glow, /255, 255, 255/, `the marked card glows white: ${glow}`);
    assert.equal(await target.evaluate((node) => getComputedStyle(node).borderTopColor), "rgb(255, 255, 255)");
    // A buried target is scrolled into view rather than left off the edge.
    const visible = await target.evaluate((node) => {
      const list = node.closest(".zone-pile-card-list");
      const box = node.getBoundingClientRect();
      const bounds = list.getBoundingClientRect();
      return box.left >= bounds.left - 1 && box.right <= bounds.right + 1;
    });
    assert.ok(visible, "the marked card is within the strip's scroll window");
    const neighbourGlow = await strip.locator('.zone-pile-card-row[data-object-id="31"]').evaluate((node) => getComputedStyle(node).boxShadow);
    assert.equal(neighbourGlow, "none", "its neighbours are untouched");

    // Leaving the stack object lets the pile close again.
    await page.mouse.move(600, 560);
    await page.waitForFunction(() => !document.querySelector(".zone-pile-menu"), null, { timeout: 5000 });

    // A spell aimed at a permanent opens nothing.
    await bolt.hover();
    await page.waitForTimeout(400);
    assert.equal(await strip.count(), 0, "no pile holds Bolt's target");
    assert.deepEqual(errors, []);
  } finally {
    await browser.close();
    await vite.close();
  }
});
