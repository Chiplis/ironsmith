import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { createServer } from "vite";
const out = process.argv[2];
const commanders = Number(process.argv[3] || 1);
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const scenarioApi = `const e2eApi = {
  scenarioState: () => state,
  addToField: (name) => runWasmInteraction(async () => {
    const id = await game.addCardToZone(0, name, "battlefield", true); console.log("ADDED", name, id);
    await finalizeState(game, await game.uiState());
  }),
  setupCommander: (count) => runWasmInteraction(async () => {
    setAutoPassEnabled(false);
    setHoldRule("always");
    await game.resetEmpty(["Alice", "Bob"], 40);
    const names = ["Thrasios, Triton Hero", "Tymna the Weaver"].slice(0, count);
    for (const name of names) await game.addCardToZone(0, name, "command", true);
    await game.addCardToZone(1, "Yidris, Maelstrom Wielder", "command", true);
    for (const name of ["Island", "Forest", "Plains"]) await game.addCardToZone(0, name, "battlefield", true);
    for (const name of ["Opt", "Island"]) await game.addCardToZone(0, name, "hand", true);
    for (let i = 0; i < 6; i++) { await game.addCardToZone(0, "Forest", "library", true); await game.addCardToZone(1, "Swamp", "library", true); }
    await game.finishPuzzleSetup();
    let next = await game.uiState();
    for (let i = 0; i < 30; i++) {
      if (next.phase === "first main phase" && next.active_player === 0) break;
      const action = next.decision?.actions?.find(action =>
        ["keep_opening_hand", "begin_game", "continue_pregame", "pass_priority"].includes(action.kind));
      if (!action) break;
      next = await game.dispatch({ type: "priority_action", action_ref: action.action_ref });
    }
    await finalizeState(game, next);
  }),`;
process.env.VITE_E2E_TEST = "true";
const server = await createServer({ root, logLevel: "error", server: { host: "127.0.0.1", port: 0, hmr: false },
  plugins: [{ name: "commander-scenario", transform(code, id) {
    if (id.endsWith("/context/GameContext.jsx")) return code.replace("const e2eApi = {", scenarioApi);
  } }] });
await server.listen();
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1600, height: 900 } });
page.on("pageerror", (e) => console.log("PAGEERROR", e.message));
page.on("console", (m) => { if (/ADDED|rror/.test(m.text())) console.log("PAGE", m.text().slice(0, 200)); });
try {
  await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/`);
  await page.waitForSelector(".game-card.hand-card", { timeout: 120000 });
  await page.waitForTimeout(3000);
  for (let attempt = 0; attempt < 3; attempt++) {
    await page.evaluate((n) => window.__ironsmithE2E.setupCommander(n), commanders);
    await page.waitForTimeout(2500);
    if ((await page.evaluate(() => window.__ironsmithE2E.scenarioState().players[0].command_cards?.length)) === commanders) break;
  }
  console.log("command", JSON.stringify(await page.evaluate(() => window.__ironsmithE2E.scenarioState().players.map(p => [p.command_size, (p.command_cards || []).map(c => c.name)]))), "piles", await page.evaluate(() => [...document.querySelectorAll("[data-zone-pile]")].map(e => e.dataset.zonePile + ":" + e.dataset.zoneOwner).join(",")));
  for (const name of ["Lightning Bolt", "Grizzly Bears"]) {
    await page.evaluate((n) => window.__ironsmithE2E.addToField(n), name);
    await page.waitForTimeout(1500);
    const p = await page.evaluate(() => window.__ironsmithE2E.scenarioState().players[0]);
    console.log("ZONES", JSON.stringify({ hand: p.hand_cards.map(c => c.name), size: p.hand_size, gy: (p.graveyard_cards || []).map(c => c.name), bf: p.battlefield.map(c => c.name) }));
    console.log("RESULT", name, "hand:", p.hand_cards.some(c => c.name === name), "battlefield:", p.battlefield.some(c => c.name === name));
  }
} finally {
  await browser.close();
  await server.close();
}
