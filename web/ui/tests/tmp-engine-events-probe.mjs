import net from "node:net";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { createServer as createViteServer } from "vite";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, "..");
const WASM_MODULE_URL = `/@fs/${path.resolve(UI_ROOT, "../wasm_demo/pkg/ironsmith.js")}`;

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
  const page = await browser.newPage();
  page.on("pageerror", (e) => console.error("PAGEERROR", String(e).slice(0, 200)));
  await page.goto(`http://127.0.0.1:${vitePort}`);
  const result = await page.evaluate(async ({ wasmModuleUrl }) => {
    const mod = await import(wasmModuleUrl);
    await mod.default();
    const game = new mod.WasmGame();
    game.startMatch({
      playerNames: ["Alice", "Bob"],
      startingLife: 20,
      seed: 1,
      format: "normal",
      openingHandSize: 7,
      decks: [Array(60).fill("Mountain"), Array(60).fill("Mountain")],
    });
    const out = { checks: {} };
    const events = () => game.uiState().effect_events || null;

    out.checks.fieldExists = Array.isArray(events());

    // day/night via cheat
    if (typeof game.setDaytime === "function") {
      game.setDaytime(true);
      game.setDaytime(false);
      const dayNight = (events() || []).filter((e) => e.kind === "day_night");
      out.checks.dayNight = dayNight.map((e) => e.text);
    }

    // counters via battlefield card + any cheat? use addCardToZone with a card
    // that ETBs with counters
    game.addCardToZone(0, "Walking Ballista", "battlefield", true);
    game.addCardToZone(0, "Grizzly Bears", "battlefield", true);

    out.eventsSample = (events() || []).slice(-8).map((e) => ({
      id: e.id, kind: e.kind, player: e.player, stable_ids: e.stable_ids, value: e.value, text: e.text,
    }));
    out.monotonic = (() => {
      const ids = (events() || []).map((e) => e.id);
      return ids.every((id, i) => i === 0 || id > ids[i - 1]);
    })();
    return out;
  }, { wasmModuleUrl: WASM_MODULE_URL });
  console.log(JSON.stringify(result, null, 1));
} finally {
  await browser.close();
  await vite.close();
}
