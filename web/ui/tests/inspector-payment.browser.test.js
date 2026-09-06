import test from "node:test";
import assert from "node:assert/strict";
import { chromium } from "playwright";
import { createServer } from "vite";

test("inspector activation follows current payment availability in both layouts", async () => {
  const vite = await createServer({ server: { host: "127.0.0.1", port: 0 }, logLevel: "silent" });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1000, height: 850 } });
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/inspector-payment.html`);
    await page.getByRole("button", { name: "Toggle inspector" }).waitFor();
    assert.equal(await page.evaluate(() => window.paymentRequests.length), 0);
    await page.getByRole("button", { name: "Toggle inspector" }).click();
    for (let layout = 0; layout < 2; layout += 1) {
      const ability = page.locator("button[data-available]");
      await ability.waitFor();
      await page.waitForFunction(count => window.paymentRequests.length === count, 1 + layout * 2);
      await page.evaluate(() => window.resolveNextPayment());
      assert.equal(await ability.isDisabled(), true);
      assert.equal(await ability.getAttribute("data-available"), "false");
      await ability.evaluate(button => button.click());
      assert.equal(await page.locator("output").textContent(), String(layout));
      await page.getByRole("button", { name: "Toggle payment" }).click();
      assert.equal(await ability.isDisabled(), true, "loading a new snapshot must not reuse a stale result");
      await page.waitForFunction(count => window.paymentRequests.length === count, 2 + layout * 2);
      await page.evaluate(() => window.resolveNextPayment());
      await page.waitForFunction(() => document.querySelector("button[data-available]")?.disabled === false);
      assert.equal(await ability.isEnabled(), true);
      const requestsBeforeReopen = await page.evaluate(() => window.paymentRequests.length);
      await page.getByRole("button", { name: "Toggle inspector" }).click();
      await page.getByRole("button", { name: "Toggle inspector" }).click();
      assert.equal(await ability.isEnabled(), true, "reopening uses the completed result immediately");
      assert.equal(await page.evaluate(() => window.paymentRequests.length), requestsBeforeReopen);
      await ability.click();
      assert.equal(await page.locator("output").textContent(), String(layout + 1));
      await page.getByRole("button", { name: "Toggle payment" }).click();
      assert.equal(await ability.isDisabled(), true);
      if (layout === 0) await page.getByRole("button", { name: "Toggle layout" }).click();
    }
    await page.evaluate(() => window.resolveNextPayment());
    await page.getByRole("button", { name: "Toggle payment" }).click();
    await page.waitForFunction(() => window.paymentRequests.length === 6);
    await page.getByRole("button", { name: "Toggle payment" }).click();
    await page.waitForFunction(() => window.paymentRequests.length === 7);
    await page.evaluate(() => window.resolveNextPayment());
    assert.equal(await page.locator("button[data-available]").isDisabled(), true, "late payable results from old snapshots are ignored");
    await page.getByRole("button", { name: "Toggle inspector" }).click();
    const count = await page.evaluate(() => window.paymentRequests.length);
    await page.getByRole("button", { name: "Toggle payment" }).click();
    await page.evaluate(() => window.resolveNextPayment());
    assert.equal(await page.evaluate(() => window.paymentRequests.length), count, "closed inspectors do not plan payments");
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/inspector-payment.html?two=1`);
    await page.getByRole("button", { name: "Toggle inspector" }).click();
    await page.waitForFunction(() => window.paymentRequests.length === 2);
    await page.evaluate(() => window.resolveNextPayment());
    const first = page.locator("button[data-available]").nth(0);
    const second = page.locator("button[data-available]").nth(1);
    await page.waitForFunction(() => document.querySelector("button[data-available]")?.disabled === false);
    assert.equal(await first.isEnabled(), true, "first ability does not wait for the second ability's planner");
    assert.equal(await second.isDisabled(), true);
    await page.getByRole("button", { name: "Toggle inspector" }).click();
    await page.getByRole("button", { name: "Toggle inspector" }).click();
    assert.equal(await first.isEnabled(), true);
    assert.equal(await page.evaluate(() => window.paymentRequests.length), 2, "pending requests are also shared across reopenings");
    await page.evaluate(() => window.resolveNextPayment());
    assert.equal(await second.isDisabled(), true);
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/inspector-payment.html?dual=1`);
    await page.getByRole("button", { name: "Toggle inspector" }).click();
    await page.waitForFunction(() => window.paymentRequests.length === 2);
    await page.evaluate(() => { window.resolveNextPayment(); window.resolveNextPayment(); });
    for (let layout = 0; layout < 2; layout += 1) {
      await page.waitForFunction(() => {
        const buttons = [...document.querySelectorAll("button[data-available]")];
        return buttons.length === 2 && buttons.every(button => !button.disabled);
      });
      await first.click();
      await second.click();
      assert.deepEqual(await page.evaluate(() => window.activatedAbilities), Array.from({ length: layout + 1 }, () => [1, 0]).flat(), "each mana line dispatches its own color ability");
      if (layout === 0) await page.getByRole("button", { name: "Toggle layout" }).click();
    }
  } finally {
    await browser.close();
    await vite.close();
  }
});
