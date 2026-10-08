import test from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { createServer } from 'vite';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const fixture = `import React from 'react';
import {createRoot} from 'react-dom/client';
import {GameProvider,useGame} from '/src/context/GameContext.jsx';
import {I18nProvider} from '/src/i18n/I18nContext.jsx';
import PhaseTrack from '/src/components/board/PhaseTrack.jsx';
import PriorityHoldButton from '/src/components/decisions/PriorityHoldButton.jsx';
import '/src/index.css';
const h=React.createElement;
function Probe() {
  const c=useGame(); window.context=c;
  return h('div',{className:'table-shell','data-focused-hud':'true',style:{width:600,padding:30}},
    h('div',{className:'topbar-phase-shell'},h(PhaseTrack)),
    c.postActionPriorityWindow ? h(PriorityHoldButton,{className:'h-12 w-32 text-[14px]'}) :
      h('button',{onClick:()=>c.dispatch({type:'priority_action',action_ref:{kind:'pass_priority'}})},'Resolve'),
    h('button',{onClick:()=>c.dispatch({type:'priority_action',action_ref:{kind:'cast_spell',spell_id:7,casting_method:{kind:'normal'}}})},'Cast'),
    h('button',{onClick:()=>c.dispatch({type:'priority_action',action_ref:{kind:'activate_ability',source:7,ability_index:0}})},'Activate'));
}
createRoot(document.getElementById('root')).render(h(I18nProvider,null,h(GameProvider,null,h(Probe))));`;
const mockPeer = `import {useEffect} from 'react';
const multiplayer={matchStarted:true,role:'client',submittingAction:false};
window.submissions=[];
export function usePeerLobby({setState}) {
  useEffect(()=>{window.publish=state=>{window.currentState=state;setState(state);};},[setState]);
  return {multiplayer,submitMultiplayerCommand:async command=>{
    window.submissions.push({command,time:performance.now()});
    const before=window.currentState, kind=command.action_ref?.kind;
    const next={...before,snapshot_id:before.snapshot_id+1};
    if(kind==='cast_spell'||kind==='activate_ability') {
      next.stack_size=1;next.stack_objects=[{id:next.snapshot_id,controller:0,name:'Test spell'}];
    } else {next.decision={...before.decision,player:1};next.priority_player=1;}
    window.publish(next);
  }};
}`;
const mockWasm = `const game={setAutoCleanupDiscard:async()=>{}};
export function useWasmGame(){return {game,loading:false};}`;
const actions = [
  { index: 0, kind: 'pass_priority', label: 'Pass priority', action_ref: { kind: 'pass_priority' } },
  { index: 1, kind: 'cast_spell', label: 'Cast', action_ref: { kind: 'cast_spell', spell_id: 7, casting_method: { kind: 'normal' } } },
  { index: 2, kind: 'activate_ability', label: 'Activate', action_ref: { kind: 'activate_ability', source: 7, ability_index: 0 } },
];
const priority = (snapshot_id, overrides = {}) => ({
  snapshot_id, turn_number: 1, active_player: 0, perspective: 0, priority_player: 0,
  phase: 'first main phase', step: null, stack_size: 0, stack_objects: [],
  decision: { kind: 'priority', player: 0, analysis_complete: true, actions }, ...overrides,
});

