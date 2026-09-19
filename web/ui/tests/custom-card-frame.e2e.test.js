import test from "node:test";
import assert from "node:assert/strict";
import net from "node:net";
import path from "node:path";
import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { createServer as createViteServer } from "vite";
import { encodePuzzlePayload } from "../src/lib/puzzles.js";
import corpus from "./fixtures/bilingual-frames/corpus.js";

// The forge compiles a card from a template, so its name can be an existing
// printing's. That printing is then only a scan to lay the compiled card's own
// text over, and a compiled card with a name of its own has no scan at all.
const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, "..");
const fixtures = new URL("./fixtures/bilingual-frames/", import.meta.url);
const english = corpus.filter(entry => entry.printing.lang === "en");
const template = english.find(entry => entry.printing.name === "Birds of Paradise");
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

async function routeScryfall(page) {
  await page.route("https://cards.scryfall.io/**", async route => {
    const url = route.request().url();
    const entry = english.find(entry => url.includes(entry.printing.id));
    if (!entry) return route.abort();
    return route.fulfill({contentType: "image/jpeg", headers: {"Access-Control-Allow-Origin": "*"},
      body: await readFile(new URL(`${entry.slug}${url.includes("/art_crop/") ? "-art_crop" : "-normal"}.jpg`, fixtures))});
  });
  await page.route("https://api.scryfall.com/**", route => {
    const url = new URL(route.request().url());
    if (url.pathname.startsWith("/sets/")) return route.fulfill({json: {icon_svg_uri: template.setSymbol}});
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
}

test("a compiled custom card carries its own text on the template's frame, with or without art", {timeout: 300000}, async () => {
  const port = await freePort();
  const vite = await createViteServer({root: UI_ROOT, configFile: path.join(UI_ROOT, "vite.config.js"),
    clearScreen: false, logLevel: "silent",
    server: {host: "127.0.0.1", port, strictPort: true, hmr: false, watch: null}});
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({viewport: {width: 1600, height: 1200}});
    const errors = [];
    page.on("pageerror", error => errors.push(String(error?.message || error)));
    await routeScryfall(page);
    await page.goto(`http://127.0.0.1:${port}/?puzzle=${PUZZLE}`);
    await page.locator('.game-card[data-card-name="Birds of Paradise"]').first().waitFor({timeout: 120000});
    for (let step = 0; step < 12; step += 1) {
      const button = page.locator(".decision-main-button").first();
      if (await button.count() === 0) break;
      const label = (await button.textContent() || "").trim();
      if (!/^(keep hand|pregame|continue|begin game|go to)/i.test(label) || /^go to combat/i.test(label)) break;
      await button.click();
      await page.waitForTimeout(900);
    }

    const compile = async ({name, rules, zone}) => {
      await page.getByRole("button", {name: "Compile Card"}).first().click();
      await page.locator(".card-forge-inline-name").waitFor({timeout: 30000});
      await page.locator(".card-forge-inline-name").fill(name);
      await page.locator(".card-forge-inline-rules").fill(rules);
      await page.locator("select.card-forge-input").nth(1).selectOption(zone);
      await page.waitForTimeout(1200);
      await page.locator(".card-forge-submit").click();
      await page.waitForTimeout(2500);
      await page.keyboard.press("Escape");
      await page.mouse.move(700, 250);
      await page.waitForTimeout(1500);
    };
    const handCard = name => page.evaluate(cardName => {
      const card = [...document.querySelectorAll(`.game-card[data-card-name="${cardName}"]`)]
        .find(node => node.querySelector(".interactive-card-frame__rule-line")?.textContent === "Spells have split second.");
      const stage = card?.querySelector(".interactive-card-frame-stage");
      return card ? {
        mode: stage?.dataset.frameMode, ready: stage?.dataset.renderReady,
        title: card.querySelector(".interactive-card-frame__title")?.textContent,
        rules: [...card.querySelectorAll(".interactive-card-frame__rule-line")].map(line => line.textContent),
      } : null;
    }, name);

    // Reusing the template's name: the printing is masked and the compiled
    // text is laid over it, instead of the scan's own printed lettering.
    await compile({name: "Birds of Paradise", rules: "Spells have split second.", zone: "hand"});
    await page.waitForTimeout(3500);
    const templated = await handCard("Birds of Paradise");
    assert.ok(templated, "the compiled card draws its own frame in hand");
    assert.equal(templated.mode, "masked");
    assert.equal(templated.ready, "true");
    // None of the template printing's own wording (rules or flavor) may stand
    // in for the compiled card's text.
    assert.deepEqual(templated.rules, ["Spells have split second."]);

    // A name of its own resolves to no printing at all: the placeholder frame
    // still renders, immediately, rather than leaving a hidden inert stage.
    await compile({name: "Zzz Compiled Probe", rules: "Spells have split second.", zone: "battlefield"});
    await page.waitForTimeout(3500);
    const probe = page.locator('.game-card[data-card-name="Zzz Compiled Probe"]').first();
    await probe.click({force: true});
    await page.waitForTimeout(3500);
    const preview = await page.evaluate(() => {
      const stage = document.querySelector('[data-card-hover-preview] .interactive-card-frame-stage');
      return stage ? {
        visible: document.querySelector("[data-card-hover-preview]")?.dataset.visible,
        mode: stage.dataset.frameMode, ready: stage.dataset.renderReady, inert: stage.inert,
        title: stage.querySelector(".interactive-card-frame__title")?.textContent,
        rules: [...stage.querySelectorAll(".interactive-card-frame__rule-line")].map(line => line.textContent),
      } : null;
    });
    assert.deepEqual(preview, {
      visible: "true", mode: "placeholder", ready: "true", inert: false,
      title: "Zzz Compiled Probe", rules: ["Spells have split second."],
    });
    assert.deepEqual(errors, []);
  } finally { await browser.close(); await vite.close(); }
});
