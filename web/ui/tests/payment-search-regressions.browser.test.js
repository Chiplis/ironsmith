import test from "node:test";
import assert from "node:assert/strict";
import { chromium } from "playwright";
import { createServer } from "vite";

test("Pay survives a replacement plan and viewed search cards select legal candidates", async () => {
  const vite = await createServer({server:{host:"127.0.0.1",port:0},logLevel:"silent"});
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({viewport:{width:1400,height:900}});
    const errors = [];
    page.on("pageerror", e => errors.push(e.message));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/payment-search-regressions.html`);
    await page.getByRole("button", {name: "Pay", exact:true}).click();
    assert.equal(await page.locator("[data-commands]").textContent(), "[]");
    await page.getByRole("button", {name: "Finish planning"}).click();
    await page.waitForFunction(() => JSON.parse(document.querySelector("[data-commands]").textContent).length === 1);
    assert.deepEqual(JSON.parse(await page.locator("[data-commands]").textContent()), [{type:"mana_payment",response:{action:"confirm",plan_id:"optimized",request_hash:"request"}}]);
    await page.getByRole("button", {name: "Fetch land",exact:true}).click();
    await page.locator(".action-strip-view-card").filter({hasText:"Lightning Bolt"}).click();
    assert.equal(await page.locator('.decision-option-row[aria-pressed="true"]').count(),0);
    await page.locator(".action-strip-view-card").filter({hasText:"Swamp"}).click();
    await page.waitForFunction(() => document.querySelector('.decision-option-row[aria-pressed="true"]')?.textContent === "Swamp");
    await page.getByRole("button", {name:"Submit (1/0-1)",exact:true}).click();
    assert.deepEqual(JSON.parse(await page.locator("[data-commands]").textContent()).at(-1), {type:"select_objects",object_ids:[11]});
    assert.deepEqual(errors, []);
  } finally { await browser.close(); await vite.close(); }
});
