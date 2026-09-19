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
const SCREENSHOT_DIR = process.env.STACK_TARGET_SCREENSHOT_DIR || null;

// Alice bolts and shocks the Bears, keeping priority with both spells on the
// stack, and then drags Counterspell out of her hand: with two spells to
// choose from, the stack tiles are the targets. (A lone legal target is
// placed by the engine before the drag can aim at anything.)
const PUZZLE = encodePuzzlePayload({
  version: 1,
  players: [
    {
      name: "Alice",
      life: 20,
      zones: {
        battlefield: ["Island", "Island", "Mountain", "Mountain"],
        hand: ["Lightning Bolt", "Shock", "Counterspell"],
        library: Array.from({ length: 8 }, () => "Island"),
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

/** The stack header counts its entries; the wording depends on the rail layout. */
function stackCountPattern(count) {
  return new RegExp(`(Stack entries: ${count}|Entries: ${count}|${count} stack entr)`);
}

async function dragHandCardTo(page, handCard, point) {
  const card = await handCard.boundingBox();
  await page.mouse.move(card.x + (card.width / 2), card.y + 24);
  await page.mouse.down();
  await page.mouse.move(card.x + (card.width / 2), card.y - 80, { steps: 10 });
  await page.mouse.move(point.x, point.y, { steps: 12 });
  await page.waitForTimeout(400);
}

test(
  "dragging Counterspell onto a spell's stack tile casts it at that spell",
  { timeout: 240000 },
  async () => {
    const { vite, baseUrl } = await startUiServer();
    let browser = null;

    try {
      browser = await chromium.launch();
      const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
      const pageErrors = [];
      page.on("pageerror", (error) => pageErrors.push(String(error?.stack || error)));

      await page.goto(`${baseUrl}/?puzzle=${PUZZLE}`);
      const boltInHand = page.locator('.game-card.hand-card[data-card-name="Lightning Bolt"]');
      await boltInHand.waitFor({ timeout: 60000 });
      assert.equal(await advanceToFirstMain(page), "Go to Combat", "puzzle should reach Alice's first main phase");

      // Bolt and Shock the Bears, paying for each. Alice, as the caster, gets
      // priority back with both spells still on the stack.
      const bears = await page.locator('.game-card[data-card-name="Grizzly Bears"]').first().boundingBox();
      for (const [spell, count] of [["Lightning Bolt", 1], ["Shock", 2]]) {
        const inHand = page.locator(`.game-card.hand-card[data-card-name="${spell}"]`);
        await dragHandCardTo(page, inHand, { x: bears.x + (bears.width / 2), y: bears.y + (bears.height / 2) });
        await page.mouse.up();
        await page.waitForFunction(
          (name) => new RegExp(`Pay for ${name}`, "i").test(document.body.innerText),
          spell,
          { timeout: 15000 },
        );
        await page.getByRole("button", { name: "Pay", exact: true }).click();
        await page.waitForFunction(
          ({ pattern, name }) => new RegExp(pattern).test(document.body.innerText)
            && !new RegExp(`Pay for ${name}`, "i").test(document.body.innerText),
          { pattern: stackCountPattern(count).source, name: spell },
          { timeout: 15000 },
        );
      }
      const boltTile = page.locator('.stack-card[data-card-name="Lightning Bolt"]').first();
      const shockTile = page.locator('.stack-card[data-card-name="Shock"]').first();
      await shockTile.waitFor({ timeout: 15000 });
      assert.equal(await boltTile.evaluate((node) => node.classList.contains("target-legal")), false,
        "nothing is being aimed yet");

      // Drag Counterspell out of the hand: both spells' tiles light up as legal
      // targets, the one under the held card marks itself, and the release
      // casts Counterspell at it without a separate Submit.
      const counterspell = page.locator('.game-card.hand-card[data-card-name="Counterspell"]');
      const counterspellBox = await counterspell.boundingBox();
      await page.mouse.move(counterspellBox.x + (counterspellBox.width / 2), counterspellBox.y + 24);
      await page.mouse.down();
      await page.mouse.move(counterspellBox.x + (counterspellBox.width / 2), counterspellBox.y - 120, { steps: 10 });
      await page.waitForSelector('.stack-card.card-targeting-mode.target-legal[data-card-name="Shock"]', { timeout: 15000 });
      assert.equal(await boltTile.evaluate((node) => node.classList.contains("target-legal")), true);
      const counterspellTile = page.locator('.stack-card.card-targeting-mode[data-card-name="Counterspell"]');
      if (await counterspellTile.count() > 0) {
        assert.equal(await counterspellTile.first().evaluate((node) => node.classList.contains("target-legal")), false,
          "a spell is not its own target");
      }
      const tile = await page.locator('.stack-card.target-legal[data-card-name="Shock"]').first().boundingBox();
      await page.mouse.move(tile.x + (tile.width / 2), tile.y + (tile.height / 2), { steps: 12 });
      await page.waitForSelector('.stack-card.target-legal.hovered[data-card-name="Shock"]', { timeout: 5000 });
      assert.equal(await boltTile.evaluate((node) => node.classList.contains("hovered")), false);
      if (SCREENSHOT_DIR) await page.screenshot({ path: path.join(SCREENSHOT_DIR, "stack-target-hover.png") });
      await page.mouse.up();

      await page.waitForFunction(
        () => /Pay for Counterspell/i.test(document.body.innerText),
        null,
        { timeout: 15000 },
      );
      const afterDrop = await page.evaluate(() => document.body.innerText);
      assert.equal(/Submit Targets \(/.test(afterDrop), false, afterDrop.slice(0, 400));
      await page.getByRole("button", { name: "Pay", exact: true }).click();
      await page.waitForFunction(
        (pattern) => new RegExp(pattern).test(document.body.innerText),
        stackCountPattern(3).source,
        { timeout: 15000 },
      );
      if (SCREENSHOT_DIR) await page.screenshot({ path: path.join(SCREENSHOT_DIR, "stack-target-after.png") });

      // Both players pass: Counterspell resolves and it is Shock, the tile the
      // release landed on, that leaves the stack.
      for (let pass = 0; pass < 2; pass += 1) {
        await page.locator(".decision-main-button").first().click();
        await page.waitForTimeout(1200);
      }
      await page.waitForFunction(
        (pattern) => new RegExp(pattern).test(document.body.innerText),
        stackCountPattern(1).source,
        { timeout: 15000 },
      );
      assert.equal(await page.locator('.stack-card[data-card-name="Shock"]').count(), 0, "Shock was countered");
      assert.equal(await page.locator('.stack-card[data-card-name="Lightning Bolt"]').count(), 1, "Bolt is still on the stack");

      assert.deepEqual(pageErrors, []);
    } finally {
      await browser?.close();
      await vite.close();
    }
  }
);
