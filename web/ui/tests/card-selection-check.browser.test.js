import test from "node:test";
import assert from "node:assert/strict";
import { chromium } from "playwright";
import { createServer } from "vite";

test("chosen cards carry a check that is the only way to unchoose them", async () => {
  const vite = await createServer({server: {host: "127.0.0.1", port: 0}, logLevel: "silent"});
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({viewport: {width: 1200, height: 900}});
    const errors = [];
    page.on("pageerror", (error) => errors.push(error.message));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-selection-check.html`);
    const submit = page.getByRole("button", {name: /^Submit \(/});
    await submit.waitFor();
    assert.equal(await submit.textContent(), "Submit (0/0-2)");

    // A click anywhere on a battlefield card chooses it and checks it.
    const fieldCard = page.locator('.game-card[data-object-id="10"]');
    await fieldCard.click();
    await page.waitForFunction(() => document.querySelectorAll(".card-selection-check").length === 1);
    assert.equal(await submit.textContent(), "Submit (1/0-2)");
    assert.equal(await page.locator('.decision-option-row[aria-pressed="true"]').textContent(), "Llanowar Elves");

    // Clicking the card again keeps the choice: only the check gives it back.
    await fieldCard.click({position: {x: 40, y: 120}});
    await page.waitForTimeout(150);
    assert.equal(await submit.textContent(), "Submit (1/0-2)");
    await fieldCard.locator(".card-selection-check").click();
    await page.waitForFunction(() => document.querySelectorAll(".card-selection-check").length === 0);
    assert.equal(await submit.textContent(), "Submit (0/0-2)");

    // The same holds for cards inside a zone, which stays open across picks.
    await page.locator('[data-zone-pile="graveyard"]').hover();
    const strip = page.locator(".zone-pile-menu");
    await strip.waitFor();
    await strip.locator('[data-object-id="20"]').click();
    await strip.locator('[data-object-id="21"]').click();
    await page.waitForFunction(() => document.querySelectorAll(".card-selection-check").length === 2);
    assert.equal(await strip.isVisible(), true);
    assert.equal(await submit.textContent(), "Submit (2/0-2)");
    assert.equal(await strip.locator(".zone-pile-card-row.is-chosen").count(), 2);

    // Re-clicking a chosen zone card is inert; its check still removes it.
    await strip.locator('[data-object-id="20"]').click();
    await page.waitForTimeout(150);
    assert.equal(await submit.textContent(), "Submit (2/0-2)");
    await strip.locator(".zone-pile-card-slot", {has: page.locator('[data-object-id="20"]')})
      .locator(".card-selection-check").click();
    await page.waitForFunction(() => document.querySelectorAll(".card-selection-check").length === 1);
    assert.equal(await submit.textContent(), "Submit (1/0-2)");

    await submit.click();
    assert.deepEqual(
      JSON.parse(await page.locator("[data-commands]").textContent()),
      [{type: "select_objects", object_ids: [21]}],
    );
    assert.deepEqual(errors, []);
  } finally {
    await browser.close();
    await vite.close();
  }
});
