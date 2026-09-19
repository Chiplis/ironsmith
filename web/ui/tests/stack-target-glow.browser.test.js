import test from "node:test";
import assert from "node:assert/strict";
import { chromium } from "playwright";
import { createServer } from "vite";

test("stack tiles light up and pick as targets for the spell they stand for", { timeout: 90000 }, async () => {
  const vite = await createServer({ server: { host: "127.0.0.1", port: 0 }, logLevel: "silent" });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1000, height: 700 } });
    const errors = [];
    page.on("pageerror", (error) => errors.push(error.message));
    await page.route("**/api.scryfall.com/**", (route) => route.fulfill({ json: { name: "x", image_uris: {} } }));
    await page.route("**/cards/*.json", (route) => route.fulfill({ status: 404, body: "" }));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/stack-target-glow.html`);
    const bolt = page.locator('.stack-card[data-object-id="272"]');
    const fanatic = page.locator('.stack-card[data-object-id="45"]');
    await bolt.waitFor();
    await page.evaluate(() => {
      window.__choices = [];
      window.addEventListener("ironsmith:target-choice", (event) => window.__choices.push(event.detail));
    });

    // A live targets decision: only the spell tile is a legal target, even
    // though the ability's permanent and the spell's drawn id are both legal.
    await page.waitForSelector('.stack-card.card-targeting-mode.target-legal[data-object-id="272"]');
    assert.equal(await fanatic.evaluate((node) => node.classList.contains("card-targeting-mode")), true);
    assert.equal(await fanatic.evaluate((node) => node.classList.contains("target-legal")), false);
    assert.equal(await bolt.getAttribute("data-target-object-ids"), "136");
    assert.equal(await fanatic.getAttribute("data-target-object-ids"), "");
    const boltGlow = await bolt.evaluate((node) => getComputedStyle(node).boxShadow);
    assert.match(boltGlow, /255, 255, 255/, "legal stack tile glows white");

    // The pick lands on pointerdown and the trailing click neither undoes it
    // nor turns into an inspector request.
    await bolt.click();
    await page.waitForTimeout(100);
    assert.deepEqual(await page.evaluate(() => window.__choices), [{ target: { kind: "object", object: 136 } }]);
    assert.deepEqual(await page.evaluate(() => window.__stackClicks || []), []);
    await fanatic.click();
    await page.waitForTimeout(100);
    assert.equal(await page.evaluate(() => window.__choices.length), 1, "an ability tile is never a target pick");
    assert.deepEqual(await page.evaluate(() => window.__stackClicks), [{ id: 134, stackId: 45 }]);

    // A provisional hand gesture: the preview the gesture carries lights the
    // tile, and hovering it with the held card marks it the way a creature is.
    await page.getByRole("button", { name: "Priority" }).click();
    await page.waitForFunction(() => !document.querySelector(".stack-card.card-targeting-mode"));
    await page.getByRole("button", { name: "Start gesture" }).click();
    await page.waitForSelector('.stack-card.card-targeting-mode[data-object-id="272"]');
    assert.equal(await bolt.evaluate((node) => node.classList.contains("target-legal")), false, "no preview yet");
    await page.getByRole("button", { name: "Preview targets" }).click();
    await page.waitForSelector('.stack-card.card-targeting-mode.target-legal[data-object-id="272"]');
    assert.equal(await fanatic.evaluate((node) => node.classList.contains("target-legal")), false);
    const box = await bolt.boundingBox();
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2, { steps: 4 });
    await page.waitForSelector('.stack-card.target-legal.hovered[data-object-id="272"]');
    const [hoveredTranslate, hoveredShadow] = await bolt.evaluate((node) => {
      const style = getComputedStyle(node);
      return [style.translate, style.boxShadow];
    });
    assert.ok(["none", "0px", "0px 0px"].includes(hoveredTranslate), `stack tile stays put in its list (${hoveredTranslate})`);
    assert.notEqual(hoveredShadow, "none");
    await page.mouse.move(box.x + box.width + 200, box.y + box.height / 2, { steps: 4 });
    await page.waitForFunction(() => !document.querySelector(".stack-card.hovered"));
    await page.getByRole("button", { name: "End gesture" }).click();
    await page.waitForFunction(() => !document.querySelector(".stack-card.card-targeting-mode"));

    assert.deepEqual(errors, []);
  } finally {
    await browser.close();
    await vite.close();
  }
});
