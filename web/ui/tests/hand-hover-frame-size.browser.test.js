import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

// Node cannot import the .jsx module that owns these, so they are restated:
// CARD_FRAME_RENDER_WIDTH / _HEIGHT in src/components/cards/MiniatureCardFrame.jsx.
const CARD_FRAME_RENDER_WIDTH = 380;
const CARD_FRAME_RENDER_HEIGHT = 531;

test('a hovered hand card grows to the size the table renders frames at', async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/hand-hover-frame-size.html`);
    const card = page.locator('.game-card.hand-card').nth(3);
    await card.waitFor();
    await page.waitForTimeout(400);

    const resting = await card.boundingBox();
    // Hover by the card's top edge: its centre is the part of the fan that hangs
    // below the window, so that is where a player actually reaches for it.
    const grabbed = { x: resting.x + (resting.width / 2), y: resting.y + 24 };
    await page.mouse.move(grabbed.x, grabbed.y);
    await page.waitForTimeout(600);
    const hovered = await card.boundingBox();

    // The frame renderer lays every card out at 380x531 before scaling it into
    // its slot, so the hand's zoom lands on whichever of the two binds first.
    const grown = Math.min(
      CARD_FRAME_RENDER_WIDTH / resting.width,
      CARD_FRAME_RENDER_HEIGHT / resting.height,
    );
    assert.ok(
      Math.abs(hovered.height - (resting.height * grown)) < 2,
      `hovered ${hovered.height} should reach ${resting.height * grown}`,
    );
    assert.ok(hovered.width > resting.width * 2, `hovered width ${hovered.width} vs resting ${resting.width}`);

    // Hand cards scale from their bottom edge, so the zoom grows upwards on its
    // own and the lift stays small. It has to: if the enlarged card slid out
    // from under the pointer that opened it, the hover would drop, the card
    // would shrink back under the pointer and the two would oscillate — and a
    // drag started from that pointer would never reach the card at all.
    assert.ok(
      grabbed.x >= hovered.x && grabbed.x <= hovered.x + hovered.width
      && grabbed.y >= hovered.y && grabbed.y <= hovered.y + hovered.height,
      `the grabbed point ${JSON.stringify(grabbed)} stays on ${JSON.stringify(hovered)}`,
    );

    // The enlarged card still has to fit the window it grew into.
    assert.ok(hovered.y >= 0, `hovered top ${hovered.y} is on screen`);
  } finally {
    await browser.close();
    await vite.close();
  }
});
