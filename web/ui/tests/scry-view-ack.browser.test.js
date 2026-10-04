import test from "node:test";
import assert from "node:assert/strict";
import { chromium } from "playwright";
import { createServer } from "vite";

for (const { mechanic, mobile } of ["scry", "surveil", "choose"].flatMap(mechanic => [false, true].map(mobile => ({ mechanic, mobile })))) {
  test(`completed ${mechanic} needs no extra acknowledgement (${mobile ? "mobile" : "desktop"})`, async () => {
    const vite = await createServer({ server: { host: "127.0.0.1", port: 0 }, logLevel: "silent" });
    await vite.listen();
    const browser = await chromium.launch();
    try {
      const page = await browser.newPage({ viewport: { width: mobile ? 390 : 1200, height: 800 } });
      const errors = [];
      page.on("pageerror", error => errors.push(error.message));
      await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/scry-view-ack.html?${mechanic}${mobile ? "&mobile" : ""}`);
      const submit = page.getByRole("button", { name: /Submit.*0\/0-3/i });
      await submit.click();
      await submit.waitFor({ state: "hidden" });
      const order = page.getByRole("button", { name: "Submit Order", exact: true });
      await order.click();
      await order.waitFor({ state: "hidden" });
      assert.equal(await page.getByRole("button", { name: "Done Looking", exact: true }).count(), 0);
      assert.equal(await page.evaluate(() => window.__lookDoneCount), 1, `completed ${mechanic} dismisses the temporary Look pile`);
      assert.equal(await page.locator(".look-eye-effect").count(), 0);
      // Advancing priority does not reactivate an acknowledged view.
      await page.evaluate(() => window.__publishSnapshot(current => ({ ...current,
        snapshot_id: current.snapshot_id + 1,
        decision: { ...current.decision, description: "Next priority prompt" },
      })));
      await page.waitForFunction(() => window.__fixtureState?.decision.description === "Next priority prompt");
      assert.equal(await page.locator(".look-eye-effect").count(), 0);
      assert.equal(await page.getByRole("button", { name: "Done Looking", exact: true }).count(), 0);
      // A different private look is not covered by the completed arrangement.
      await page.evaluate(() => window.__publishSnapshot(current => ({ ...current,
        snapshot_id: current.snapshot_id + 1,
        viewed_cards: { ...current.viewed_cards, acknowledged: false, source: 11, description: "Look at the top card",
          card_ids: [4], cards: [{ id: 4, name: "Mountain" }] },
      })));
      await page.getByRole("button", { name: "Done Looking", exact: true }).waitFor();
      // A fresh standalone reveal still needs acknowledgement.
      await page.evaluate(() => window.__publishSnapshot(current => ({ ...current,
        snapshot_id: current.snapshot_id + 1,
        viewed_cards: { ...current.viewed_cards, acknowledged: false, visibility: "public", zone: "hand",
          description: "Reveal Sphinx of Foresight", card_ids: [10], cards: [{ id: 10, name: "Sphinx of Foresight" }] },
      })));
      const done = page.getByRole("button", { name: "Done Looking", exact: true });
      await done.waitFor();
      await done.click();
      assert.equal(await done.count(), 0);
      assert.deepEqual(errors, []);
    } finally { await browser.close(); await vite.close(); }
  });
}

for (const mobile of [false, true]) {
  test(`opening-hand reveal is not acknowledged again for its upkeep trigger (${mobile ? "mobile" : "desktop"})`, async () => {
    const vite = await createServer({ server: { host: "127.0.0.1", port: 0 }, logLevel: "silent" });
    await vite.listen();
    const browser = await chromium.launch();
    try {
      const page = await browser.newPage({ viewport: { width: mobile ? 390 : 1200, height: 800 } });
      const errors = [];
      page.on("pageerror", error => errors.push(error.message));
      await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/scry-view-ack.html?${mobile ? "mobile" : ""}`);
      await page.waitForFunction(() => window.__publishSnapshot);
      await page.evaluate(() => window.__publishSnapshot(current => ({ ...current,
        viewed_cards: { inspector_only: false, acknowledged: false, viewer: 0, subject: 0,
          zone: "hand", visibility: "public", source: 10, description: "Reveal Sphinx of Foresight from opening hand",
          card_ids: [10], cards: [{ id: 10, name: "Sphinx of Foresight" }] },
        decision: { kind: "priority", player: 0, description: "Opening actions",
          actions: [{ index: 0, kind: "pass_priority", label: "Begin game" }] },
      })));
      const done = page.getByRole("button", { name: "Done Looking", exact: true });
      await done.click();
      await done.waitFor({ state: "hidden" });
      await page.evaluate(() => window.__publishSnapshot(current => ({ ...current,
        stack_size: 1, stack_preview: ["Sphinx of Foresight"],
        viewed_cards: { ...current.viewed_cards, inspector_only: true, description: "Revealed while on the stack" },
        decision: { kind: "priority", player: 0, description: "First upkeep",
          actions: [{ index: 0, kind: "pass_priority", label: "Pass priority" }] },
      })));
      await page.waitForFunction(() => window.__fixtureState?.decision.description === "First upkeep");
      assert.equal(await done.count(), 0, "the stack source is not a new reveal instruction");
      assert.equal(await page.locator('[data-zone-pile="look"]').count(), 1, "stack source remains inspectable");
      assert.equal(await page.locator(".look-eye-effect").count(), 0);
      // Explicit reveal instructions still prompt, even with a stack entry present.
      await page.evaluate(() => window.__publishSnapshot(current => ({ ...current,
        viewed_cards: { ...current.viewed_cards, inspector_only: false, description: "A new explicit reveal" },
      })));
      await done.waitFor();
      assert.deepEqual(errors, []);
    } finally { await browser.close(); await vite.close(); }
  });
}
