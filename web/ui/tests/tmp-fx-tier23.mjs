import net from "node:net";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { createServer as createViteServer } from "vite";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, "..");

async function freePort() {
  return new Promise((resolve, reject) => {
    const server = net.createServer();
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const port = server.address().port;
      server.close(() => resolve(port));
    });
  });
}

const vitePort = await freePort();
const vite = await createViteServer({
  root: UI_ROOT, configFile: path.join(UI_ROOT, "vite.config.js"),
  clearScreen: false, logLevel: "silent",
  server: { host: "127.0.0.1", port: vitePort, strictPort: true, hmr: false, watch: null },
});
await vite.listen();

const browser = await chromium.launch();
const results = {};
try {
  const page = await browser.newPage({ viewport: { width: 1600, height: 1000 } });
  const errors = [];
  page.on("pageerror", (e) => errors.push(String(e).slice(0, 250)));
  page.on("console", (m) => { if (m.type() === "error") errors.push(m.text().slice(0, 250)); });
  await page.goto(`http://127.0.0.1:${vitePort}`);
  await page.waitForSelector(".game-card", { timeout: 30000 });
  await page.waitForTimeout(3000);

  const stableIds = await page.evaluate(() => (
    [...document.querySelectorAll(".game-card.field-card[data-stable-id]")]
      .slice(0, 4)
      .map((el) => el.getAttribute("data-stable-id"))
  ));

  const inject = (events) => page.evaluate((detailEvents) => {
    window.dispatchEvent(new CustomEvent("ironsmith:game-effect-animations-debug", {
      detail: { events: detailEvents },
    }));
  }, events);

  const probe = (selector) => page.evaluate(
    (sel) => document.querySelectorAll(sel).length,
    selector
  );

  // Tier 3 engine-effect-driven flourishes
  await inject([{ type: "engine-effect", kind: "coin_flip", id: "d1", playerKey: "0", otherPlayerKey: "", stableIds: [], value: 1, text: "heads" }]);
  await page.waitForTimeout(400);
  results.coin = await probe(".game-fx-coin-chip");
  await page.screenshot({ path: path.join(__dirname, "tmp-t3-coin.png") });
  await page.waitForTimeout(1800);

  await inject([{ type: "engine-effect", kind: "die_roll", id: "d2", playerKey: "1", otherPlayerKey: "", stableIds: [], value: 17, text: "d20" }]);
  await page.waitForTimeout(1300);
  results.die = await page.evaluate(() => document.querySelector(".game-fx-coin-result")?.textContent || null);
  await page.waitForTimeout(900);

  await inject([{ type: "engine-effect", kind: "level_up", id: "d3", playerKey: "0", otherPlayerKey: "", stableIds: [stableIds[0]], value: null, text: "monstrous" }]);
  await page.waitForTimeout(300);
  results.levelUp = {
    ring: await probe(".game-fx-levelup-ring"),
    label: await page.evaluate(() => document.querySelector(".game-fx-levelup-label")?.textContent || null),
  };
  await page.screenshot({ path: path.join(__dirname, "tmp-t3-levelup.png") });
  await page.waitForTimeout(1100);

  await inject([{ type: "engine-effect", kind: "transform", id: "d4", playerKey: "0", otherPlayerKey: "", stableIds: [stableIds[1]], value: null, text: "" }]);
  await page.waitForTimeout(350);
  results.flip = await probe(".game-fx-flip-sheen");
  await page.waitForTimeout(900);

  await inject([
    { type: "engine-effect", kind: "monarch", id: "d5", playerKey: "0", otherPlayerKey: "", stableIds: [], value: null, text: "" },
  ]);
  await page.waitForTimeout(450);
  results.monarch = await page.evaluate(() => document.querySelector(".game-fx-hud-label")?.textContent || null);
  await page.screenshot({ path: path.join(__dirname, "tmp-t3-monarch.png") });
  await page.waitForTimeout(1700);

  await inject([{ type: "engine-effect", kind: "day_night", id: "d6", playerKey: "", otherPlayerKey: "", stableIds: [], value: null, text: "night" }]);
  await page.waitForTimeout(700);
  results.night = await probe(".game-fx-board-tint--night");
  await page.screenshot({ path: path.join(__dirname, "tmp-t3-night.png") });
  await page.waitForTimeout(1800);

  await inject([{ type: "engine-effect", kind: "extra_turn", id: "d7", playerKey: "0", otherPlayerKey: "", stableIds: [], value: null, text: "" }]);
  await page.waitForTimeout(500);
  results.extraTurn = await page.evaluate(() => document.querySelector(".game-fx-turn-banner-text")?.textContent || null);
  await page.waitForTimeout(1700);

  await inject([{ type: "engine-effect", kind: "life_exchange", id: "d8", playerKey: "0", otherPlayerKey: "1", stableIds: [], value: null, text: "" }]);
  await page.waitForTimeout(350);
  results.lifeSwapOrbs = await probe(".game-fx-lifeswap-orb");
  await page.waitForTimeout(1300);

  await inject([{ type: "engine-effect", kind: "mana_added", id: "d9", playerKey: "0", otherPlayerKey: "", stableIds: [stableIds[2]], value: 2, text: "{G}{G}" }]);
  await page.waitForTimeout(300);
  results.manaMotes = await probe(".game-fx-mana-mote");
  await page.waitForTimeout(1100);

  await inject([{ type: "engine-effect", kind: "damage", id: "d10", playerKey: "", otherPlayerKey: "", stableIds: [stableIds[0], stableIds[1]], value: 3, text: "" }]);
  await page.waitForTimeout(250);
  results.damageLunge = await page.evaluate((sid) => {
    const el = [...document.querySelectorAll(".game-card[data-stable-id]")]
      .find((node) => node.getAttribute("data-stable-id") === sid);
    return el ? el.style.getPropertyValue("--card-jolt-x") !== "" : false;
  }, stableIds[0]);
  await page.waitForTimeout(700);

  await inject([{ type: "engine-effect", kind: "phase_out", id: "d11", playerKey: "0", otherPlayerKey: "", stableIds: [stableIds[3]], value: null, text: "" }]);
  await page.waitForTimeout(300);
  results.phaseGhost = await probe(".game-fx-phase-ghost");
  await page.waitForTimeout(900);

  // Tier 2: reanimation flight (graveyard->battlefield, lands on a real card rect)
  const targetObjectId = await page.evaluate(() => (
    document.querySelector(".game-card.field-card[data-object-id]")?.getAttribute("data-object-id")
  ));
  await inject([{ type: "zone-flight", kind: "reanimate", id: "d12", playerKey: "0", fromZone: "graveyard", toZone: "battlefield", cardName: "Ornithopter", objectId: targetObjectId, revealsFace: false }]);
  await page.waitForTimeout(350);
  results.reanimateFlight = await probe(".game-fx-flight--necro");
  await page.waitForTimeout(1100);

  results.errors = errors.slice(0, 8);
} finally {
  await browser.close();
  await vite.close();
}
console.log(JSON.stringify(results, null, 1));
