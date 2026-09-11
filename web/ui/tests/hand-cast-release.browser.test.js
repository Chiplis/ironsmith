import test from "node:test";
import assert from "node:assert/strict";
import { chromium } from "playwright";
import { createServer } from "vite";

/** Somewhere on the table that holds nothing a click could pick. */
const deadSpace = (page) => page.evaluate(async () => {
  const { castHoverTargetAtPoint } = await import("/src/lib/hand-drag-intent.js");
  const mine = document.querySelector("[data-my-zone]");
  const rect = mine.getBoundingClientRect();
  const chrome = ".game-card, button, a, input, [role='button'], [role='dialog'],"
    + " [data-action-popover], .interactive-card-frame-stage, .table-persistent-utility-strip";
  for (let y = Math.ceil(rect.top) + 10; y < rect.bottom - 10; y += 8) {
    for (let x = Math.ceil(rect.left) + 10; x < rect.right - 10; x += 8) {
      const el = document.elementFromPoint(x, y);
      if (!el || !el.closest("[data-drop-zone]") || el.closest(chrome)) continue;
      if (castHoverTargetAtPoint(x, y)) continue;
      return { x, y };
    }
  }
  return null;
});

/** Opens the fixture and drags the spell out of the hand onto dead space. */
async function dragToDeadSpace(page, baseUrl, { twoOptions = false } = {}) {
  const errors = [];
  page.on("pageerror", (error) => errors.push(String(error?.message || error)));
  await page.route("**/api.scryfall.com/**", (route) => route.abort());
  await page.route("**/cards.scryfall.io/**", (route) => route.abort());
  await page.goto(`${baseUrl}/tests/hand-cast-release.html${twoOptions ? "?two" : ""}`);
  const hand = page.locator('.game-card.hand-card[data-object-id="5"]');
  await hand.waitFor({ timeout: 30000 });
  const handBox = await hand.boundingBox();
  const dead = await deadSpace(page);
  assert.ok(dead, "the table has somewhere dead to let go over");
  // The card's centre sits below the viewport, so take it by its top edge.
  await page.mouse.move(handBox.x + (handBox.width / 2), handBox.y + 24);
  await page.mouse.down();
  await page.mouse.move(dead.x, dead.y, { steps: 12 });
  await page.waitForFunction(() => document.querySelectorAll(".cast-intent-drag-arrow").length === 1);
  return { dead, errors };
}

test("letting go of a targeted cast over dead space keeps it aimable", { timeout: 120000 }, async () => {
  const vite = await createServer({ server: { host: "127.0.0.1", port: 0 }, logLevel: "silent" });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1400, height: 900 }, reducedMotion: "reduce" });
    const errors = [];
    page.on("pageerror", (error) => errors.push(String(error?.message || error)));
    await page.route("**/api.scryfall.com/**", (route) => route.abort());
    await page.route("**/cards.scryfall.io/**", (route) => route.abort());
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/hand-cast-release.html`);

    const hand = page.locator('.game-card.hand-card[data-object-id="5"]');
    await hand.waitFor({ timeout: 30000 });
    const handBox = await hand.boundingBox();
    const dead = await deadSpace(page);
    assert.ok(dead, "the table has somewhere dead to let go over");

    // Drag the spell out of the hand. Leaving the hand casts it, and the
    // engine comes back asking for a target.
    await page.mouse.move(handBox.x + (handBox.width / 2), handBox.y + 24);
    await page.mouse.down();
    await page.mouse.move(dead.x, dead.y, { steps: 12 });
    await page.waitForFunction(() => window.__dispatched.some((command) => command?.type === "priority_action"));
    await page.waitForFunction(() => document.body.innerText.includes("Submit Targets"));
    assert.ok(await page.locator(".cast-intent-drag-arrow").count(), "the gesture draws its own arrow");

    // Letting go over dead space must not throw the cast away.
    await page.mouse.up();
    await page.waitForTimeout(400);
    assert.equal(await page.evaluate(() => window.__cancelled), 0, "the release must not cancel the cast");
    assert.ok(await page.evaluate(() => document.body.innerText.includes("Submit Targets")), "still asking for a target");
    assert.equal(await page.locator(".cast-intent-drag-arrow").count(), 0, "the gesture's own arrow is gone");

    // The decision's arrow takes over and follows the mouse, so the player can
    // keep aiming without holding anything down.
    await page.waitForFunction(() => window.__arrow != null, null, { timeout: 5000 });
    assert.equal(await page.evaluate(() => window.__arrow.fromId), 90, "aimed from the spell on the stack");
    for (const point of [[dead.x + 120, dead.y - 60], [dead.x - 40, dead.y + 30]]) {
      await page.mouse.move(point[0], point[1]);
      await page.waitForFunction(
        (aim) => window.__arrow && Math.abs(window.__arrow.x - aim[0]) < 2 && Math.abs(window.__arrow.y - aim[1]) < 2,
        point,
        { timeout: 5000 },
      );
    }
    assert.equal(await page.evaluate(() => window.__cancelled), 0);

    // Only a deliberate click on dead space abandons it.
    await page.mouse.click(dead.x, dead.y);
    await page.waitForFunction(() => window.__cancelled === 1, null, { timeout: 5000 });
    assert.deepEqual(errors, []);
  } finally {
    await browser.close();
    await vite.close();
  }
});

test("letting go on a legal target still casts at it", { timeout: 120000 }, async () => {
  const vite = await createServer({ server: { host: "127.0.0.1", port: 0 }, logLevel: "silent" });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1400, height: 900 }, reducedMotion: "reduce" });
    const errors = [];
    page.on("pageerror", (error) => errors.push(String(error?.message || error)));
    await page.route("**/api.scryfall.com/**", (route) => route.abort());
    await page.route("**/cards.scryfall.io/**", (route) => route.abort());
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/hand-cast-release.html`);

    const hand = page.locator('.game-card.hand-card[data-object-id="5"]');
    await hand.waitFor({ timeout: 30000 });
    const handBox = await hand.boundingBox();
    const targetBox = await page.locator('.game-card[data-object-id="20"]').first().boundingBox();

    await page.mouse.move(handBox.x + (handBox.width / 2), handBox.y + 24);
    await page.mouse.down();
    await page.mouse.move(targetBox.x + (targetBox.width / 2), targetBox.y + (targetBox.height / 2), { steps: 12 });
    await page.waitForFunction(() => document.body.innerText.includes("Submit Targets"));
    await page.mouse.up();

    // The drop is read as the choice, so the target is submitted rather than
    // left to be aimed at again.
    await page.waitForFunction(
      () => window.__dispatched.some((command) => command?.type === "select_targets"),
      null,
      { timeout: 5000 },
    );
    assert.match(await page.locator("body").innerText(), /Submit Targets \(1/);
    assert.equal(await page.evaluate(() => window.__cancelled), 0);
    assert.deepEqual(errors, []);
  } finally {
    await browser.close();
    await vite.close();
  }
});

