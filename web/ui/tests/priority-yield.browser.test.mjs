import test from 'node:test';
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { createServer } from 'vite';

const fixture = `import React from 'react';
import {createRoot} from 'react-dom/client';
import {GameProvider,useGame} from '/src/context/GameContext.jsx';
import {I18nProvider} from '/src/i18n/I18nContext.jsx';
function Probe(){const c=useGame();window.context=c;return React.createElement('div',null,
React.createElement('button',{onClick:c.startResolveAll},'Resolve all'),
React.createElement('button',{onClick:c.startPhaseAdvance},c.phasePassing?'Pause':'Advance'),
React.createElement('span',null,c.state?.snapshot_id));}
createRoot(document.getElementById('root')).render(React.createElement(I18nProvider,null,React.createElement(GameProvider,null,React.createElement(Probe))));`;
const peer = `import {useEffect} from 'react';
const multiplayer={matchStarted:true,role:'client',submittingAction:false};
window.submissions=[];
export function usePeerLobby({setState}) {
useEffect(()=>{window.publish=state=>{window.currentState=state;setState(state);};},[setState]);
return {multiplayer,submitMultiplayerCommand:async command=>{
window.submissions.push(command);
const s=window.currentState;
window.publish({...s,snapshot_id:s.snapshot_id+1,pass_intents:command.type==='cancel_priority_yield'?[]:[0],
priority_player:1,decision:{...s.decision,player:1}});
}};}`;
const wasm = `const game={setAutoCleanupDiscard:async()=>{}};export function useWasmGame(){return {game,loading:false};}`;
const priority = (id, extra = {}) => ({ snapshot_id:id, turn_number:1, active_player:0, perspective:0,
  priority_player:0, phase:'first main phase', step:null, stack_size:0, stack_objects:[], pass_intents:[],
  decision:{kind:'priority',player:0,analysis_complete:true,actions:[{index:0,kind:'pass_priority',label:'Pass priority',action_ref:{kind:'pass_priority'}}]}, ...extra });

test('provider sends one intention, waits for peers, and cancels phase passing out of turn', {timeout:60000}, async () => {
  const server = await createServer({root:fileURLToPath(new URL('..',import.meta.url)),logLevel:'error',
    server:{host:'127.0.0.1',port:0,hmr:false},plugins:[{
      name:'yield-fixture',
      transform(code,id){if(id.endsWith('/hooks/usePeerLobby.js'))return peer;if(id.endsWith('/hooks/useWasmGame.js'))return wasm;},
      configureServer(server){server.middlewares.use((req,res,next)=>{if(req.url==='/yield.html'){res.setHeader('Content-Type','text/html');res.end('<div id="root"></div><script type="module" src="/yield-fixture.jsx"></script>');}else next();});},
      resolveId(id){if(id==='/yield-fixture.jsx')return '\0yield-fixture.jsx';},
      load(id){if(id==='\0yield-fixture.jsx')return fixture;},
    }]});
  await server.listen();
  const browser=await chromium.launch();
  try {
    const page=await browser.newPage();
    const errors=[];page.on('pageerror',error=>errors.push(error.message));
    await page.addInitScript(()=>localStorage.setItem('ironsmith.priorityStops',JSON.stringify({version:3,stops:{'phase:Main2':'always','phase:Upkeep':'off'}})));
    const load=async state=>{
      await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/yield.html`);
      await page.waitForFunction(()=>window.context&&window.publish);
      await page.evaluate(state=>window.publish(state),state);
      await page.waitForFunction(id=>window.context.state?.snapshot_id===id,state.snapshot_id);
    };
    await load(priority(1,{stack_size:100,stack_objects:Array.from({length:100},(_,id)=>({id,controller:0}))}));
    await page.getByRole('button',{name:'Resolve all',exact:true}).click();
    await page.waitForFunction(()=>window.submissions.length===1);
    await page.waitForTimeout(250);
    assert.deepEqual(await page.evaluate(()=>window.submissions),[{type:'priority_yield',mode:'stack',stops:['phase:Draw','phase:Main','phase:Combat','phase:Main2','phase:End','phase:Cleanup',
      'step:BeginCombat','step:DeclareAttackers','step:DeclareBlockers','step:FirstStrikeDamage','step:CombatDamage','step:EndCombat']}]);
    await page.evaluate(()=>window.publish({...window.currentState,snapshot_id:3,pass_intents:[],stack_objects:[{id:999,controller:0}],stack_size:1,
      priority_player:0,decision:{...window.currentState.decision,player:0}}));
    await page.waitForTimeout(250);
    assert.equal(await page.evaluate(()=>window.submissions.length),1,'a new trigger does not restart the run');

    // Double-click Pay: this seat's own additions join the run; an opponent's pauses it.
    await load(priority(20,{stack_size:1,stack_objects:[{id:5,controller:0}]}));
    await page.evaluate(()=>window.context.startResolveAll({continueThroughOwn:true}));
    await page.waitForFunction(()=>window.submissions.length===1);
    const back=(id,stack)=>page.evaluate(([id,stack])=>window.publish({...window.currentState,snapshot_id:id,pass_intents:[],
      stack_objects:stack,stack_size:stack.length,priority_player:0,decision:{...window.currentState.decision,player:0}}),[id,stack]);
    await back(30,[{id:6,controller:0},{id:5,controller:0}]);
    await page.waitForFunction(()=>window.submissions.length===2);
    assert.equal(await page.evaluate(()=>window.submissions[1].type),'priority_yield','own trigger resumes the run');
    await back(40,[{id:7,controller:1},{id:6,controller:0},{id:5,controller:0}]);
    await page.waitForTimeout(250);
    assert.equal(await page.evaluate(()=>window.submissions.length),2,"an opponent's response pauses it");

    await load(priority(10));
    await page.getByRole('button',{name:'Advance',exact:true}).click();
    await page.waitForFunction(()=>window.submissions.length===1&&window.context.phasePassing);
    assert.equal(await page.evaluate(()=>window.submissions[0].mode),'phases');
    await page.getByRole('button',{name:'Pause',exact:true}).click();
    await page.waitForFunction(()=>window.submissions.length===2);
    assert.deepEqual(await page.evaluate(()=>window.submissions[1]),{type:'cancel_priority_yield',player:0});
    await page.waitForFunction(()=>!window.context.phasePassing);
    assert.deepEqual(errors,[]);
  } finally {await browser.close();await server.close();}
});
