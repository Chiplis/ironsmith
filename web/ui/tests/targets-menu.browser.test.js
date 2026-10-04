import test from "node:test";
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { createServer } from "vite";

test("the target menu lists every legal player and object and submits the chosen target", { timeout: 60000 }, async () => {
  const server = await createServer({
    root: fileURLToPath(new URL("../", import.meta.url)),
    server: { host: "127.0.0.1", port: 0 }, logLevel: "silent",
  });
  let browser;
  try {
    await server.listen();
    browser = await chromium.launch();
    for (const width of [2048, 1365, 1024]) {
      const page = await browser.newPage({ viewport: { width, height: 900 }, reducedMotion: "reduce" });
      const errors = [];
      page.on("pageerror", error => errors.push(error.message));
      await page.route("**/api.scryfall.com/**", route => route.abort());
      await page.route("**/cards.scryfall.io/**", route => route.abort());
      const url = `http://127.0.0.1:${server.httpServer.address().port}/tests/diagnostics-layout.html?kind=targets&scenario=any-target`;
      for (const [label, occurrence, target] of [
        ["Alice", 0, { kind: "player", player: 0 }],
        ["Bob", 0, { kind: "player", player: 1 }],
        ["Ornithopter", 1, { kind: "object", object: 101 }],
        ["Omniscience", 3, { kind: "object", object: 303 }],
      ]) {
        await page.goto(url);
        const popup = page.locator(".battlefield-human-decision-panel").first();
        const rows = popup.locator(".decision-option-row--vertical-target-select");
        await rows.last().waitFor();
        assert.equal(await rows.count(), 16, "all four players and twelve legal objects are listed");
        assert.deepEqual(await rows.allTextContents(), ["Alice", "Ornithopter", "Myr Moonvessel", "Omniscience", "Bob", "Ornithopter", "Myr Moonvessel", "Omniscience", "Charlie", "Ornithopter", "Myr Moonvessel", "Omniscience", "Diana", "Ornithopter", "Myr Moonvessel", "Omniscience"]);
        const list = popup.locator(".decision-strip-scroll--vertical-target-options");
        const metrics = await list.evaluate(node => ({ h: node.clientHeight, sh: node.scrollHeight, w: node.clientWidth, sw: node.scrollWidth }));
        assert.ok(metrics.sh > metrics.h, "long target lists scroll vertically");
        assert.ok(metrics.sw <= metrics.w + 1, "target lists have no horizontal overflow");
        const first = await rows.nth(0).boundingBox();
        const second = await rows.nth(1).boundingBox();
        assert.ok(second.y >= first.y + first.height, "targets stack vertically");
        const submit = popup.locator(".decision-submit-button");
        assert.ok(await submit.isDisabled(), "a choice is required before submitting");
        const choice = rows.filter({ hasText: new RegExp(`^${label}$`) }).nth(occurrence);
        await choice.click();
        assert.equal(await choice.getAttribute("aria-pressed"), "true");
        await page.waitForFunction(() => {
          const button = document.querySelector(".battlefield-human-decision-panel .decision-submit-button");
          return button && !button.disabled;
        });
        assert.ok(await submit.isEnabled());
        assert.ok(await submit.isVisible(), "submit remains visible below the scrolling list");
        const popupBounds = await popup.boundingBox();
        const submitBounds = await submit.boundingBox();
        assert.ok(submitBounds.y + submitBounds.height <= popupBounds.y + popupBounds.height + 1, "submit fits inside the popup");
        await submit.click();
        assert.deepEqual(await page.evaluate(() => window.__dispatched), { type: "select_targets", targets: [target] });
      }
      await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/tests/diagnostics-layout.html?kind=targets&scenario=prevention`);
      const preventionPopup = page.locator(".battlefield-human-decision-panel").first();
      await preventionPopup.locator(".decision-option-row--vertical-target-select").last().waitFor();
      const summary = await preventionPopup.locator(".action-strip-decision-inline-summary").textContent();
      assert.match(summary, /Prevent the next 2 damage/);
      assert.doesNotMatch(summary, /deals 2 damage/);
      assert.match(await preventionPopup.locator(".decision-target-meta").textContent(), /target to protect/);
      assert.deepEqual(errors, []);
      await page.close();
    }
  } finally {
    await browser?.close();
    await server.close();
  }
});

test("optional target instructions wrap within the decision panel and keep Submit reachable", { timeout: 60000 }, async () => {
  const server = await createServer({
    root: fileURLToPath(new URL("../", import.meta.url)),
    server: { host: "127.0.0.1", port: 0 }, logLevel: "silent",
  });
  let browser;
  try {
    await server.listen();
    browser = await chromium.launch();
    for (const viewport of [{ width: 2048, height: 990 }, { width: 1365, height: 594 }, { width: 1024, height: 768 }]) {
      const page = await browser.newPage({ viewport, reducedMotion: "reduce" });
      await page.route("**/api.scryfall.com/**", route => route.abort());
      await page.route("**/cards.scryfall.io/**", route => route.abort());
      await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/tests/diagnostics-layout.html?kind=targets&scenario=optional-target`);
      const popup = page.locator(".battlefield-human-decision-panel").first();
      await popup.locator(".decision-option-row--vertical-target-select").last().waitFor();
      const summary = popup.locator(".action-strip-decision-inline-summary");
      assert.match(await summary.textContent(), /Put a -1\/-1 counter on up to one target creature and draw a card/);
      assert.match(await popup.locator(".decision-target-heading-label").textContent(), /target creature for counters/);
      assert.match(await popup.locator(".decision-target-heading-limit").textContent(), /0-1, optional/);
      for (const selector of [".action-strip-decision-inline-summary", ".decision-target-heading", ".decision-target-heading-label", ".decision-target-heading-limit"]) {
        const metrics = await popup.locator(selector).evaluate(node => ({ w: node.clientWidth, sw: node.scrollWidth, h: node.clientHeight, sh: node.scrollHeight }));
        assert.ok(metrics.sw <= metrics.w + 1, `${viewport.width}: ${selector} wraps without horizontal clipping`);
        assert.ok(metrics.sh <= metrics.h + 1, `${viewport.width}: ${selector} displays its complete text`);
      }
      const cancelBounds = await popup.locator(".decision-cancel-button").boundingBox();
      const summaryBounds = await summary.boundingBox();
      assert.ok(summaryBounds.y >= cancelBounds.y + cancelBounds.height - 1, "the effect description has its own row below the controls");
      assert.ok(summaryBounds.width > cancelBounds.width * 2, "the effect description uses the panel width");
      const popupBounds = await popup.boundingBox();
      const submit = popup.locator(".decision-submit-button");
      const submitBounds = await submit.boundingBox();
      assert.ok(submitBounds.y + submitBounds.height <= popupBounds.y + popupBounds.height + 1, "Submit stays inside the panel");
      assert.ok(popupBounds.y + popupBounds.height <= viewport.height + 1, "the panel stays on screen");
      const zones = await page.locator('[data-local-zone-piles="true"] > .zone-pile-slot').evaluateAll(nodes => nodes.map(node => {
        const rect = node.getBoundingClientRect();
        return { x: rect.x, y: rect.y, width: rect.width, height: rect.height };
      }).filter(rect => rect.width > 0 && rect.height > 0));
      for (const zone of zones) {
        const overlaps = popupBounds.x < zone.x + zone.width && popupBounds.x + popupBounds.width > zone.x
          && popupBounds.y < zone.y + zone.height && popupBounds.y + popupBounds.height > zone.y;
        assert.equal(overlaps, false, "the larger panel stays clear of the zone piles");
      }
      assert.ok(await submit.isEnabled(), "an optional target permits submission without choosing a target");
      await submit.click();
      assert.deepEqual(await page.evaluate(() => window.__dispatched), { type: "select_targets", targets: [] });
      await page.close();
    }
  } finally {
    await browser?.close();
    await server.close();
  }
});
