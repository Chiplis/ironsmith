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
const browser = await chromium.launch({ args: ["--use-gl=angle"] });
try {
  const page = await browser.newPage({ viewport: { width: 1600, height: 1000 } });
  const errors = [];
  page.on("pageerror", (e) => errors.push(String(e).slice(0, 300)));
  page.on("console", (m) => { if (m.type() === "error") errors.push(m.text().slice(0, 300)); });
  await page.goto(`http://127.0.0.1:${vitePort}`);
  await page.waitForSelector(".game-card", { timeout: 30000 });
  await page.waitForTimeout(2500);

  await page.evaluate(() => {
    const cards = [...document.querySelectorAll('.game-card.field-card[data-card-name="Ornithopter"]')];
    const el = cards[cards.length - 1];
    const r = el.getBoundingClientRect();
    const plain = { left: r.left, top: r.top, width: r.width, height: r.height };
    const clone = el.cloneNode(true);
    clone.classList.remove("battlefield-row-card--layout-hold");
    window.dispatchEvent(new CustomEvent("ironsmith:zone-move-effects", {
      detail: { effects: [
        { id: "s1", kind: "death-collapse", collapseVariant: "destroyed", rect: plain, travelsToInspector: false,
          includeSourceClone: true, sourceCloneHtml: clone.outerHTML, card: { name: "Ornithopter" }, playerKey: "0", startDelayMs: 0 },
        { id: "s2", kind: "marquee-stream", streamProfile: "death", rect: plain, travelsToInspector: true,
          includeSourceClone: false, card: { name: "Ornithopter" }, playerKey: "0", targetToken: "missing-token",
          targetScope: "inspector", groupId: "g1", startDelayMs: 0, accentRgb: "142, 211, 255" },
        { id: "s3", kind: "wipe-wave", rect: { left: plain.left - 200, top: plain.top - 40, width: 700, height: 240 } },
        { id: "s4", kind: "counter-shatter", rect: { left: plain.left + 220, top: plain.top, width: plain.width, height: plain.height },
          travelsToInspector: false, includeSourceClone: true, sourceCloneHtml: clone.outerHTML, card: { name: "Ornithopter" }, playerKey: "0" },
        { id: "s5", kind: "marquee-stream", streamProfile: "sacrifice", rect: plain, travelsToInspector: true,
          includeSourceClone: false, card: { name: "X" }, playerKey: "1", targetToken: "missing-token",
          targetScope: "inspector", groupId: "g1", startDelayMs: 90 },
        { id: "s6", kind: "marquee-stream", streamProfile: "counter", rect: plain, travelsToInspector: true,
          includeSourceClone: false, card: { name: "Y" }, playerKey: "1", targetToken: "missing-token",
          targetScope: "inspector", groupId: "g1", startDelayMs: 180 },
      ] },
    }));
  });
  await page.waitForTimeout(1500);
  const probe = await page.evaluate(() => ({
    canvas: Boolean(document.querySelector(".zone-move-effects-layer canvas")),
    collapse: document.querySelectorAll(".zone-death-effect").length,
    shatterShards: document.querySelectorAll(".zone-shatter-shard").length,
  }));
  await page.screenshot({ path: path.join(__dirname, "tmp-shader-smoke.png") });
  await page.waitForTimeout(1800);
  console.log(JSON.stringify(probe), "ERRORS:", JSON.stringify(errors.slice(0, 6)));
} finally {
  await browser.close();
  await vite.close();
}
