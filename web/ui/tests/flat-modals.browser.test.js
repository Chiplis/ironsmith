import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

// Every modal wears the Load Decks workspace's flat chrome: a near-black
// surface, no border or corner radius, a #0e0f11 header rule, and the UI sans
// on the title rather than the display serif the parchment sheets used.
const SURFACE = 'rgb(8, 9, 10)';
const HEADER = 'rgb(14, 15, 17)';

// Each modal, and the control that opens it.
const MODALS = [
  ['decklist', '[data-open="decklist"]'],
  ['log', '[data-open="log"]'],
  ['random game', '[data-open="random"]'],
  ['add card', '[data-open="addcard"]'],
  ['diagnostics', '[data-open="diagnostics"]'],
  ['verify match', '[data-open="verify"]'],
  ['card forge', '[data-open="forge"]'],
  ['lobby', '[data-open="lobby"]'],
  ['table settings', '.topbar-menu-trigger'],
];

// One type scale everywhere: five sizes, nothing under 12px, nothing lighter
// than medium. Rendered cards and keycaps keep their own typography.
const textOffScale = (element) => {
  const scale = new Set([12, 14, 15, 17, 20]);
  const found = [];
  const walker = document.createTreeWalker(element, NodeFilter.SHOW_TEXT);
  for (let node = walker.nextNode(); node; node = walker.nextNode()) {
    const owner = node.parentElement;
    if (!node.textContent.trim() || !owner.getClientRects().length) continue;
    if (owner.closest('.sr-only, code, kbd, .game-card, [data-card-era], .card-glint-live')) continue;
    const style = getComputedStyle(owner);
    const size = Math.round(parseFloat(style.fontSize) * 10) / 10;
    if (!scale.has(size) || Number(style.fontWeight) < 500) {
      found.push(`${size}px/${style.fontWeight} "${node.textContent.trim().slice(0, 24)}"`);
    }
  }
  return found;
};

// Dialogs that are not Radix sheets but share the type scale.
const SURFACES = [
  ['keyboard shortcuts', '[data-open="shortcuts"]', '.keyboard-shortcuts-panel'],
  ['load decks', '[data-open="loaddecks"]', '.deck-loading-screen'],
];

test('every modal wears the flat dark chrome', async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1280, height: 900 } });
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/flat-modals.html`);
    await page.locator('[data-open="decklist"]').waitFor();

    for (const [name, opener] of MODALS) {
      await page.locator(opener).first().click();
      const sheet = page.locator('[data-slot="sheet-content"]');
      await sheet.waitFor();
      const chrome = await sheet.evaluate((element) => {
        const style = getComputedStyle(element);
        const header = element.querySelector('[data-slot="sheet-header"]');
        const title = element.querySelector('[data-slot="sheet-title"]');
        const ornament = getComputedStyle(element, '::before');
        return {
          background: style.backgroundColor,
          gradient: style.backgroundImage,
          borderWidth: style.borderTopWidth,
          radius: style.borderTopLeftRadius,
          header: header ? getComputedStyle(header).backgroundColor : null,
          titleFont: title ? getComputedStyle(title).fontFamily : null,
          ornament: ornament.content,
        };
      });
      assert.equal(chrome.background, SURFACE, `${name} surface`);
      assert.equal(chrome.gradient, 'none', `${name} keeps no gradient`);
      assert.equal(chrome.borderWidth, '0px', `${name} border`);
      assert.equal(chrome.radius, '0px', `${name} corner radius`);
      assert.equal(chrome.ornament, 'none', `${name} rune ornament`);
      if (chrome.header !== null) assert.equal(chrome.header, HEADER, `${name} header`);
      if (chrome.titleFont !== null) {
        assert.ok(/Rajdhani/.test(chrome.titleFont), `${name} title font: ${chrome.titleFont}`);
      }
      const offScale = await sheet.evaluate(textOffScale);
      assert.deepEqual(offScale, [], `${name} text off the modal type scale`);
      await page.keyboard.press('Escape');
      await sheet.waitFor({ state: 'detached' });
    }

    for (const [name, opener, root] of SURFACES) {
      await page.locator(opener).click();
      const surface = page.locator(root);
      await surface.waitFor();
      assert.deepEqual(await surface.evaluate(textOffScale), [], `${name} text off the modal type scale`);
      await page.keyboard.press('Escape');
    }
  } finally {
    await browser.close();
    await vite.close();
  }
});
