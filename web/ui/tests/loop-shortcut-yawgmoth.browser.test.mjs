import test from "node:test";
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { createServer } from "vite";

const wasmUrl = `/@fs/${fileURLToPath(new URL('../../wasm_demo/pkg/ironsmith.js', import.meta.url))}`;
const wasmMock = `import {loopObjectIndex} from '/src/lib/loop-shortcut.js';
import init,{WasmGame} from '${wasmUrl}';
await init({verifier:false});
const game=new WasmGame();
const sources=await Promise.all(['yawgmoth-thran-physician','butcher-ghoul','swamp'].map(async n=>(await fetch('/cards/'+n+'.json')).json()));
game.registerExternalCardSourcesJson(JSON.stringify(sources));
game.resetEmpty(['Alice','Bob'],100);
game.addCardToZone(1,'Swamp','battlefield',true);
game.addCardToZone(0,'Yawgmoth, Thran Physician','battlefield',true);
const ghouls=[0,1].map(()=>Number(game.addCardToZone(0,'Butcher Ghoul','battlefield',true)));
for(let i=0;i<40;i++)game.addCardToZone(0,'Swamp','library',true);
game.finishPuzzleSetup();
let initial=game.uiState();
while(!initial.decision?.actions?.some(a=>a.action_ref?.kind==='activate_ability')) {
 const a=initial.decision.actions.find(a=>['keep_opening_hand','continue_pregame','begin_game','pass_priority'].includes(a.action_ref?.kind));
 initial=game.dispatch({type:'priority_action',action_ref:a.action_ref});
}

window.engine=game;window.initial=initial;window.ghouls=ghouls;window.commands=[];window.targetChoices=[];
for(const method of ['dispatch','dispatchForAutomation']){
 const original=game[method].bind(game);
 game[method]=command=>{
  if(command.type==='respond_loop_shortcut'&&new URLSearchParams(location.search).has('interrupt'))command={...command,interrupt_at:Number(new URLSearchParams(location.search).get('interrupt'))};
  if(command.type==='select_targets'&&game.uiState().loop_shortcut?.running){
   const index=loopObjectIndex(game.uiState());
   window.targetChoices.push(command.targets.map(t=>index.get(Number(t.object))?.stable_id));
  }
  window.commands.push(command);return original(command);
 };
}
const asyncGame=new Proxy(game,{get(target,key){const value=target[key];return typeof value==='function'?async (...args)=>value.apply(target,args):value;}});
export function useWasmGame(){return {game:asyncGame,loading:false};}`;
const peerMock = `const multiplayer={matchStarted:false};export function usePeerLobby(){return {multiplayer};}`;
const fixture = `import React,{useEffect,useLayoutEffect} from 'react';
import {createRoot} from 'react-dom/client';
import {GameProvider,useGame} from '/src/context/GameContext.jsx';
import DecisionPopupLayer from '/src/components/overlays/DecisionPopupLayer.jsx';
import {loopObjectIndex} from '/src/lib/loop-shortcut.js';
function Probe(){
 const c=useGame();
 useEffect(()=>{c.setAutoResolveMode('all');c.setState(window.initial);},[]);
 window.loop=c.loopShortcut;window.state=c.state;window.send=c.dispatch;window.setMode=c.setAutoResolveMode;window.setAutoPass=c.setAutoPassEnabled;window.setHold=c.setHoldRule;
 window.targetViews=window.targetViews||[];
 if(c.state?.loop_shortcut?.running&&c.state?.decision?.kind==='targets')window.targetViews.push(c.loopShortcut.replayingTargets);
 useLayoutEffect(()=>{if(c.state?.loop_shortcut?.running&&c.state?.decision?.kind==='targets')window.targetViews.push(Boolean(document.querySelector('[data-loop-controls]')));});
 window.messages=window.messages||[];window.messages.push(c.status?.msg||c.status);
 window.ghoul=stable=>[...loopObjectIndex(window.state)].find(([,o])=>o.stable_id===stable)?.[0];
 window.activate=()=>c.dispatch({type:'priority_action',action_ref:window.state.decision.actions.find(a=>a.action_ref?.kind==='activate_ability').action_ref});
 return React.createElement(React.Fragment,null,
  React.createElement('output',null,c.state?.snapshot_id),
  c.state?.loop_shortcut?.running&&c.state?.decision?.kind==='targets'
   ?React.createElement(DecisionPopupLayer,{quickControls:React.createElement('span',{'data-loop-controls':true},'Loop running')})
   :null);
}
createRoot(document.getElementById('root')).render(React.createElement(GameProvider,null,React.createElement(Probe)));`;

