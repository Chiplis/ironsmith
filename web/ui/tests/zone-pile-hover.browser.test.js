import test from "node:test";
import assert from "node:assert/strict";
import { chromium } from "playwright";
import { createServer } from "vite";

test("target strips stay open across disabled cards until the pointer leaves", { timeout: 90000 }, async () => {
  const vite = await createServer({ server: { host: "127.0.0.1", port: 0 }, logLevel: "silent" });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1000, height: 600 } });
    await page.route("**/api.scryfall.com/**", route => route.fulfill({ json: { name: "x", image_uris: {} } }));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/zone-piles.html`, { waitUntil: "domcontentloaded" });
    await page.getByText("Toggle targeting", { exact: true }).click();
    for (const zone of ["graveyard", "exile"]) {
      await page.locator(`[data-zone-pile="${zone}"]`).hover();
      const strip = page.locator(".zone-pile-menu");
      await strip.waitFor();
      await strip.locator('[data-target-legal="true"]').hover();
      await page.waitForTimeout(300);
      await strip.locator('button[aria-disabled="true"]').last().hover({ force: true });
      await page.waitForTimeout(400);
      assert.equal(await strip.isVisible(), true, `${zone} stays expanded over an unselectable card`);
      await page.mouse.move(500, 550);
      await strip.waitFor({ state: "hidden" });
    }
  } finally {
    await browser.close();
    await vite.close();
  }
});


test("zone glossary interactions preserve the strip and outside clicks dismiss it", { timeout: 90000 }, async () => {
  const vite = await createServer({ server: { host: "127.0.0.1", port: 0 }, logLevel: "silent" });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1000, height: 600 } });
    await page.route("**/api.scryfall.com/**", route => route.fulfill({ json: { name: "x", image_uris: {} } }));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/zone-piles.html?glossary`, { waitUntil: "domcontentloaded" });
    for (const zone of ["graveyard", "exile"]) {
      await page.locator(`[data-zone-pile="${zone}"]`).hover();
      const strip = page.locator(".zone-pile-menu");
      await strip.waitFor();
      await strip.locator(`[data-object-id="${zone === "graveyard" ? 20 : 31}"]`).click();
      const helper = page.locator("[data-keyword-helper]");
      await helper.hover();
      await page.waitForTimeout(400);
      assert.equal(await strip.isVisible(), true, `${zone} stays open while inspecting its card`);
      await helper.click();
      const tooltip = page.locator('[data-ui-layer="tooltip"]');
      await tooltip.waitFor();
      assert.equal(await strip.isVisible(), true, `${zone} survives glossary click and focus`);
      await tooltip.click();
      assert.equal(await strip.isVisible(), true, `${zone} survives tooltip dismissal`);
      await page.mouse.click(500, 550);
      await strip.waitFor({ state: "hidden" });
    }
  } finally {
    await browser.close();
    await vite.close();
  }
});

test("looked-at deck cards can be inspected regardless of selection legality", { timeout: 90000 }, async () => {
  const vite = await createServer({ server: { host: "127.0.0.1", port: 0 }, logLevel: "silent" });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1000, height: 600 } });
    await page.route("**/api.scryfall.com/**", route => route.fulfill({ json: { name: "x", image_uris: {} } }));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/zone-piles.html?look-selection`, { waitUntil: "domcontentloaded" });
    await page.evaluate(() => {
      window.choices = [];
      window.addEventListener("ironsmith:select-object-choice", event => window.choices.push(event.detail));
    });
    await page.locator('[data-zone-pile="library"][data-looking="true"]').hover();
    const invalid = page.locator('[data-zone-card="library"][data-object-id="41"]');
    await invalid.hover();
    await page.waitForFunction(() => document.querySelector('[data-card-hover-preview][data-preview-object-id="41"]'));
    // aria-disabled preserves inspection; the handler still rejects selection.
    await invalid.dispatchEvent("click");
    await invalid.focus();
    await page.keyboard.press("Enter");
    assert.deepEqual(await page.evaluate(() => window.choices), []);
    assert.equal(await page.locator("output").textContent(), "none");
    const valid = page.locator('[data-zone-card="library"][data-object-id="42"]');
    await valid.hover();
    await page.waitForFunction(() => document.querySelector('[data-card-hover-preview][data-preview-object-id="42"]'));
    await valid.click();
    assert.deepEqual(await page.evaluate(() => window.choices), [{objectId:42,mode:"add"}]);
  } finally {
    await browser.close();
    await vite.close();
  }
});

test('hovering decision cards previews the collapsed zone without expanding it', async () => {
  const vite = await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({viewport:{width:1200,height:800}});
    await page.route('**/api.scryfall.com/**', route => route.fulfill({json:{name:'x',image_uris:{}}}));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/zone-piles.html?option-hover`,{waitUntil:'domcontentloaded'});
    for (const [id,zone] of [[19,'graveyard'],[31,'exile']]) {
      await page.getByRole('button',{name:`Choose ${id}`,exact:true}).hover();
      const pile = page.locator(`[data-zone-pile="${zone}"]`);
      await page.waitForFunction(({id,zone}) => document.querySelector(`[data-zone-pile="${zone}"]`).dataset.objectId === String(id), {id,zone});
      await page.locator(`[data-card-hover-preview][data-visible="true"][data-preview-object-id="${id}"]`).waitFor();
      await page.waitForTimeout(250);
      assert.equal(await pile.getAttribute('data-option-hovered'),'true');
      assert.equal(await pile.getAttribute('aria-expanded'),'false');
      assert.equal(await page.locator('.zone-pile-menu').count(),0);
      await page.mouse.move(600,750);
      await page.waitForFunction(zone => !document.querySelector(`[data-zone-pile="${zone}"]`).hasAttribute('data-option-hovered'),zone);
      assert.equal(await pile.getAttribute('data-object-id'),null);
    }
  } finally {await browser.close();await vite.close();}
});
