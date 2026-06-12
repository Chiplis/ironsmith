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
try {
  const page = await browser.newPage({ viewport: { width: 1600, height: 1000 } });
  await page.goto(`http://127.0.0.1:${vitePort}`);
  await page.waitForSelector(".pass-priority-btn", { timeout: 30000 });
  await page.waitForTimeout(1200);

  const mainText = () => page.evaluate(() =>
    document.querySelector(".pass-priority-btn")?.textContent?.trim() || "");

  for (let i = 0; i < 16; i += 1) {
    const text = await mainText();
    if (/keep hand/i.test(text)) {
      await page.click(".pass-priority-btn");
      await page.waitForTimeout(500);
      continue;
    }
    if (/pregame/i.test(text)) {
      await page.evaluate(() => {
        const select = document.querySelector(".add-card-toolbar-perspective-select");
        if (!select) return;
        const options = [...select.options].map((o) => o.value);
        const idx = options.indexOf(select.value);
        select.value = options[(idx + 1) % options.length];
        select.dispatchEvent(new Event("change", { bubbles: true }));
      });
      await page.waitForTimeout(600);
      continue;
    }
    break;
  }
  await page.evaluate(() => {
    const select = document.querySelector(".add-card-toolbar-perspective-select");
    if (select && select.value !== select.options[0].value) {
      select.value = select.options[0].value;
      select.dispatchEvent(new Event("change", { bubbles: true }));
    }
  });
  await page.waitForTimeout(600);

  for (let i = 0; i < 20; i += 1) {
    const text = await mainText();
    if (/main/i.test(text)) break;
    await page.click(".pass-priority-btn");
    await page.waitForTimeout(500);
  }
  console.log("at:", await mainText());

  // Open the action menu for Divination.
  const hand = await page.waitForSelector('.game-card.hand-card[data-card-name="Divination"]', { timeout: 8000 });
  await hand.click();
  await page.waitForTimeout(600);

  // Park the mouse away from cards so hover doesn't drive the inspector.
  // (We will click the cast button via JS.)

  // Start per-frame recorder.
  await page.evaluate(() => {
    window.__frames = [];
    const record = () => {
      const shells = [...document.querySelectorAll('[data-card-inspector="true"]')];
      const stackEl = document.querySelector("[data-inspector-stack-timeline]");
      window.__frames.push({
        t: performance.now(),
        stack: !!stackEl,
        shells: shells.map((el) => {
          const r = el.getBoundingClientRect();
          const aside = el.closest("aside");
          const ar = aside ? aside.getBoundingClientRect() : null;
          return {
            w: Math.round(r.width), h: Math.round(r.height),
            x: Math.round(r.left), y: Math.round(r.top),
            asideW: ar ? Math.round(ar.width) : null,
            name: el.querySelector(".inspector-banner--identity")?.textContent?.trim() || null,
            dock: el.closest("[data-inspector-dock]")?.getAttribute("data-inspector-dock") || null,
          };
        }),
      });
      window.__rafId = requestAnimationFrame(record);
    };
    window.__rafId = requestAnimationFrame(record);
  });

  // Cast without paying mana cost — puts Divination on the stack immediately.
  await page.evaluate(() => {
    const button = [...document.querySelectorAll("button")].find((b) =>
      /cast without paying mana cost/i.test(`${b.textContent} ${b.getAttribute("aria-label")}`) && !b.disabled);
    if (button) button.click();
  });
  await page.waitForTimeout(3000);

  const frames = await page.evaluate(() => {
    cancelAnimationFrame(window.__rafId);
    return window.__frames;
  });

  let prevKey = null;
  const t0 = frames.length ? frames[0].t : 0;
  for (const frame of frames) {
    const key = JSON.stringify([frame.stack, frame.shells.map((s) => [s.w, s.h, s.x, s.y, s.name, s.dock])]);
    if (key !== prevKey) {
      const desc = frame.shells
        .filter((s) => s.w > 0 || s.h > 0)
        .map((s) => `dock=${s.dock} ${s.w}x${s.h}@(${s.x},${s.y}) asideW=${s.asideW} name=${s.name}`)
        .join(" | ");
      console.log(`t=${Math.round(frame.t - t0)}ms stack=${frame.stack ? "Y" : "n"} ${desc}`);
      prevKey = key;
    }
  }
  await page.screenshot({ path: path.join(__dirname, "tmp-inspector-flicker.png") });
} finally {
  await browser.close();
  await vite.close();
}
