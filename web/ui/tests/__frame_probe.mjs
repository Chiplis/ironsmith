import net from "node:net";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { createServer as createViteServer } from "vite";
import { encodePuzzlePayload } from "../src/lib/puzzles.js";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, "..");
const PUZZLE = encodePuzzlePayload({
  version: 1,
  players: [
    { name: "Alice", life: 20, zones: {
      battlefield: ["Agatha's Soul Cauldron", "Yawgmoth, Thran Physician", "Swamp", "Swamp"],
      graveyard: ["Walking Ballista"],
      library: Array.from({ length: 8 }, () => "Swamp") } },
    { name: "Bob", life: 20, zones: { battlefield: ["Grizzly Bears"], library: Array.from({ length: 8 }, () => "Plains") } },
  ],
});
async function freePort() {
  return new Promise((res, rej) => { const s = net.createServer(); s.once("error", rej);
    s.listen(0, "127.0.0.1", () => { const a = s.address(); const p = typeof a === "object" && a ? a.port : 0; s.close(() => res(p)); }); });
}
const port = await freePort();
const vite = await createViteServer({ root: UI_ROOT, configFile: path.join(UI_ROOT, "vite.config.js"),
  clearScreen: false, logLevel: "silent", server: { host: "127.0.0.1", port, strictPort: true, hmr: false, watch: null } });
await vite.listen();
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1600, height: 1000 } });
page.on("pageerror", (e) => console.log("PAGEERROR", String(e?.stack || e)));

await page.goto(`http://127.0.0.1:${port}/?puzzle=${PUZZLE}`);
await page.locator('.game-card[data-card-name="Agatha\'s Soul Cauldron"]').first().waitFor({ timeout: 120000 });
for (let i = 0; i < 12; i++) {
  const b = page.locator(".decision-main-button").first();
  if (await b.count() === 0) break;
  const label = (await b.textContent() || "").trim();
  if (/^go to combat/i.test(label)) break;
  if (!/^(keep hand|pregame|continue|begin game|go to)/i.test(label)) break;
  await b.click(); await page.waitForTimeout(1200);
}
async function hover(name) {
  await page.mouse.move(10, 500); await page.waitForTimeout(400);
  const c = page.locator(`.game-card[data-card-name="${name}"]`).first();
  const box = await c.boundingBox();
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.waitForTimeout(2500);
}
const dump = (label) => page.evaluate((lbl) => {
  const out = { label: lbl };
  const reg = document.querySelector(".registered-card-frame");
  const inter = document.querySelector(".interactive-card-frame, [data-frame-mode]");
  out.frameMode = inter?.dataset?.frameMode ?? null;
  out.registered = Boolean(reg);
  if (reg) {
    out.rulesScale = reg.dataset.rulesScale;
    out.rulesShrink = reg.dataset.rulesShrink;
    out.fields = [...reg.querySelectorAll('.registered-card-frame__field[data-field-kind="rule"]')].map((f) => {
      const line = f.querySelector(".interactive-card-frame__rule-line");
      const r = f.getBoundingClientRect();
      return {
        text: (f.dataset.liveText || "").slice(0, 44),
        replaced: f.dataset.replaced,
        flowTop: f.dataset.flowTop, flowBottom: f.dataset.flowBottom, flowLimit: f.dataset.flowLimit,
        fontPx: line ? +parseFloat(getComputedStyle(line).fontSize).toFixed(2) : null,
        top: +r.top.toFixed(1), bottom: +r.bottom.toFixed(1),
      };
    });
    const surface = reg.querySelector(".registered-card-frame__surface").getBoundingClientRect();
    out.surface = { top: +surface.top.toFixed(1), bottom: +surface.bottom.toFixed(1), h: +surface.height.toFixed(1) };
  }
  const stage = document.querySelector('.interactive-card-frame-stage[data-source-frame="true"]');
  if (stage) {
    const cs = getComputedStyle(stage);
    out.printed = {
      rulesFontSize: cs.getPropertyValue('--printed-rules-font-size').trim(),
      rulesLineHeight: cs.getPropertyValue('--printed-rules-line-height').trim(),
      firstLine: cs.getPropertyValue('--printed-rules-first-line').trim().slice(0, 200),
      paddingTop: cs.getPropertyValue('--printed-rules-padding-top').trim(),
      stageW: +stage.getBoundingClientRect().width.toFixed(1),
      all: Object.fromEntries((stage.getAttribute('style')||'').split(';')
        .map(d => d.split(':')).filter(p => p[0] && p[0].trim().startsWith('--printed'))
        .map(p => [p[0].trim(), p.slice(1).join(':').trim().slice(0, 150)])),
      scan: document.querySelector('.interactive-card-frame__art img')?.src || null,
    };
  }
  const boxes = [...document.querySelectorAll(".interactive-card-frame__rules[data-fit-text]")];
  out.rulesBoxes = boxes.map((b) => {
    const r = b.getBoundingClientRect();
    const line = b.querySelector(".interactive-card-frame__rule-line");
    return {
      fitScale: b.style.getPropertyValue("--card-rules-fit-scale"),
      spacingScale: b.style.getPropertyValue("--card-rules-spacing-scale"),
      overflow: b.dataset.textOverflow,
      boxH: +r.height.toFixed(1),
      fontPx: line ? +parseFloat(getComputedStyle(line).fontSize).toFixed(2) : null,
      preferredPx: +parseFloat(getComputedStyle(b).fontSize).toFixed(2),
    };
  });
  return out;
}, label);

await hover("Agatha's Soul Cauldron");
console.log("ASC:", JSON.stringify(await dump("ASC"), null, 1));
{
  const frame = page.locator('.interactive-card-frame-stage[data-source-frame="true"]').first();
  if (await frame.count()) await frame.screenshot({ path: "/private/tmp/claude-501/-Users-chiplis-ironsmith/5f97778b-f740-43d9-be31-8d0fa408ae50/scratchpad/asc-after.png" });
}
await hover("Yawgmoth, Thran Physician");
console.log("YAWGMOTH BASE:", JSON.stringify(await dump("yawgmoth-base"), null, 1));

// Grant the abilities.
await hover("Agatha's Soul Cauldron");
await page.locator("button", { hasText: /Exile\s+target card from a graveyard/ }).first().click();
await page.waitForTimeout(1500);
for (let i = 0; i < 8; i++) {
  const b = page.locator(".decision-main-button").first();
  const label = (await b.textContent() || "").trim();
  if (!/^resolve$/i.test(label)) break;
  await b.click(); await page.waitForTimeout(1500);
}
await page.waitForFunction(() => /Exile\s*1/.test(document.body.innerText), null, { timeout: 20000 });
await hover("Yawgmoth, Thran Physician");
console.log("YAWGMOTH GRANTED:", JSON.stringify(await dump("yawgmoth-granted"), null, 1));
{
  const frame = page.locator('.interactive-card-frame-stage[data-source-frame="true"]').first();
  if (await frame.count()) await frame.screenshot({ path: "/private/tmp/claude-501/-Users-chiplis-ironsmith/5f97778b-f740-43d9-be31-8d0fa408ae50/scratchpad/yawg-granted-after.png" });
}
await page.screenshot({ path: "/private/tmp/claude-501/-Users-chiplis-ironsmith/5f97778b-f740-43d9-be31-8d0fa408ae50/scratchpad/frame-granted.png" });
await browser.close();
await vite.close();
