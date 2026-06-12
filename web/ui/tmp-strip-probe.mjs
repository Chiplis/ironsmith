import net from "node:net";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { createServer as createViteServer } from "vite";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = __dirname;

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

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1680, height: 960 } });
page.on("pageerror", (error) => console.log("PAGEERROR:", String(error)));

const pillTexts = () => page.evaluate(() => (
  Array.from(document.querySelectorAll(".action-strip-pill"))
    .map((el) => (el.innerText || "").replace(/\s+/g, " ").trim())
    .filter(Boolean)
));

try {
  await page.goto(`http://127.0.0.1:${vitePort}`);
  await page.waitForSelector(".game-card[data-object-id]", { timeout: 60000 });
  await page.waitForTimeout(3000);

  // Get past the mulligan step for every seat that shows a keep button.
  for (let i = 0; i < 8; i += 1) {
    const keep = page.locator("button", { hasText: /keep hand/i }).first();
    if (!(await keep.count())) break;
    try {
      await keep.click({ timeout: 2000 });
    } catch {
      break;
    }
    await page.waitForTimeout(800);
  }

  await page.waitForSelector(".action-strip-pill", { timeout: 30000 });
  await page.waitForTimeout(1000);

  const initialPills = await pillTexts();
  console.log("STRIP (no selection):", JSON.stringify(initialPills));
  const manaPillPattern = /\badd\b/i;
  const initialManaPills = initialPills.filter((text) => manaPillPattern.test(text));
  console.log(initialManaPills.length === 0
    ? "PASS: no mana-ability pills in the default strip"
    : `FAIL: mana pills visible by default: ${JSON.stringify(initialManaPills)}`);

  // Find a battlefield Plains by hovering cards and reading the inspector.
  const cardIds = await page.evaluate(() => (
    Array.from(document.querySelectorAll(".game-card[data-object-id]"))
      .map((el) => el.getAttribute("data-object-id"))
  ));
  let plainsId = null;
  for (const id of cardIds) {
    const box = await page.locator(`.game-card[data-object-id="${id}"]`).first().boundingBox();
    if (!box) continue;
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2, { steps: 3 });
    await page.waitForTimeout(450);
    const title = await page.evaluate(() => (
      document.querySelector('[data-card-inspector="true"] .inspector-banner--identity')?.innerText || ""
    ));
    if (/^plains/i.test(title.trim()) && /battlefield/i.test(title)) {
      plainsId = id;
      break;
    }
  }
  if (!plainsId) throw new Error("no battlefield Plains found");

  await page.locator(`.game-card[data-object-id="${plainsId}"]`).first().click();
  await page.waitForTimeout(800);
  const selectedPills = await pillTexts();
  console.log("STRIP (Plains selected):", JSON.stringify(selectedPills));
  console.log(selectedPills.some((text) => manaPillPattern.test(text))
    ? "PASS: selecting the Plains reveals its mana ability"
    : "FAIL: no mana ability shown after selecting the Plains");

  // Deselect by pressing Escape, then start casting Divination ({2}{U}).
  // Sorceries only become castable at main phase, so advance priority a bit.
  await page.keyboard.press("Escape");
  await page.waitForTimeout(500);
  let castPill = page.locator(".action-strip-pill", { hasText: /divination/i }).first();
  for (let i = 0; i < 8 && !(await castPill.count()); i += 1) {
    const pass = page.locator(".pass-priority-btn").first();
    if (!(await pass.count())) break;
    try {
      await pass.click({ timeout: 2000 });
    } catch {
      break;
    }
    await page.waitForTimeout(900);
    castPill = page.locator(".action-strip-pill", { hasText: /divination/i }).first();
  }
  if (await castPill.count()) {
    await castPill.click();
    await page.waitForTimeout(1200);
    const paying = await page.evaluate(() => Boolean(window.__ironsmithE2E?.snapshot?.()?.state) || true);
    const paymentPills = await pillTexts();
    console.log("STRIP (paying for Divination):", JSON.stringify(paymentPills), paying ? "" : "(no snapshot)");
    console.log(paymentPills.some((text) => manaPillPattern.test(text))
      ? "PASS: mana abilities appear during mana payment"
      : "FAIL: no mana abilities during mana payment");
    await page.screenshot({ path: "tmp-strip-payment.png" });
  } else {
    console.log("SKIP: no Divination cast pill found");
    await page.screenshot({ path: "tmp-strip-nodivination.png" });
  }
} finally {
  await browser.close();
  await vite.close();
}
