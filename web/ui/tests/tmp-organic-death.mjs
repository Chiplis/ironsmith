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
const log = [];
try {
  const page = await browser.newPage({ viewport: { width: 1600, height: 1000 } });
  const errors = [];
  page.on("pageerror", (e) => errors.push(String(e).slice(0, 250)));
  page.on("console", (m) => { if (m.type() === "error") errors.push(m.text().slice(0, 250)); });
  await page.goto(`http://127.0.0.1:${vitePort}`);
  await page.waitForSelector(".game-card", { timeout: 30000 });
  await page.waitForTimeout(3000);

  // Add Murder to hand via the Add Card sheet
  await page.click("text=ADD CARD");
  await page.waitForSelector('input[placeholder="Card name"]', { timeout: 5000 });
  await page.fill('input[placeholder="Card name"]', "Murder");
  await page.click(".add-card-submit");
  await page.waitForTimeout(900);
  log.push("added Murder");

  // Resolve the pregame mulligan and get to a phase with priority
  for (let i = 0; i < 6; i += 1) {
    const kept = await page.evaluate(() => {
      const button = [...document.querySelectorAll("button")].find((b) => /keep hand/i.test(b.textContent || "") && !b.disabled);
      if (button) { button.click(); return true; }
      return false;
    });
    await page.waitForTimeout(500);
    if (!kept) break;
  }
  log.push("kept hands");
  for (let i = 0; i < 6; i += 1) {
    const playable = await page.evaluate(() => document.querySelectorAll(".game-card.hand-card.playable, .game-card.hand-card[class*='glow-']").length);
    if (playable > 0) break;
    const passButton = await page.$(".pass-priority-btn");
    if (passButton) await passButton.click().catch(() => null);
    await page.waitForTimeout(600);
  }
  log.push("reached castable phase");

  // Expand the collapsed hand dock by hovering it, then click Murder
  const dock = await page.waitForSelector(".hand-zone-surface, .hand-reveal-shell", { timeout: 5000 });
  const dockBox = await dock.boundingBox();
  if (dockBox) {
    await page.mouse.move(dockBox.x + dockBox.width / 2, dockBox.y + Math.min(10, dockBox.height / 2));
    await page.waitForTimeout(700);
  }
  const handCard = await page.waitForSelector('.game-card.hand-card[data-card-name="Murder"]', { timeout: 8000 });
  await handCard.hover();
  await page.waitForTimeout(400);
  await handCard.click();
  await page.waitForTimeout(600);
  log.push("clicked Murder");

  // Adaptive decision-clicking loop: cast -> free cast option -> target -> confirm
  let deathSeen = false;
  let targetChosen = false;
  for (let round = 0; round < 14 && !deathSeen; round += 1) {
    const acted = await page.evaluate(() => {
      const visible = (el) => {
        const rect = el.getBoundingClientRect();
        return rect.width > 4 && rect.height > 4;
      };
      // 1) explicit cast/affirmative buttons (text, aria-label, or title)
      const buttons = [...document.querySelectorAll("button")].filter(visible);
      const labelOf = (b) => `${b.textContent || ""} ${b.getAttribute("aria-label") || ""} ${b.getAttribute("title") || ""}`.trim();
      const patterns = [/submit targets/i, /^resolve/i];
      if (!window.__fxMurderCast) patterns.push(/cast murder/i);
      for (const pattern of patterns) {
        const button = buttons.find((b) => pattern.test(labelOf(b)) && !b.disabled && b.getAttribute("aria-disabled") !== "true" && !/auto/i.test(labelOf(b)));
        if (button) {
          if (/cast murder/i.test(labelOf(button))) window.__fxMurderCast = true;
          button.click();
          return `button:${labelOf(button).trim().slice(0, 50)}`;
        }
      }
      // 2) legal target on the battlefield (prefer a creature: Ornithopter).
      // Selection toggles, so the loop only dispatches it once.
      if (!window.__fxTargetChosen) {
        const targets = [...document.querySelectorAll(".game-card.target-legal")].filter(visible);
        const creature = targets.find((t) => t.getAttribute("data-card-name") === "Ornithopter") || targets[0];
        if (creature) {
          const objectId = Number(creature.getAttribute("data-object-id"));
          window.__fxTargetChosen = true;
          window.dispatchEvent(new CustomEvent("ironsmith:target-choice", {
            detail: { target: { kind: "object", object: objectId } },
          }));
          return `target-choice:${creature.getAttribute("data-card-name")}:${objectId}`;
        }
      }
      // 3) pass priority to let the spell resolve once the stack has it
      const pass = document.querySelector(".pass-priority-btn");
      if (pass && !pass.disabled) {
        pass.click();
        return "pass";
      }
      return null;
    });
    log.push(`round ${round}: ${acted}`);
    await page.waitForTimeout(650);
    deathSeen = (await page.evaluate(() => document.querySelectorAll(".zone-death-effect").length)) > 0;
    if (!acted && !deathSeen && round > 6) break;
  }

  // Capture the choreography: collapse + stream + inspector reveal
  const frames = [];
  for (let frame = 0; frame < 10; frame += 1) {
    const snapshot = await page.evaluate(() => ({
      death: document.querySelectorAll(".zone-death-effect").length,
      canvas: Boolean(document.querySelector(".zone-move-effects-layer canvas")),
      inspectorToken: document.querySelectorAll("[data-zone-transition-token]").length,
    }));
    frames.push(snapshot);
    await page.screenshot({ path: path.join(__dirname, `tmp-od-${String(frame).padStart(2, "0")}.png`) });
    await page.waitForTimeout(280);
  }
  console.log(JSON.stringify({ log, deathSeen, frames, errors: errors.slice(0, 8) }, null, 1));
} finally {
  await browser.close();
  await vite.close();
}
