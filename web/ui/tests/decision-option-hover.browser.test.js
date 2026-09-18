import test from "node:test";
import assert from "node:assert/strict";
import { chromium } from "playwright";
import { createServer } from "vite";

const ART = "https://cards.scryfall.io/art_crop/front/a/b/x.jpg";
const PRINTING = {
  name: "Fixture Card",
  frame: "2015",
  type_line: "Creature",
  image_uris: { art_crop: ART, normal: ART },
};

async function withFixture(run) {
  const vite = await createServer({ server: { host: "127.0.0.1", port: 0 }, logLevel: "silent" });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1500, height: 950 }, reducedMotion: "reduce" });
    const errors = [];
    page.on("pageerror", (error) => errors.push(String(error?.message || error)));
    // Echo the requested name back: the image lookup keys on it, and without a
    // match the card frame never reports ready and the preview stays hidden.
    const named = (url) => {
      const parsed = new URL(url);
      return parsed.searchParams.get("exact")
        || parsed.searchParams.get("fuzzy")
        || decodeURIComponent(parsed.pathname.split("/").pop().replace(/\.json$/, ""));
    };
    await page.route("**/api.scryfall.com/**", (r) =>
      r.fulfill({ json: { ...PRINTING, name: named(r.request().url()) } }));
    await page.route("**/cards/*.json", (r) =>
      r.fulfill({ json: { scryfall: { ...PRINTING, name: named(r.request().url()) } } }));
    await page.route("**/cards.scryfall.io/**", (r) => r.fulfill({
      contentType: "image/svg+xml",
      headers: { "access-control-allow-origin": "*" },
      body: '<svg xmlns="http://www.w3.org/2000/svg" width="488" height="680"><rect width="488" height="680" fill="#dac9a8"/></svg>',
    }));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/decision-option-hover.html`);
    await page.locator(".decision-option-row").first().waitFor({ timeout: 30000 });
    await run(page);
    assert.deepEqual(errors, [], "no page errors");
  } finally {
    await browser.close();
    await vite.close();
  }
}

// Target rows by the object they stand for, not by index: the decision renders
// both a strip and a panel, so indices are not stable. Playwright's .hover()
// can also land without leaving the previous row, hiding the mouseleave these
// handlers depend on, so move the pointer explicitly.
async function hoverOptionFor(page, objectId) {
  const row = page.locator(`[data-decision-option-object="${objectId}"]`).first();
  const box = await row.boundingBox();
  await page.mouse.move(box.x + (box.width / 2), box.y + (box.height / 2));
}

async function hoverOptionWithoutObject(page) {
  const row = page.locator(".decision-option-row:not([data-decision-option-object])").first();
  const box = await row.boundingBox();
  await page.mouse.move(box.x + (box.width / 2), box.y + (box.height / 2));
}

async function moveAway(page) {
  await page.mouse.move(750, 20);
}

test("hovering an option shows its card beside the object on the battlefield", { timeout: 120000 }, async () => {
  await withFixture(async (page) => {
    await hoverOptionFor(page, 20);
    await page.waitForFunction(
      () => document.querySelector(".floating-card-preview")?.dataset.visible === "true",
      null,
      { timeout: 10000 },
    );

    const frame = await page.locator(".floating-card-preview").boundingBox();
    const druid = await page.locator('.game-card[data-object-id="20"]').boundingBox();
    // Beside the object it stands for, not beside the option row.
    assert.ok(
      frame.x >= druid.x + druid.width,
      `frame (${Math.round(frame.x)}) must sit right of the creature (${Math.round(druid.x + druid.width)})`,
    );

    await moveAway(page);
    await page.waitForFunction(
      () => document.querySelector(".floating-card-preview")?.dataset.visible !== "true",
      null,
      { timeout: 10000 },
    );
  });
});

test("an option with no object behind it leaves the board alone", { timeout: 120000 }, async () => {
  await withFixture(async (page) => {
    await hoverOptionWithoutObject(page);
    await page.waitForTimeout(900);
    assert.equal(await page.evaluate(() => window.__hovered ?? null), null);
    assert.deepEqual(await page.evaluate(() => window.__zoneViews), []);
    assert.notEqual(
      await page.locator(".floating-card-preview").getAttribute("data-visible"),
      "true",
      "nothing to preview means nothing is previewed",
    );
  });
});

test("clicking the object itself takes the option that names it", { timeout: 120000 }, async () => {
  await withFixture(async (page) => {
    assert.equal(await page.evaluate(() => window.__dispatched ?? null), null);

    await page.locator('.game-card[data-object-id="20"]').first().click();
    await page.waitForFunction(() => window.__dispatched != null, null, { timeout: 10000 });

    // Option 0 is the one naming object 20, and single-select submits at once.
    assert.deepEqual(await page.evaluate(() => window.__dispatched.command), {
      type: "select_options",
      option_indices: [0],
    });
  });
});

test("clicking an object no option names does nothing", { timeout: 120000 }, async () => {
  await withFixture(async (page) => {
    // Object 10 is on the battlefield but no option stands for it.
    await page.locator('.game-card[data-object-id="10"]').first().click();
    await page.waitForTimeout(900);
    assert.equal(await page.evaluate(() => window.__dispatched ?? null), null,
      "an unrelated card must not take an option");
  });
});
