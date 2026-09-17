import test from "node:test";
import assert from "node:assert/strict";
import { chromium } from "playwright";
import { createServer } from "vite";

async function withFixture(scenario, run) {
  const vite = await createServer({ server: { host: "127.0.0.1", port: 0 }, logLevel: "silent" });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1400, height: 900 }, reducedMotion: "reduce" });
    const errors = [];
    page.on("pageerror", (error) => errors.push(String(error?.message || error)));
    await page.route("**/api.scryfall.com/**", (route) => route.abort());
    await page.route("**/cards.scryfall.io/**", (route) => route.abort());
    await page.goto(
      `http://127.0.0.1:${vite.httpServer.address().port}/tests/forced-target-auto-pick.html?scenario=${scenario}`
    );
    await page.locator(".stack-card").first().waitFor({ timeout: 30000 });
    await run(page);
    assert.deepEqual(errors, [], "no page errors");
  } finally {
    await browser.close();
    await vite.close();
  }
}

// The requirement is deliberately left active so the aiming arrow survives, so
// a placed target reads as a pressed option rather than a completed chip.
const placedTargets = (page) => page.$$eval(
  '.decision-option-row[aria-pressed="true"]',
  (nodes) => nodes.map((node) => node.textContent.trim()),
);

test("the only legal target is placed for the player, who still submits", { timeout: 120000 }, async () => {
  await withFixture("forced-opponent", async (page) => {
    await page.waitForSelector('.decision-option-row[aria-pressed="true"]', { timeout: 10000 });
    assert.deepEqual(await placedTargets(page), ["Bob"], "the lone opponent is placed");
    assert.equal(await page.evaluate(() => window.__dispatched), null,
      "placing a forced target must not submit the decision");
    assert.equal(await page.evaluate(() => window.__cancelled), 0, "and must not cancel it");
  });
});

test("a lone legal creature on the battlefield is placed too", { timeout: 120000 }, async () => {
  await withFixture("forced-creature", async (page) => {
    await page.waitForSelector('.decision-option-row[aria-pressed="true"]', { timeout: 10000 });
    assert.deepEqual(await placedTargets(page), ["Goblin Piker"]);
    assert.equal(await page.evaluate(() => window.__dispatched), null,
      "placing a forced target must not submit the decision");
  });
});

test("a real choice between opponents is left alone", { timeout: 120000 }, async () => {
  await withFixture("two-opponents", async (page) => {
    await page.waitForTimeout(600);
    assert.deepEqual(await placedTargets(page), [],
      "two legal players is a decision, not a forced target");
    assert.equal(await page.evaluate(() => window.__dispatched), null);
  });
});

test("an optional target is a choice even when only one is legal", { timeout: 120000 }, async () => {
  await withFixture("optional-opponent", async (page) => {
    await page.waitForTimeout(600);
    assert.deepEqual(await placedTargets(page), [],
      "'up to one target' must stay up to the player");
    assert.equal(await page.evaluate(() => window.__dispatched), null);
  });
});

test("a stack tile stays readable while a targets decision owns the click", { timeout: 120000 }, async () => {
  await withFixture("forced-opponent", async (page) => {
    const tile = page.locator(".stack-card").first();
    await tile.hover();
    // The tile's click is spoken for by the decision; hover is what hands the
    // spell to the inspector, and it resolves to the card, not the stack object.
    await page.waitForFunction(() => window.__hoveredObjectId === "10", null, { timeout: 5000 });
    await page.mouse.move(5, 5);
    await page.waitForFunction(() => window.__hoveredObjectId == null, null, { timeout: 5000 });
  });
});
