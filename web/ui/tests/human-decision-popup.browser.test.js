import test from "node:test";
import assert from "node:assert/strict";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { createServer } from "vite";

const UI_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

test(
  "active decision instructions stay readable while choice details can collapse",
  { timeout: 60000 },
  async () => {
    const server = await createServer({
      root: UI_ROOT,
      server: { host: "127.0.0.1", port: 0 },
      logLevel: "silent",
    });
    let browser;
    try {
      await server.listen();
      browser = await chromium.launch();
      const page = await browser.newPage({ viewport: { width: 1365, height: 594 }, reducedMotion: 'reduce' });
      const errors = [];
      page.on("pageerror", (error) => errors.push(error.message));
      await page.route('https://**/*', route => /scryfall\.io|\.(jpg|png|webp)/.test(route.request().url())
        ? route.fulfill({ status: 200, contentType: 'image/png', body: Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAAC0lEQVR4nGP4DwQACfsD/fteaysAAAAASUVORK5CYII=', 'base64') }) : route.abort());
      const base = "http://127.0.0.1:" + server.httpServer.address().port;
      await page.goto(base + "/tests/diagnostics-layout.html?kind=select_options");
      const popup = page.locator(".battlefield-human-decision-panel").first();
      await popup.locator(".battlefield-decision-disclosure-toggle").waitFor({ timeout: 30000 });

      const bounds = await popup.boundingBox();
      assert.ok(bounds.width >= 260 && bounds.width <= 340, "the instruction fits in the reserved dock lane");
      assert.ok(bounds.x >= 0 && bounds.x + bounds.width <= 1365, "the popup stays within the viewport");
      assert.equal(await popup.locator(".action-strip-decision-stack").getAttribute("data-details-expanded"), "true");
      assert.ok(await popup.locator(".action-strip-decision-inline-summary").isVisible());
      const toolbarBounds = await popup.locator(".action-strip-decision-toolbar").boundingBox();
      assert.ok(toolbarBounds.y >= bounds.y, "the decision heading starts inside the popup");
      assert.ok(toolbarBounds.y + toolbarBounds.height <= bounds.y + bounds.height, "the decision heading is not clipped by the popup");
      const optionsList = popup.locator(".decision-strip-scroll--vertical-options");
      await optionsList.waitFor();
      const listBounds = await optionsList.boundingBox();
      const firstOption = popup.locator(".decision-option-row--vertical-select").first();
      const firstOptionBounds = await firstOption.boundingBox();
      const secondOptionBounds = await popup.locator(".decision-option-row--vertical-select").nth(1).boundingBox();
      const listMetrics = await optionsList.evaluate((node) => ({
        clientHeight: node.clientHeight,
        scrollHeight: node.scrollHeight,
        scrollWidth: node.scrollWidth,
        clientWidth: node.clientWidth,
      }));
      assert.ok(firstOptionBounds.x >= bounds.x, "the first choice starts within the popup");
      assert.ok(firstOptionBounds.x < bounds.x + bounds.width, "the first choice is placed inside the visible popup");
      assert.ok(firstOptionBounds.height >= 36, "choices keep a compact but usable hit target");
      assert.ok(secondOptionBounds.y >= firstOptionBounds.y + firstOptionBounds.height, "choices stack vertically with a small gap");
      assert.ok(listBounds.height <= 150, "the choices list stays compact and bounded");
      assert.ok(listBounds.y + listBounds.height <= bounds.y + bounds.height, "the choices list stays inside the popup");
      assert.ok(listMetrics.scrollHeight > listMetrics.clientHeight, "long choice lists scroll vertically");
      assert.ok(listMetrics.scrollWidth <= listMetrics.clientWidth + 1, "the choices list has no horizontal overflow");
      const submitSlot = page.locator(".battlefield-human-decision-panel .decision-stack-footer .action-strip-submit-button").first();
      const submitBounds = await submitSlot.boundingBox();
      assert.ok(submitBounds, "the expanded decision keeps Submit visible");
      assert.ok(submitBounds.x >= bounds.x && submitBounds.x + submitBounds.width <= bounds.x + bounds.width, "Submit stays inside the popup width");
      assert.ok(submitBounds.y >= bounds.y && submitBounds.y + submitBounds.height <= bounds.y + bounds.height, "Submit stays inside the popup footer");

      await popup.getByRole("button", { name: "Hide options", exact: true }).click();
      assert.equal(await popup.locator(".action-strip-decision-stack").getAttribute("data-details-expanded"), "false");
      assert.ok(await popup.locator(".decision-main-button").isVisible(), "the collapsed main action labels the decision");
      assert.equal(await popup.locator(".action-strip-decision-content").isVisible(), false);
      const compactBounds = await popup.boundingBox();
      assert.ok(compactBounds.width <= 480, "collapsed actions keep their heading and controls in a bounded row");
      assert.ok(compactBounds.height >= 36, "the select action header remains visible while options are hidden");
      assert.ok(compactBounds.height <= 72, `collapsed actions occupy a single row (height=${compactBounds.height})`);
      assert.ok(await popup.locator(".decision-stage-chip").isVisible(), "the Select stage remains visible while collapsed");
      assert.equal(await popup.locator(".action-strip-decision-title").isVisible(), false, "the compact row omits the redundant long heading");
      assert.ok(await popup.getByRole("button", { name: "Show options", exact: true }).isVisible(), "the options can be reopened while collapsed");
      const dockRect = await page.locator("[data-human-action-dock]").boundingBox();
      const dockBounds = {
        left: dockRect.x,
        top: dockRect.y,
        right: dockRect.x + dockRect.width,
        bottom: dockRect.y + dockRect.height,
      };
      const zoneBounds = await page.locator(".zone-pile-slot, .deck-zone-pile").evaluateAll((nodes) => nodes
        .map((node) => {
          const rect = node.getBoundingClientRect();
          const style = getComputedStyle(node);
          return style.display === "none" || style.visibility === "hidden" || Number(style.opacity) === 0
            || rect.width <= 0 || rect.height <= 0
            ? null
            : { className: node.className, zone: node.closest("[data-zone-id]")?.getAttribute("data-zone-id"), left: rect.left, top: rect.top, right: rect.right, bottom: rect.bottom };
        })
        .filter(Boolean));
      const collidedZones = zoneBounds.filter((zone) => !(dockBounds.right <= zone.left || dockBounds.left >= zone.right
        || dockBounds.bottom <= zone.top || dockBounds.top >= zone.bottom));
      assert.deepEqual(collidedZones, [], `the collapsed action sheet avoids deck, graveyard, and exile piles; dock=${JSON.stringify(dockBounds)}`);
      assert.ok(await submitSlot.count() > 0, "the active decision keeps its submit action mounted");
      assert.equal(await submitSlot.isVisible(), true, "Submit remains available beside the collapsed heading");

      await popup.getByRole("button", { name: "Show options", exact: true }).evaluate((button) => button.click());
      assert.equal(await popup.locator(".action-strip-decision-stack").getAttribute("data-details-expanded"), "true");
      assert.ok(await popup.locator(".action-strip-decision-content").isVisible());
      assert.ok(await submitSlot.isVisible(), "Submit returns with the expanded options");

      await page.goto(base + "/tests/diagnostics-layout.html?kind=select_objects");
      const objectPopup = page.locator(".battlefield-human-decision-panel").first();
      const objectList = objectPopup.locator(".decision-strip-scroll--vertical-object-options");
      await objectList.waitFor({ timeout: 30000 });
      const objectPopupBounds = await objectPopup.boundingBox();
      assert.ok(objectPopupBounds.width >= 260, "long action details retain a usable dock width");
      assert.ok(objectPopupBounds.width <= 340, "long labels stay inside the reserved hand lane");
      const objectRows = objectPopup.locator(".decision-option-row--vertical-object-select");
      const firstObjectBounds = await objectRows.first().boundingBox();
      const secondObjectBounds = await objectRows.nth(1).boundingBox();
      const objectListMetrics = await objectList.evaluate((node) => ({
        clientHeight: node.clientHeight,
        scrollHeight: node.scrollHeight,
        scrollWidth: node.scrollWidth,
        clientWidth: node.clientWidth,
      }));
      assert.ok(secondObjectBounds.y >= firstObjectBounds.y + firstObjectBounds.height, "select-object choices stack vertically");
      assert.ok(objectListMetrics.scrollHeight > objectListMetrics.clientHeight, "select-object choices scroll vertically");
      assert.ok(objectListMetrics.scrollWidth <= objectListMetrics.clientWidth + 1, "select-object choices do not overflow horizontally");

      await page.locator('[data-zone-pile="graveyard"][data-zone-owner="1"]').evaluate((button) => button.click());
      const opponentGraveyardCard = page.locator('[data-zone-card="graveyard"][data-object-id="1001"]');
      await opponentGraveyardCard.waitFor({ timeout: 5000 });
      assert.match(await opponentGraveyardCard.getAttribute("title"), /2 \+1\/\+1/, "opponent zone hover details name the counter type");
      assert.equal(await opponentGraveyardCard.getAttribute("aria-label"), "Plains", "accessible card name stays concise");
      assert.deepEqual(errors, []);
    } finally {
      await browser?.close();
      await server.close();
    }
  },
);