for (const scenario of [
 {name:'accepted Yawgmoth loop runs with auto-resolve off',interrupt:null},
 {name:'Yawgmoth interruption on iteration 1 allows a different response',interrupt:1},
 {name:'Yawgmoth interruption on iteration 2 passes each remaining step manually',interrupt:2},
]) test(scenario.name, {timeout:90000}, async()=>{
 const server=await createServer({root:fileURLToPath(new URL('..',import.meta.url)),logLevel:'error',server:{host:'127.0.0.1',port:0,hmr:false},plugins:[{
  name:'yawgmoth-loop-fixture',
  transform(code,id){if(id.endsWith('/hooks/useWasmGame.js'))return wasmMock;if(id.endsWith('/hooks/usePeerLobby.js'))return peerMock;},
  configureServer(server){server.middlewares.use((req,res,next)=>{if(req.url.split('?')[0]==='/yawg.html'){res.setHeader('Content-Type','text/html');res.end('<div id="root"></div><script type="module" src="/yawg.jsx"></script>');}else next();});},
  resolveId(id){if(id==='/yawg.jsx')return '\0yawg.jsx';},load(id){if(id==='\0yawg.jsx')return fixture;},
 }]});
 await server.listen();const browser=await chromium.launch();
 try{
  const page=await browser.newPage();
  page.setDefaultTimeout(30000);
  const errors=[];
  page.on('pageerror',e=>errors.push(e.message));
  page.on('console',m=>{if(m.type()==='error')errors.push(m.text());});
  await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/yawg.html${scenario.interrupt ? `?interrupt=${scenario.interrupt}` : ''}`);
  await page.waitForFunction(()=>window.loop&&window.state);
  // Seed one undying counter outside the recording, as in the reported combo.
  await page.waitForTimeout(150);
  await page.evaluate(()=>window.activate());
  await page.waitForTimeout(150);
  await page.evaluate(()=>window.send({type:'select_targets',targets:[]}));
  await page.waitForTimeout(150);
  await page.evaluate(()=>window.send({type:'select_objects',object_ids:[window.ghoul(window.ghouls[0])]}));
  await page.waitForTimeout(150);
  await page.waitForFunction(()=>window.state.stack_size===0);
  await page.evaluate(()=>window.loop.startRecording());
  for(const [target,sacrifice] of [[0,1],[1,0]]){
   await page.waitForTimeout(150);
   await page.evaluate(()=>window.activate());
   await page.waitForTimeout(150);
   await page.evaluate(i=>window.send({type:'select_targets',targets:[{kind:'object',object:window.ghoul(window.ghouls[i])}]}),target);
   await page.waitForTimeout(150);
   await page.evaluate(i=>window.send({type:'select_objects',object_ids:[window.ghoul(window.ghouls[i])]}),sacrifice);
   await page.waitForTimeout(150);
   await page.waitForFunction(()=>window.state.stack_size===0);
  }
  await page.evaluate(()=>window.loop.stopRecording());
  const steps=await page.evaluate(()=>window.loop.proposalDraft.steps);
  assert.equal(steps.filter(s=>s.action?.kind==='pass_priority').length,4,'each undying trigger and Yawgmoth ability needs a recorded pass');
  assert.equal(steps.some(s=>s.type==='resolve_stack'),false,'this test must use timed passes, never Resolve all');
  assert.deepEqual(steps.filter(s=>s.type==='select_targets').map(s=>s.targets[0].object.stable_id),await page.evaluate(()=>window.ghouls));
  await page.waitForTimeout(150);
  await page.evaluate(interrupt=>{window.setMode(interrupt ? 'all' : 'off');if(!interrupt){window.setAutoPass(false);window.setHold('always');}},scenario.interrupt);
  await page.waitForTimeout(150);
  await page.evaluate(()=>window.loop.propose(3));
  if(scenario.interrupt) {
   await page.waitForFunction(()=>window.state.loop_shortcut?.stepping&&window.state.decision?.player===1);
   const boundary=await page.evaluate(()=>({life:window.state.players[0].life,library:window.state.players[0].library_size,stack:window.state.stack_size,snapshot:window.state.snapshot_id}));
   assert.equal(boundary.life,98-2*scenario.interrupt,'previous iterations finish before manual responses');
   assert.equal(boundary.library,39-2*scenario.interrupt);
   assert.equal(boundary.stack,2);
   await page.waitForTimeout(2200); // Longer than the ordinary resolve-all timer.
   assert.equal(await page.evaluate(()=>window.state.snapshot_id),boundary.snapshot,'auto-resolve all must not skip the response window');
   assert.equal(await page.evaluate(()=>{try{window.engine.dispatch({type:'priority_action',action_index:99999});return false;}catch{return Boolean(window.engine.uiState().loop_shortcut?.stepping);}}),true,'a rejected response preserves the agreed shortcut');
   await page.evaluate(()=>window.send({type:'priority_action',action_ref:{kind:'pass_priority'}}));
   await page.waitForFunction(()=>window.state.decision?.player===1&&window.state.stack_size===1);
   assert.equal(await page.evaluate(()=>window.state.players[0].library_size),boundary.library,'only undying resolved');
   if(scenario.interrupt===1) {
    await page.waitForTimeout(150);
    await page.evaluate(()=>window.send({type:'priority_action',action_ref:window.state.decision.actions.find(a=>a.action_ref?.kind==='activate_mana_ability').action_ref}));
    await page.waitForFunction(()=>!window.state.loop_shortcut);
    assert.equal(await page.evaluate(()=>window.state.players[0].life),boundary.life,'opponent response prevents further loop actions');
    assert.equal(await page.evaluate(()=>window.state.stack_size),1,'opponent may respond before Yawgmoth resolves');
   }else{
    for(let i=0;i<3;i++){
     await page.waitForTimeout(150);
     await page.evaluate(()=>window.send({type:'priority_action',action_ref:{kind:'pass_priority'}}));
     if(i<2)await page.waitForFunction(()=>window.state.decision?.player===1);
    }
    await page.waitForFunction(()=>!window.state.loop_shortcut);
    assert.equal(await page.evaluate(()=>window.state.players[0].life),93);
    assert.equal(await page.evaluate(()=>window.state.players[0].library_size),33);
    assert.equal(await page.evaluate(()=>window.state.stack_size),0);
    assert.ok(await page.evaluate(()=>window.messages.includes('Loop paused after iteration 2')));
   }
   assert.deepEqual(errors,[]);
   return;
  }
  await page.waitForFunction(()=>!window.state.loop_shortcut,null,{timeout:10000}).catch(async error=>{
   throw new Error(error.message+' '+JSON.stringify(await page.evaluate(()=>({messages:window.messages,decision:window.state.decision,commands:window.commands.slice(-6)}))));
  });
  assert.ok(await page.evaluate(()=>window.messages.includes('Loop finished: repeated 3 time(s)')),JSON.stringify(await page.evaluate(()=>window.messages)));
  assert.equal(await page.evaluate(()=>window.state.players[0].life),91);
  assert.equal(await page.evaluate(()=>window.state.players[0].library_size),31);
  assert.equal(await page.evaluate(()=>window.state.stack_size),0);
  assert.deepEqual(await page.evaluate(()=>window.state.players[0].battlefield
    .filter(card=>card.name==='Butcher Ghoul').map(card=>card.stable_id).sort()),
  await page.evaluate(()=>window.ghouls.slice().sort()));
  assert.deepEqual(await page.evaluate(()=>window.targetChoices),await page.evaluate(()=>Array.from({length:3},()=>window.ghouls.map(id=>[id])).flat()));
  assert.ok(await page.evaluate(()=>window.targetViews.length>0&&window.targetViews.every(Boolean)),"saved targets must not request manual input during replay");
  assert.deepEqual(errors,[]);
 }finally{await browser.close();await server.close();}
});
