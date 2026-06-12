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
  page.on("pageerror", (error) => console.error("PAGEERROR:", String(error)));

  await page.goto(baseUrl);
  await page.waitForSelector("[data-mobile-battle-scene]", { timeout: 30000 });
  await page.waitForTimeout(2500);

  const checks = await page.evaluate(() => {
    const out = {};
    const row = document.querySelector(".mobile-mtga-control-row");
    out.controlRowHeight = row ? Math.round(row.getBoundingClientRect().height) : null;

    const strip = document.querySelector(".mobile-mtga-phase-strip");
    out.phaseStripHeight = strip ? Math.round(strip.getBoundingClientRect().height) : null;
    const activeLabel = document.querySelector(".mobile-mtga-phase-cell--active .mobile-mtga-phase-cell-label");
    out.activeLabelVisible = activeLabel ? getComputedStyle(activeLabel).display !== "none" : false;
    const inactiveLabel = document.querySelector(".mobile-mtga-phase-cell:not(.mobile-mtga-phase-cell--active) .mobile-mtga-phase-cell-label");
    out.inactiveLabelHidden = inactiveLabel ? getComputedStyle(inactiveLabel).display === "none" : null;

    const fan = document.querySelector(".mobile-mtga-hand-fan");
    if (fan) {
      const rect = fan.getBoundingClientRect();
      out.fanHeight = Math.round(rect.height);
      out.fanBottomClearance = Math.round(window.innerHeight - rect.bottom);
      out.fanClipPath = getComputedStyle(fan).clipPath;
    }

    // Hit-test: nothing from the hand should be touchable at the very bottom
    // of the screen (home-indicator zone), but cards should be touchable in
    // the fan band itself.
    const bottomHit = document.elementFromPoint(window.innerWidth / 2, window.innerHeight - 4);
    out.bottomEdgeHit = bottomHit ? `${bottomHit.tagName}.${String(bottomHit.className).split(" ")[0]}` : "none";
    out.bottomEdgeHitsHandCard = Boolean(bottomHit && bottomHit.closest && bottomHit.closest(".game-card.hand-card"));
    const fanRect = fan ? fan.getBoundingClientRect() : null;
    if (fanRect) {
      const midFan = document.elementFromPoint(window.innerWidth / 2, fanRect.top + fanRect.height / 2);
      out.fanBandHitsHandCard = Boolean(midFan && midFan.closest && midFan.closest(".game-card.hand-card, .mobile-mtga-hand-fan"));
    }

    out.safeAreaVarSat = getComputedStyle(document.documentElement).getPropertyValue("--sat").trim();
    out.safeAreaVarSab = getComputedStyle(document.documentElement).getPropertyValue("--sab").trim();
    return out;
  });
  console.log(JSON.stringify(checks, null, 2));

  await page.screenshot({ path: path.join(UI_ROOT, "tmp-mobile-mtga.png") });

  // Fan the hand open and screenshot again.
  const fan = await page.$(".mobile-mtga-hand-fan");
  if (fan) {
    const box = await fan.boundingBox();
    if (box) {
      await page.touchscreen.tap(box.x + 30, box.y + box.height / 2);
      await page.waitForTimeout(600);
      await page.screenshot({ path: path.join(UI_ROOT, "tmp-mobile-mtga-fanned.png") });
    }
  }
} finally {
  await browser.close();
  await vite.close();
}
