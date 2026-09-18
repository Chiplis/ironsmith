import test from "node:test";
import net from "node:net";
import path from "node:path";
import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { createServer as createViteServer } from "vite";
import { encodePuzzlePayload } from "../src/lib/puzzles.js";
import corpus from "./fixtures/bilingual-frames/corpus.js";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, "..");
const base = new URL("./fixtures/bilingual-frames/", import.meta.url);
const english = corpus.filter(entry => entry.printing.lang === "en");

const PUZZLE = encodePuzzlePayload({
  version: 1,
  players: [
    {name: "Alice", life: 20, zones: {battlefield: ["Birds of Paradise", "Swamp", "Swamp"], library: Array.from({length: 8}, () => "Swamp")}},
    {name: "Bob", life: 20, zones: {battlefield: [], library: Array.from({length: 8}, () => "Swamp")}},
  ],
});

async function freePort() {
  return new Promise((resolve, reject) => {
    const server = net.createServer();
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const {port} = server.address();
      server.close(() => resolve(port));
    });
  });
}

async function frameReport(page) {
  return page.evaluate(() => {
    const preview = document.querySelector('[data-card-hover-preview]');
    const stage = preview?.querySelector('.interactive-card-frame-stage');
    return {
      previewVisible: preview?.dataset.visible,
      previewObject: preview?.dataset.previewObjectId,
      hasStage: Boolean(stage),
      renderReady: stage?.dataset.renderReady,
      mode: stage?.dataset.frameMode,
      reason: stage?.getAttribute('data-frame-fallback-reason'),
      sourceFrame: stage?.dataset.sourceFrame,
      status: stage?.style.getPropertyValue('--source-frame-status'),
      hasFallback: Boolean(stage?.querySelector('.original-card-fallback')),
      hasFrame: Boolean(stage?.querySelector('.interactive-card-frame')),
      detailsOpen: Boolean(stage?.querySelector('.original-card-details[open]')),
      rules: [...(stage?.querySelectorAll('.interactive-card-frame__rule-line') || [])].map(node => node.textContent),
      zoneImage: Boolean(preview?.querySelector('img.absolute')),
    };
  });
}

