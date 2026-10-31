import test from 'node:test';
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { createServer } from 'vite';

const fixture = `import React from 'react';
import {createRoot} from 'react-dom/client';
import {GameProvider,useGame} from '/src/context/GameContext.jsx';
import {I18nProvider} from '/src/i18n/I18nContext.jsx';
import DecisionPopupLayer from '/src/components/overlays/DecisionPopupLayer.jsx';
import {HoverProvider} from '/src/context/HoverContext.jsx';
import {TooltipProvider} from '/src/components/ui/tooltip.jsx';
import '/src/index.css';
function Probe(){const c=useGame();window.context=c;
if(new URLSearchParams(location.search).has('ui') && c.state?.decision) return React.createElement(HoverProvider,null,React.createElement(TooltipProvider,null,React.createElement('div',{style:{height:80,width:700}},React.createElement(DecisionPopupLayer,{priorityInline:true}))));
return React.createElement('div',null,
React.createElement('button',{className:'decision-main-button','data-secondary-priority-action':'true',onClick:c.startResolveAll},'Resolve all'),
React.createElement('button',{className:'decision-main-button',onClick:c.startPhaseAdvance},'Resolve'),
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

test('provider validates each automatic pass, stops for new stack entries and preserves required choices', {timeout:60000}, async () => {
  const server = await createServer({root:fileURLToPath(new URL('..',import.meta.url)),logLevel:'error',
    server:{host:'127.0.0.1',port:0,hmr:false},plugins:[{
      name:'yield-fixture',
      transform(code,id){if(id.endsWith('/hooks/usePeerLobby.js'))return peer;if(id.endsWith('/hooks/useWasmGame.js'))return wasm;},
      configureServer(server){server.middlewares.use((req,res,next)=>{if(req.url.split('?')[0]==='/yield.html'){res.setHeader('Content-Type','text/html');res.end('<div id="root"></div><script type="module" src="/yield-fixture.jsx"></script>');}else next();});},
      resolveId(id){if(id==='/yield-fixture.jsx')return '\0yield-fixture.jsx';},
      load(id){if(id==='\0yield-fixture.jsx')return fixture;},
    }]});
  await server.listen();
  const browser=await chromium.launch();
  try {
    const page=await browser.newPage();
    const errors=[];page.on('pageerror',error=>errors.push(error.message));
    await page.addInitScript(()=>localStorage.setItem('ironsmith.priorityStops',JSON.stringify({version:3,stops:{'phase:Main2':'always','phase:Upkeep':'off'}})));
    const load=async (state,ui=false)=>{
      await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/yield.html${ui?'?ui':''}`);
      await page.waitForFunction(()=>window.context&&window.publish);
      await page.evaluate(state=>window.publish(state),state);
      await page.waitForFunction(id=>window.context.state?.snapshot_id>=id,state.snapshot_id).catch(error=>{throw new Error(`${error.message}; page errors: ${errors.join("; ")}`);});
    };
    await load(priority(1,{stack_size:2,stack_objects:[{id:5,controller:1},{id:4,controller:1}]}));
    await page.getByRole('button',{name:'Resolve all',exact:true}).click();
    await page.waitForFunction(()=>window.submissions.length===1);
    assert.equal(await page.evaluate(()=>window.submissions[0].action_ref.kind),'pass_priority');
    await page.evaluate(()=>window.publish({...window.currentState,snapshot_id:3,stack_objects:[{id:999,controller:1}],stack_size:1,
      decision:{...window.currentState.decision,player:0}}));
    await page.waitForTimeout(250);
    assert.equal(await page.evaluate(()=>window.submissions.length),1,'new stack object stops a stack yield');
    await load(priority(10));
    await page.evaluate(()=>window.context.startPriorityYield('turn'));
    await page.waitForFunction(()=>window.submissions.length===1);
    await page.evaluate(()=>window.publish({...window.currentState,snapshot_id:12,decision:{kind:'select_objects',player:0,min:1,max:1,candidates:[{id:4},{id:5}]}}));
    await page.waitForTimeout(250);
    assert.equal(await page.evaluate(()=>window.submissions.length),1,'yield does not choose a discard');
    await page.evaluate(state=>window.publish(state),priority(13));
    await page.waitForFunction(()=>window.submissions.length===2);
    await page.evaluate(()=>window.context.clearPriorityYields());
    assert.equal(await page.evaluate(()=>window.context.priorityStatus.yield),null);
    await load(priority(20));
    await page.evaluate(()=>window.context.startPriorityYield('no_play'));
    await page.waitForTimeout(250);
    assert.equal(await page.evaluate(()=>window.submissions.length),0,'unknown legal analysis must wait');
    await page.evaluate(()=>window.publish({...window.currentState,snapshot_id:21,decision:{...window.currentState.decision,can_act:false}}));
    await page.waitForFunction(()=>window.submissions.length===1);
    const trigger = id => ({id,controller:1,source_stable_id:id*10,inspect_object_id:id*10,ability_kind:'Triggered',automation_identity:'trigger:surveil'});
    await load(priority(30,{stack_size:4,stack_objects:[trigger(4),trigger(3),trigger(2),trigger(1)]}));
    await page.keyboard.press('1');
    await page.waitForFunction(()=>window.submissions.length===1);
    assert.equal(await page.evaluate(()=>window.context.priorityPolicy.status(window.currentState).yield),null,'1 resolves once, not Resolve all');
    await load(priority(40,{stack_size:4,stack_objects:[trigger(4),trigger(3),trigger(2),trigger(1)]}));
    await page.keyboard.press('7');
    await page.waitForFunction(()=>window.submissions.length===1);
    assert.ok(await page.evaluate(()=>window.context.priorityPolicy.status(window.currentState).resolveGroup));
    const choice = (snapshot,id,card) => priority(snapshot,{stack_size:id-1,stack_objects:[3,2,1].filter(x=>x<id).map(trigger),resolving_stack_object:trigger(id),
      decision:{kind:'select_objects',player:0,source_id:id*10,source_name:`Source ${id}`,description:'Surveil 1',min:0,max:1,candidates:[{id:card,name:'Island',legal:true}]}});
    await page.evaluate(s=>window.publish(s),choice(42,4,100));
    await page.waitForFunction(()=>window.context.state.snapshot_id===42);
    await page.evaluate(()=>window.context.dispatch({type:'select_objects',object_ids:[100]}));
    await page.waitForFunction(()=>window.submissions.length===2);
    await page.evaluate(s=>window.publish(s),choice(44,3,101));
    await page.waitForTimeout(250);
    assert.equal(await page.evaluate(()=>window.submissions.length),2,'milling exposes a different card and pauses');
    await page.evaluate(()=>window.context.dispatch({type:'select_objects',object_ids:[]}));
    await page.waitForFunction(()=>window.submissions.length===3);
    await page.evaluate(s=>window.publish(s),choice(46,2,101));
    await page.waitForFunction(()=>window.submissions.length===4);
    assert.deepEqual(await page.evaluate(()=>window.submissions[3].object_ids),[]);
    await page.evaluate(s=>window.publish(s),choice(48,1,101));
    await page.waitForFunction(()=>window.submissions.length===5);
    assert.deepEqual(await page.evaluate(()=>window.submissions[4].object_ids),[],'keeping the same card cascades across different sources');
    await load(priority(60,{players:[],stack_size:2,stack_objects:[trigger(2),trigger(1)]}),true);
    const groupButton=page.getByRole('button',{name:'Resolve group',exact:true});
    await groupButton.waitFor();
    assert.equal(await page.getByRole('button',{name:'Resolve all',exact:true}).count(),0);
    assert.equal(await page.locator('.action-strip-resolve-all-button').getAttribute('data-secondary-priority-action'),'true');
    assert.ok((await page.locator('.priority-shortcut-hint').allTextContents()).includes('7'));
    await groupButton.click();
    await page.waitForFunction(()=>window.submissions.length===1);
    await load(priority(70,{players:[],stack_size:2,stack_objects:[trigger(2),{...trigger(1),automation_identity:'different'}]}),true);
    await page.getByRole('button',{name:'Resolve all',exact:true}).waitFor();
    assert.deepEqual(await page.locator('.priority-shortcut-hint').allTextContents(),['1'],'only Resolve owns 1 when Resolve all is visible');
    await page.keyboard.press('1');
    await page.waitForFunction(()=>window.submissions.length===1);
    assert.equal(await page.evaluate(()=>window.context.priorityPolicy.status(window.currentState).yield),null);
    assert.deepEqual(errors,[]);
  } finally {await browser.close();await server.close();}
});
