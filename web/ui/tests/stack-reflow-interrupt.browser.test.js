import test from "node:test";
import assert from "node:assert/strict";
import { chromium } from "playwright";
import { createServer } from "vite";

test("interrupting the stack reflow leaves every entry in flow", async () => {
  const vite = await createServer({server:{host:"127.0.0.1",port:0},logLevel:"silent"});
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({viewport:{width:1400,height:700}});
    const errors = [];
    page.on("pageerror", e => errors.push(e.message));
    for (const step of [25, 35, 60, 140]) {
      await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/stack-reflow-interrupt.html?step=${step}`);
      await page.locator(".stack-timeline-entry").last().waitFor();
      await page.waitForTimeout(1600);
      const entries = await page.evaluate(() => [...document.querySelectorAll(".stack-timeline-entry")].map(el => ({
        position: getComputedStyle(el).position,
        height: Math.round(el.getBoundingClientRect().height),
      })));
      assert.ok(entries.length > 0, `step ${step} rendered no entries`);
      for (const entry of entries) {
        assert.equal(entry.position, "relative", `step ${step} stranded an entry out of flow: ${JSON.stringify(entries)}`);
        assert.ok(entry.height >= 40, `step ${step} collapsed an entry: ${JSON.stringify(entries)}`);
      }
    }
    assert.deepEqual(errors, []);
  } finally { await browser.close(); await vite.close(); }
});
