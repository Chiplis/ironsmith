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

async function inspectCard(page, name) {
  // Hover the matching battlefield/hand card until the inspector shows it.
  const handles = await page.$$(".game-card[data-object-id]");
  for (const handle of handles) {
    const cardName = await handle.getAttribute("data-card-name");
    if (cardName !== name) continue;
    const box = await handle.boundingBox();
    if (!box) continue;
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
    await page.waitForTimeout(1400);
    return true;
  }
  return false;
}

async function measure(page) {
  return page.evaluate(() => {
    const shells = [...document.querySelectorAll('[data-card-inspector="true"]')]
      .map((el) => ({ el, rect: el.getBoundingClientRect() }))
      .filter(({ rect }) => rect.width > 10 && rect.height > 10);
    const myZone = document.querySelector("[data-my-zone]");
    const myZoneRect = myZone ? myZone.getBoundingClientRect() : null;
    return shells.map(({ el, rect }) => {
      const dock = el.closest("[data-inspector-dock]");
      const dockRect = dock ? dock.getBoundingClientRect() : null;
      const line = el.querySelector(".inspector-oracle-line");
      return {
        dock: dock?.getAttribute("data-inspector-dock") || null,
        name: el.querySelector(".inspector-banner--identity")?.textContent?.trim()?.slice(0, 60) || null,
        shell: { x: Math.round(rect.left), y: Math.round(rect.top), w: Math.round(rect.width), h: Math.round(rect.height), bottom: Math.round(rect.bottom) },
        dockBottom: dockRect ? Math.round(dockRect.bottom) : null,
        myZoneTop: myZoneRect ? Math.round(myZoneRect.top) : null,
        overlapsBattlefield: myZoneRect ? Math.round(rect.bottom) > Math.round(myZoneRect.top) + 1 : null,
        oracleFontSize: line ? getComputedStyle(line).fontSize : null,
      };
    });
  });
}

try {
  const page = await browser.newPage({ viewport: { width: 1600, height: 1000 } });
  await page.goto(`http://127.0.0.1:${vitePort}`);
  await page.waitForSelector(".game-card", { timeout: 30000 });
  await page.waitForTimeout(2500);

  const names = await page.evaluate(() =>
    [...new Set([...document.querySelectorAll(".game-card[data-card-name]")].map((el) => el.getAttribute("data-card-name")))]);
  console.log("cards on table:", JSON.stringify(names));

  for (const target of ["Yawgmoth, Thran Physician", "Omniscience", "Plains"]) {
    const found = await inspectCard(page, target);
    if (!found) { console.log(`-- ${target}: not found`); continue; }
    const info = await measure(page);
    console.log(`-- hovering ${target}:`);
    for (const entry of info) console.log("   ", JSON.stringify(entry));
    await page.screenshot({ path: path.join(__dirname, `tmp-bounds-${target.split(",")[0].replace(/\s+/g, "-")}.png`) });
  }
} finally {
  await browser.close();
  await vite.close();
}
