import test from "node:test";
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { createServer } from "vite";

const fixture = `import React from 'react';
import '/src/index.css';
import {createRoot} from 'react-dom/client';
import {GameProvider,useGame} from '/src/context/GameContext.jsx';
import {LoopShortcutDialogs} from '/src/components/decisions/LoopShortcutControls.jsx';
function Probe(){const c=useGame();window.context=c;return React.createElement(React.Fragment,null,React.createElement('output',null,c.state?.snapshot_id),React.createElement(LoopShortcutDialogs));}
createRoot(document.getElementById('root')).render(React.createElement(GameProvider,null,React.createElement(Probe)));`;
const peer = `import {useEffect} from 'react';
const multiplayer={matchStarted:true,role:'client',submittingAction:false};
window.submissions=[];
export function usePeerLobby({setState}) {
useEffect(()=>{window.publish=state=>{window.currentState=state;setState(state);};},[setState]);
return {multiplayer,submitMultiplayerCommand:async command=>{
window.submissions.push(command);
const s=window.currentState;
let shortcut=s.loop_shortcut;
if(command.type==='propose_loop_shortcut') shortcut={proposer:0,steps:command.steps,proposed:command.iterations,agreed:command.iterations,running:true,awaiting_player:null};
if(command.type==='cancel_loop_shortcut') shortcut=null;
window.publish({...s,snapshot_id:s.snapshot_id+1,loop_shortcut:shortcut});
}};}`;
const wasm = `const game={setAutoCleanupDiscard:async()=>{}};export function useWasmGame(){return {game,loading:false};}`;

test("proposing a recorded loop replays every iteration through the provider's interaction gate", {timeout:60000}, async () => {
  const server = await createServer({root:fileURLToPath(new URL('..',import.meta.url)),logLevel:'error',
    server:{host:'127.0.0.1',port:0,hmr:false},plugins:[{
      name:'loop-fixture',
      transform(code,id){if(id.endsWith('/hooks/usePeerLobby.js'))return peer;if(id.endsWith('/hooks/useWasmGame.js'))return wasm;},
      configureServer(server){server.middlewares.use((req,res,next)=>{if(req.url==='/loop.html'){res.setHeader('Content-Type','text/html');res.end('<div id="root"></div><script type="module" src="/loop-fixture.jsx"></script>');}else next();});},
      resolveId(id){if(id==='/loop-fixture.jsx')return '\0loop-fixture.jsx';},
      load(id){if(id==='\0loop-fixture.jsx')return fixture;},
    }]});
  await server.listen();
  const browser=await chromium.launch();
  try {
    const page=await browser.newPage();
    page.setDefaultTimeout(15000);
    const errors=[];page.on('pageerror',error=>errors.push(error.message));
    await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/loop.html`,{waitUntil:"domcontentloaded"});
    await page.waitForFunction(()=>window.context&&window.publish);
    await page.evaluate(()=>{
      window.context.setAutoPassEnabled(false);
      window.publish({snapshot_id:1,turn_number:1,active_player:0,perspective:0,priority_player:0,
        phase:'first main phase',stack_size:0,pass_intents:[],loop_shortcut:null,
        decision:{kind:'priority',player:0,analysis_complete:true,actions:[{index:0,kind:'pass_priority',label:'Pass priority',action_ref:{kind:'pass_priority'}}]}});
    });
    await page.waitForFunction(()=>window.context.state?.snapshot_id===1);
    await page.evaluate(()=>window.context.loopShortcut.startRecording());
    await page.evaluate(()=>window.context.dispatch({type:'priority_action',action_index:0,action_ref:{kind:'pass_priority'}}));
    await page.waitForFunction(()=>window.context.loopShortcut.recordedStepCount===1);
    await page.evaluate(()=>window.context.loopShortcut.stopRecording());
    await page.waitForFunction(()=>window.context.loopShortcut.proposalDraft);
    await page.waitForTimeout(150); // The proposal is a fresh user click.
    await page.evaluate(()=>window.context.loopShortcut.propose(3));
    await page.waitForFunction(()=>window.submissions.some(c=>c.type==='cancel_loop_shortcut'),null,{timeout:3000});
    assert.deepEqual(await page.evaluate(()=>window.submissions.map(c=>c.type)),[
      'priority_action','propose_loop_shortcut','priority_action','priority_action','priority_action','cancel_loop_shortcut',
    ]);
    await page.evaluate(()=>window.publish({...window.currentState,snapshot_id:window.currentState.snapshot_id+1,
      perspective:1,players:[{id:0,name:'Alice'},{id:1,name:'Bob'}],
      loop_shortcut:{proposer:0,steps:[{type:'priority_action',kind:'priority',action:{kind:'pass_priority'}}],
        proposed:5,agreed:5,running:false,awaiting_player:1}}));
    await page.getByText('Interrupt on iteration',{exact:true}).waitFor();
    await page.getByRole('spinbutton').fill('2');
    await page.getByRole('button',{name:'Interrupt on iteration 2',exact:true}).click();
    await page.waitForFunction(()=>window.submissions.some(c=>c.type==='respond_loop_shortcut'));
    assert.deepEqual(await page.evaluate(()=>window.submissions.find(c=>c.type==='respond_loop_shortcut')),
      {type:'respond_loop_shortcut',player:1,iterations:5,interrupt_at:2});
    assert.deepEqual(errors,[]);
  } finally {await browser.close();await server.close();}
});
