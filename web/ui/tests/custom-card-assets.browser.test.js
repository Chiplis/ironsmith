import test from "node:test";
import assert from "node:assert/strict";
import {chromium} from "playwright";
import {createServer} from "vite";

test("custom full prints and live frames use local art, types, and attribution", async () => {
  const vite = await createServer({server: {host: "127.0.0.1", port: 0}, logLevel: "silent"});
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({viewport: {width: 1600, height: 900}});
    const errors = [];
    page.on("pageerror", error => errors.push(error.message));
    await page.goto("http://127.0.0.1:" + vite.httpServer.address().port + "/tests/custom-card-assets.html", {waitUntil: "domcontentloaded", timeout: 90000});
    await page.waitForFunction(() => {
      const images = [...document.querySelectorAll("img[src*='/custommagic/images/']")];
      return images.length >= 12 && images.every(image => image.complete && image.naturalWidth > 0);
    }, null, {timeout: 60000});
    const cleveland = page.locator('[data-custom-name="Cleveland"]');
    assert.match(await cleveland.innerText(), /Land/);
    assert.match(await cleveland.innerText(), /reddit.com/);
    assert.match(await page.locator('[data-custom-name="Magikarp"]').innerText(), /Creature.*Fish/);
    assert.match(await page.locator('[data-custom-name="Tribute To..."]').innerText(), /Enchantment.*Saga/);
    assert.match(await page.locator('[data-custom-name="Winrar"]').innerText(), /Artifact.*Software/);
    assert.match(await page.locator('[data-custom-name="Syl of Many Deaths"]').innerText(), /Legendary Creature.*Human Thrall/);
    assert.equal(await page.locator('.interactive-card-frame__art img[src$="-art.webp"]').count(), 6);
    assert.equal(await page.locator('img[src$="-full.webp"]').count(), 6);
    await page.screenshot({path: "/tmp/custommagic/local-frames-result.png", fullPage: true});
    assert.deepEqual(errors, []);
  } finally { await browser.close(); await vite.close(); }
});
