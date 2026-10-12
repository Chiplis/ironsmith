import test from "node:test";
import assert from "node:assert/strict";
import {chromium} from "playwright";
import {readFile} from "node:fs/promises";
import {createServer} from "vite";

test("Custom cards button replaces the hand through the real worker and retains draw history", async () => {
  const vite = await createServer({server: {host: "127.0.0.1", port: 0}, logLevel: "silent"});
  await vite.listen();
  const browser = await chromium.launch({args: ["--enable-unsafe-swiftshader"]});
  try {
    const page = await browser.newPage({viewport: {width: 1440, height: 1000}});
    const errors = [];
    page.on("pageerror", error => errors.push(error.message));
    await page.goto("http://127.0.0.1:" + vite.httpServer.address().port + "/", {waitUntil: "domcontentloaded", timeout: 90000});
    const button = page.getByRole("button", {name: "Custom cards", exact: true}).first();
    try { await button.waitFor({timeout: 90000}); } catch (error) {
      await page.screenshot({path: "/tmp/custommagic/button-failure.png"});
      console.error((await page.locator("body").innerText()).slice(0, 3000), errors);
      throw error;
    }
    let prior = [];
    for (let click = 0; click < 2; click++) {
      await button.click({timeout: 60000});
      await page.waitForFunction(() => !document.querySelector('button[aria-busy="true"]'), null, {timeout: 60000});
      const state = await page.evaluate(() => ({
        bag: JSON.parse(localStorage.getItem("ironsmith-custom-card-draw-bag-v1")),
        flavor: JSON.parse(localStorage.getItem("ironsmith-custom-card-flavor-v1")),
      }));
      assert.equal(state.bag.recent.length, 7);
      assert.equal(state.bag.recent.filter(id => prior.includes(id)).length, 0);
      assert.ok(state.flavor && Object.keys(state.flavor).length >= 7,
        (await page.locator("body").innerText()).slice(0, 3000));
      prior = state.bag.recent;
    }
    assert.deepEqual(errors, []);

  } finally { await browser.close(); await vite.close(); }
});


test("all seven custom hand cards load their bundled full prints", async () => {
  const pool = JSON.parse(await readFile(new URL("../public/custommagic/pool.json", import.meta.url)));
  const names = ["Cleveland", "Magikarp", "Tribute To...", "Winrar", "Syl of Many Deaths", "10 Fucking Sol Rings", "Thoughts"];
  pool.cards = pool.cards.filter(card => names.includes(card.draft.faces[0].name));
  assert.equal(pool.cards.length, 7);
  const vite = await createServer({server: {host: "127.0.0.1", port: 0}, logLevel: "silent"});
  await vite.listen();
  const browser = await chromium.launch({args: ["--enable-unsafe-swiftshader"]});
  try {
    const page = await browser.newPage({viewport: {width: 1600, height: 1100}});
    await page.route("**/custommagic/pool.json", route => route.fulfill({json: pool}));
    const errors = [];
    page.on("pageerror", error => errors.push(error.message));
    await page.goto("http://127.0.0.1:" + vite.httpServer.address().port + "/", {waitUntil: "domcontentloaded", timeout: 90000});
    const button = page.getByRole("button", {name: "Custom cards", exact: true}).first();
    await button.click({timeout: 90000});
    await page.waitForFunction(() => !document.querySelector('button[aria-busy="true"]'), null, {timeout: 60000});
    await page.waitForFunction(() => {
      const cards = [...document.querySelectorAll('[data-card-image-url*="/custommagic/images/"][data-card-image-url$="-full.webp"]')];
      return cards.length >= 7 && cards.every(card => {
        const image = card.querySelector("img");
        return image?.complete && image.naturalWidth > 0;
      });
    });
    const card = page.locator('[data-card-name="Cleveland"]').first();
    assert.match(await card.getAttribute("data-card-image-url"), /custommagic\/images\/.+-full.webp$/);
    // Hand focus enlarges the original print; the separate live frame's type,
    // art, and attribution are covered by custom-card-assets.browser.test.js.
    await card.focus();
    await card.locator("img").first().evaluate(image => image.decode());
    assert.deepEqual(errors, []);
  } finally { await browser.close(); await vite.close(); }
});
