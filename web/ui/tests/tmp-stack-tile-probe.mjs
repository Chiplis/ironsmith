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
      const address = server.address();
      const port = typeof address === "object" && address ? address.port : 0;
      server.close(() => resolve(port));
    });
  });
}

const vitePort = await freePort();
const vite = await createViteServer({
  root: UI_ROOT,
  configFile: path.join(UI_ROOT, "vite.config.js"),
  clearScreen: false,
  logLevel: "silent",
  server: { host: "127.0.0.1", port: vitePort, strictPort: true, hmr: false, watch: null },
});
await vite.listen();
const baseUrl = `http://127.0.0.1:${vitePort}`;

const browser = await chromium.launch();
try {
  const context = await browser.newContext({
    viewport: { width: 844, height: 390 },
    hasTouch: true,
    isMobile: true,
  });
  const page = await context.newPage();
  await page.goto(baseUrl);
  await page.waitForSelector("[data-mobile-battle-scene]", { timeout: 30000 });
  await page.waitForTimeout(1500);

  // Mount a synthetic stack rail with the exact markup StackCard's compact
  // variant emits, including a long name to exercise the 2-line clamp.
  await page.evaluate(() => {
    const art = (hue) => {
      const canvas = document.createElement("canvas");
      canvas.width = 100;
      canvas.height = 80;
      const ctx = canvas.getContext("2d");
      const grad = ctx.createLinearGradient(0, 0, 100, 80);
      grad.addColorStop(0, `hsl(${hue}, 60%, 45%)`);
      grad.addColorStop(1, `hsl(${hue + 40}, 70%, 25%)`);
      ctx.fillStyle = grad;
      ctx.fillRect(0, 0, 100, 80);
      return canvas.toDataURL();
    };
    const tile = (name, hue, focused) => `
      <div class="mobile-mtga-stack-rail-entry${focused ? " mobile-mtga-stack-rail-entry--focused" : ""}">
        <div class="game-card stack-card stack-card--compact overflow-hidden mobile-mtga-stack-rail-card${focused ? " stack-card-active" : ""}">
          <img class="stack-card-compact-art" src="${art(hue)}" alt="" />
          <div class="stack-card-compact-scrim"></div>
          <div class="stack-card-compact-name">${name}</div>
        </div>
      </div>`;
    const rail = document.createElement("aside");
    rail.className = "mobile-mtga-stack-rail";
    rail.id = "probe-rail";
    // The live scene zeroes --mobile-mtga-stack-rail-width while the stack is
    // empty; pin the real visible-stack width for the probe.
    rail.style.width = "56px";
    rail.innerHTML = [
      tile("Lightning Bolt", 10, true),
      tile("Kozilek, Butcher of Truth", 260, false),
      tile("Inspiration from Beyond", 200, false),
      tile("Opt", 140, false),
    ].join("");
    document.querySelector("[data-mobile-battle-scene]").appendChild(rail);
  });
  await page.waitForTimeout(300);

  const rail = await page.$("#probe-rail");
  const box = await rail.boundingBox();
  await page.screenshot({
    path: path.join(UI_ROOT, "tmp-stack-tiles.png"),
    clip: { x: box.x - 8, y: box.y, width: box.width + 16, height: Math.min(box.height, 320) },
  });
  console.log("rail box:", box);
} finally {
  await browser.close();
  await vite.close();
}
