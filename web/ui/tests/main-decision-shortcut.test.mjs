import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { readFile } from 'node:fs/promises';

test('Enter activates only an available main decision and respects editing and dialogs', async () => {
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    await page.setContent('<button class="decision-main-button">Pass</button><input /><div tabindex="0" id="board">Board</div>');
    const source = await readFile(new URL('../src/lib/main-decision-shortcut.js', import.meta.url), 'utf8');
    await page.addScriptTag({ content: source.replace('export function', 'function') + ';window.disposeShortcut = installMainDecisionShortcut(document);window.clicks=0;document.querySelector("button").onclick=()=>window.clicks++;' });
    const clicks = () => page.evaluate(() => window.clicks);
    await page.locator('#board').focus();
    await page.keyboard.press('Enter');
    assert.equal(await clicks(), 1);
    await page.evaluate(() => document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', repeat: true, bubbles: true })));
    assert.equal(await clicks(), 1);
    await page.locator('input').focus();
    await page.keyboard.press('Enter');
    assert.equal(await clicks(), 1);
    await page.locator('#board').focus();
    for (const attribute of ['disabled', 'aria-disabled']) {
      await page.locator('button').evaluate((b, name) => b.setAttribute(name, 'true'), attribute);
      await page.keyboard.press('Enter');
      assert.equal(await clicks(), 1);
      await page.locator('button').evaluate((b, name) => b.removeAttribute(name), attribute);
    }
    await page.evaluate(() => { const dialog = document.createElement('div'); dialog.role='dialog'; dialog.textContent='Diagnostics'; document.body.append(dialog); });
    await page.keyboard.press('Enter');
    assert.equal(await clicks(), 1);
    await page.evaluate(() => document.querySelector('[role="dialog"]').remove());
    await page.locator('button').focus();
    await page.keyboard.press('Enter');
    assert.equal(await clicks(), 2); // Native activation, never double-submit.
    await page.evaluate(() => window.disposeShortcut());
    await page.locator('#board').focus();
    await page.keyboard.press('Enter');
    assert.equal(await clicks(), 2);
  } finally { await browser.close(); }
});
