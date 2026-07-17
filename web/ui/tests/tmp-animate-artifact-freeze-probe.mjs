// Probe: does the real UI freeze when Animate Artifact animates a non-creature artifact?
// Usage: node tests/tmp-animate-artifact-freeze-probe.mjs [TargetCardName]
import net from "node:net";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { createServer as createViteServer } from "vite";

const TARGET_CARD = process.argv[2] || "Howling Mine";
const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, "..");
const step = (msg) => console.error(`[${new Date().toISOString()}] ${msg}`);

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

async function mainThreadAlive(page, timeoutMs = 5000) {
  try {
    await Promise.race([
      page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => resolve(true)))),
      new Promise((_, reject) => setTimeout(() => reject(new Error("main thread unresponsive")), timeoutMs)),
    ]);
    return true;
  } catch {
    return false;
  }
}

const vitePort = await freePort();
const vite = await createViteServer({
  root: UI_ROOT, configFile: path.join(UI_ROOT, "vite.config.js"),
  clearScreen: false, logLevel: "silent",
  server: { host: "127.0.0.1", port: vitePort, strictPort: true, hmr: false, watch: null },
});
await vite.listen();
const browser = await chromium.launch();
let exitCode = 0;
try {
  const page = await browser.newPage({ viewport: { width: 1600, height: 1000 } });
  const pageErrors = [];
  page.on("pageerror", (e) => pageErrors.push(String(e?.stack || e)));
  page.on("console", (m) => { if (m.type() === "error") pageErrors.push(m.text()); });

  await page.goto(`http://127.0.0.1:${vitePort}`);
  await page.waitForSelector(".game-card", { timeout: 30000 });
  await page.waitForTimeout(3000);
  step("app loaded");

  for (let i = 0; i < 16; i += 1) {
    const acted = await page.evaluate(() => {
      const pick = [...document.querySelectorAll("button")].find(
        (b) => /keep hand|begin game|continue|^pregame$/i.test((b.textContent || "").trim()) && !b.disabled
      );
      if (pick) { pick.click(); return (pick.textContent || "").trim(); }
      return null;
    });
    step(`pregame action: ${acted}`);
    await page.waitForTimeout(600);
    const inGame = await page.evaluate(() =>
      ![...document.querySelectorAll("button")].some((b) => /^pregame$/i.test((b.textContent || "").trim()))
    );
    if (inGame && !acted) break;
    if (inGame) { step("pregame complete"); break; }
  }
  step("hands kept");

  async function addCard(name, zone) {
    await page.click("text=ADD CARD");
    await page.fill('input[placeholder="Card name"]', name);
    const zoneSelect = page.locator("select").filter({ has: page.locator('option[value="battlefield"]') }).first();
    await zoneSelect.selectOption(zone);
    await page.click(".add-card-submit");
    await page.waitForTimeout(1200);
    step(`added ${name} to ${zone}`);
  }

  for (let i = 0; i < 20; i += 1) {
    const phase = await page.evaluate(() => {
      const chip = [...document.querySelectorAll("button")].find((b) =>
        /upkeep|draw|main|combat|end/i.test(b.getAttribute("aria-label") || "")
      );
      return chip?.getAttribute("aria-label") || null;
    });
    if (phase && /main/i.test(phase)) { step(`reached phase: ${phase}`); break; }
    let advanced = null;
    try {
      const arrow = page.locator(".priority-control-stack").getByText(/^→/).first();
      advanced = (await arrow.textContent({ timeout: 1500 }))?.trim() || "→?";
      await arrow.click({ timeout: 1500, force: true });
    } catch { advanced = null; }
    if (!advanced) {
      if (i === 0) {
        const dump = await page.evaluate(() => ({
          decisionArea: [...document.querySelectorAll("[class*='decision'], [class*='priority'], [class*='action-strip'], [class*='prompt']")]
            .map((el) => `${el.className?.slice?.(0, 60)}: ${(el.textContent || "").trim().slice(0, 120)}`)
            .slice(0, 10),
          hints: [...document.querySelectorAll("[class*='hint'], [class*='phase']")]
            .map((el) => (el.textContent || "").trim().slice(0, 80)).filter(Boolean).slice(0, 10),
        }));
        step(`stuck dump: ${JSON.stringify(dump, null, 1)}`);
      }
      await page.keyboard.press("Space");
    }
    step(`advance from ${phase}: clicked ${JSON.stringify(advanced)}`);
    await page.waitForTimeout(700);
  }

  await addCard(TARGET_CARD, "battlefield");
  if (!process.env.NO_OMNISCIENCE) await addCard("Omniscience", "battlefield");
  if (process.env.NO_OMNISCIENCE) { await addCard("Island", "battlefield"); await addCard("Island", "battlefield"); }
  await addCard("Animate Artifact", "hand");

  const handCard = await page.waitForSelector('.game-card.hand-card[data-card-name="Animate Artifact"]', { timeout: 8000 });
  await handCard.click();
  await page.waitForTimeout(700);
  const visibleButtons = await page.evaluate(() =>
    [...document.querySelectorAll("button")]
      .filter((b) => { const r = b.getBoundingClientRect(); return r.width > 4 && r.height > 4; })
      .map((b) => `${(b.textContent || "").trim().slice(0, 50)}|aria:${(b.getAttribute("aria-label") || "").slice(0, 40)}|disabled:${b.disabled}`)
      .filter((t) => t !== "|aria:|disabled:false")
      .slice(0, 30)
  );
  step(`buttons after hand click: ${JSON.stringify(visibleButtons)}`);
  const castClicked = await page.evaluate(() => {
    const button = [...document.querySelectorAll("button")].find((b) => /cast/i.test(`${b.textContent} ${b.getAttribute("aria-label")}`) && !b.disabled);
    if (button) { button.click(); return button.textContent?.trim() || button.getAttribute("aria-label"); }
    return null;
  });
  await page.waitForTimeout(1200);
  step(`cast clicked: ${JSON.stringify(castClicked)}`);
  if (!(await mainThreadAlive(page))) throw new Error("FROZE after cast click");

  // Casting-method chooser (Omniscience adds a free-cast option).
  try {
    await page.getByText(/Cast without paying mana cost/i).first().click({ timeout: 3000 });
    step("picked free casting method");
    await page.getByText(/^Submit$/).first().click({ timeout: 3000 });
    step("submitted casting method");
    await page.waitForTimeout(800);
  } catch { step("no casting-method chooser"); }
  if (!(await mainThreadAlive(page))) throw new Error("FROZE after casting-method submit");

  const targetingDump = await page.evaluate(() => ({
    strip: document.querySelector("[class*='action-strip']")?.textContent?.trim()?.slice(0, 200) || null,
    cardClasses: [...document.querySelectorAll(".game-card")].map((el) =>
      `${el.getAttribute("data-card-name")}: ${[...el.classList].filter((c) => /target|legal|select|highlight/i.test(c)).join(",")}`
    ).filter((s) => !s.endsWith(": ")).slice(0, 10),
  }));
  step(`targeting dump: ${JSON.stringify(targetingDump)}`);
  const legal = await page.$$(`.game-card.target-legal`);
  step(`legal targets visible: ${legal.length}`);
  const targetEl = await page.$(`.game-card.target-legal[data-card-name="${TARGET_CARD}"]`)
    || await page.$(`.game-card[data-card-name="${TARGET_CARD}"]`);
  if (!targetEl) throw new Error(`target element for ${TARGET_CARD} not found`);
  await targetEl.click();
  await page.waitForTimeout(500);
  step("target clicked");
  if (!(await mainThreadAlive(page))) throw new Error("FROZE after target click");

  // Confirm-targets button if present, then let auto-pass resolve.
  await page.evaluate(() => {
    const button = [...document.querySelectorAll("button")].find((b) => /confirm|done|submit/i.test(b.textContent || "") && !b.disabled);
    if (button) button.click();
  });
  step("confirm clicked (if present)");

  for (let i = 0; i < 10; i += 1) {
    await page.waitForTimeout(1000);
    if (!(await mainThreadAlive(page))) throw new Error(`FROZE during resolution wait (t=${i + 1}s)`);
    const done = await page.evaluate((name) => {
      const cards = [...document.querySelectorAll(`.game-card[data-card-name="${name}"]`)];
      return cards.some((el) => (el.getAttribute("data-power-toughness") || el.textContent || "").includes("/"));
    }, TARGET_CARD);
    if (done) break;
  }

  const summary = await page.evaluate((name) => {
    const el = document.querySelector(`.game-card[data-card-name="${name}"]`);
    return {
      found: !!el,
      classes: el?.className || null,
      text: (el?.textContent || "").slice(0, 200),
      auraOnBattlefield: !!document.querySelector('.game-card[data-card-name="Animate Artifact"]:not(.hand-card)'),
    };
  }, TARGET_CARD);
  step(`final: ${JSON.stringify(summary)}`);
  step(`pageErrors: ${JSON.stringify(pageErrors.slice(0, 5))}`);
  if (!(await mainThreadAlive(page))) throw new Error("FROZE at end");
  console.log("NO FREEZE");
} catch (err) {
  exitCode = 1;
  console.error("PROBE FAILURE:", err.message);
} finally {
  await browser.close().catch(() => {});
  await vite.close().catch(() => {});
  process.exit(exitCode);
}
