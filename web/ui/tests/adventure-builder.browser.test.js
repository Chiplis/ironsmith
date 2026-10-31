import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('Adventure builder submits the main card and its Adventure spell', { timeout: 60000 }, async () => {
  const vite = await createServer({ root: new URL('..', import.meta.url).pathname, server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1440, height: 1000 }, reducedMotion: 'reduce' });
    await page.route('https://**', route => route.abort());
    // Keep this authoring test independent of card-frame image generation.
    await page.route('**/src/components/layout/CardPrintingStage.jsx*', route => route.fulfill({ contentType: 'text/javascript', body: 'export default function CardPrintingStage() { return null; }' }));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/build-workspace.html`);
    await page.addStyleTag({ content: '*, *::before, *::after { animation: none !important; transition: none !important; }' });
    await page.getByRole('button', { name: 'Build', exact: true }).click();
    const dialog = page.getByRole('dialog');
    await dialog.getByRole('button', { name: 'Adventure', exact: true }).click();
    await dialog.getByRole('textbox', { name: 'Card name', exact: true }).fill('Forge Traveler');
    await dialog.getByRole('tab', { name: /Adventure Spell/ }).click({ force: true });
    await dialog.getByRole('textbox', { name: 'Card name', exact: true }).fill('Forge Journey');
    await dialog.getByRole('textbox', { name: 'Rules text', exact: true }).fill('Draw a card.');
    await dialog.getByRole('button', { name: 'Add', exact: true }).click();
    await page.waitForFunction(() => window.lastAdded?.custom);
    const payload = (await page.evaluate(() => window.lastAdded)).custom;
    assert.equal(payload.draft.layout, 'adventure');
    assert.deepEqual(payload.draft.faces.map(face => face.name), ['Forge Traveler', 'Forge Journey']);
    assert.deepEqual(payload.draft.faces[1].cardTypes, ['Instant']);
    assert.deepEqual(payload.draft.faces[1].subtypes, ['Adventure']);
    assert.equal(payload.draft.faces[1].oracleText, 'Draw a card.');
    assert.equal(payload.draft.hasFuse, false);
  } finally {
    await browser.close();
    await vite.close();
  }
});
