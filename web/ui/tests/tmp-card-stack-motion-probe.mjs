// Temporary probe: drives a real match through every card-stack motion and
// screenshots each one mid-flight. Usage: node tests/tmp-card-stack-motion-probe.mjs <outDir>
import fs from "node:fs";
import net from "node:net";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { createServer as createViteServer } from "vite";
import { encodePuzzlePayload } from "../src/lib/puzzles.js";

const UI_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const OUT = process.argv[2] || "/tmp/card-stack-probe";
fs.mkdirSync(OUT, { recursive: true });

const library = Array.from({ length: 8 }, () => "Island");
const PUZZLE = encodePuzzlePayload({
  version: 1,
  players: [
    {
      name: "Alice",
      life: 20,
      zones: {
        battlefield: ["Mountain", "Mountain", "Mountain", "Mountain", "Island", "Island", "Island", "Island", "Blood Artist", "Prodigal Pyromancer"],
        hand: ["Lightning Bolt", "Reverberate", "Counterspell", "Hieroglyphic Illumination", "Urza's Saga", "Shock"],
        library,
      },
    },
    { name: "Bob", life: 20, zones: { battlefield: ["Grizzly Bears", "Llanowar Elves", "Island"], library } },
  ],
});

async function freePort() {
  return new Promise((resolve, reject) => {
    const server = net.createServer();
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const { port } = server.address();
      server.close(() => resolve(port));
    });
  });
}

const port = await freePort();
const vite = await createViteServer({
  root: UI_ROOT,
  configFile: path.join(UI_ROOT, "vite.config.js"),
  clearScreen: false,
  logLevel: "silent",
  server: { host: "127.0.0.1", port, strictPort: true, hmr: false, watch: null },
});
await vite.listen();
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
const errors = [];
page.on("pageerror", (error) => errors.push(String(error)));
page.on("console", (message) => {
  if (message.type() === "error" && /stack|fx|card/i.test(message.text())) errors.push(message.text());
});
const sleep = (ms) => page.waitForTimeout(ms);
let shot = 0;
const snap = async (name) => {
  shot += 1;
  const file = path.join(OUT, `${String(shot).padStart(2, "0")}-${name}.png`);
  await page.screenshot({ path: file });
  console.log("shot", file);
};
const label = () => page.locator(".decision-main-button").first().textContent().catch(() => "");
const box = (locator) => locator.first().boundingBox();
const center = (b) => ({ x: b.x + b.width / 2, y: b.y + b.height / 2 });
const battlefieldCard = (name) => page.locator(`.battlefield-row .game-card[data-card-name="${name}"]`);
const handCard = (name) => page.locator(`.game-card.hand-card[data-card-name="${name}"]`);
const pileCard = (name) => page.locator(`.card-stack-slot:not([data-leaving]) .stack-card--frame[data-card-name="${name}"]`);
const slots = () => page.evaluate(() => [...document.querySelectorAll(".card-stack-slot")].map((s) => {
  const r = s.getBoundingClientRect();
  return `${s.querySelector("[data-card-name]")?.dataset.cardName}[${Object.keys(s.dataset).filter((k) => k !== "stackKey").join("+")}] ${Math.round(r.width)}x${Math.round(r.height)} op=${s.querySelector(".card-stack-motion")?.style.opacity}`;
}));

async function dragHandTo(name, to) {
  const b = await box(handCard(name));
  await page.mouse.move(b.x + b.width / 2, b.y + 24);
  await page.mouse.down();
  await page.mouse.move(b.x + b.width / 2, b.y - 120, { steps: 8 });
  await page.mouse.move(to.x, to.y, { steps: 18 });
  await page.mouse.up();
}

const bobButton = () => page.locator("button", { hasText: /^Bob$/ });
const trail = [];

// Answers whatever the engine asks until it hands back priority: a target
// (the given one, else Bob), payment. Returns the label it stopped at.
async function settle(target) {
  for (let i = 0; i < 8; i += 1) {
    await sleep(650);
    const text = ((await label()) || "").trim();
    trail.push(text);
    if (/submit targets \(0\)/i.test(text)) {
      const pick = target && await target.count() ? target : bobButton();
      const b = await box(pick);
      await page.mouse.click(b.x + b.width / 2, b.y + Math.min(30, b.height / 2));
    } else if (/submit targets \(1\)/i.test(text)) {
      await clickMain(/submit targets \(1\)/);
    } else if (/^pay/i.test(text)) {
      await clickMain(/^\s*pay/);
    } else if (await page.locator("button:visible", { hasText: /^No$/ }).count()) {
      // A yes/no prompt (e.g. "Choose new targets for the copy"): keep as is.
      await page.locator("button:visible", { hasText: /^No$/ }).first().click();
    } else {
      return text;
    }
  }
  return label();
}

// Resolves everything on the stack (answering targets) back to an empty one.
async function drain() {
  for (let i = 0; i < 12; i += 1) {
    const text = await settle(null);
    if ((await page.locator(".card-stack-slot:not([data-leaving])").count()) === 0) return;
    if (/resolve/i.test(text || "")) await clickMain(/resolve/);
    await sleep(900);
  }
}

