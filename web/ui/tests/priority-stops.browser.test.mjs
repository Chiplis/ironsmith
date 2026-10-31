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
import StackCard from '/src/components/cards/StackCard.jsx';
import {HoverProvider} from '/src/context/HoverContext.jsx';
import {DragProvider} from '/src/context/DragContext.jsx';
import PriorityHoldButton from '/src/components/decisions/PriorityHoldButton.jsx';
import PriorityHoldControl from '/src/components/decisions/PriorityHoldControl.jsx';
import {beginStackArrival} from '/src/lib/stack-arrival-activity.js';
window.beginStackArrival=beginStackArrival;
import '/src/index.css';
const h=React.createElement;
function Probe() {
  const c=useGame(); window.context=c;
  return h('div',{className:'table-shell','data-focused-hud':'true',style:{width:600,padding:30}},
    h('div',{className:'topbar-phase-shell'},h(PhaseTrack)),
    h('div',{className:'battlefield-human-action-dock'},h('div',{className:'battlefield-human-quick-controls decision-quick-controls'},h(PriorityHoldControl,{battlefield:true}))),
    c.postActionPriorityWindow ? h(PriorityHoldButton,{className:'h-12 w-32 text-[14px]'}) :
      h('button',{onClick:()=>c.dispatch({type:'priority_action',action_ref:{kind:'pass_priority'}})},'Resolve'),
    h('button',{onClick:()=>c.dispatch({type:'priority_action',action_ref:{kind:'cast_spell',spell_id:7,casting_method:{kind:'normal'}}})},'Cast'),
    h('button',{onClick:()=>c.dispatch({type:'priority_action',action_ref:{kind:'activate_ability',source:7,ability_index:0}})},'Activate'),
    h(HoverProvider,null,h(DragProvider,null,(c.state?.stack_objects||[]).map(entry=>h(StackCard,{key:entry.id,entry})))));
}
createRoot(document.getElementById('root')).render(h(I18nProvider,null,h(GameProvider,null,h(Probe))));`;
const mockPeer = `import {useEffect} from 'react';
const local = new URLSearchParams(location.search).has('local');
const multiplayer={matchStarted:!local,role:local?null:'client',submittingAction:false};
window.submissions=[];
export function usePeerLobby({setState}) {
  useEffect(()=>{window.publish=state=>{window.currentState=state;setState(state);};},[setState]);
  return {multiplayer,submitMultiplayerCommand:async command=>{
    window.submissions.push({command,time:performance.now()});
    const before=window.currentState, kind=command.action_ref?.kind;
    const next={...before,snapshot_id:before.snapshot_id+1};
    if(command.type==='priority_yield') next.pass_intents=[0];
    if(kind==='cast_spell'||kind==='activate_ability') {
      next.stack_size=1;next.stack_objects=[{id:next.snapshot_id,controller:0,name:'Test spell'}];
    } else {next.decision={...before.decision,player:1};next.priority_player=1;}
    window.publish(next);
  }};
}`;
const mockWasm = `const game={setAutoCleanupDiscard:async()=>{},
 uiState:async()=>window.currentState,
 dispatch:async command=>{window.submissions.push({command});
 window.currentState={...window.currentState,snapshot_id:window.currentState.snapshot_id+1,decision:null,game_over:{kind:'draw'}};
 return window.currentState;}};
