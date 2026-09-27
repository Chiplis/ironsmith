import test from 'node:test';
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { mkdtemp, rm, readFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { PlayerBrowser } from './browser.mjs';

// A small DOM fixture tests the adapter itself. Actual games use the full UI.
const html = `<!doctype html><title>Adapter test</title><style>
.sr-only{position:absolute;width:1px;height:1px;clip:rect(0,0,0,0)}
</style><label>Your Name<input id="name"></label>
<label>Invite Link<input readonly value="https://example.test/?lobby=abc"></label>
<label id="verified">Verified<input class="sr-only" type="radio" name="mode" value="verified"></label>
<div data-card-navigation-scope="hand"><div role="button" tabindex="0" data-object-id="7" data-card-name="Mountain" class="game-card playable">Mountain</div></div>
<div data-action-row onclick="document.querySelector('#result').textContent='Played Mountain'">Play Mountain</div>
<div data-player-target="1">Opponent</div><div id="result">Ready</div>
<button id="pass">Pass priority</button><button disabled>Disabled action</button>
<button id="clock">Clock 19:59</button><button id="error" onclick="console.error('Cheat detected from test peer')">Report sync failure</button>
<a download href="/download">Export Match</a>
<div hidden><button>SECRET DECK IDENTITY</button></div><div style="opacity:0"><span>INVISIBLE IDENTITY</span></div>
<div id="drag-card" role="button" tabindex="0" style="position:absolute;left:100px;top:600px;width:80px;height:100px;background:gold">Draggable card</div>
<div id="field" data-bf-side="self" data-battlefield-drop-grid="true" data-battlefield-grid-columns="4"
 style="position:absolute;left:300px;top:600px;width:400px;height:200px;background:silver"></div>
<script>
let held = false;
document.querySelector('#drag-card').addEventListener('pointerdown', () => { held = 'drag'; });
document.querySelector('#drag-card').addEventListener('keydown', event => { if (event.key === 'Enter') held = 'keyboard'; });
document.addEventListener('pointerup', event => {
  if (held && event.target.closest('[data-bf-side]')) document.querySelector('#result').textContent = 'Placed by ' + held;
  held = false;
});
</script>`;

test('browser adapter exposes visible DOM and guards real UI actions', { timeout: 30000 }, async t => {
  const server = createServer((request, response) => {
    if (request.url.startsWith('/download')) {
      response.writeHead(200, { 'content-type': 'application/json', 'content-disposition': 'attachment; filename="test-export.json"' });
      response.end('{"adapterTest":true}');
    } else { response.writeHead(200, { 'content-type': 'text/html' }); response.end(html); }
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  t.after(() => new Promise(resolve => server.close(resolve)));
  const evidenceDir = await mkdtemp(path.join(os.tmpdir(), 'player-browser-test-'));
  t.after(() => rm(evidenceDir, { recursive: true, force: true }));
  const browser = new PlayerBrowser({ evidenceDir, settleMs: 20, settleTimeoutMs: 180 });
  t.after(() => browser.close());
  const url = `http://127.0.0.1:${server.address().port}/`;
  const first = await browser.openPlayer({ name: 'Alice', url, deckText: '4 Lightning Bolt', headless: true });
  const commanderText = '1 Éowyn, Shieldmaiden';
  const second = await browser.openPlayer({ name: 'Bob', url, deckText: '99 Plains', commanderText, headless: true });
  assert.notEqual(browser.player(first.playerId).context, browser.player(second.playerId).context);
  assert.match(first.observation.url, /name=Alice/);
  assert.equal(Buffer.from(new URL(first.observation.url).searchParams.get('deck'), 'base64url').toString(), '4 Lightning Bolt');
  assert.equal(Buffer.from(new URL(second.observation.url).searchParams.get('commander'), 'base64url').toString(), commanderText);
  assert.doesNotMatch(first.observation.text, /SECRET|INVISIBLE/);
  assert.equal(first.observation.controls.find(control => control.name === 'Invite Link' && control.tag === 'input').value, 'https://example.test/?lobby=abc');
  assert.equal(first.observation.cards[0].attributes['data-object-id'], '7');
  assert.equal(first.observation.cards[0].ancestry[0].attributes['data-card-navigation-scope'], 'hand');
  assert.ok(first.observation.controls.some(control => control.attributes['data-player-target'] === '1'));
  let observation = first.observation;
  const perform = async (predicate, action, value, position) => {
    const target = observation.controls.find(predicate);
    assert.ok(target, 'expected visible action');
    observation = await browser.act({ playerId: first.playerId, observationId: observation.observationId, ref: target.ref, action, value, position });
  };
  await perform(control => control.tag === 'input' && control.name === 'Your Name', 'fill', 'Alice');
  await perform(control => control.tag === 'label' && control.name === 'Verified', 'click');
  assert.ok(observation.controls.find(control => control.tag === 'input' && control.type === 'radio').checked);
  await perform(control => control.attributes['data-action-row'] !== undefined, 'click');
  assert.match(observation.text, /Played Mountain/);
  const page = browser.player(first.playerId).page;
  await page.locator('#clock').evaluate(element => { element.textContent = 'Clock 19:58'; });
  await perform(control => control.name === 'Pass priority', 'click');
  await page.locator('[data-object-id="7"]').evaluate(element => element.classList.add('tapped'));
  const pass = observation.controls.find(control => control.name === 'Pass priority');
  await assert.rejects(browser.act({ playerId: first.playerId, observationId: observation.observationId, ref: pass.ref, action: 'click' }), error => {
    assert.equal(error.code, 'STALE_OBSERVATION'); observation = error.observation; return true;
  });
  await perform(control => control.name === 'Report sync failure', 'click');
  assert.ok(observation.evidence.some(entry => entry.kind === 'sync-failure'));
  const field = observation.controls.find(control => control.attributes['data-bf-side'] === 'self');
  assert.equal(field.name, 'self battlefield');
  assert.deepEqual(field.rect, { x: 300, y: 600, width: 400, height: 200 });
  const point = { x: field.rect.x + field.rect.width / 2, y: field.rect.y + field.rect.height / 2 };
  await perform(control => control.name === 'Draggable card', 'drag', undefined, point);
  assert.match(observation.text, /Placed by drag/);
  await perform(control => control.name === 'Draggable card', 'press', 'Enter');
  await perform(control => control.attributes['data-bf-side'] === 'self', 'pointer_click', undefined, point);
  assert.match(observation.text, /Placed by keyboard/);
  await assert.rejects(browser.act({ playerId: first.playerId, observationId: observation.observationId,
    ref: field.ref, action: 'pointer_click', position: { x: -1, y: 700 } }), /inside the visible viewport/);
  await page.evaluate(() => {
    const cover = document.createElement('div');
    cover.id = 'cover';
    cover.style.cssText = 'position:absolute;left:300px;top:600px;width:400px;height:200px;background:black;z-index:999';
    document.body.append(cover);
  });
  observation = await browser.observe({ playerId: first.playerId });
  await assert.rejects(browser.act({ playerId: first.playerId, observationId: observation.observationId,
    ref: field.ref, action: 'pointer_click', position: point }), /covered by another element/);
  await page.locator('#cover').evaluate(element => element.remove());
  observation = await browser.observe({ playerId: first.playerId });
  await perform(control => control.name === 'Export Match', 'click');
  for (let i = 0; i < 20 && !observation.downloads.some(item => item.status === 'saved'); i++) {
    await page.waitForTimeout(20); observation = await browser.observe({ playerId: first.playerId });
  }
  assert.equal(JSON.parse(await readFile(observation.downloads[0].path, 'utf8')).adapterTest, true);
  const screenshot = await browser.screenshot({ playerId: first.playerId });
  assert.equal(screenshot.mimeType, 'image/png');
  assert.equal(screenshot.data.subarray(1, 4).toString(), 'PNG');
  assert.ok(screenshot.path.startsWith(evidenceDir));
  const joined = await browser.joinLobby({ playerId: second.playerId, lobby: 'visible-lobby-code' });
  assert.equal(new URL(joined.url).searchParams.get('lobby'), 'visible-lobby-code');
  assert.equal(Buffer.from(new URL(joined.url).searchParams.get('commander'), 'base64url').toString(), commanderText);
  const joinedInvite = await browser.joinLobby({ playerId: second.playerId, lobby: `${url}?lobby=invite-code` });
  assert.equal(new URL(joinedInvite.url).searchParams.get('lobby'), 'invite-code');
  assert.equal(Buffer.from(new URL(joinedInvite.url).searchParams.get('deck'), 'base64url').toString(), '99 Plains');
  assert.equal(Buffer.from(new URL(joinedInvite.url).searchParams.get('commander'), 'base64url').toString(), commanderText);
  assert.equal((await browser.closePlayer({ playerId: second.playerId })).closed, true);
});
