import { build, preview, createServer } from 'vite';
import { chromium } from 'playwright';
import { mkdir, writeFile, readdir, symlink } from 'node:fs/promises';
import { resolve } from 'node:path';
import { availableParallelism, loadavg } from 'node:os';
import assert from 'node:assert/strict';
import { encodePuzzlePayload } from '../../src/lib/puzzles.js';

const output = resolve(process.env.PROFILE_OUTPUT || `../../output/stack-animation-profile/${new Date().toISOString().replaceAll(':', '-')}`);
await mkdir(output, { recursive: true });
const field = ['Seal of Fire', 'Forest', 'Island', 'Mountain', 'Plains', 'Swamp', 'Sol Ring', 'Arcane Signet', 'Llanowar Elves', 'Elvish Mystic', 'Birds of Paradise', 'Wall of Roots', 'Grizzly Bears', 'Runeclaw Bear', 'Hill Giant'];
const puzzle = encodePuzzlePayload({ version: 1, players: [
  { name: 'Alice', life: 20, zones: { battlefield: field, hand: ['Memnite', 'Ornithopter', 'Welding Jar'], library: Array(12).fill('Forest') } },
  { name: 'Bob', life: 20, zones: { battlefield: ['Grizzly Bears', 'Runeclaw Bear', 'Hill Giant', 'Wall of Wood', 'Llanowar Elves', 'Elvish Mystic', 'Forest', 'Island', 'Mountain', 'Plains', 'Swamp', 'Sol Ring'], library: Array(12).fill('Forest') } },
] });
let server, browser, page, cdp;
let tracing = false;
const errors = [];
const workers = [];
const hostLoad = [];
const sampleHostLoad = () => hostLoad.push({ time: new Date().toISOString(), load: loadavg() });
sampleHostLoad();
const hostLoadTimer = setInterval(sampleHostLoad, 5000);
async function mark(label) {
  console.log(label);
  await page.evaluate(label => { window.profilePhase = label; performance.mark(label); }, label);
}
async function resolveStack() {
  for (let pass = 0; pass < 6; pass++) {
    if (!await page.locator('.card-stack-slot:not([data-leaving])').count()) return;
    await page.locator('.decision-main-button').first().click();
    await page.waitForTimeout(700);
  }
  throw new Error('Stack did not resolve through visible priority controls');
}
async function settle(ms = 2200) { await page.mouse.move(1430, 890); await page.waitForTimeout(ms); }
async function dump(name) {
  const phase = await page.evaluate(name => { const old = window.profilePhase; window.profilePhase = `capture-${name}`; performance.mark(window.profilePhase); return old; }, name);
  await page.screenshot({ path: `${output}/${name}.png` });
  await writeFile(`${output}/${name}.txt`, await page.locator('body').innerText());
  await writeFile(`${output}/${name}.json`, JSON.stringify(await page.evaluate(() => ({
    state: window.__ironsmithE2E?.snapshot()?.state,
    cards: [...document.querySelectorAll('.game-card:not(.hand-card):not(.stack-card)')].map(node => ({
      id: node.dataset.objectId, name: node.dataset.cardName, rect: node.getBoundingClientRect().toJSON(),
    })),
  })), null, 2));
  await page.evaluate(phase => { window.profilePhase = phase; performance.mark(phase); }, phase);
}
async function finishTrace() {
  if (!tracing) return;
  const completed = new Promise(resolve => cdp.once('Tracing.tracingComplete', resolve));
  await cdp.send('Tracing.end');
  const { stream } = await completed;
  let trace = '';
  while (true) {
    const chunk = await cdp.send('IO.read', { handle: stream });
    trace += chunk.base64Encoded ? Buffer.from(chunk.data, 'base64').toString() : chunk.data;
    if (chunk.eof) break;
  }
  await cdp.send('IO.close', { handle: stream });
  await writeFile(`${output}/trace.json`, trace);
  tracing = false;
}
try {
  if (process.env.PROFILE_DEV) {
    server = await createServer({ logLevel: 'silent', server: { host: '127.0.0.1', port: 0, hmr: false, watch: null } });
    await server.listen();
  } else {
    console.log(process.env.PROFILE_APP ? 'Using captured production app' : 'Building production app');
    const outDir = process.env.PROFILE_APP || `${output}/app`;
    if (!process.env.PROFILE_APP) await build({ logLevel: 'warn', define: { 'import.meta.env.VITE_E2E_TEST': '"true"' }, build: { outDir, copyPublicDir: false, emptyOutDir: true, sourcemap: process.env.PROFILE_CPU ? 'hidden' : false } });
    for (const name of process.env.PROFILE_APP ? [] : await readdir('public')) {
      await symlink(resolve('public', name), `${outDir}/${name}`).catch(e => { if (e.code !== 'EEXIST') throw e; });
    }
    server = await preview({ build: { outDir }, preview: { host: '127.0.0.1', port: 0 } });
  }
  const base = `http://127.0.0.1:${server.httpServer.address().port}`;
  browser = await chromium.launch({ channel: 'chrome', headless: process.env.PROFILE_HEADLESS === '1' });
  page = await browser.newPage({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: Number(process.env.PROFILE_DPR || 2) });
  page.on('pageerror', error => errors.push(error.stack));
  page.on('worker', worker => workers.push(worker.url()));
  cdp = await page.context().newCDPSession(page);
  const browserCdp = await browser.newBrowserCDPSession();
  const gpu = await browserCdp.send('SystemInfo.getInfo');
  await writeFile(`${output}/environment.json`, JSON.stringify({ browser: await browser.version(), headless: process.env.PROFILE_HEADLESS === '1', production: !process.env.PROFILE_DEV,
    app: process.env.PROFILE_APP || `${output}/app`, viewport: { width: 1440, height: 900 }, dpr: Number(process.env.PROFILE_DPR || 2), tracing: !process.env.PROFILE_LIGHT,
    observerHooks: 'Existing E2E snapshot observer enabled; all gameplay uses normal visible UI controls', gpu: gpu.gpu,
    host: { availableParallelism: availableParallelism(), initialLoad: loadavg() } }, null, 2));
  await page.goto(`${base}/?puzzle=${puzzle}`, { timeout: 120000 });
  await page.locator('.game-card[data-card-name="Seal of Fire"]').first().waitFor({ timeout: 120000 });
  for (let i = 0; i < 5; i++) {
    const toggle = page.getByRole('button', { name: /^Auto-pass:/ }).first();
    const label = await toggle.getAttribute('aria-label');
    if (/off\./i.test(label)) break;
    await toggle.click(); await page.waitForTimeout(100);
  }
  for (let i = 0; i < 15; i++) {
    if (await page.locator('.keyboard-shortcuts-backdrop').isVisible()) await page.keyboard.press('Escape');
    const button = page.locator('.decision-main-button').first();
    const label = await button.textContent();
    const state = await page.evaluate(() => window.__ironsmithE2E?.snapshot()?.state);
    console.log('pregame', label, state?.phase, state?.step);
    if (/go to combat/i.test(label) || (/first main/i.test(state?.phase || '') && state?.decisionDetail?.kind === 'priority')) break;
    if (/skipping|waiting|processing/i.test(label)) { await page.waitForTimeout(1000); continue; }
    if (!/keep hand|pregame|continue|begin game|go to/i.test(label)) throw new Error(`Unexpected pregame ${label}`);
    await button.click(); await page.waitForTimeout(1000);
  }
  await settle(6000);
  await page.evaluate(() => {
    window.profilePhase = 'idle';
    window.profileData = { frames: [], longTasks: [], longAnimationFrames: [], mutations: [] };
    let previous, previousCallback, previousPhase;
    function frame(time) {
      const callback = performance.now();
      if (previous !== undefined && previousPhase === window.profilePhase) window.profileData.frames.push({ time, delta: time - previous,
        callbackDelta: callback - previousCallback, callbackDelay: callback - time, phase: window.profilePhase });
      previous = time; previousCallback = callback; previousPhase = window.profilePhase; requestAnimationFrame(frame);
    }
    requestAnimationFrame(frame);
    new PerformanceObserver(list => window.profileData.longTasks.push(...list.getEntries().map(e => ({ start: e.startTime, duration: e.duration, phase: window.profilePhase })))).observe({ type: 'longtask' });
    if (PerformanceObserver.supportedEntryTypes.includes('long-animation-frame')) new PerformanceObserver(list => window.profileData.longAnimationFrames.push(...list.getEntries().map(e => e.toJSON()))).observe({ type: 'long-animation-frame' });
    new MutationObserver(records => {
      const interesting = records.filter(r => r.type === 'childList' || /data-(leaving|arriving|materialize|burning)/.test(r.attributeName));
      if (interesting.length) window.profileData.mutations.push({ time: performance.now(), phase: window.profilePhase, changes: interesting.length, stackCards: document.querySelectorAll('.stack-card').length, battlefieldCards: document.querySelectorAll('.game-card:not(.hand-card):not(.stack-card)').length });
    }).observe(document.body, { childList: true, subtree: true, attributes: true, attributeFilter: ['data-leaving', 'data-arriving', 'data-materialize', 'data-burning'] });
  });
  if (!process.env.PROFILE_LIGHT) {
    await cdp.send('Tracing.start', { categories: 'devtools.timeline,blink.user_timing,cc,gpu,benchmark,disabled-by-default-devtools.timeline.frame', transferMode: 'ReturnAsStream' });
    tracing = true;
  }
  if (process.env.PROFILE_CPU) {
    await cdp.send('Profiler.enable');
    await cdp.send('Profiler.setSamplingInterval', { interval: 500 });
    await cdp.send('Profiler.start');
  }
  await mark('idle'); await settle(3000); await dump('populated-board');
  for (const name of process.env.PROFILE_SKIP_CASTS ? [] : ['Memnite', 'Ornithopter', 'Welding Jar']) {
    await mark(`cast-${name}`);
    const hand = page.locator(`.game-card.hand-card[data-card-name="${name}"]`);
    const box = await hand.boundingBox();
    await page.mouse.move(box.x + box.width / 2, box.y + 24);
    await page.mouse.down();
    await page.mouse.move(box.x + box.width / 2, box.y - 150, { steps: 12 });
    await page.mouse.move(900, 460, { steps: 10 });
    await page.mouse.up();
    await page.waitForTimeout(500);
    const pay = page.getByRole('button', { name: 'Pay', exact: true });
    if (await pay.isVisible()) await pay.click();
    await page.locator(`.stack-card[data-card-name="${name}"]`).first().waitFor({ timeout: 15000 });
    await settle();
    await dump(`stack-${name}`);
    await mark(`resolve-${name}`);
    await resolveStack();
    await settle(2600);
    await page.locator(`.game-card:not(.hand-card):not(.stack-card)[data-card-name="${name}"]`).first().waitFor({ timeout: 15000 });
  }
  await dump('three-permanents-entered');
  await mark('inspect-seal');
  const seal = page.locator('.game-card[data-card-name="Seal of Fire"]').first();
  await seal.click(); await page.waitForTimeout(700);
  await dump('seal-inspector');
  await mark('activate-seal-choose-targets');
  if (!await page.getByRole('button', { name: /^Submit Targets/ }).isVisible()) {
    await page.getByRole('button', { name: /^(?:Activate )?Seal of Fire:/ }).first().click({ timeout: 10000 });
  }
  await page.waitForTimeout(500);
  await dump('activation-targets');
  await mark('activate-seal-sacrifice');
  const target = page.getByRole('button', { name: 'Grizzly Bears', exact: true }).last();
  await target.click();
  const submit = page.getByRole('button', { name: /^Submit Targets/ });
  if (await submit.isVisible()) await submit.click();
  await page.locator('.stack-card').first().waitFor({ timeout: 15000 });
  await settle(2500);
  await mark('resolve-seal-destroy-creature');
  await resolveStack();
  await settle(3500);
  await dump('final-board');
  const final = await page.evaluate(() => ({
    stack: document.querySelectorAll('.card-stack-slot:not([data-leaving])').length,
    cards: [...document.querySelectorAll('.game-card:not(.hand-card):not(.stack-card)')].map(node => node.dataset.cardName),
    graveyards: window.__ironsmithE2E?.snapshot()?.state?.players?.map(player => player.graveyard_size),
  }));
  assert.equal(final.stack, 0);
  assert.equal(final.cards.length, process.env.PROFILE_SKIP_CASTS ? 25 : 28);
  assert.equal(final.cards.filter(name => name === 'Seal of Fire').length, 0);
  assert.equal(final.cards.filter(name => name === 'Grizzly Bears').length, 1);
  assert.deepEqual(final.graveyards, [1, 1]);
  await mark('final-idle'); await settle(3000);
  if (process.env.PROFILE_CPU) {
    const {profile} = await cdp.send('Profiler.stop');
    await writeFile(`${output}/main-thread.cpuprofile`, JSON.stringify(profile));
  }
  await finishTrace();
  const data = await page.evaluate(() => window.profileData);
  sampleHostLoad();
  await writeFile(`${output}/samples.json`, JSON.stringify({ ...data, errors, workers, hostLoad }, null, 2));
  console.log('PROFILE_OUTPUT', output);
} catch (error) {
  if (page) await dump('failure').catch(() => {});
  await finishTrace().catch(() => {});
  if (page) await writeFile(`${output}/samples-partial.json`, JSON.stringify({
    ...await page.evaluate(() => window.profileData || {}), errors, workers, hostLoad,
  }, null, 2));
  throw error;
} finally {
  clearInterval(hostLoadTimer);
  await browser?.close();
  if (server?.close) await server.close();
  else if (server?.httpServer) await new Promise(resolve => server.httpServer.close(resolve));
}
