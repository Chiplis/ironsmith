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
try {
  const page = await browser.newPage({ viewport: { width: 1600, height: 1000 } });
  await page.goto(`http://127.0.0.1:${vitePort}`);
  await page.waitForSelector(".game-card", { timeout: 30000 });
  await page.waitForTimeout(3000);
  await page.click("text=ADD CARD");
  await page.fill('input[placeholder="Card name"]', "Murder");
  await page.click(".add-card-submit");
  await page.waitForTimeout(900);

  const dock = await page.waitForSelector(".hand-zone-surface, .hand-reveal-shell", { timeout: 5000 });
  const dockBox = await dock.boundingBox();
  await page.mouse.move(dockBox.x + dockBox.width / 2, dockBox.y + 8);
  await page.waitForTimeout(700);
  for (let i = 0; i < 6; i += 1) {
    const kept = await page.evaluate(() => {
      const button = [...document.querySelectorAll("button")].find((b) => /keep hand/i.test(b.textContent || "") && !b.disabled);
      if (button) { button.click(); return true; }
      return false;
    });
    await page.waitForTimeout(500);
    if (!kept) break;
  }
  const handCard = await page.waitForSelector('.game-card.hand-card[data-card-name="Murder"]', { timeout: 8000 });
  await handCard.click();
  await page.waitForTimeout(700);
  await page.evaluate(() => {
    const button = [...document.querySelectorAll("button")].find((b) => /cast/i.test(`${b.textContent} ${b.getAttribute("aria-label")}`) && !b.disabled);
    if (button) button.click();
  });
  await page.waitForTimeout(900);

  const probe = await page.evaluate(() => {
    const visible = (el) => {
      const r = el.getBoundingClientRect();
      return r.width > 4 && r.height > 4;
    };
    return {
      decisionKindButtons: [...document.querySelectorAll("button")].filter(visible)
        .map((b) => `${(b.textContent || "").trim().slice(0, 60)}|aria:${(b.getAttribute("aria-label") || "").slice(0, 40)}`)
        .filter((t) => t !== "|aria:").slice(0, 40),
      targetLegal: [...document.querySelectorAll(".game-card.target-legal")].map((el) => el.getAttribute("data-card-name")),
      decisionText: document.querySelector("[class*='decision-popup'], [class*='decision-panel'], [class*='action-strip']")?.textContent?.slice(0, 300) || null,
    };
  });
  console.log(JSON.stringify(probe, null, 1));
  await page.screenshot({ path: path.join(__dirname, "tmp-probe-cast.png") });
} finally {
  await browser.close();
  await vite.close();
}
