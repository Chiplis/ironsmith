// THROWAWAY probe: verify the engine snapshot exposes effect_events and that
// state mutations produce events with monotonic ids. Delete after use.
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
  server: {
    host: "127.0.0.1",
    port: vitePort,
    strictPort: true,
    hmr: false,
    watch: null,
  },
});
await vite.listen();
const baseUrl = `http://127.0.0.1:${vitePort}`;

let browser = null;
try {
  browser = await chromium.launch();
  const page = await browser.newPage();
  const pageErrors = [];
  page.on("pageerror", (error) => pageErrors.push(String(error?.stack || error)));
  page.on("console", (message) => {
    if (message.type() === "error") pageErrors.push(message.text());
  });

  await page.goto(baseUrl);
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

    const out = {};
    const events = () => {
      const state = game.uiState();
      return Array.isArray(state?.effect_events) ? state.effect_events : null;
    };

    out.initialEffectEventsIsArray = Array.isArray(events());
    out.initialEvents = events();

    // 1. Counters: Triskelion enters with three +1/+1 counters.
    const trike = game.addCardToZone(0, "Triskelion", "battlefield", true);
    out.afterTriskelion = events();

    // 2. Transform: daybound Huntmaster flips when night falls.
    game.addCardToZone(0, "Huntmaster of the Fells", "battlefield", true);
    game.setDaytime(true);
    game.setDaytime(false); // day -> night: day_night event + daybound transform
    out.afterNightfall = events();

    // 3. Monotonic ids across the whole feed.
    const all = events() || [];
    out.idsMonotonic = all.every((event, idx) => idx === 0 || all[idx - 1].id < event.id);
    out.kinds = all.map((event) => event.kind);
    out.finalEvents = all;
    out.trikeObjectId = trike;
    return out;
  }, { wasmModuleUrl: WASM_MODULE_URL });

  console.log("pageErrors:", JSON.stringify(pageErrors, null, 2));
  console.log("initialEffectEventsIsArray:", result.initialEffectEventsIsArray);
  console.log("idsMonotonic:", result.idsMonotonic);
  console.log("kinds:", JSON.stringify(result.kinds));
  console.log("finalEvents:", JSON.stringify(result.finalEvents, null, 2));

  if (!result.initialEffectEventsIsArray) throw new Error("effect_events missing from snapshot");
  if (!result.idsMonotonic) throw new Error("effect event ids are not monotonic");
  if (!(result.finalEvents || []).length) throw new Error("no effect events recorded after mutations");
  console.log("PROBE OK");
} finally {
  await browser?.close();
  await vite.close();
}
