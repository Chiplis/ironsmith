import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('Meld builder edits all three cards and submits both components with their result', { timeout: 60000 }, async () => {
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
    await dialog.getByRole('button', { name: 'Meld', exact: true }).click();
    await dialog.getByRole('textbox', { name: 'Card name', exact: true }).fill('Forge Left');
    await dialog.getByRole('textbox', { name: 'Rules text', exact: true }).fill('{T}: If you both own and control this creature and a creature named Forge Right, exile them, then meld them into Forge Whole.');
    await dialog.getByRole('tab', { name: /Second Card/ }).click({ force: true });
    await dialog.getByRole('textbox', { name: 'Card name', exact: true }).fill('Forge Right');
    await dialog.getByRole('tab', { name: /Meld Result/ }).click({ force: true });
    await dialog.getByRole('textbox', { name: 'Card name', exact: true }).fill('Forge Whole');
    await dialog.getByRole('textbox', { name: 'Rules text', exact: true }).fill('Trample');
    await page.screenshot({ path: 'web/ui/test-results/meld-builder.png' });
    await dialog.getByRole('button', { name: 'Add', exact: true }).click();
    const payload = (await page.evaluate(() => window.lastAdded)).custom;
    assert.equal(payload.draft.layout, 'meld');
    assert.deepEqual(payload.draft.faces.map(face => face.name), ['Forge Left', 'Forge Right', 'Forge Whole']);
    assert.equal(payload.draft.faces[2].oracleText, 'Trample');
    assert.equal(payload.zoneName, 'battlefield');
  } finally {
    await browser.close();
    await vite.close();
  }
});
