import test from "node:test";
import assert from "node:assert/strict";
import { chromium } from "playwright";
import { createServer } from "vite";
test("Delve shows its exile action and can exclude an exact graveyard card", async () => {
  const server = await createServer({server:{host:"127.0.0.1",port:0},logLevel:"silent"});
  await server.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    const errors=[]; page.on("pageerror", error=>errors.push(error.message));
    await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/tests/alternative-payment.html`, {waitUntil:"domcontentloaded"});
    await page.getByText("Exile for delve", {exact:false}).waitFor();
    await page.getByRole("button", {name:"Remove Chosen graveyard card from payment",exact:true}).click();
    await page.waitForFunction(() => document.querySelector('[data-command]').textContent !== 'null');
    const command=JSON.parse(await page.locator("[data-command]").textContent());
    assert.equal(command.response.action,"replan");
    assert.deepEqual(command.response.excluded_source_ids,["42"]);
    assert.deepEqual(errors,[]);
  } finally {await browser.close();await server.close();}
});
