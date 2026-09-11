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

// "Tap up to one target creature. Scry 1, then draw a card." — one target
// requirement whose minimum is zero.
const OPTIONAL_TARGET_SPELL = "Plunge into Winter";

const PUZZLE = encodePuzzlePayload({
  version: 1,
  players: [
    {
      name: "Alice",
      life: 20,
      zones: {
        battlefield: ["Plains", "Plains"],
        hand: [OPTIONAL_TARGET_SPELL],
        library: Array.from({ length: 8 }, () => "Plains"),
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

test(
  "releasing an up-to-one-target cast over nothing keeps the decision open for no targets",
  { timeout: 180000 },
  async () => {
    const { vite, baseUrl } = await startUiServer();
    let browser = null;

    try {
      browser = await chromium.launch();
      const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
      const pageErrors = [];
      page.on("pageerror", (error) => pageErrors.push(String(error?.stack || error)));

      await page.goto(`${baseUrl}/?puzzle=${PUZZLE}`);
      const handCard = page.locator(`.game-card.hand-card[data-card-name="${OPTIONAL_TARGET_SPELL}"]`);
      await handCard.waitFor({ timeout: 60000 });
      const mainLabel = await advanceToFirstMain(page);
      assert.equal(mainLabel, "Go to Combat", "puzzle should reach Alice's first main phase");

      // Drag the spell out of the hand and release it over the empty right
      // edge of the table: no card, no player box, nothing legal.
      const card = await handCard.boundingBox();
      const table = await page.locator("[data-drop-zone]").first().boundingBox();
      await page.mouse.move(card.x + (card.width / 2), card.y + 24);
      await page.mouse.down();
      await page.mouse.move(card.x + (card.width / 2), card.y - 80, { steps: 10 });
      await page.mouse.move(table.x + table.width - 24, table.y + (table.height / 2), { steps: 12 });
      await page.waitForTimeout(400);
      await page.mouse.up();

      const submit = page.getByRole("button", { name: /^Submit Targets \(/ });
      await submit.waitFor({ timeout: 15000 });
      assert.equal(await submit.textContent(), "Submit Targets (0)");
      assert.equal(await submit.isEnabled(), true);

      // Submitting none of them casts the spell rather than cancelling it: the
      // cast moves on to paying for the spell.
      await submit.click();
      await page.waitForFunction(
        () => /Pay for Plunge into Winter/i.test(document.body.innerText),
        null,
        { timeout: 15000 },
      );
      const afterSubmit = await page.evaluate(() => document.body.innerText);
      assert.equal(/Submit Targets \(/.test(afterSubmit), false, afterSubmit.slice(0, 400));
      assert.match(afterSubmit, /1 stack entry/);

      assert.deepEqual(pageErrors, []);
    } finally {
      await browser?.close();
      await vite.close();
    }
  }
);

test(
  "releasing an up-to-one-target cast on a legal creature casts it at that target",
  { timeout: 180000 },
  async () => {
    const { vite, baseUrl } = await startUiServer();
    let browser = null;

    try {
      browser = await chromium.launch();
      const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
      const pageErrors = [];
      page.on("pageerror", (error) => pageErrors.push(String(error?.stack || error)));

      await page.goto(`${baseUrl}/?puzzle=${PUZZLE}`);
      const handCard = page.locator(`.game-card.hand-card[data-card-name="${OPTIONAL_TARGET_SPELL}"]`);
      await handCard.waitFor({ timeout: 60000 });
      assert.equal(await advanceToFirstMain(page), "Go to Combat");

      const card = await handCard.boundingBox();
      const bears = await page.locator('.game-card[data-card-name="Grizzly Bears"]').first().boundingBox();
      await page.mouse.move(card.x + (card.width / 2), card.y + 24);
      await page.mouse.down();
      await page.mouse.move(card.x + (card.width / 2), card.y - 80, { steps: 10 });
      await page.mouse.move(bears.x + (bears.width / 2), bears.y + (bears.height / 2), { steps: 12 });
      await page.waitForTimeout(400);
      await page.mouse.up();

      // An optional single target needs no separate Submit: the release named it.
      await page.waitForFunction(
        () => /Pay for Plunge into Winter/i.test(document.body.innerText),
        null,
        { timeout: 15000 },
      );
      const afterDrop = await page.evaluate(() => document.body.innerText);
      assert.equal(/Submit Targets \(/.test(afterDrop), false, afterDrop.slice(0, 400));
      assert.match(afterDrop, /1 stack entry/);

      assert.deepEqual(pageErrors, []);
    } finally {
      await browser?.close();
      await vite.close();
    }
  }
);