// Clicks the main decision button only if its label (at click time) matches.
async function clickMain(expected = /./) {
  const clicked = await page.evaluate((source) => {
    const button = document.querySelector(".decision-main-button");
    if (!button || !new RegExp(source, "i").test(button.textContent || "")) return false;
    button.click();
    return true;
  }, expected.source);
  if (clicked) await sleep(expected.source.includes("pay") ? 1800 : 300);
  return clicked;
}

async function step(name, fn) {
  try {
    await fn();
    const pager = await page.locator(".card-stack-pager").first().innerText().catch(() => "");
    console.log("ok", name, JSON.stringify(await slots()), "pager:", JSON.stringify(pager.replace(/\s+/g, " ").trim()));
  } catch (error) {
    console.log("FAILED", name, String(error).split("\n")[0]);
    await snap(`failed-${name}`);
  }
}

try {
  await page.goto(`http://127.0.0.1:${port}/?puzzle=${PUZZLE}`);
  await handCard("Lightning Bolt").waitFor({ timeout: 90000 });
  await page.evaluate(() => import("/tests/tmp-card-stack-harness.js"));
  for (let i = 0; i < 14; i += 1) {
    const text = ((await label()) || "").trim();
    if (/^go to combat/i.test(text)) break;
    await clickMain(/keep hand|pregame|continue|begin game|go to/);
    await sleep(1300);
  }
  console.log("main phase:", await label());
  // Resolve manually, so spells stay on the stack until we pass.
  const autoPass = page.locator('button[aria-label="Auto-pass"][aria-pressed="true"]');
  if (await autoPass.count()) await autoPass.first().click();
  await page.mouse.move(700, 60);

  await step("drag-cast-bolt", async () => {
    await dragHandTo("Lightning Bolt", center(await box(battlefieldCard("Grizzly Bears"))));
    await sleep(120);
    await snap("bolt-comet-flight");
    await sleep(300);
    await snap("bolt-comet-impact");
    await settle(battlefieldCard("Grizzly Bears"));
    await page.mouse.move(700, 60);
    await sleep(700);
    await snap("bolt-on-stack");
  });

  await step("reverberate-on-bolt", async () => {
    await dragHandTo("Reverberate", center(await box(pileCard("Lightning Bolt"))));
    await sleep(150);
    await snap("reverberate-comet");
    await settle(pileCard("Lightning Bolt"));
    await page.mouse.move(700, 60);
    await sleep(1000);
    await snap("reverberate-pulls-bolt");
  });

  await step("navigate-chevrons", async () => {
    await page.getByRole("button", { name: "Next stack entry" }).click();
    await sleep(120);
    await snap("nav-spark-hop");
    await sleep(500);
    await snap("nav-viewing-2");
    await page.getByRole("button", { name: "Previous stack entry" }).click();
    await sleep(700);
  });

  await step("reverberate-resolves-copy", async () => {
    await clickMain(/resolve/);
    await snap("reverberate-burns-copy-splits");
    await sleep(450);
    await snap("copy-landing");
    await settle(battlefieldCard("Llanowar Elves"));
    await sleep(300);
    await snap("copy-after-prompt");
    await page.mouse.move(700, 60);
    await sleep(900);
    await snap("copy-and-bolt");
  });

  await step("counterspell-on-bolt", async () => {
    // Resolve the copy first (Elves die -> Blood Artist triggers).
    await clickMain(/resolve/);
    await snap("copy-disperses");
    await sleep(450);
    await snap("blood-artist-sigil-wisp");
    await settle(bobButton());
    await page.mouse.move(700, 60);
    await sleep(900);
    await snap("trigger-on-bolt");
    await clickMain(/resolve/);
    await snap("trigger-resolves-mote");
    await sleep(900);
    await dragHandTo("Counterspell", center(await box(pileCard("Lightning Bolt"))));
    await settle(pileCard("Lightning Bolt"));
    await page.mouse.move(700, 60);
    await sleep(1100);
    await snap("counterspell-pulls-bolt");
    await clickMain(/resolve/);
    await snap("counter-shatter");
    await sleep(450);
    await snap("after-counter");
    await drain();
  });

  await step("cycling", async () => {
    await handCard("Hieroglyphic Illumination").click();
    await sleep(600);
    await snap("cycling-menu");
    // Cycling is the "Cycling" line on the zoomed card's frame.
    await page.locator(".inspector-oracle-line-action", { hasText: /Cycling/ }).first().click({ timeout: 5000 });
    await sleep(400);
    const menuChoice = page.locator("button:visible").filter({ hasText: /cycl/i });
    if (await menuChoice.count()) await menuChoice.first().click();
    await settle(null);
    await sleep(200);
    await snap("cycling-spiral");
    await sleep(700);
    await snap("cycling-landed");
    await drain();
  });

  await step("urzas-saga", async () => {
    const board = await box(page.locator("[data-my-zone] .battlefield-row"));
    await dragHandTo("Urza's Saga", { x: board.x + board.width / 2, y: board.y + board.height / 2 });
    await settle(null);
    await sleep(500);
    await snap("saga-sigil");
    await page.mouse.move(700, 60);
    await sleep(1200);
    await snap("saga-chapter-frame");
    await drain();
  });

  const log = await page.evaluate(() => window.H?.log || []);
  console.log("motion log entries:", log.length);
  console.log(JSON.stringify(log.map((entry) => entry.slice(1).join(" "))));
} finally {
  console.log("errors:", JSON.stringify(errors, null, 1));
  await browser.close();
  await vite.close();
}
