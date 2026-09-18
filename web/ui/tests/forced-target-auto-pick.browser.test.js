import test from "node:test";
import assert from "node:assert/strict";
import { chromium } from "playwright";
import { createServer } from "vite";

async function withFixture(scenario, run, extraQuery = "") {
  const vite = await createServer({ server: { host: "127.0.0.1", port: 0 }, logLevel: "silent" });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1400, height: 900 }, reducedMotion: "reduce" });
    const errors = [];
    page.on("pageerror", (error) => errors.push(String(error?.message || error)));
    await page.route("**/api.scryfall.com/**", (route) => route.abort());
    await page.route("**/cards.scryfall.io/**", (route) => route.abort());
    await page.goto(
      `http://127.0.0.1:${vite.httpServer.address().port}/tests/forced-target-auto-pick.html?scenario=${scenario}${extraQuery}`
    );
    await page.locator(".stack-card").first().waitFor({ timeout: 30000 });
    await run(page);
    assert.deepEqual(errors, [], "no page errors");
  } finally {
    await browser.close();
    await vite.close();
  }
}

const placedTargets = (page) => page.$$eval(
  '.decision-option-row[aria-pressed="true"]',
  (nodes) => nodes.map((node) => node.textContent.trim()),
);

const submitButton = (page) => page.locator(".decision-submit-button");

async function submitState(page) {
  const button = submitButton(page);
  return {
    label: (await button.textContent()).replace(/\s+/g, " ").trim(),
    enabled: await button.isEnabled(),
  };
}

test("the only legal target is placed for the player, who still submits", { timeout: 120000 }, async () => {
  await withFixture("forced-opponent", async (page) => {
    await page.waitForSelector('.decision-option-row[aria-pressed="true"]', { timeout: 10000 });
    assert.deepEqual(await placedTargets(page), ["Bob"], "the lone opponent is placed");
    assert.deepEqual(await submitState(page), { label: "Submit Targets (1)", enabled: true },
      "the count advances and the button turns on, exactly as after a click");

    // It stops there. Nothing is submitted until the player says so.
    await page.waitForTimeout(800);
    assert.equal(await page.evaluate(() => window.__dispatched), null,
      "placing a forced target must not submit the decision");
    assert.equal(await page.evaluate(() => window.__cancelled), 0, "and must not cancel it");

    await submitButton(page).click();
    assert.deepEqual(await page.evaluate(() => window.__dispatched), {
      type: "select_targets",
      targets: [{ kind: "player", player: 1 }],
    }, "the player's submit sends the placed target");
  });
});

test("an auto-placed target is the same state a manual click produces", { timeout: 120000 }, async () => {
  const read = async (scenario, beforeRead) => {
    let captured = null;
    await withFixture(scenario, async (page) => {
      if (beforeRead) await beforeRead(page);
      await page.waitForSelector('.decision-option-row[aria-pressed="true"]', { timeout: 10000 });
      captured = {
        placed: await placedTargets(page),
        submit: await submitState(page),
        // allDone is what a completed requirement looks like; a manual click
        // advances past the requirement and so must the automatic one.
        activeRequirements: await page.$$eval(
          ".decision-target-requirement.is-active",
          (nodes) => nodes.length,
        ),
      };
    });
    return captured;
  };

  const automatic = await read("forced-opponent");
  const manual = await read("two-opponents", async (page) => {
    await page.locator('.decision-option-row:has-text("Bob")').first().click();
  });
  assert.deepEqual(automatic, manual,
    "auto-placing must land in the same state as clicking the opponent");
});

test("a lone legal creature on the battlefield is placed too", { timeout: 120000 }, async () => {
  await withFixture("forced-creature", async (page) => {
    await page.waitForSelector('.decision-option-row[aria-pressed="true"]', { timeout: 10000 });
    assert.deepEqual(await placedTargets(page), ["Goblin Piker"]);
    assert.equal(await page.evaluate(() => window.__dispatched), null,
      "placing a forced target must not submit the decision");
  });
});

test("a real choice between opponents is left alone", { timeout: 120000 }, async () => {
  await withFixture("two-opponents", async (page) => {
    await page.waitForTimeout(600);
    assert.deepEqual(await placedTargets(page), [],
      "two legal players is a decision, not a forced target");
    assert.equal(await page.evaluate(() => window.__dispatched), null);
  });
});

test("an optional target is a choice even when only one is legal", { timeout: 120000 }, async () => {
  await withFixture("optional-opponent", async (page) => {
    await page.waitForTimeout(600);
    assert.deepEqual(await placedTargets(page), [],
      "'up to one target' must stay up to the player");
    assert.equal(await page.evaluate(() => window.__dispatched), null);
  });
});

