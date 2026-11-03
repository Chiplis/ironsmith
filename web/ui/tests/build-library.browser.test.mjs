import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('Library browses definitions across sets, filters sources, and keeps navigation independent', { timeout: 60000 }, async () => {
  const server = await createServer({ server: { host: '127.0.0.1', port: 0, hmr: false }, logLevel: 'error' });
  await server.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1440, height: 1000 } });
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.route('https://**', route => route.abort());
    await page.route('**/custom-sets/index.json', route => route.fulfill({ json: { sets: [] } }));
    await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/tests/build-workspace.html`);
    await page.evaluate(async () => {
      const { saveCommunityItem } = await import('/src/lib/community-library.js');
      const { validateMechanic } = await import('/src/lib/custom-mechanics.js');
      for (const [name, reminder] of [['Embers', 'Draw a card.'], ['Tides', 'Draw two cards.']]) {
        const def = validateMechanic({ keyword: 'Kindle', reminder });
        await saveCommunityItem('set', { name, cards: [{ id: name, draft: { faces: [{ name: `${name} Mage`, cardTypes: ['Creature'], subtypes: ['Moonkin'], oracleText: 'Kindle' }], customMechanics: [def], customSubtypes: [{ singular: 'moonkin', plural: 'moonkin', family: 'Creature' }] } }] });
      }
    });
    await page.getByRole('button', { name: 'Build', exact: true }).click();
    await page.getByRole('tab', { name: 'Sets', exact: true }).click();
    await page.getByLabel('Custom set name', { exact: true }).fill('Unsaved set');
    await page.getByRole('button', { name: /Set mechanics/ }).click();
    assert.equal(await page.getByLabel('Custom set name', { exact: true }).inputValue(), 'Unsaved set');
    await page.getByRole('tab', { name: 'Library', exact: true }).click();
    await page.getByRole('button', { name: 'Refresh library', exact: true }).waitFor();
    await page.getByText('Embers Mage', { exact: true }).waitFor();
    await page.getByRole('button', { name: 'Mechanics', exact: true }).click();
    const panel = page.getByRole('tabpanel', { name: 'Library', exact: true });
    const list = panel.getByTestId('build-catalog-list').first();
    assert.equal(await list.locator('li > div > details').count(), 2);
    await list.locator('summary').first().click();
    await list.getByText('Draw a card.', { exact: true }).waitFor();
    await page.getByLabel('Source set', { exact: true }).selectOption('Tides');
    assert.equal(await list.locator('li > div > details').count(), 1);
    await list.locator('summary').click();
    await list.getByText('Draw two cards.', { exact: true }).waitFor();
    await page.getByRole('button', { name: 'Subtypes', exact: true }).click();
    await list.locator('summary').click();
    await list.getByText('Tides Mage', { exact: true }).waitFor();
    await page.getByLabel('Search library').fill('missing');
    await panel.getByText('No matching entries.', { exact: true }).filter({ visible: true }).waitFor();
    await page.getByLabel('Search library').fill('');
    await page.screenshot({ path: '/tmp/build-library.png' });
    await page.setViewportSize({ width: 390, height: 844 });
    assert.equal(await page.getByRole('tabpanel', { name: 'Library', exact: true }).evaluate(el => el.scrollWidth <= el.clientWidth), true);
    await page.screenshot({ path: '/tmp/build-library-mobile.png' });
    await page.getByRole('tab', { name: 'Sets', exact: true }).click();
    assert.equal(await page.getByLabel('Custom set name', { exact: true }).inputValue(), 'Unsaved set');
    await page.getByRole('tab', { name: 'Card', exact: true }).click();
    await page.getByLabel('Card name', { exact: true }).waitFor();
    assert.deepEqual(errors, []);
  } finally { await browser.close(); await server.close(); }
});
