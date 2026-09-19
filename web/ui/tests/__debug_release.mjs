import path from "node:path";
import { chromium } from "playwright";
import { createServer as createViteServer } from "vite";
import { encodePuzzlePayload } from "../src/lib/puzzles.js";
const UI_ROOT = "/Users/chiplis/ironsmith/web/ui";
const OUT = "/private/tmp/claude-501/-Users-chiplis-ironsmith/5a70c0c0-a661-4e62-88ca-a0a521540ae4/scratchpad";
const PUZZLE = encodePuzzlePayload({ version: 1, players: [
  { name: "Alice", life: 20, zones: { battlefield: ["Island", "Island", "Mountain", "Mountain"], hand: ["Lightning Bolt", "Shock", "Counterspell"], library: Array.from({ length: 8 }, () => "Island") } },
  { name: "Bob", life: 20, zones: { battlefield: ["Grizzly Bears"], library: Array.from({ length: 8 }, () => "Plains") } },
]});
const vite = await createViteServer({ root: UI_ROOT, configFile: path.join(UI_ROOT, "vite.config.js"), logLevel: "silent", server: { host: "127.0.0.1", port: 0, hmr: false, watch: null } });
await vite.listen();
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
page.on("pageerror", (e) => console.log("PAGEERROR", e.message));
page.on("console", (m) => { if (m.type() === "error" || m.text().startsWith("[dbg]")) console.log("CONSOLE", m.text().slice(0, 300)); });
await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/?puzzle=${PUZZLE}`);
await page.locator('.game-card.hand-card[data-card-name="Lightning Bolt"]').waitFor({ timeout: 60000 });
for (let step = 0; step < 12; step += 1) {
  const button = page.locator(".decision-main-button").first();
  const label = ((await button.textContent()) || "").trim();
  if (/^go to combat/i.test(label)) break;
  await button.click(); await page.waitForTimeout(1200);
}
const bears = await page.locator('.game-card[data-card-name="Grizzly Bears"]').first().boundingBox();
for (const [spell, count] of [["Lightning Bolt", 1], ["Shock", 2]]) {
  const inHand = page.locator(`.game-card.hand-card[data-card-name="${spell}"]`);
  const card = await inHand.boundingBox();
  await page.mouse.move(card.x + card.width / 2, card.y + 24); await page.mouse.down();
  await page.mouse.move(card.x + card.width / 2, card.y - 80, { steps: 10 });
  await page.mouse.move(bears.x + bears.width / 2, bears.y + bears.height / 2, { steps: 12 });
  await page.waitForTimeout(400); await page.mouse.up();
  await page.waitForFunction((n) => new RegExp(`Pay for ${n}`, "i").test(document.body.innerText), spell, { timeout: 15000 });
  await page.getByRole("button", { name: "Pay", exact: true }).click();
  await page.waitForFunction((c) => new RegExp(`Entries: ${c}`).test(document.body.innerText), count, { timeout: 15000 });
}
await page.evaluate(() => {
  window.__choices = [];
  window.addEventListener("ironsmith:target-choice", (e) => window.__choices.push({ t: performance.now(), detail: e.detail }));
  document.addEventListener("pointerup", (e) => console.log("[dbg] pointerup target", e.target?.className?.toString().slice(0, 60), e.clientX, e.clientY), true);
  document.addEventListener("click", (e) => console.log("[dbg] click target", e.target?.className?.toString().slice(0, 60)), true);
});
const counter = page.locator('.game-card.hand-card[data-card-name="Counterspell"]');
const card = await counter.boundingBox();
await page.mouse.move(card.x + card.width / 2, card.y + 24); await page.mouse.down();
await page.mouse.move(card.x + card.width / 2, card.y - 120, { steps: 10 });
await page.waitForSelector('.stack-card.card-targeting-mode.target-legal[data-card-name="Shock"]', { timeout: 15000 });
const tile = await page.locator('.stack-card.target-legal[data-card-name="Shock"]').first().boundingBox();
console.log("shock tile", JSON.stringify(tile));
await page.mouse.move(tile.x + tile.width / 2, tile.y + tile.height / 2, { steps: 12 });
await page.waitForSelector('.stack-card.target-legal.hovered[data-card-name="Shock"]', { timeout: 5000 });
console.log("elementsFromPoint:", await page.evaluate(([x, y]) => document.elementsFromPoint(x, y).slice(0, 6).map((n) => `${n.tagName}.${String(n.className).slice(0, 50)}`), [tile.x + tile.width / 2, tile.y + tile.height / 2]));
await page.mouse.up();
await page.waitForTimeout(2500);
await page.screenshot({ path: `${OUT}/debug-release.png` });
console.log("choices:", JSON.stringify(await page.evaluate(() => window.__choices)));
console.log("BODY:", (await page.evaluate(() => document.body.innerText)).replace(/\n/g, " | ").slice(0, 900));
await browser.close(); await vite.close();
