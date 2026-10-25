import test from "node:test";
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { createServer } from "vite";
test("Submit follows the full alternative as cards are selected and removed", async () => {
  const server=await createServer({root:fileURLToPath(new URL('..',import.meta.url)),server:{host:"127.0.0.1",port:0},logLevel:"silent"});await server.listen();
  const browser=await chromium.launch();
  try {
    const page=await browser.newPage();const errors=[];page.on("pageerror",e=>errors.push(e.message));
    await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/tests/combined-object-selection.html`,{waitUntil:"domcontentloaded"});
    const submit=page.getByRole("button",{name:/Submit/});await submit.waitFor();
    const choose=async id=>{await page.evaluate(id=>window.dispatchEvent(new CustomEvent("ironsmith:select-object-choice",{detail:{objectId:id}})),id);};
    const enabled=async expected=>{await page.waitForFunction(expected=>{
      const button=Array.from(document.querySelectorAll('button')).find(b=>b.textContent.includes('Submit'));
      return button && button.disabled !== expected;
    },expected);assert.equal(await submit.isEnabled(),expected);};
    await enabled(false);
    await choose(20);await enabled(false);
    await choose(10);await enabled(true);
    await choose(20);await enabled(true);
    await choose(10);await enabled(false);
    await choose(20);await choose(30);await enabled(true);
    await submit.click();
    await page.waitForFunction(()=>document.querySelector('[data-command]').textContent!=='null');
    assert.deepEqual(JSON.parse(await page.locator('[data-command]').textContent()),{type:'select_objects',object_ids:[20,30]});
    assert.deepEqual(errors,[]);
  } finally {await browser.close();await server.close();}
});