export function useWasmGame(){return {game,loading:false};}`;
const actions = [
  { index: 0, kind: 'pass_priority', label: 'Pass priority', action_ref: { kind: 'pass_priority' } },
  { index: 1, kind: 'cast_spell', label: 'Cast', action_ref: { kind: 'cast_spell', spell_id: 7, casting_method: { kind: 'normal' } } },
  { index: 2, kind: 'activate_ability', label: 'Activate', action_ref: { kind: 'activate_ability', source: 7, ability_index: 0 } },
];
const priority = (snapshot_id, overrides = {}) => ({
  snapshot_id, turn_number: 1, active_player: 0, perspective: 0, priority_player: 0,
  phase: 'first main phase', step: null, stack_size: 0, stack_objects: [],
  // Default "I can act" stops stay quiet unless a scenario says the seat can act.
  decision: { kind: 'priority', player: 0, analysis_complete: true, can_act: false, actions }, ...overrides,
});
const ALL_STOPS = ['phase:Upkeep', 'phase:Draw', 'phase:Main', 'phase:Combat', 'phase:Main2', 'phase:End', 'phase:Cleanup',
  'step:BeginCombat', 'step:DeclareAttackers', 'step:DeclareBlockers', 'step:FirstStrikeDamage', 'step:CombatDamage', 'step:EndCombat'];

test('real provider uses persistent stops and Ctrl hold without countdowns', { timeout: 90000 }, async () => {
  const server = await createServer({ root, logLevel: 'error', server: { host: '127.0.0.1', port: 0, hmr: false }, plugins: [{
    name: 'priority-stop-fixture',
    transform(code, id) {
      if (id.endsWith('/hooks/usePeerLobby.js')) return mockPeer;
      if (id.endsWith('/hooks/useWasmGame.js')) return mockWasm;
    },
    configureServer(server) {
      server.middlewares.use((request, response, next) => {
        if (request.url?.startsWith('/priority-stops.html')) {
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
    await page.addInitScript(() => localStorage.setItem('ironsmith.priorityStops', '[]'));
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    const load = async (state, local = false) => {
      await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/priority-stops.html${local ? "?local" : ""}`);
      await page.waitForFunction(() => window.context && window.publish).catch(error => { throw new Error(error.message + ': ' + errors.join('; ')); });
      await page.evaluate(state => window.publish(state), state);
      await page.waitForFunction(id => window.context.state?.snapshot_id >= id, state.snapshot_id);
      await page.waitForTimeout(150);
    };
    await load(priority(1));
    // This compact fixture omits the table divider that separates these HUDs
    // in the app. Hide its overlapping action dock while exercising the track.
    await page.locator('.battlefield-human-action-dock').evaluate(el => { el.style.display = 'none'; });
    const lock = page.getByRole('button', { name: 'Lock stops', exact: true });
    assert.equal(await lock.evaluate(el => getComputedStyle(el).pointerEvents), 'auto');
    const upkeep = page.locator('[data-stop-phase="Upkeep"]').first();
    const originalStop = await upkeep.getAttribute('data-stop');
    await lock.click({ timeout: 3000 });
    await page.getByRole('button', { name: 'Unlock stops', exact: true }).waitFor();
    await upkeep.click();
    assert.equal(await upkeep.getAttribute('data-stop'), originalStop, 'locked stops ignore edits');
    await page.getByRole('button', { name: 'Unlock stops', exact: true }).click();
    await upkeep.click();
    assert.notEqual(await upkeep.getAttribute('data-stop'), originalStop, 'unlocked stops accept edits');
    await load(priority(1));
    await page.waitForTimeout(2200);
    assert.equal(await page.evaluate(()=>window.submissions.length),0,'main stop has no timeout');
    assert.equal(await page.evaluate(()=>window.context.postActionPriorityWindow),null);
    await load(priority(10,{active_player:1,phase:'Beginning',step:'Upkeep'}));
    await page.waitForFunction(()=>window.submissions.length===1);
    await load(priority(20,{stack_size:1,stack_objects:[{id:50,controller:1,name:'Opponent spell'}]}));
    await page.waitForTimeout(250);
    assert.equal(await page.evaluate(()=>window.submissions.length),0,'opponent spell always gets a response');
    await load(priority(30));
    await page.keyboard.down('Control');
    await page.evaluate(()=>window.context.dispatch({type:'priority_action',action_ref:{kind:'cast_spell',spell_id:7,casting_method:{kind:'normal'}}}));
    await page.keyboard.up('Control');
    await page.waitForTimeout(300);
    assert.equal(await page.evaluate(()=>window.submissions.length),1,'Ctrl retains priority after casting');
    await load(priority(40));
    await page.getByRole('button',{name:'Cast',exact:true}).click();
    await page.waitForFunction(()=>window.submissions.length===2);
    assert.equal(await page.evaluate(()=>window.submissions[1].command.action_ref.kind),'pass_priority');
    await load(priority(50));
    await page.getByRole('button',{name:'Full control',exact:true}).click();
    await page.evaluate(()=>window.publish({...window.currentState,snapshot_id:51,phase:'Beginning',step:'Upkeep'}));
    await page.waitForTimeout(250);
    assert.equal(await page.evaluate(()=>window.submissions.length),0);
    await load(priority(60,{active_player:1,decision:{kind:'select_objects',player:1,min:1,max:1,candidates:[{id:5},{id:6}]}}),true);
    await page.evaluate(()=>{window.context.setAutoPassEnabled(false);window.context.toggleAutomatedSeat(1);});
    await page.waitForFunction(()=>window.submissions.length===1);
    assert.deepEqual(await page.evaluate(()=>window.submissions[0].command),{type:'select_objects',object_ids:[5]});
    assert.deepEqual(errors,[]);
  } finally { await browser.close(); await server.close(); }
});
