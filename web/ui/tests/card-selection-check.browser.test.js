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

test("a card chosen inside a closed zone still shows its check on the pile", async () => {
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

    // Choose a card out of the graveyard, then leave the strip.
    await page.locator('[data-zone-pile="graveyard"]').hover();
    const strip = page.locator(".zone-pile-menu");
    await strip.waitFor();
    await strip.locator('[data-object-id="20"]').click();
    await page.waitForFunction(() => document.querySelectorAll(".card-selection-check").length === 1);
    await page.mouse.move(1150, 60);
    await strip.waitFor({state: "hidden"});

    // The pile itself carries the check: a closed zone shows only its top
    // card's art, so nothing else would say a card inside was chosen.
    const pileCheck = page.locator(".zone-pile-slot > .zone-pile-check");
    await pileCheck.waitFor();
    assert.equal(await pileCheck.count(), 1);
    assert.equal(await pileCheck.getAttribute("title"), "Deselect Island");
    const pileBox = await page.locator('[data-zone-pile="graveyard"]').boundingBox();
    const checkBox = await pileCheck.boundingBox();
    assert.ok(checkBox.width > 8 && checkBox.height > 8, "and it is visible");
    const labelBox = await page.locator(".zone-pile-label").first().boundingBox();
    assert.ok(checkBox.y >= labelBox.y + labelBox.height - 1, "below the count label");
    assert.ok(checkBox.y >= pileBox.y - 1, "and over the pile art");
    assert.ok(checkBox.x < pileBox.x + (pileBox.width / 2), "in the corner the label leaves free");
    assert.equal(await strip.isVisible(), false, "without opening the zone");

    // Clicking it unchooses that card. Reaching for it hovers the pile, which
    // opens the strip as it always does, so the check hands over to the row's
    // own one rather than leaving a second copy behind.
    await pileCheck.click();
    await page.waitForFunction(() => document.querySelectorAll(".card-selection-check").length === 0);
    assert.equal(await submit.textContent(), "Submit (0/0-2)");
    assert.equal(await page.locator(".zone-pile-slot > .zone-pile-check").count(), 0);
    assert.deepEqual(errors, []);
  } finally {
    await browser.close();
    await vite.close();
  }
});


test("Look shows the latest remaining choice when the pointer leaves", async () => {
  const vite = await createServer({server: {host: "127.0.0.1", port: 0}, logLevel: "silent"});
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({viewport: {width: 1200, height: 900}});
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-selection-check.html?look`);
    const pile = page.locator('[data-zone-pile="look"]');
    const check = page.locator('.player-look-pile .zone-pile-check');
    await pile.hover();
    const strip = page.locator('.zone-pile-menu--look');
    await strip.locator('[data-object-id="20"]').click();
    await strip.locator('[data-object-id="21"]').click();
    // Selection must not reorder cards under the pointer in the open strip.
    assert.deepEqual(await strip.locator('[data-zone-card]').evaluateAll((cards) => cards.map((card) => card.dataset.objectId)), ["21", "20"]);
    await page.mouse.move(1150, 60);
    await strip.waitFor({state: "hidden"});
    assert.equal(await check.getAttribute("title"), "Deselect Swamp");
    const swampArt = await pile.locator('img').getAttribute('src');
    await check.click();
    await page.mouse.move(1150, 60);
    await strip.waitFor({state: "hidden"});
    assert.equal(await check.getAttribute("title"), "Deselect Island");
    assert.notEqual(await pile.locator('img').getAttribute('src'), swampArt);
    await check.click();
    await page.mouse.move(1150, 60);
    await strip.waitFor({state: "hidden"});
    assert.equal(await check.count(), 0);
  } finally {
    await browser.close();
    await vite.close();
  }
});
