import test from "node:test";
import assert from "node:assert/strict";
import { chromium } from "playwright";
import { createServer } from "vite";

test("a cost-payment decision localizes its frame, its quoted cost and its card rows", async () => {
  const vite = await createServer({server:{host:"127.0.0.1",port:0},logLevel:"silent"});
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const port = vite.httpServer.address().port;
    const errors = [];
    const page = await browser.newPage({viewport:{width:1200,height:600}});
    page.on("pageerror", e => errors.push(e.message));

    await page.goto(`http://127.0.0.1:${port}/tests/decision-sacrifice-locale.html?locale=es`);
    const probe = page.locator("[data-sacrifice-probe]");
    await probe.waitFor();
    await page.waitForTimeout(900);
    // Highlighting wraps matched words, so innerText carries stray breaks.
    const flatten = (value) => value.replace(/\s+/g, " ").trim();
    const text = flatten(await probe.innerText());

    assert.match(text, /Elige una criatura para sacrificar ?: Sacrificar otra criatura/, text);
    assert.match(text, /Ornitóptero/, text);
    assert.match(text, /Elfos de Llanowar/, text);
    assert.doesNotMatch(text, /Choose a creature to sacrifice/, text);
    assert.doesNotMatch(text, /Ornithopter/, text);

    await page.goto(`http://127.0.0.1:${port}/tests/decision-sacrifice-locale.html?locale=en`);
    await probe.waitFor();
    await page.waitForTimeout(900);
    const english = flatten(await probe.innerText());
    assert.match(english, /Choose a creature to sacrifice ?: Sacrifice another creature/, english);
    assert.match(english, /Ornithopter/, english);

    assert.deepEqual(errors, []);
  } finally { await browser.close(); await vite.close(); }
});
