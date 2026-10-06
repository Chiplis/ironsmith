import test from "node:test";
import assert from "node:assert/strict";
import { chromium } from "playwright";
import { createServer } from "vite";

test("replacement choices share trigger arrows and require explicit single-effect confirmation", async () => {
  const vite = await createServer({ server: { host: "127.0.0.1", port: 0 }, logLevel: "silent" });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const ordering = await vite.ssrLoadModule("/src/lib/effect-ordering.js");
    const replacement = { kind: "select_options", player: 0, reason: "Replacement effect", min: 1, max: 1, options: [{ index: 3, description: "First" }, { index: 9, description: "Second" }] };
    assert.equal(ordering.isReplacementOrderingDecision(replacement), true);
    assert.equal(ordering.isReplacementOrderingDecision({ ...replacement, reason: "Modal choice" }), false);
    assert.deepEqual(ordering.effectOrderingOptionIndices(replacement, [9, 3]), [9]);
    const triggers = { ...replacement, reason: "Order triggers", description: "Order triggered abilities", min: 2, max: 2 };
    assert.deepEqual(ordering.effectOrderingOptionIndices(triggers, [9, 3]), [9, 3], "triggers still submit the full order");
    assert.notEqual(ordering.buildEffectOrderingKey(replacement), ordering.buildEffectOrderingKey({ ...replacement, options: replacement.options.map(option => ({ ...option, related_object_ids: [101] })) }), "source identity invalidates stale order");

    const optional = { ...replacement, options: [
      { index: 9, object_id: 40, description: "Do not apply Golgari Thug" },
      { index: 3, object_id: 40, description: "Golgari Thug" },
    ] };
    assert.equal(ordering.isEffectOrderingDecision(optional), false);
    const presented = ordering.presentOptionalReplacementDecision(optional);
    assert.equal(presented.reason, "May ability");
    assert.deepEqual(presented.options.map(option => [option.index, option.description]), [[3, "Yes"], [9, "No"]]);
    assert.equal(ordering.isReplacementOrderingDecision({ ...optional, options: [...optional.options, { index: 12, description: "Another replacement" }] }), true);
    assert.equal(ordering.isReplacementOrderingDecision({ ...optional, options: [optional.options[0], { ...optional.options[1], object_id: 41 }] }), true);

    for (const suffix of ["", "&mobile", "&spectator"]) {
      const page = await browser.newPage({ viewport: { width: suffix.includes("mobile") ? 700 : 1100, height: 850 } });
      const errors = [];
      page.on("pageerror", error => errors.push(error.message));
      await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/stack-replacement-ordering.html?optional${suffix}`);
      await page.getByText("May ability", { exact: true }).first().waitFor();
      assert.equal(await page.locator('.stack-card[data-pending-replacement="true"]').count(), 0);
      assert.deepEqual(await page.evaluate(() => window.__commands), []);
      if (!suffix.includes("spectator")) {
        const yes = page.getByRole("button", { name: "Yes", exact: true });
        await yes.waitFor({ state: "visible" });
        await yes.click();
        assert.deepEqual(await page.evaluate(() => window.__commands), [{ type: "select_options", option_indices: [3] }]);
        await page.getByRole("button", { name: "No", exact: true }).click();
        assert.deepEqual(await page.evaluate(() => window.__commands), [{ type: "select_options", option_indices: [3] }, { type: "select_options", option_indices: [9] }]);
      }
      assert.deepEqual(errors, []);
      await page.close();
    }

    for (const query of ["", "?mobile", "?spectator", "?narrow"]) {
      const page = await browser.newPage({ viewport: { width: query.includes("mobile") ? 700 : 1100, height: 850 } });
      const errors = [];
      page.on("pageerror", error => errors.push(error.message));
      await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/stack-replacement-ordering.html${query}`);
      const pending = page.locator('.stack-card[data-pending-replacement="true"]');
      await pending.nth(1).waitFor();
      assert.equal(await pending.count(), 2);
      assert.deepEqual(await pending.locator(".stack-card-title").allTextContents(), ["Opposition Agent", "Opposition Agent"]);
      assert.deepEqual(await page.locator(".stack-panel-title").allTextContents(), ["Replacement effects", "Stack"], "replacement choices are separated from the real stack");
      assert.equal(await pending.nth(0).locator(".stack-card-position").textContent(), "Apply first");
      assert.match(await pending.nth(0).textContent(), /Bob/);
      assert.match(await pending.nth(1).textContent(), /Carol/);
      for (const [index, name] of ["Bob", "Carol"].entries()) {
        const detail = pending.nth(index).locator(".stack-card-effect");
        assert.match(await detail.textContent(), new RegExp(`^${name} · Exile The Underworld Cookbook`));
        assert.equal(await detail.evaluate((node, name) => {
          const walker = document.createTreeWalker(node, NodeFilter.SHOW_TEXT);
          const text = walker.nextNode();
          const range = document.createRange();
          range.setStart(text, 0);
          range.setEnd(text, name.length);
          return range.getBoundingClientRect().right <= node.getBoundingClientRect().right;
        }, name), true, "source controller remains visible before long effect text is clipped");
      }
      assert.notEqual(await pending.nth(0).getAttribute("style"), await pending.nth(1).getAttribute("style"), "source controllers have distinct accents");
      assert.equal(await page.locator('.stack-card[data-pending-trigger="true"]').count(), 0);
      assert.deepEqual(await page.evaluate(() => window.__commands), []);
      if (query.includes("spectator")) {
        assert.equal(await pending.locator(".stack-card-reorder-button:not(:disabled)").count(), 0);
      } else {
        await pending.nth(0).locator(".stack-card-reorder-button-down").click();
        assert.deepEqual(await pending.evaluateAll(nodes => nodes.map(node => node.dataset.objectId)), ["replacement-order-1", "replacement-order-0"]);
        assert.deepEqual(await page.evaluate(() => window.__commands), [], "reordering does not submit");
        assert.deepEqual(await page.evaluate(() => window.__inspections), [], "arrows do not inspect");
        assert.equal(await pending.nth(0).locator(".stack-card-reorder-button-up").isDisabled(), true);
        assert.equal(await pending.nth(1).locator(".stack-card-reorder-button-down").isDisabled(), true);
        await pending.nth(0).hover();
        await page.waitForFunction(() => document.querySelector('[data-card-hover-preview][data-visible="true"][data-preview-object-id="41"]'));
        await pending.nth(0).click();
        assert.deepEqual(await page.evaluate(() => window.__inspections), [41], "a replacement tile inspects its own source");
        assert.deepEqual(await page.evaluate(() => window.__commands), [], "inspecting a replacement does not apply it");
        const applyButton = page.getByRole("button", { name: /Apply First/i });
        assert.equal(await applyButton.locator(".sr-only").count(), 0, "the confirmation label is visibly rendered");
        await applyButton.click();
        assert.deepEqual(await page.evaluate(() => window.__commands), [{ type: "select_options", option_indices: [1] }]);
      }
      assert.deepEqual(errors, []);
      await page.close();
    }
  } finally {
    await browser.close();
    await vite.close();
  }
});
