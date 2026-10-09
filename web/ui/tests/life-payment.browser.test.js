import test from "node:test";
import assert from "node:assert/strict";
import { chromium } from "playwright";
import { createServer } from "vite";

test("life payment pip selection reaches replan and requires explicit confirmation", async () => {
  const vite = await createServer({ server: { host: "127.0.0.1", port: 0 }, logLevel: "silent" });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/payment-editor.html`);
    await page.getByRole("button", { name: "Choose payment for Prism", exact: true }).click();
    await page.getByRole("dialog", { name: "Choose payment source" })
      .getByText("Pay 2 life", { exact: true }).click();
    await page.waitForFunction(() => JSON.parse(document.querySelector("[data-commands]").textContent).length === 1);
    const commands = () => page.locator("[data-commands]").textContent().then(JSON.parse);
    const [replan] = await commands();
    assert.equal(replan.response.action, "replan");
    assert.deepEqual(replan.response.required_life_pips, [1]);
    await page.waitForFunction(() => !document.querySelector(".mana-payment-pay-button").disabled);
    assert.equal((await commands()).length, 1, "choosing life only changes the proposal");
    await page.getByRole("button", { name: "Pay", exact: true }).click();
    const confirm = (await commands()).at(-1);
    assert.equal(confirm.response.action, "confirm");
    assert.equal(confirm.response.request_hash, `hash:${JSON.stringify(replan.response)}`);
  } finally {
    await browser.close();
    await vite.close();
  }
});