test("a cast with several ways to pay keeps aiming after the button comes up", { timeout: 120000 }, async () => {
  const vite = await createServer({ server: { host: "127.0.0.1", port: 0 }, logLevel: "silent" });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1400, height: 900 }, reducedMotion: "reduce" });
    const baseUrl = `http://127.0.0.1:${vite.httpServer.address().port}`;
    const { dead, errors } = await dragToDeadSpace(page, baseUrl, { twoOptions: true });

    // Nothing has entered the engine yet: the gesture is still provisional.
    assert.deepEqual(await page.evaluate(() => window.__dispatched), []);
    await page.mouse.up();
    await page.waitForTimeout(300);

    // Letting go over dead space must not put the card back: the gesture
    // carries on without the button, still drawing its arrow.
    assert.equal(await page.locator(".cast-intent-drag-arrow").count(), 1, "the arrow stays");
    const held = await page.evaluate(() => window.__drag);
    assert.ok(held, "the card is still in hand and still being aimed");
    assert.equal(held.held, true);
    assert.equal(held.castIntent, true);

    // The aim follows the bare pointer.
    await page.mouse.move(dead.x + 100, dead.y - 50);
    await page.waitForFunction(
      (point) => window.__drag && window.__drag.x === point[0] && window.__drag.y === point[1],
      [dead.x + 100, dead.y - 50],
      { timeout: 5000 },
    );

    // Clicking a legal target resolves the gesture, and because there is more
    // than one way to cast it, the picker asks which.
    const target = await page.locator('.game-card[data-object-id="20"]').first().boundingBox();
    await page.mouse.click(target.x + (target.width / 2), target.y + (target.height / 2));
    await page.waitForFunction(
      () => document.querySelectorAll("[data-action-popover] [data-action-row]").length === 2,
      null,
      { timeout: 5000 },
    );
    assert.equal(await page.evaluate(() => window.__cancelled), 0);
    assert.deepEqual(errors, []);
  } finally {
    await browser.close();
    await vite.close();
  }
});

test("clicking dead space again lets a provisional cast go", { timeout: 120000 }, async () => {
  const vite = await createServer({ server: { host: "127.0.0.1", port: 0 }, logLevel: "silent" });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1400, height: 900 }, reducedMotion: "reduce" });
    const baseUrl = `http://127.0.0.1:${vite.httpServer.address().port}`;
    const { dead, errors } = await dragToDeadSpace(page, baseUrl, { twoOptions: true });
    await page.mouse.up();
    await page.waitForFunction(() => window.__drag?.held === true);

    // A second release over dead space is the player saying no.
    await page.mouse.click(dead.x + 100, dead.y - 50);
    await page.waitForFunction(() => window.__drag === null, null, { timeout: 5000 });
    assert.equal(await page.locator(".cast-intent-drag-arrow").count(), 0, "the arrow is gone");
    assert.equal(await page.locator('.game-card.hand-card[data-object-id="5"]').count(), 1, "the card stayed in hand");
    assert.deepEqual(await page.evaluate(() => window.__dispatched), [], "and nothing was cast");
    assert.deepEqual(errors, []);
  } finally {
    await browser.close();
    await vite.close();
  }
});
