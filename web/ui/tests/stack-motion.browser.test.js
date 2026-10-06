import test from "node:test";
import assert from "node:assert/strict";
import { chromium } from "playwright";
import { createServer } from "vite";

test("stack cards exit and trigger/replacement ordering visibly moves tiles", async () => {
  const vite = await createServer({ server: { host: "127.0.0.1", port: 0 }, logLevel: "silent" });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1100, height: 850 } });
    const errors = [];
    page.on("pageerror", error => errors.push(error.message));
    const origin = `http://127.0.0.1:${vite.httpServer.address().port}`;
    const spell = id => ({ id, name: `Spell ${id}`, controller: 0 });
    for (const suffix of ["", "&mobile"]) {
      await page.goto(`${origin}/tests/stack-reflow-interrupt.html?manual${suffix}`);
      await page.waitForFunction(() => typeof window.__showStack === "function");
      await page.evaluate(entries => window.__showStack(entries), [spell(1), spell(2)]);
      await page.waitForTimeout(600);
      const samples = await page.evaluate(async () => {
        window.__showStack([{ id: 2, name: "Spell 2", controller: 0 }]);
        const samples = [];
        for (let i = 0; i < 14; i++) {
          await new Promise(requestAnimationFrame);
          const node = document.querySelector('[data-leaving="true"]');
          const card = node?.querySelector('.stack-card');
          if (card) samples.push({ opacity: Number(getComputedStyle(card).opacity), inert: node.inert });
        }
        return samples;
      });
      assert.ok(samples.length > 2, `removed card stays mounted in ${suffix || "desktop"}`);
      assert.ok(samples.every(sample => sample.inert), "departing cards cannot be interacted with");
      assert.ok(samples.some(sample => sample.opacity > 0 && sample.opacity < 0.95), "exit visibly fades");
      assert.ok(samples.at(-1).opacity < samples[0].opacity, "opacity decreases during exit");
      await page.waitForTimeout(550);
      assert.deepEqual(await page.locator('.stack-card').evaluateAll(nodes => nodes.map(node => node.dataset.objectId)), ['2']);
      // A returning occurrence must cancel its previous removal deadline.
      await page.evaluate(() => window.__showStack([]));
      await page.waitForTimeout(80);
      await page.evaluate(entries => window.__showStack(entries), [spell(2)]);
      await page.waitForTimeout(500);
      assert.equal(await page.locator('.stack-card').count(), 1);
      const lastExit = await page.evaluate(async () => {
        window.__showStack([]);
        const opacities = [];
        for (let i = 0; i < 12; i++) {
          await new Promise(requestAnimationFrame);
          const card = document.querySelector('[data-leaving="true"] .stack-card');
          if (card) opacities.push(Number(getComputedStyle(card).opacity));
        }
        return opacities;
      });
      assert.ok(lastExit.length > 2, "last card remains for its exit");
      assert.ok(lastExit.some(opacity => opacity > 0 && opacity < 0.95), "last card visibly fades");
      await page.waitForTimeout(600);
      assert.equal(await page.locator('.stack-card').count(), 0, "last card is removed after its exit");
    }
    for (const kind of ["trigger", "replacement"]) {
      await page.goto(`${origin}/tests/stack-${kind}-ordering.html`);
      await page.locator(`.stack-card[data-pending-${kind}="true"]`).last().waitFor();
      await page.waitForTimeout(600);
      const movement = await page.evaluate(async kind => {
        const tile = document.querySelector(`.stack-card[data-pending-${kind}="true"]`);
        const wrapper = tile.closest('.stack-timeline-entry');
        const before = wrapper.getBoundingClientRect().top;
        tile.querySelector('.stack-card-reorder-button-down').click();
        const positions = [];
        for (let i = 0; i < 30; i++) {
          await new Promise(requestAnimationFrame);
          positions.push(wrapper.getBoundingClientRect().top);
        }
        return { before, positions, id: tile.dataset.objectId };
      }, kind);
      assert.ok(new Set(movement.positions.map(y => Math.round(y))).size > 3, `${kind} tiles animate through intermediate positions: ${JSON.stringify(movement)}`);
      assert.ok(Math.abs(movement.positions.at(-1) - movement.before) > 20, `${kind} tile moves to its reordered position`);
      await page.waitForTimeout(450);
      assert.equal(await page.locator(`.stack-card[data-pending-${kind}="true"]`).nth(1).getAttribute('data-object-id'), movement.id);
      assert.equal(await page.locator('[data-leaving="true"]').count(), 0, "reordering does not start an exit");
    }
    assert.deepEqual(errors, []);
  } finally {
    await browser.close();
    await vite.close();
  }
});
