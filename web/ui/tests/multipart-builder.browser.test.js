import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

for (const [layout, label, tab, subtype] of [['adventure','Adventure','Adventure Spell','Adventure'], ['omen','Omen','Omen Spell','Omen'], ['prepare','Prepared','Prepared Spell',null]]) {
test(`${label} builder submits the main card and its spell`, { timeout: 60000 }, async () => {
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
    await dialog.getByRole('button', { name: label, exact: true }).click();
    await dialog.getByRole('textbox', { name: 'Card name', exact: true }).fill('Forge Traveler');
    await dialog.getByRole('tab', { name: new RegExp(tab) }).click({ force: true });
    await dialog.getByRole('textbox', { name: 'Card name', exact: true }).fill('Forge Journey');
    const spellTab = dialog.getByRole('tab', { name: tab, exact: true });
    assert.equal(await spellTab.locator('.card-forge-tab-subtitle').count(), 0);
    assert.equal((await spellTab.textContent()).trim(), tab);
    await dialog.getByRole('textbox', { name: 'Rules text', exact: true }).fill('Draw a card.');
    await dialog.getByRole('button', { name: 'Add', exact: true }).click();
    await page.waitForFunction(() => window.lastAdded?.custom);
    const payload = (await page.evaluate(() => window.lastAdded)).custom;
    assert.equal(payload.draft.layout, layout);
    assert.deepEqual(payload.draft.faces.map(face => face.name), ['Forge Traveler', 'Forge Journey']);
    assert.deepEqual(payload.draft.faces[1].cardTypes, ['Instant']);
    assert.deepEqual(payload.draft.faces[1].subtypes, subtype ? [subtype] : []);
    assert.equal(payload.draft.faces[1].oracleText, 'Draw a card.');
    assert.equal(payload.draft.hasFuse, false);
  } finally {
    await browser.close();
    await vite.close();
  }
});

}
