import test from "node:test";
import assert from "node:assert/strict";
import { chromium } from "playwright";
import { createServer } from "vite";
test("the stack scroll viewport fits the board and its last entry is reachable", async () => {
  const vite = await createServer({server:{host:"127.0.0.1",port:0},logLevel:"silent"});
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({viewport:{width:1100,height:800}});
    const errors = [];
    page.on("pageerror", e => errors.push(e.message));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/stack-layout.html`);
    await page.locator(".stack-timeline-entry").last().waitFor();
    await page.waitForTimeout(700);
    for (const height of [180, 110, 340]) {
      await page.locator(".my-zone-board-shell").evaluate((node, value) => { node.style.height = `${value}px`; }, height);
      await page.waitForTimeout(350);
      const bounds = await page.evaluate(() => {
        const rail = document.querySelector(".my-zone-stack-rail").getBoundingClientRect();
        const scroll = document.querySelector(".stack-timeline-scroll");
        scroll.scrollTop = scroll.scrollHeight;
        const viewport = scroll.getBoundingClientRect();
        const last = document.querySelector(".stack-timeline-entry:last-child").getBoundingClientRect();
        return {railBottom:rail.bottom,viewportBottom:viewport.bottom,lastBottom:last.bottom,viewportHeight:viewport.height};
      });
      assert.ok(bounds.viewportHeight > 0);
      assert.ok(bounds.viewportBottom <= bounds.railBottom + 1, JSON.stringify(bounds));
      assert.ok(bounds.lastBottom <= bounds.viewportBottom + 1, JSON.stringify(bounds));
    }
    assert.deepEqual(errors, []);
  } finally { await browser.close(); await vite.close(); }
});
