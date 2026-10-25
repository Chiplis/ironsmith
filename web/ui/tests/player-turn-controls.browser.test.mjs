import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('per-player tracks, full control, public choices and double-click hand play', { timeout: 60000 }, async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'error' });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
    await page.route('https://**/*', route => route.fulfill({ status: 404, body: '' }));
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.addInitScript(() => {
      window.handCasts = [];
      window.addEventListener('ironsmith:hand-card-keyboard-cast', event => window.handCasts.push(event.detail));
    });
    const url = `http://127.0.0.1:${vite.httpServer.address().port}/tests/zone-piles-table.html`;
    await page.goto(`${url}?singleOpponent&decisionDock&hand=7&playableHand&chosenValues&otherPriority`);
    const stop = (player, phase) => page.locator(`.phase-track--shared [data-stop-player="${player}"][data-stop-phase="${phase}"]`);
    await stop(0, 'Main').waitFor();
    assert.equal(await page.locator('.phase-track--shared:not(.phase-track--combat) .phase-track-step-label').count(), 8, 'duels share one set of phase labels');
    for (const player of [0, 1]) {
      for (const phase of ['Main', 'Main2']) assert.equal(await stop(player, phase).getAttribute('data-stop'), 'always');
    }
    assert.equal(await stop(0, 'Main').locator('..').getAttribute('data-active-turn'), 'true');
    assert.equal(await stop(1, 'Main').locator('..').getAttribute('data-has-priority'), 'true');
    assert.equal(await stop(1, 'Main').getAttribute('data-opponent-stop'), 'true');
    assert.equal(await stop(1, 'Main').locator('svg').evaluate(node => getComputedStyle(node).transform), 'matrix(-1, 0, 0, -1, 0, 0)');
    await stop(0, 'Main').click();
    assert.equal(await stop(0, 'Main').getAttribute('data-stop'), 'off');
    assert.equal(await stop(1, 'Main').getAttribute('data-stop'), 'always');
    for (const mode of ['once', 'always', 'off']) {
      await stop(1, 'Upkeep').click();
      assert.equal(await stop(1, 'Upkeep').getAttribute('data-stop'), mode);
    }
    const control = page.getByRole('button', { name: 'Full control', exact: true }).last();
    await control.click();
    assert.equal(await control.getAttribute('aria-pressed'), 'true');
    await control.click();
    assert.equal(await control.getAttribute('aria-pressed'), 'false');
    assert.ok(await page.locator('.battlefield-chosen-values').filter({ hasText: 'Choice: odd' }).count());
    assert.ok(await page.locator('.battlefield-chosen-values').filter({ hasText: 'Type: Human' }).count());
    await page.locator('.battlefield-chosen-values').filter({ hasText: 'Choice: odd' }).first().locator('..').hover({ force: true });
    await page.locator('.floating-card-preview .inspector-chosen-values').filter({ hasText: 'Choice: odd' }).waitFor();
    await page.mouse.move(700, 10);
    const card = page.locator('.hand-layout-item .game-card').first();
    await card.dblclick({ force: true });
    await page.waitForFunction(() => window.handCasts.length === 1);
    assert.equal(await page.evaluate(() => window.handCasts[0].objectId), 8000);
    assert.equal(await page.evaluate(() => window.handCasts[0].actions[0].kind), 'cast_spell');
    for (const width of [1440, 1024]) {
      await page.setViewportSize({ width, height: 900 });
      for (const count of [3, 4]) {
        for (const perspective of [0, 1]) {
          await page.goto(`${url}?playerCount=${count}&perspective=${perspective}&utilities`);
          await page.locator('.phase-track-player--self').waitFor();
          assert.equal(await page.locator('.phase-track-player').count(), count);
          const boxes = await page.locator('.phase-track-opponents .phase-track-player').evaluateAll(nodes => nodes.map(node => {
            const track = node.getBoundingClientRect();
            const field = document.querySelector(`[data-opponent-zone="${node.dataset.phasePlayer}"]`).getBoundingClientRect();
            return { player: node.dataset.phasePlayer, track: { x: track.x, width: track.width }, field: { x: field.x, width: field.width } };
          }));
          for (const box of boxes) {
            assert.ok(Math.abs(box.track.x - box.field.x) < 1 && Math.abs(box.track.width - box.field.width) < 1,
              `${width}px / ${count} players / seat ${perspective}: ${JSON.stringify(box)}`);
          }
          const alignment = await page.locator('.table-phase-divider').evaluate(divider => {
            const icon = divider.querySelector('.phase-track-player--self .phase-track-stop-icon');
            const logo = divider.querySelector('.topbar-brand-menu-trigger');
            const label = icon.closest('button').querySelector('.phase-track-step-label').getBoundingClientRect();
            const links = divider.querySelector('.topbar-brand-links').getBoundingClientRect();
            const iconBox = icon.getBoundingClientRect(), logoBox = logo.getBoundingClientRect();
            return { labelY: label.top + label.height / 2, linksY: links.top + links.height / 2, iconY: iconBox.top + iconBox.height / 2, logoY: logoBox.top + logoBox.height / 2,
              localLift: getComputedStyle(icon).translate, localOverflow: getComputedStyle(icon.closest('nav')).overflow,
              opponentLift: getComputedStyle(divider.querySelector('.phase-track-opponents .phase-track-stop-icon')).translate };
          });
          assert.ok(Math.abs(alignment.iconY - alignment.logoY) < 1, `${width}px logo aligns with local phase icons: ${JSON.stringify(alignment)}`);
          assert.ok(Math.abs(alignment.labelY - alignment.linksY) < 1, 'site icons align with local phase names');
          assert.equal(alignment.localLift, '0px -6px');
          assert.equal(alignment.localOverflow, 'visible', 'raised hands remain fully visible');
          assert.equal(alignment.opponentLift, 'none');
          const local = await page.locator('.phase-track-player--self').boundingBox();
          // Resolve the original span expression through a temporary CSS width.
          const expected = await page.locator('.phase-track-player--self').evaluate(node => {
            const probe = document.createElement('div');
            probe.style.width = 'var(--phase-track-span)';
            node.parentElement.append(probe);
            const width = probe.getBoundingClientRect().width;
            probe.remove();
            return width;
          });
          assert.ok(Math.abs(local.width - expected) < 1, `${width}px local tracker keeps original span`);
          assert.equal(await page.locator('.phase-track-player--self').getAttribute('data-phase-player'), String(perspective));
        }
      }
    }
    await page.goto(`${url}?singleOpponent&perspective=1&utilities`);
    await page.locator('.phase-track--shared').first().waitFor();
    assert.equal(await page.locator('.phase-track--shared [data-stop-player]').first().getAttribute('data-stop-player'), '1', 'your hand icon comes first from either seat');
    const duelAlignment = await page.locator('.table-phase-divider').evaluate(divider => {
      const icon = divider.querySelector('.phase-track-player-stop:first-child .phase-track-stop-icon');
      const label = icon.closest('.phase-track-cell').querySelector('.phase-track-step-label');
      const center = node => { const rect = node.getBoundingClientRect(); return rect.top + rect.height / 2; };
      return { icon: center(icon), logo: center(divider.querySelector('.topbar-brand-menu-trigger')),
        label: center(label), links: center(divider.querySelector('.topbar-brand-links')) };
    });
    assert.ok(Math.abs(duelAlignment.icon - duelAlignment.logo) < 1, 'duel logo follows your raised icons');
    assert.ok(Math.abs(duelAlignment.label - duelAlignment.links) < 1, 'duel site icons follow phase names');
    const duelSpacing = await page.locator('.phase-track-group--duel').evaluate(group => {
      const untap = group.querySelector('[data-phase-name="Untap"]');
      const upkeep = group.querySelector('[data-phase-name="Upkeep"]');
      const legend = untap.querySelector('.phase-track-duel-legend');
      return { left: group.getBoundingClientRect().left, untap: untap.getBoundingClientRect().width,
        upkeep: upkeep.getBoundingClientRect().width, underline: getComputedStyle(untap).borderBottomWidth,
        stops: untap.querySelectorAll('.phase-track-stop-button').length,
        fits: [...legend.querySelectorAll('.phase-track-player-heading, .phase-track-player-name')]
          .every(node => node.scrollWidth <= node.clientWidth + 1) };
    });
    assert.ok(duelSpacing.left <= 24, 'duel tracker starts at the left side of the table');
    assert.ok(duelSpacing.untap > duelSpacing.upkeep, 'Untap reserves extra space for both player indicators');
    assert.equal(duelSpacing.underline, '0px');
    assert.equal(duelSpacing.stops, 0, 'Untap has no underlined disabled stop controls');
    assert.ok(duelSpacing.fits, 'player names and Turn/Priority labels fit in Untap');
    assert.deepEqual(errors, []);
  } finally { await browser.close(); await vite.close(); }
});
