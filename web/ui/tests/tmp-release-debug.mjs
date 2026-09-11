import { chromium } from "playwright";
import { createServer } from "vite";
const vite = await createServer({ server: { host: "127.0.0.1", port: 0 }, logLevel: "silent" });
await vite.listen();
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1400, height: 900 } });
const errors = [];
page.on("pageerror", (e) => errors.push(String(e?.stack || e).slice(0, 300)));
await page.route("**/api.scryfall.com/**", (r) => r.abort());
await page.route("**/cards.scryfall.io/**", (r) => r.abort());
await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/hand-cast-release.html`);
const probe = async (label) => {
  const info = await page.evaluate(() => ({
    cancelled: window.__cancelled,
    dispatched: window.__dispatched.map(c => c.type),
    decision: document.body.innerText.includes("Choose targets") ? "targets" : "other",
    arrow: Boolean(document.querySelector(".cast-intent-drag-arrow, svg .arrow-path, [class*='drag-arrow']")),
  }));
  console.log(label, JSON.stringify(info));
};
const hand = await page.locator('.game-card.hand-card[data-object-id="5"]').boundingBox();
const dead = await page.evaluate(async () => {
  const { castHoverTargetAtPoint } = await import("/src/lib/hand-drag-intent.js");
  const mine = document.querySelector("[data-my-zone]");
  const rect = mine.getBoundingClientRect();
  for (let y = Math.ceil(rect.top) + 10; y < rect.bottom - 10; y += 8) {
    for (let x = Math.ceil(rect.left) + 10; x < rect.right - 10; x += 8) {
      const el = document.elementFromPoint(x, y);
      if (!el || !el.closest("[data-drop-zone]")) continue;
      if (el.closest(".game-card, button, a, input, [role='button'], [role='dialog'], .interactive-card-frame-stage, .table-persistent-utility-strip")) continue;
      if (castHoverTargetAtPoint(x, y)) continue;
      return { x, y, el: (el.className || el.tagName).toString().slice(0, 60) };
    }
  }
  return null;
});
console.log("hand", JSON.stringify(hand), "dead", JSON.stringify(dead));
await probe("before   ");
await page.mouse.move(hand.x + hand.width / 2, hand.y + hand.height / 2);
await page.mouse.down();
await page.mouse.move(dead.x, dead.y, { steps: 12 });
await page.waitForTimeout(700);
await probe("dragging ");
await page.mouse.up();
await page.waitForTimeout(900);
await probe("released ");
await page.mouse.move(dead.x + 80, dead.y - 40);
await page.waitForTimeout(300);
await probe("moved    ");
console.log("errors", JSON.stringify(errors.slice(0, 2)));
await page.screenshot({ path: process.env.OUT + "/release-after.png" });
await browser.close(); await vite.close();