test('real provider drains the Hold button, retains priority on click, and honors tracker stops', { timeout: 60000 }, async () => {
  const server = await createServer({ root, logLevel: 'error', server: { host: '127.0.0.1', port: 0, hmr: false }, plugins: [{
    name: 'priority-stop-fixture',
    transform(code, id) {
      if (id.endsWith('/hooks/usePeerLobby.js')) return mockPeer;
      if (id.endsWith('/hooks/useWasmGame.js')) return mockWasm;
    },
    configureServer(server) {
      server.middlewares.use((request, response, next) => {
        if (request.url === '/priority-stops.html') {
          response.setHeader('Content-Type', 'text/html');
          response.end('<div id="root"></div><script type="module" src="/priority-stops-fixture.jsx"></script>');
        } else next();
      });
    },
    resolveId(id) { if (id === '/priority-stops-fixture.jsx') return '\0priority-stops-fixture.jsx'; },
    load(id) { if (id === '\0priority-stops-fixture.jsx') return fixture; },
  }] });
  await server.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1100, height: 700 } });
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    const load = async state => {
      await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/priority-stops.html`);
      await page.waitForFunction(() => window.context && window.publish).catch(error => { throw new Error(error.message + ': ' + errors.join('; ')); });
      await page.evaluate(state => window.publish(state), state);
      await page.waitForFunction(id => window.context.state?.snapshot_id === id, state.snapshot_id);
    };
    await load(priority(1));
    await page.evaluate(() => window.context.setAutoResolveEnabled(true));
    await page.getByRole('button', { name: 'Cast', exact: true }).click();
    const hold = page.getByRole('button', { name: 'Hold Priority', exact: true });
    await hold.waitFor();
    assert.equal(await hold.innerText(), 'Hold Priority\nHold Priority');
    const visual = await hold.evaluate(button => {
      const fill = getComputedStyle(button.querySelector('.priority-hold-window-fill'));
      return { animation: fill.animationName, background: fill.backgroundColor, base: getComputedStyle(button).backgroundColor };
    });
    assert.equal(visual.animation, 'priority-hold-drain');
    assert.equal(visual.background, 'rgb(246, 214, 74)');
    assert.equal(visual.base, 'rgb(8, 8, 8)');
    await page.waitForTimeout(600);
    const clip = await hold.locator('.priority-hold-window-fill').evaluate(el => getComputedStyle(el).clipPath);
    assert.match(clip, /inset\(/);
    assert.notEqual(clip, 'inset(0px)');
    if (process.env.PRIORITY_STOP_SCREENSHOTS) await page.locator('.table-shell').screenshot({ path: '/tmp/priority-stop-hold.png' });
    await hold.click();
    await page.waitForTimeout(2100);
    assert.equal(await page.evaluate(() => window.submissions.length), 1, 'Hold blocks even enabled Auto-pass');
    await page.getByRole('button', { name: 'Resolve', exact: true }).click();
    await page.waitForFunction(() => window.submissions.length === 2);

    await load(priority(10));
    await page.getByRole('button', { name: 'Activate', exact: true }).click();
    await hold.waitFor();
    await page.waitForFunction(() => window.submissions.length === 2, null, { timeout: 4000 });
    const elapsed = await page.evaluate(() => window.submissions[1].time - window.submissions[0].time);
    assert.ok(elapsed >= 1900 && elapsed < 3500, `automatic pass follows the two-second window: ${elapsed}`);

    await load(priority(20));
    const upkeep = page.getByRole('button', { name: /^Upkeep:/ });
    assert.equal(await upkeep.locator('svg').count(), 1, 'off stops remain discoverable');
    assert.equal(await upkeep.evaluate(el => getComputedStyle(el).pointerEvents), 'auto', 'stop controls override the HUD pointer-events guard');
    await upkeep.click();
    assert.equal(await upkeep.locator('svg').evaluate(el => getComputedStyle(el).fill), 'rgb(189, 138, 255)');
    await page.evaluate(state => window.publish(state), priority(21, { active_player: 1, phase: 'beginning phase', step: 'upkeep' }));
    await page.waitForTimeout(150);
    assert.equal(await page.evaluate(() => window.submissions.length), 0, 'phase stop gates off-turn auto-passing');
    await page.getByRole('button', { name: 'Resolve', exact: true }).click();
    await page.waitForFunction(() => window.submissions.length === 1);
    assert.equal(await upkeep.locator('..').getAttribute('data-stop'), 'off', 'one-time stop is consumed');
    assert.equal(await upkeep.locator('svg').evaluate(el => getComputedStyle(el).fill), 'none');

    await load(priority(30));
    await upkeep.click();
    await upkeep.click();
    assert.equal(await upkeep.locator('svg').evaluate(el => getComputedStyle(el).fill), 'rgb(246, 214, 74)');
    await upkeep.click();
    assert.equal(await upkeep.locator('..').getAttribute('data-stop'), 'off');
    assert.equal(await upkeep.locator('svg').evaluate(el => getComputedStyle(el).fill), 'none');
    await page.evaluate(state => window.publish(state), priority(31, { phase: 'combat phase', step: 'declare blockers' }));
    await page.getByRole('navigation', { name: 'Combat steps' }).waitFor();
    assert.deepEqual(await page.locator('.phase-track-group > nav').evaluateAll(nodes => nodes.map(node => node.getAttribute('aria-label'))), ['Combat steps', 'Turn phases']);
    const combatIcons = await page.getByRole('navigation', { name: 'Combat steps' }).locator('svg').evaluateAll(nodes => nodes.map(node => node.getAttribute('class')));
    assert.equal(new Set(combatIcons).size, 6, 'each combat step has a dedicated icon');
    const blockers = page.getByRole('button', { name: /^Blockers:/ });
    await blockers.click();
    assert.equal(await blockers.locator('svg').count(), 1);
    assert.equal(await blockers.locator('svg').evaluate(el => getComputedStyle(el).fill), 'rgb(189, 138, 255)');
    if (process.env.PRIORITY_STOP_SCREENSHOTS) {
      await blockers.click();
      assert.equal(await blockers.locator('svg').evaluate(el => getComputedStyle(el).fill), 'rgb(246, 214, 74)');
      await page.locator('.topbar-phase-shell').screenshot({ path: '/tmp/priority-stop-trackers.png' });
    }
    const firstStrike = page.getByRole('button', { name: /^First strike:/ });
    await firstStrike.click();
    await firstStrike.click();
    await page.evaluate(state => window.publish(state), priority(32, {
      phase: 'combat phase', step: 'combat damage', combat_damage_step: 'first_strike',
    }));
    await page.waitForFunction(() => window.context.state?.snapshot_id === 32);
    assert.equal(await firstStrike.locator('..').getAttribute('data-phase-active'), 'true');
    const damage = page.getByRole('button', { name: /^Damage:/ });
    await damage.click();
    await page.evaluate(state => window.publish(state), priority(33, {
      phase: 'combat phase', step: 'combat damage', combat_damage_step: 'regular',
    }));
    await page.waitForFunction(() => window.context.state?.snapshot_id === 33);
    assert.equal(await damage.locator('..').getAttribute('data-phase-active'), 'true');
    assert.equal(await firstStrike.locator('..').getAttribute('data-phase-active'), 'false');
    // Stack additions always stop by default, regardless of their controller.
    for (const controller of [0, 1]) {
      await load(priority(40 + controller, { stack_size: 1, stack_objects: [{ id: 70, controller }] }));
      await page.waitForTimeout(150);
      assert.equal(await page.evaluate(() => window.submissions.length), 0);
      await page.evaluate(() => window.context.setAutoResolveEnabled(true));
      await page.waitForFunction(() => window.submissions.length === 1);
    }
    await load(priority(50));
    await upkeep.click();
    await upkeep.click();
    await page.evaluate(() => window.context.setAutoResolveEnabled(true));
    const stoppedStack = priority(51, { active_player: 1, phase: 'beginning phase', step: 'upkeep',
      stack_size: 1, stack_objects: [{ id: 71, controller: 1 }] });
    await page.evaluate(state => window.publish(state), stoppedStack);
    await page.waitForTimeout(150);
    assert.equal(await page.evaluate(() => window.submissions.length), 0, 'tracker stop overrides Auto-pass');
    await page.getByRole('button', { name: 'Resolve', exact: true }).click();
    await page.waitForFunction(() => window.submissions.length === 1);
    await page.evaluate(state => window.publish(state), { ...stoppedStack, snapshot_id: 52, turn_number: 2 });
    await page.waitForTimeout(150);
    assert.equal(await page.evaluate(() => window.submissions.length), 1, 'recurring stop returns next turn');
    // Reproduce the multiplayer stall: no pause, opponent turn, unfinished analysis.
    await load(priority(60, { active_player: 1,
      decision: { kind: 'priority', player: 0, analysis_complete: false, actions } }));
    await page.waitForFunction(() => window.submissions.length === 1);

    // A stack-only hold must not strand empty-stack phases either. Keep
    // analysis unfinished across successive snapshots; no manual clicks.
    await load(priority(62));
    await page.evaluate(() => window.context.setHoldRule('stack'));
    for (const [id, phase, step] of [[63, 'first main phase', null],
      [65, 'combat phase', 'begin combat'], [67, 'second main phase', null]]) {
      const before = await page.evaluate(() => window.submissions.length);
      await page.evaluate(state => window.publish(state), priority(id, { active_player: 1, phase, step,
        decision: { kind: 'priority', player: 0, analysis_complete: false, actions } }));
      await page.waitForFunction(count => window.submissions.length === count + 1, before);
    }

    // Pass continues across empty-stack phases, yields to the remote player,
    // and stops at the configured step without consuming that stop.
    await load(priority(70));
    await page.evaluate(() => window.context.setAutoResolveEnabled(true));
    await page.evaluate(() => {
      window.context.cyclePriorityStop('step:DeclareBlockers');
      window.context.togglePhasePassing();
    });
    await page.waitForFunction(() => window.submissions.length === 1);
    await page.waitForTimeout(100);
    assert.equal(await page.evaluate(() => window.submissions.length), 1, 'never submits remote priority');
    await page.evaluate(state => window.publish(state), priority(72, { phase: 'combat phase', step: 'begin combat' }));
    await page.waitForFunction(() => window.submissions.length === 2);
    await page.evaluate(state => window.publish(state), priority(74, { phase: 'combat phase', step: 'declare blockers' }));
    await page.waitForFunction(() => window.context.phasePassing === false);
    assert.equal(await page.evaluate(() => window.submissions.length), 2);
    assert.equal(await page.evaluate(() => window.context.priorityStops['step:DeclareBlockers']), 'once');

    // Consuming a pause manually must not restart either passing path.
    await page.waitForTimeout(350); // Let the previous automatic pass cooldown finish.
    assert.equal(await page.evaluate(() => window.context.autoPassEnabled), false);
    assert.equal(await page.evaluate(() => window.context.autoResolveEnabled), false);
    await page.getByRole('button', { name: 'Resolve', exact: true }).click();
    await page.waitForFunction(() => window.submissions.length === 3);
    await page.evaluate(state => window.publish(state), priority(75, {
      phase: 'combat phase', step: 'declare blockers', stack_size: 1,
      stack_objects: [{ id: 900, controller: 0, name: 'Test spell' }],
    }));
    await page.waitForTimeout(200);
    assert.equal(await page.evaluate(() => window.submissions.length), 3, 'manual action after a pause does not resume passing');
    await page.evaluate(state => window.publish(state), priority(76, { active_player: 1 }));
    await page.waitForTimeout(200);
    assert.equal(await page.evaluate(() => window.submissions.length), 3, 'off-turn smart auto-pass stays stopped');

    await load(priority(76));
    await page.evaluate(() => window.context.togglePhasePassing());
    await page.waitForFunction(() => window.submissions.length === 1);
    await page.evaluate(state => window.publish(state), priority(78, {
      phase: 'combat phase', step: 'declare attackers',
      decision: { kind: 'attackers', player: 0, attacker_options: [{ creature: 7, must_attack: false }] },
    }));
    await page.waitForFunction(() => window.submissions.length === 2);
    assert.deepEqual(await page.evaluate(() => window.submissions[1].command),
      { type: 'declare_attackers', declarations: [] });

    // An attackers-step stop must take effect before the empty declaration.
    await load(priority(90));
    await page.evaluate(() => {
      window.context.cyclePriorityStop('step:DeclareAttackers');
      window.context.togglePhasePassing();
    });
    await page.waitForFunction(() => window.submissions.length === 1);
    await page.evaluate(state => window.publish(state), priority(92, {
      phase: 'combat phase', step: 'declare attackers',
      decision: { kind: 'attackers', player: 0, attacker_options: [] },
    }));
    await page.waitForFunction(() => window.context.phasePassing === false);
    assert.equal(await page.evaluate(() => window.submissions.length), 1);

    // Even a later blockers pause overrides automatic no-attack combat.
    await load(priority(93));
    await page.evaluate(() => {
      window.context.cyclePriorityStop('step:DeclareBlockers');
      window.context.togglePhasePassing();
    });
    await page.waitForFunction(() => window.submissions.length === 1);
    await page.evaluate(state => window.publish(state), priority(95, {
      phase: 'combat phase', step: 'declare attackers',
      decision: { kind: 'attackers', player: 0, attacker_options: [{ creature: 7, must_attack: false }] },
    }));
    await page.waitForFunction(() => window.context.phasePassing === false);
    assert.equal(await page.evaluate(() => window.submissions.length), 1, 'blockers pause overrides no-attack shortcut');

    await load(priority(94));
    await page.evaluate(() => window.context.togglePhasePassing());
    await page.waitForFunction(() => window.submissions.length === 1);
    await page.evaluate(state => window.publish(state), priority(96, {
      phase: 'combat phase', step: 'declare attackers',
      decision: { kind: 'attackers', player: 0, attacker_options: [{ creature: 7, must_attack: true }] },
    }));
    await page.waitForFunction(() => window.context.phasePassing === false);
    assert.equal(await page.evaluate(() => window.submissions.length), 1, 'mandatory attacks remain manual');

    await load(priority(80));
    await page.evaluate(() => window.context.togglePhasePassing());
    await page.waitForFunction(() => window.submissions.length === 1);
    await page.evaluate(state => window.publish(state), priority(82, { turn_number: 2 }));
    await page.waitForFunction(() => window.context.phasePassing === false);
    assert.equal(await page.evaluate(() => window.submissions.length), 1, 'does not pass the next turn');
    assert.deepEqual(errors, []);
  } finally { await browser.close(); await server.close(); }
});
