import test from "node:test";
import assert from "node:assert/strict";
import net from "node:net";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { createServer as createViteServer } from "vite";

import { encodePuzzlePayload } from "../src/lib/puzzles.js";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, "..");

// Agatha's Soul Cauldron exiles Walking Ballista out of the graveyard and puts
// a +1/+1 counter on Yawgmoth, which then has Ballista's activated abilities.
// Two Swamps are enough mana to leave "{4}: Put a +1/+1 counter" unaffordable,
// so the board has an ability that must stay disabled next to one that must not.
const PUZZLE = encodePuzzlePayload({
  version: 1,
  players: [
    {
      name: "Alice",
      life: 20,
      zones: {
        battlefield: ["Agatha's Soul Cauldron", "Yawgmoth, Thran Physician", "Swamp", "Swamp"],
        graveyard: ["Walking Ballista"],
        library: Array.from({ length: 8 }, () => "Swamp"),
      },
    },
    {
      name: "Bob",
      life: 20,
      zones: {
        battlefield: ["Grizzly Bears"],
        library: Array.from({ length: 8 }, () => "Plains"),
      },
    },
  ],
});

const COPIED_DAMAGE_ABILITY = /Remove a \+1\/\+1 counter from this creature: It deals 1 damage to any target\./;

async function freePort() {
  return new Promise((resolve, reject) => {
    const server = net.createServer();
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      const port = typeof address === "object" && address ? address.port : 0;
      server.close(() => resolve(port));
    });
  });
}

async function startUiServer() {
  const vitePort = await freePort();
  const vite = await createViteServer({
    root: UI_ROOT,
    configFile: path.join(UI_ROOT, "vite.config.js"),
    clearScreen: false,
    logLevel: "silent",
    server: { host: "127.0.0.1", port: vitePort, strictPort: true, hmr: false, watch: null },
  });
  await vite.listen();
  return { vite, baseUrl: `http://127.0.0.1:${vitePort}` };
}

/** Walk the pregame steps with the main decision button up to the first main phase. */
async function advanceToFirstMain(page) {
  for (let step = 0; step < 12; step += 1) {
    const button = page.locator(".decision-main-button").first();
    if (await button.count() === 0) break;
    const label = (await button.textContent() || "").trim();
    if (/^go to combat/i.test(label)) return label;
    if (!/^(keep hand|pregame|continue|begin game|go to)/i.test(label)) break;
    await button.click();
    await page.waitForTimeout(1200);
  }
  return null;
}

/** Hovering a permanent opens its inspector, where each rules line is its own action. */
async function hoverPermanent(page, name) {
  const card = page.locator(`.game-card[data-card-name="${name}"]`).first();
  const box = await card.boundingBox();
  await page.mouse.move(box.x + (box.width / 2), box.y + (box.height / 2));
  await page.waitForTimeout(700);
}

async function inspectorRuleLines(page) {
  return page.evaluate(() => Array.from(document.querySelectorAll(".interactive-card-frame__rule"))
    .map((rule) => {
      const button = rule.querySelector("button");
      return {
        text: rule.innerText.replace(/\s+/g, " ").trim(),
        activatable: button ? !button.disabled : null,
      };
    }));
}

test(
  "an ability granted by Agatha's Soul Cauldron can be activated from the inspector",
  { timeout: 300000 },
  async () => {
    const { vite, baseUrl } = await startUiServer();
    let browser = null;

    try {
      browser = await chromium.launch();
      const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
      const pageErrors = [];
      page.on("pageerror", (error) => pageErrors.push(String(error?.stack || error)));

      await page.goto(`${baseUrl}/?puzzle=${PUZZLE}`);
      await page
        .locator('.game-card[data-card-name="Agatha\'s Soul Cauldron"]')
        .first()
        .waitFor({ timeout: 120000 });
      assert.equal(await advanceToFirstMain(page), "Go to Combat", "puzzle should reach Alice's first main phase");

      // Exile the Ballista with the Cauldron. Both targets are forced, so the
      // ability and its reflexive trigger resolve without a target prompt.
      await hoverPermanent(page, "Agatha's Soul Cauldron");
      await page
        .locator("button", { hasText: /Exile\s+target card from a graveyard/ })
        .first()
        .click();
      await page.waitForTimeout(1500);
      for (let step = 0; step < 8; step += 1) {
        const button = page.locator(".decision-main-button").first();
        const label = (await button.textContent() || "").trim();
        if (!/^resolve$/i.test(label)) break;
        await button.click();
        await page.waitForTimeout(1500);
      }
      await page.waitForFunction(
        () => /Exile\s*1/.test(document.body.innerText),
        null,
        { timeout: 20000 },
      );

      await hoverPermanent(page, "Yawgmoth, Thran Physician");
      // Affordability for each line is answered asynchronously, so let the
      // inspector settle before reading which lines ended up clickable.
      let lines = [];
      let copied = null;
      for (let poll = 0; poll < 20; poll += 1) {
        lines = await inspectorRuleLines(page);
        copied = lines.find((line) => COPIED_DAMAGE_ABILITY.test(line.text)) || null;
        if (copied?.activatable) break;
        await page.waitForTimeout(500);
      }
      assert.ok(copied, `Yawgmoth should print Ballista's damage ability: ${JSON.stringify(lines)}`);
      // The regression: the line showed up in the text box but the engine's
      // action never matched it, so it rendered as a permanently dead button.
      assert.equal(copied.activatable, true, "the copied ability must be clickable");
      // Its unaffordable sibling still has to stay disabled, so an always-on
      // button would not satisfy this test.
      const unaffordable = lines.find((line) => /Put a \+1\/\+1 counter on this creature\./.test(line.text));
      assert.equal(unaffordable?.activatable, false, "{4} is unpayable with two Swamps");

      // Clicking it really activates: the ability goes on the stack and asks
      // for the target its printed text calls for.
      await page.locator("button", { hasText: COPIED_DAMAGE_ABILITY }).first().click();
      await page.waitForFunction(
        () => /Submit Targets \(/.test(document.body.innerText),
        null,
        { timeout: 20000 },
      );
      assert.match(await page.evaluate(() => document.body.innerText), /target for damage/);

      assert.deepEqual(pageErrors, []);
    } finally {
      await browser?.close();
      await vite.close();
    }
  },
);
