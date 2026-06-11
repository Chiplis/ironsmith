import test from "node:test";
import assert from "node:assert/strict";
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
      const address = server.address();
      const port = typeof address === "object" && address ? address.port : 0;
      server.close(() => resolve(port));
    });
  });
}

async function startWasmServer() {
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
  return {
    vite,
    baseUrl: `http://127.0.0.1:${vitePort}`,
  };
}

test(
  "main decision button stays mounted across priority passes and keeps its local accent",
  { timeout: 60000 },
  async () => {
    const { vite, baseUrl } = await startWasmServer();
    let browser = null;

    try {
      browser = await chromium.launch();
      const page = await browser.newPage();
      const pageErrors = [];
      page.on("pageerror", (error) => pageErrors.push(String(error?.stack || error)));

      await page.goto(baseUrl);
      await page.waitForSelector(".pass-priority-btn", { timeout: 30000 });

      const checks = await page.evaluate(async () => {
        const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
        const btn = document.querySelector(".pass-priority-btn.decision-main-button");
        if (!btn) return { error: "main decision button not found" };

        // The accent custom properties must be registered via @property so the
        // browser can interpolate them. Two independent signals:
        // 1. the CSSPropertyRule survives the CSS pipeline,
        // 2. registered <color> values compute to normalized rgb() form
        //    (an unregistered var would echo the raw "#731bde" token).
        let accentRegistered = false;
        let rgbRegistered = false;
        for (const sheet of document.styleSheets) {
          let rules;
          try {
            rules = sheet.cssRules;
          } catch {
            continue;
          }
          for (const rule of rules) {
            const text = rule.cssText || "";
            if (text.startsWith("@property --decision-main-accent")) accentRegistered = true;
            if (text.startsWith("@property --decision-main-rgb")) rgbRegistered = true;
          }
        }

        const accent = getComputedStyle(btn).getPropertyValue("--decision-main-accent").trim();

        // The local-turn pulse must be live: keyframes valid (they animate
        // `filter`, which nothing sets with !important) and actually running.
        const pulseEl = document.querySelector(
          '.decision-main-button[data-local-action="true"], .action-strip-main-region[data-local-action="true"]'
        );
        const pulseAnimation = pulseEl ? getComputedStyle(pulseEl).animationName : "";
        const pulseFilterA = pulseEl ? getComputedStyle(pulseEl).filter : "";
        await sleep(400);
        const pulseFilterB = pulseEl ? getComputedStyle(pulseEl).filter : "";

        // Tag the node, pass priority a few times, and confirm the same DOM
        // node is still in place (no unmount/remount churn).
        btn.__stabilityTag = true;
        let passes = 0;
        for (let i = 0; i < 4; i += 1) {
          const current = document.querySelector(".pass-priority-btn.decision-main-button");
          if (!current) break;
          current.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true, button: 0 }));
          current.click();
          passes += 1;
          await sleep(450);
        }
        const after = document.querySelector(".pass-priority-btn.decision-main-button");

        return {
          accentRegistered,
          rgbRegistered,
          accent,
          pulseAnimation,
          pulseFilterA,
          pulseFilterB,
          passes,
          sameNode: after === btn,
          taggedSurvived: Boolean(after && after.__stabilityTag),
        };
      });

      assert.equal(checks.error, undefined, checks.error || "");
      assert.equal(checks.accentRegistered, true, "--decision-main-accent should be registered via @property");
      assert.equal(checks.rgbRegistered, true, "--decision-main-rgb should be registered via @property");
      assert.match(
        checks.accent,
        /^rgb\(115, 27, 222\)$/,
        "local decision accent should compute to the registered (interpolable) purple"
      );
      assert.match(
        checks.pulseAnimation,
        /decision-main-local-pulse/,
        "local decision target should run the local-turn pulse animation"
      );
      assert.match(checks.pulseFilterA, /drop-shadow/, "pulse should produce a drop-shadow filter");
      assert.notEqual(
        checks.pulseFilterA,
        checks.pulseFilterB,
        "pulse filter should change over time (animation actually running)"
      );
      assert.equal(checks.passes, 4, "should have clicked pass four times");
      assert.equal(checks.sameNode, true, "pass button must not remount across priority passes");
      assert.equal(checks.taggedSurvived, true, "pass button DOM node should survive decision churn");

      const fatalErrors = pageErrors.filter((text) => !text.includes("favicon"));
      assert.deepEqual(fatalErrors, [], `page errors: ${fatalErrors.join("\n")}`);
    } finally {
      if (browser) await browser.close();
      await vite.close();
    }
  }
);
