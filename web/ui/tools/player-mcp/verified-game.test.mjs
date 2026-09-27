// Opt in after building the real WASM package and local tournament catalog:
// IRONSMITH_MCP_VERIFIED_GAME=1 node --test tools/player-mcp/verified-game.test.mjs
// This uses only the public MCP browser tools and ordinary production UI.
import test from 'node:test';
import assert from 'node:assert/strict';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { mkdir, mkdtemp, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { StreamableHTTPClientTransport } from '@modelcontextprotocol/sdk/client/streamableHttp.js';
import { createRuntime, startHttpServer } from './server.mjs';
import { REPO_ROOT } from './local-table.mjs';

const directory = path.dirname(fileURLToPath(import.meta.url));
const run = promisify(execFile);
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const enabled = control => !control.disabled && !control.blocked;
const named = name => control => control.tag === 'button' && control.name === name;
const handCards = observation => observation.controls.filter(control =>
  control.attributes?.['data-object-id'] && control.attributes?.['data-card-name']
  && control.ancestry?.some(ancestor => ancestor.attributes?.['data-card-navigation-scope'] === 'hand'));

test('real Verified MCP peers mulligan tournament 61+15 decks in both declaration orders', {
  skip: process.env.IRONSMITH_MCP_VERIFIED_GAME !== '1', timeout: 600000,
}, async t => {
  const base = path.join(REPO_ROOT, 'reports/player-mcp');
  await mkdir(base, { recursive: true });
  const evidenceDir = await mkdtemp(path.join(base, 'verified-mulligan-'));
  const service = await startHttpServer({ port: 0, runtime: createRuntime({ evidenceDir }) });
  const client = new Client({ name: 'verified-mulligan-regression', version: '0.1.0' });
  t.after(async () => { await client.close(); await service.close(); });
  await client.connect(new StreamableHTTPClientTransport(new URL(service.url)));
  let players;

  async function call(name, input = {}) {
    const response = await client.callTool({ name, arguments: input }, undefined, { timeout: 120000 });
    const value = JSON.parse(response.content.find(item => item.type === 'text').text);
    if (response.isError) throw Object.assign(new Error(`${name}: ${value.error}`), value);
    return value;
  }
  function healthy(observation) {
    assert.deepEqual(observation.syncFailures, [], `${observation.name}: ${observation.text}`);
    assert.doesNotMatch(observation.text, /Sync failed|Cheat detected|Match start failed|Auto-pass failed/i);
    assert.deepEqual(observation.evidence.filter(event => event.kind === 'pageerror'), [], 'no browser exceptions');
  }
  async function pair() {
    const observations = await Promise.all(Object.values(players).map(playerId => call('observe', { playerId })));
    observations.forEach(healthy);
    return Object.fromEntries(observations.map(observation => [observation.name, observation]));
  }
  async function until(predicate, description, timeout = 90000) {
    const deadline = Date.now() + timeout;
    let observations;
    while (Date.now() < deadline) {
      observations = await pair();
      if (predicate(observations)) return observations;
      await pause(250);
    }
    assert.fail(`${description}\n${Object.values(observations || {}).map(value => value.text).join('\n')}`);
  }
  async function click(playerId, predicate) {
    for (let attempt = 0; attempt < 10; attempt++) {
      const observation = await call('observe', { playerId });
      healthy(observation);
      const target = observation.controls.find(control => enabled(control) && predicate(control));
      assert.ok(target, `Expected a visible enabled control: ${observation.text}`);
      try {
        return await call('act', { playerId, observationId: observation.observationId, ref: target.ref, action: 'click' });
      } catch (error) {
        // STALE_OBSERVATION guarantees no action was sent. Never retry other errors.
        if (error.code !== 'STALE_OBSERVATION') throw error;
      }
    }
    assert.fail('The selected control never had a current observation');
  }
  async function downloadDiagnostics(playerId) {
    let observation = await call('observe', { playerId });
    for (const name of ['Diagnostics', 'Download report']) {
      let clicked = false;
      for (let attempt = 0; attempt < 30; attempt++) {
        const target = observation.controls.find(control => enabled(control) && named(name)(control));
        if (target) {
          try {
            observation = await call('act', { playerId, observationId: observation.observationId,
              ref: target.ref, action: 'click' });
            clicked = true;
            break;
          } catch (error) {
            if (error.code !== 'STALE_OBSERVATION') throw error;
            observation = error.observation;
            continue;
          }
        }
        await pause(100);
        observation = await call('observe', { playerId });
      }
      if (!clicked) throw new Error(`Could not reach normal UI ${name} control`);
    }
    return observation.downloads;
  }

  try {
    const completed = new Set();
    // No seed or engine controls: independently shuffled games naturally choose
    // the starter. Repeated starters are closed before making a declaration.
    for (let attempt = 1; attempt <= 8 && completed.size < 2; attempt++) {
      // Reuse the real setup client, including its explicit Verified radio check.
      const { stdout } = await run(process.execPath, [path.join(directory, 'setup-game.mjs'),
        '--url', service.url, '--deck', 'mtgtop8-90965-890825', '--headless'], { timeout: 180000 });
      const setup = JSON.parse(stdout);
      players = setup.players;
      assert.equal(setup.securityMode, 'verified');
      assert.equal(setup.deck.counts.mainboard, 61);
      assert.equal(setup.deck.counts.sideboard, 15);
      const setupRecords = (await readFile(path.join(evidenceDir, 'tools.jsonl'), 'utf8')).trim().split('\n').map(JSON.parse);
      assert.ok(setupRecords.some(record => record.result?.controls?.some(control =>
        control.tag === 'input' && control.type === 'radio' && control.value === 'verified' && control.checked)),
      'the visible Verified radio was actually selected');

      const initial = await until(value => Object.values(value).every(observation => handCards(observation).length === 7)
        && Object.values(value).some(observation => observation.controls.some(control => enabled(control) && named('Keep hand')(control))),
        'Both players must receive their seven-card opening hand');
      const order = initial.Bob.controls.some(control => enabled(control) && named('Keep hand')(control))
        ? 'guest-first-mulligan' : 'host-keep-then-guest-mulligan';
      if (completed.has(order)) {
        await Promise.all(Object.values(players).map(playerId => call('close_player', { playerId })));
        players = null;
        continue;
      }
      t.diagnostic(`Attempt ${attempt}: ${order}`);
      const originalIds = new Set(handCards(initial.Bob).map(card => card.attributes['data-object-id']));
      let aliceKept = false, bobMulliganed = false;
      // Starting player is genuinely randomized, so declarations can occur in either order.
      while (!aliceKept || !bobMulliganed) {
        const state = await until(value => (!aliceKept && value.Alice.controls.some(control => enabled(control) && named('Keep hand')(control)))
          || (!bobMulliganed && value.Bob.controls.some(control => enabled(control) && named('Mulligan')(control))),
        'The next opening-hand declaration must become available');
        if (!aliceKept && state.Alice.controls.some(control => enabled(control) && named('Keep hand')(control))) {
          await click(players.Alice, named('Keep hand'));
          aliceKept = true;
        } else {
          await click(players.Bob, named('Mulligan'));
          bobMulliganed = true;
        }
      }

      const redraw = await until(value => {
        const cards = handCards(value.Bob);
        return cards.length === 7 && cards.every(card => !originalIds.has(card.attributes['data-object-id'])
          && card.attributes['data-card-name'] !== 'Hidden Card');
      }, 'Mulligan must replace every original hand object with seven newly revealed cards');
      const redrawNames = new Set(handCards(redraw.Bob).map(card => card.attributes['data-card-name']));
      let kept = false, bottomed = false;
      while (!kept || !bottomed) {
        const state = await until(value => (!bottomed && /Choose 1 card\(s\) to put on the bottom of your library/i.test(value.Bob.text))
          || (!kept && value.Bob.controls.some(control => enabled(control) && named('Keep hand')(control))),
        'The redraw must offer one bottom-card choice and a keep decision');
        if (!bottomed && /Choose 1 card\(s\) to put on the bottom of your library/i.test(state.Bob.text)) {
          await until(value => value.Bob.controls.some(control => enabled(control)
            && control.tag === 'button' && redrawNames.has(control.name)),
          'The bottom-card choices must reveal their names to the owner', 15000);
          await click(players.Bob, control => control.tag === 'button' && redrawNames.has(control.name));
          await until(value => value.Bob.controls.some(control => enabled(control) && /^Submit\b/i.test(control.name)),
            'The chosen bottom card must be submittable');
          await click(players.Bob, control => control.tag === 'button' && /^Submit\b/i.test(control.name));
          bottomed = true;
        } else {
          await click(players.Bob, named('Keep hand'));
          kept = true;
        }
      }
      await until(value => handCards(value.Bob).length === 6, 'Exactly one card must leave the new hand');
      const deadline = Date.now() + 90000;
      while (true) {
        const state = await pair();
        if (Object.values(state).every(observation => /Main I\nTurn 1\n/.test(observation.text))) break;
        assert.ok(Date.now() < deadline, 'Both peers must reach the real first main phase');
        for (const observation of Object.values(state)) {
          const advance = observation.controls.find(control => enabled(control) && control.tag === 'button'
            && /^(Pregame|Begin Game|Continue|Go to Upkeep|Go to Draw|Go to Main I)$/i.test(control.name));
          if (advance) await click(observation.playerId, named(advance.name));
        }
        await pause(250);
      }
      // The observed failure arrived at the 15s reveal-token timeout. Keep both
      // clients healthy beyond that window, not merely until a transient UI change.
      const healthDeadline = Date.now() + 17000;
      while (Date.now() < healthDeadline) { await pair(); await pause(500); }
      completed.add(order);
      await Promise.all(Object.values(players).map(playerId => call('close_player', { playerId })));
      players = null;
    }
    assert.deepEqual([...completed].sort(), ['guest-first-mulligan', 'host-keep-then-guest-mulligan'],
      'Both natural declaration orders must be exercised within eight fresh games');
    t.diagnostic(`Real Verified mulligan passed; MCP evidence: ${evidenceDir}`);
  } catch (error) {
    for (const [name, playerId] of Object.entries(players || {})) {
      const screenshot = await client.callTool({ name: 'screenshot', arguments: { playerId } }).catch(() => null);
      const image = screenshot?.content?.find(item => item.type === 'image');
      if (image) await writeFile(path.join(evidenceDir, `${name}-failure.png`), Buffer.from(image.data, 'base64'));
      const diagnostics = await downloadDiagnostics(playerId).catch(error => ({ error: error.message }));
      t.diagnostic(`${name} normal UI diagnostics: ${JSON.stringify(diagnostics)}`);
    }
    t.diagnostic(`Failure evidence: ${evidenceDir}`);
    throw error;
  }
});
