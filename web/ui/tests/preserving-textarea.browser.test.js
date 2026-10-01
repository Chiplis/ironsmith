import test from "node:test";
import assert from "node:assert/strict";
import { chromium } from "playwright";
import { createServer } from "vite";

test("decklist editing keeps the caret when the controlled value arrives later", async () => {
  const vite = await createServer({ server: { host: "127.0.0.1", port: 0 }, logLevel: "silent" });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/preserving-textarea.html`, { waitUntil: "domcontentloaded" });
    const textarea = page.getByLabel("Decklist");
    await textarea.evaluate((element) => {
      element.focus();
      element.setSelectionRange(2, 2);
    });
    await textarea.press("X");
    await page.waitForFunction(() => document.querySelector("textarea").value === "4 Lightning Bolt\n2 Counterspell\n20 Island");
    assert.deepEqual(await textarea.evaluate((element) => [element.value, element.selectionStart, element.selectionEnd]), [
      "4 Lightning Bolt\n2 Counterspell\n20 Island",
      3,
      3,
    ]);
    await page.evaluate(() => window.__applyPendingDeckEdit());
    assert.deepEqual(await textarea.evaluate((element) => [element.value, element.selectionStart, element.selectionEnd]), [
      "4 XLightning Bolt\n2 Counterspell\n20 Island",
      3,
      3,
    ]);
  } finally {
    await browser.close();
    await vite.close();
  }
});
