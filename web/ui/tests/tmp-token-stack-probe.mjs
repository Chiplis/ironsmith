import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { createServer as createViteServer } from "vite";
import { encodePuzzlePayload } from "../src/lib/puzzles.js";
const UI_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const scratch = '/private/tmp/claude-501/-Users-chiplis-ironsmith/184fcd3d-3ee2-441b-9868-1366be20aa4d/scratchpad';
const PUZZLE = encodePuzzlePayload({ version: 1, players: [
  { name: "Alice", life: 20, zones: { battlefield: ["Island", "Island", "Island", "Island"], hand: ["Stormchaser's Talent", "Opt"], library: Array.from({ length: 8 }, () => "Island") } },
  { name: "Bob", life: 20, zones: { battlefield: ["Grizzly Bears"], library: Array.from({ length: 8 }, () => "Plains") } } ] });
const vite = await createViteServer({ root: UI_ROOT, configFile: path.join(UI_ROOT, "vite.config.js"), clearScreen: false, logLevel: "silent", server: { host: "127.0.0.1", port: 0, hmr: false, watch: null } });
await vite.listen();
const browser = await chromium.launch();
const report = {};
try {
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = []; page.on("pageerror", e => errors.push(String(e?.message || e)));
  page.on("console", m => { if (m.type() === 'error') errors.push('console: ' + m.text().slice(0, 200)); });
  await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/?puzzle=${PUZZLE}`);
  await page.locator(".game-card[data-object-id]").first().waitFor({ timeout: 120000 });
  for (let step = 0; step < 12; step += 1) {
    const button = page.locator(".decision-main-button").first();
    if (await button.count() === 0) break;
    const label = (await button.textContent() || "").trim();
    if (/^go to combat/i.test(label)) break;
    if (!/^(keep hand|pregame|continue|begin game|go to)/i.test(label)) break;
    await button.click(); await page.waitForTimeout(1000);
  }
  const dragCast = async (name, stopAt) => {
    const hand = page.locator(`.game-card.hand-card[data-card-name="${name}"]`).first();
    const hb = await hand.boundingBox();
    const table = await page.locator("[data-drop-zone]").first().boundingBox();
    await page.mouse.move(hb.x + hb.width / 2, hb.y + 24); await page.mouse.down();
    await page.mouse.move(hb.x + hb.width / 2, hb.y - 80, { steps: 10 });
    await page.mouse.move(table.x + table.width / 2, table.y + table.height / 2, { steps: 12 });
    await page.waitForTimeout(400); await page.mouse.up(); await page.waitForTimeout(1200);
    const labels = [];
    for (let i = 0; i < 8; i++) {
      const b = page.locator(".decision-main-button").first();
      if (!(await b.count())) break;
      const label = (await b.textContent() || '').trim(); labels.push(label);
      if (stopAt.test(label) || /go to|pass|attack/i.test(label)) break;
      await b.click(); await page.waitForTimeout(1500);
    }
    return labels;
  };
  report.talent = await dragCast("Stormchaser's Talent", /^$/);
  report.opt = await dragCast("Opt", /^resolve/i);
  await page.waitForTimeout(800);
  report.stackEntries = await page.evaluate(() => [...document.querySelectorAll('[class*="stack"] [class*="entry"], [class*="stack"] li')].map(e => e.innerText.replace(/\s+/g, ' ').slice(0, 80)));
  const imgs = page.locator('[class*="stack"] img');
  report.stackImages = await imgs.count();
  for (let i = 0; i < Math.min(await imgs.count(), 3); i++) {
    const sb = await imgs.nth(i).boundingBox(); if (!sb) continue;
    await page.mouse.move(sb.x + sb.width / 2, sb.y + sb.height / 2); await page.waitForTimeout(2000);
    report[`hover${i}`] = await page.evaluate(() => {
      const stages = [...document.querySelectorAll('.interactive-card-frame-stage')].map(s => ({ pres: s.dataset.framePresentation, id: s.dataset.inspectedObjectId, mode: s.dataset.frameMode, ready: s.dataset.renderReady, title: s.querySelector('.interactive-card-frame__title')?.textContent?.trim(), lines: [...s.querySelectorAll('.interactive-card-frame__rule-line')].map(l => l.textContent.trim().slice(0, 40)) }));
      const preview = document.querySelector('[data-card-inspector="true"], .floating-card-preview, [class*="FloatingCardPreview"], [class*="floating-card"]');
      const inspectorish = [...document.querySelectorAll('[class*="inspector"], [class*="preview"]')].map(e => e.className.toString().slice(0, 60)).filter((v, i, a) => a.indexOf(v) === i).slice(0, 12);
      return { stages: stages.filter(s => s.pres !== 'miniature'), previewText: preview ? preview.innerText.replace(/\s+/g, ' ').slice(0, 200) : null, inspectorish, fallbacks: document.querySelectorAll('.original-card-fallback, .original-card-details').length };
    });
    await page.screenshot({ path: `${scratch}/token-stack-hover${i}.png` });
  }
  console.log(JSON.stringify(report, null, 1)); console.log('errors', errors.slice(0, 8));
} finally { await browser.close(); await vite.close(); }