test("repro custom card frames", {timeout: 600000}, async () => {
  const vitePort = await freePort();
  const vite = await createViteServer({root: UI_ROOT, configFile: path.join(UI_ROOT, "vite.config.js"),
    clearScreen: false, logLevel: "silent",
    server: {host: "127.0.0.1", port: vitePort, strictPort: true, hmr: false, watch: null}});
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({viewport: {width: 1440, height: 900}});
    const errors = [];
    page.on("pageerror", error => errors.push(String(error?.message || error)));
    await page.route("https://cards.scryfall.io/**", async route => {
      const url = route.request().url();
      const entry = english.find(entry => url.includes(entry.printing.id));
      if (!entry) return route.abort();
      return route.fulfill({contentType: "image/jpeg", headers: {"Access-Control-Allow-Origin": "*"},
        body: await readFile(new URL(`${entry.slug}${url.includes("/art_crop/") ? "-art_crop" : "-normal"}.jpg`, base))});
    });
    await page.route("https://api.scryfall.com/**", route => {
      const url = new URL(route.request().url());
      if (url.pathname.startsWith("/sets/")) return route.fulfill({json: {icon_svg_uri: english[0].setSymbol}});
      if (url.pathname === "/cards/named") {
        const name = url.searchParams.get("exact") || url.searchParams.get("fuzzy") || "";
        const entry = english.find(entry => entry.printing.name.toLowerCase() === name.toLowerCase());
        return route.fulfill({status: entry ? 200 : 404, json: entry?.printing || {}});
      }
      if (url.pathname === "/cards/search") {
        const query = url.searchParams.get("q") || "";
        return route.fulfill({json: {data: english.filter(entry => query.includes(entry.printing.name)).map(entry => entry.printing), has_more: false}});
      }
      const entry = english.find(entry => url.pathname.endsWith(`/${entry.printing.id}`));
      return route.fulfill({status: entry ? 200 : 404, json: entry?.printing || {}});
    });
    await page.route("https://svgs.scryfall.io/**", route => route.abort());

    await page.goto(`http://127.0.0.1:${vitePort}/?puzzle=${PUZZLE}`);
    await page.locator('.game-card[data-card-name="Birds of Paradise"]').first().waitFor({timeout: 120000});
    for (let step = 0; step < 12; step += 1) {
      const button = page.locator(".decision-main-button").first();
      if (await button.count() === 0) break;
      const label = (await button.textContent() || "").trim();
      if (/^go to combat/i.test(label)) break;
      if (!/^(keep hand|pregame|continue|begin game|go to)/i.test(label)) break;
      await button.click();
      await page.waitForTimeout(900);
    }

    const hover = async (name, index = 0) => {
      const card = page.locator(`.game-card[data-card-name="${name}"]`).nth(index);
      const box = await card.boundingBox();
      await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2, {steps: 8});
      await page.waitForTimeout(3000);
      const at = await page.evaluate(point => {
        const node = document.elementFromPoint(point.x, point.y);
        return node?.closest('.game-card')?.dataset.cardName || node?.className || null;
      }, {x: box.x + box.width / 2, y: box.y + box.height / 2});
      console.log("HOVER", JSON.stringify({name, index, at, previews: await page.locator('[data-card-hover-preview]').count()}));
    };
    await hover("Birds of Paradise");
    console.log("CONTROL (real printing):", JSON.stringify(await frameReport(page)));
    await page.mouse.move(10, 10);
    await page.waitForTimeout(600);

    const compile = async ({name, rules, artUrl}) => {
      await page.getByRole("button", {name: "Compile Card"}).first().click();
      await page.locator(".card-forge-inline-name").waitFor({timeout: 30000});
      await page.locator(".card-forge-inline-name").fill(name);
      await page.locator(".card-forge-inline-rules").fill(rules);
      if (artUrl != null) await page.locator('input[aria-label="Art URL"]').fill(artUrl);
      await page.locator("select").nth(1).selectOption("battlefield").catch(() => {});
      await page.waitForTimeout(1200);
      const canCompile = await page.locator(".card-forge-submit").isEnabled();
      const status = await page.locator(".card-forge-status").first().textContent().catch(() => null);
      console.log("FORGE", JSON.stringify({name, canCompile, status}));
      await page.locator(".card-forge-submit").click();
      await page.waitForTimeout(3000);
      await page.keyboard.press("Escape");
      await page.mouse.move(700, 250);
      await page.waitForTimeout(1500);
      console.log("BOARD", JSON.stringify(await page.locator(".game-card").evaluateAll(cards => cards.map(card => card.dataset.cardName))));
    };

    await compile({name: "Birds of Paradise", rules: "Spells have split second."});
    await hover("Birds of Paradise", 1);
    console.log("TEMPLATE NAME:", JSON.stringify(await frameReport(page)));
    await page.screenshot({path: "/private/tmp/claude-501/-Users-chiplis-ironsmith/335f81de-4634-416d-8183-6eb92d7e5abd/scratchpad/repro-template.png"});
    await page.mouse.move(10, 10);
    await page.waitForTimeout(600);

    await compile({name: "Zzz Custom Probe", rules: "Spells have split second."});
    await hover("Zzz Custom Probe");
    console.log("CUSTOM NAME:", JSON.stringify(await frameReport(page)));
    await page.screenshot({path: "/private/tmp/claude-501/-Users-chiplis-ironsmith/335f81de-4634-416d-8183-6eb92d7e5abd/scratchpad/repro-custom.png"});
    console.log("ERRORS:", JSON.stringify(errors.slice(0, 5)));
  } finally { await browser.close(); await vite.close(); }
});
