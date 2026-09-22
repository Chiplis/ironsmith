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


test("resolved stack entries disappear when later spells arrive", async () => {
  const vite = await createServer({server:{host:"127.0.0.1",port:0},logLevel:"silent"});
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({viewport:{width:1400,height:700}});
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/stack-reflow-interrupt.html?manual`);
    await page.waitForFunction(() => typeof window.__showStack === "function");
    for (const ids of [[11], [12,11], [12], [], [13], [14,13], [14], [15]]) {
      await page.evaluate(ids => window.__showStack(ids.map(id => ({id, name:`Spell ${id}`, controller:0, text:`Effect ${id}`})), {id: 99, name: "Previously resolved spell", controller:0}), ids);
      await page.waitForTimeout(90);
    }
    await page.waitForTimeout(700);
    const ids = await page.locator('.stack-timeline-entry [data-object-id], .stack-timeline-entry[data-object-id]').evaluateAll(nodes => nodes.map(node => node.dataset.objectId));
    assert.ok(ids.length > 0);
    assert.deepEqual([...new Set(ids)], ['15']);
  } finally { await browser.close(); await vite.close(); }
});

test("shared ability ids do not orphan resolved Top tiles on desktop or mobile", async () => {
  const vite = await createServer({server:{host:"127.0.0.1",port:0},logLevel:"silent"});
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({viewport:{width:1400,height:700}});
    const errors = [];
    page.on("pageerror", error => errors.push(error.message));
    page.on("console", message => {
      if (message.type() === "error" && message.text().includes("same key")) errors.push(message.text());
    });
    for (const mode of ["", "&mobile"]) {
      await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/stack-reflow-interrupt.html?manual${mode}`);
      await page.waitForFunction(() => typeof window.__showStack === "function");
      const spell = {id: 10, name: "Underlying spell", controller: 0};
      const trigger = {id: 31, name: "First trigger", controller: 0, ability_kind: "Triggered"};
      // Triggers from one event can share an id. Resolving them and adding a
      // new entry in one update made React lose track of the first duplicate.
      const sequence = [
        [trigger, {...trigger, name: "Second trigger"}, spell],
        [{id: 50, name: "Next spell", controller: 0}, spell],
        [spell],
        [{id: 70, name: "Later spell", controller: 0}],
      ];
      for (const entries of sequence) {
        await page.evaluate(entries => window.__showStack(entries), entries);
        await page.waitForTimeout(450);
        const names = await page.locator(".stack-card").evaluateAll(nodes => nodes.map(node => node.dataset.cardName));
        assert.deepEqual(names, entries.map(entry => entry.name), `stale entries in ${mode || "desktop"}`);
        if (!mode) assert.equal(await page.locator(".stack-card-position--top").count(), 1);
      }
    }
    assert.deepEqual(errors, []);
  } finally { await browser.close(); await vite.close(); }
});