test("a stack tile stays readable while a targets decision owns the click", { timeout: 120000 }, async () => {
  await withFixture("forced-opponent", async (page) => {
    const tile = page.locator(".stack-card").first();
    await tile.hover();
    // The tile's click is spoken for by the decision; hover is what hands the
    // spell to the inspector, and it resolves to the card, not the stack object.
    await page.waitForFunction(() => window.__hoveredObjectId === "10", null, { timeout: 5000 });
    await page.mouse.move(5, 5);
    await page.waitForFunction(() => window.__hoveredObjectId == null, null, { timeout: 5000 });
  });
});

// Publishing hover state is not the same as showing a card frame. The passive
// hover surface rejects anything a visible stack entry refers to, and every
// Workspace inspector dock is mounted with allowHoverFallback={false}, so this
// asserts the frame itself -- against the fixture mounting both real surfaces.
for (const [label, query] of [
  ["waiting on the stack", "&resolving=0"],
  ["resolving", "&resolving=1"],
  // A dies trigger: its source is already in the graveyard under a new object
  // id, so inspect_object_id is stale and only source_stable_id finds it.
  ["resolving after its source died", "&resolving=1&orphan=1"],
]) {
  test(`a stack card's frame is readable while the ability is ${label}`, { timeout: 120000 }, async () => {
    await withFixture("forced-opponent", async (page) => {
      const frame = page.locator(".interactive-card-frame-stage");
      assert.equal(await frame.count(), 0, "nothing is previewed before the hover");

      await page.locator(".stack-card").first().hover();
      await page.waitForSelector(".interactive-card-frame-stage", { timeout: 5000 });
      const text = (await frame.first().textContent()).replace(/\s+/g, " ");
      assert.match(text, /Sephiroth, Fabled SOLDIER/, "the source card's frame is shown");
      assert.match(text, /Whenever another creature dies/, "including its rules text");

      await page.mouse.move(5, 5);
      await page.waitForFunction(
        () => document.querySelectorAll(".interactive-card-frame-stage").length === 0,
        null,
        { timeout: 5000 },
      );
    }, query);
  });
}

test("the stack's card frame opens flush against the stack and survives the trip into it", { timeout: 120000 }, async () => {
  await withFixture("forced-opponent", async (page) => {
    const frame = page.locator(".floating-card-preview");
    const stackBox = await page.locator("[data-stack-preview-anchor]").boundingBox();

    await page.locator(".stack-card").first().hover();
    await page.waitForFunction(
      () => document.querySelector(".floating-card-preview")?.dataset.visible === "true",
      null,
      { timeout: 5000 },
    );
    assert.equal(await frame.getAttribute("data-placement"), "beside-stack");

    // Flush against the stack: any gap here is a dead zone the pointer would
    // cross on the way in, closing the frame before it arrives.
    const frameBox = await frame.boundingBox();
    assert.equal(
      Math.round(frameBox.x),
      Math.round(stackBox.x + stackBox.width),
      "the frame starts exactly where the stack ends",
    );

    // Walking off the tile and into the frame must not dismiss it.
    await page.mouse.move(frameBox.x + 20, frameBox.y + 60);
    await page.waitForTimeout(500);
    assert.equal(await frame.getAttribute("data-visible"), "true",
      "entering the frame keeps it open, so its contents can be used");

    await page.mouse.move(5, 5);
    await page.waitForFunction(
      () => document.querySelector(".floating-card-preview")?.dataset.visible !== "true",
      null,
      { timeout: 5000 },
    );
  }, "&resolving=1");
});

test("clicking a stack tile leaves the hovered card frame exactly as it was", { timeout: 120000 }, async () => {
  await withFixture("forced-opponent", async (page) => {
    const frame = page.locator(".floating-card-preview");
    const tile = page.locator(".stack-card").first();

    await tile.hover();
    await page.waitForFunction(
      () => document.querySelector(".floating-card-preview")?.dataset.visible === "true",
      null,
      { timeout: 10000 },
    );
    const before = {
      visible: await frame.getAttribute("data-visible"),
      previewObjectId: await frame.getAttribute("data-preview-object-id"),
      left: Math.round((await frame.boundingBox()).x),
    };

    await tile.click();
    await page.waitForTimeout(700);
    const after = {
      visible: await frame.getAttribute("data-visible"),
      previewObjectId: await frame.getAttribute("data-preview-object-id"),
      left: Math.round((await frame.boundingBox()).x),
    };

    // The click has nothing to reveal that hovering did not already show, so
    // it must not blink the frame out and back in from the pinned path.
    assert.deepEqual(after, before, "clicking a stack tile changes nothing");
    assert.equal(before.visible, "true");
  }, "&resolving=1");
});
