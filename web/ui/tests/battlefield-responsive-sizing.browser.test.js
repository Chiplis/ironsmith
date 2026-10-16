import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('2/3/4-player fields grow, shrink, and fit independently in both desktop appearances', async () => {
  const vite = await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});
  await vite.listen();
  const browser = await chromium.launch();
  try {
    for (const compactCards of [true, false]) {
      const page = await browser.newPage({viewport:{width:1710,height:954}});
      await page.addInitScript(value => localStorage.setItem('ironsmith.battlefieldAppearance', JSON.stringify({compactCards:value,compactLands:true})), compactCards);
      const measure = async () => {
        await page.waitForFunction(() => [...document.querySelectorAll('.battlefield-row')].every(n => parseFloat(n.style.getPropertyValue('--bf-card-width')) > 0));
        await page.waitForTimeout(200);
        return page.locator('.battlefield-row').evaluateAll(nodes => nodes.map(n => ({
          width: parseFloat(n.style.getPropertyValue('--bf-card-width')),
          height: parseFloat(n.style.getPropertyValue('--bf-card-height')),
          available: n.clientWidth,
          columns: Number(n.dataset.battlefieldGridColumns),
        })));
      };
      for (const players of [2,3,4]) {
        const url = `http://127.0.0.1:${vite.httpServer.address().port}/tests/diagnostics-layout.html?kind=priority&scenario=phase-controls&players=${players}&objects=1`;
        await page.setViewportSize({width:1710,height:954});
        await page.goto(url);
        const small = await measure();
        assert.equal(small.length, players);
        await page.setViewportSize({width:2560,height:1440});
        const large = await measure();
        for (let i=0;i<players;i++) {
          assert.ok(large[i].width > small[i].width * 1.15, `${players} players, compact=${compactCards}, field ${i}: must grow ${JSON.stringify({small:small[i],large:large[i]})}`);
          assert.ok(large[i].width * large[i].columns <= large[i].available);
        }
        await page.goto(`${url}&opponentObjects=30`);
        const crowded = await measure();
        assert.ok(crowded[0].width < large[0].width, 'crowded opponent shrinks');
        assert.ok(Math.abs(crowded.at(-1).width - large.at(-1).width) <= 1, 'opponent crowding does not shrink local field');
        await page.setViewportSize({width:1710,height:954});
        const shrunk = await measure();
        assert.ok(shrunk.at(-1).width < crowded.at(-1).width, 'field shrinks after resize');
      }
      await page.close();
    }
  } finally { await browser.close(); await vite.close(); }
});
