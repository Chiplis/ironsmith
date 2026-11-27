import test from "node:test";
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { createServer } from "vite";

// The engine side is mocked: after every accepted activation it reports the
// loop it "detected" (window.offerSummary) the way the web session does, with
// `start_serial` naming the snapshot the loop's first command was sent against.
const fixture = `import React from 'react';
import '/src/index.css';
import {createRoot} from 'react-dom/client';
import {GameProvider,useGame} from '/src/context/GameContext.jsx';
import {LoopShortcutButton,LoopShortcutDialogs} from '/src/components/decisions/LoopShortcutControls.jsx';
function Probe(){const c=useGame();window.context=c;return React.createElement(React.Fragment,null,React.createElement('output',null,c.state?.snapshot_id),React.createElement(LoopShortcutButton),React.createElement(LoopShortcutDialogs));}
createRoot(document.getElementById('root')).render(React.createElement(GameProvider,null,React.createElement(Probe)));`;
const peer = `import {useEffect} from 'react';
const multiplayer={matchStarted:true,role:'client',submittingAction:false};
window.submissions=[];
window.statuses=[];
export function usePeerLobby({setState}) {
useEffect(()=>{window.publish=state=>{window.currentState=state;setState(state);};},[setState]);
return {multiplayer,submitMultiplayerCommand:async command=>{
window.submissions.push(command);
const s=window.currentState;
let shortcut=s.loop_shortcut;
let offer=s.loop_offer;
let serial=s.loop_serial;
if(command.type==='propose_loop_shortcut') shortcut={proposer:0,steps:command.steps,proposed:command.iterations,agreed:command.iterations,running:true,awaiting_player:null};
if(command.type==='cancel_loop_shortcut') shortcut=null;
if(command.type==='priority_action'){
  const summary=typeof window.offerSummary==='function'?window.offerSummary():window.offerSummary;
  offer=summary?{proposer:0,start_serial:serial,summary,per_lap:[],responders:[]}:null;
  serial+=1;
}
window.publish({...s,snapshot_id:s.snapshot_id+1,loop_shortcut:shortcut,loop_offer:offer,loop_serial:serial});
}};}`;
const wasm = `const game={setAutoCleanupDiscard:async()=>{}};export function useWasmGame(){return {game,loading:false};}`;

const activate = { index: 1, kind: "activate_mana_ability", label: "Activate Life engine", object_id: 10, ability_index: 0,
  action_ref: { kind: "activate_mana_ability", source: 10, ability_index: 0 } };

async function open(run) {
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
    // A cold Vite start can take minutes on a busy machine.
    await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/loop.html`,{waitUntil:"domcontentloaded",timeout:300000});
    await page.waitForFunction(()=>window.context&&window.publish,null,{timeout:300000});
    await page.evaluate((action)=>{
      window.context.setAutoPassEnabled(false);
      window.offerSummary='+1 red mana (Alice)';
      window.publish({snapshot_id:1,turn_number:3,active_player:0,perspective:0,priority_player:0,
        phase:'first main phase',stack_size:0,pass_intents:[],loop_shortcut:null,loop_offer:null,loop_serial:7,
        players:[{id:0,name:'Alice',battlefield:[{id:10,stable_id:900,name:'Life engine'}],hand_cards:[]},{id:1,name:'Bob',battlefield:[],hand_cards:[]}],
        decision:{kind:'priority',player:0,analysis_complete:true,actions:[
          {index:0,kind:'pass_priority',label:'Pass priority',action_ref:{kind:'pass_priority'}},action]}});
    }, activate);
    await page.waitForFunction(()=>window.context.state?.snapshot_id===1);
    await run(page);
    assert.deepEqual(errors,[]);
  } finally {await browser.close();await server.close();}
}

test("a loop the engine detects is offered, proposed from the logged commands, and replayed", {timeout:900000}, async () => {
  await open(async (page) => {
    assert.equal(await page.evaluate(()=>window.context.loopShortcut.detectedLoop), null, "nothing repeated yet");
    assert.equal(await page.getByRole('button',{name:/Repeat loop/}).count(), 0);
    // The player performs the loop once, by hand.
    await page.evaluate((action)=>window.context.dispatch({type:'priority_action',action_index:1,action_ref:action.action_ref}), activate);
    await page.waitForFunction(()=>window.context.loopShortcut.detectedLoop);
    const detected = await page.evaluate(()=>window.context.loopShortcut.detectedLoop);
    assert.equal(detected.summary, '+1 red mana (Alice)');
    assert.equal(detected.steps.length, 1);
    assert.deepEqual(detected.steps[0].action.object, {stable_id:900,name:'Life engine'}, "the step names the card, not its ObjectId");
    await page.waitForTimeout(150);
    await page.getByRole('button',{name:'Repeat loop (+1 red mana (Alice))',exact:true}).click();
    await page.getByText('Repeat this loop',{exact:true}).waitFor();
    await page.getByText(/Each repetition: \+1 red mana \(Alice\)\./).waitFor();
    await page.getByRole('spinbutton').fill('3');
    await page.getByRole('button',{name:'Propose ×3',exact:true}).click();
    await page.waitForFunction(()=>window.submissions.some(c=>c.type==='cancel_loop_shortcut'),null,{timeout:10000});
    assert.deepEqual(await page.evaluate(()=>window.submissions.map(c=>c.type)),[
      'priority_action','propose_loop_shortcut','priority_action','priority_action','priority_action','cancel_loop_shortcut',
    ]);
    // Replayed commands joined the log, so the loop is offered again.
    await page.waitForFunction(()=>window.context.loopShortcut.detectedLoop);
    assert.equal(await page.evaluate(()=>window.context.loopShortcut.detectedLoop.steps.length), 1);
  });
});

test("replay of a detected loop stops when a repetition changes the game differently", {timeout:900000}, async () => {
  await open(async (page) => {
    await page.evaluate((action)=>window.context.dispatch({type:'priority_action',action_index:1,action_ref:action.action_ref}), activate);
    await page.waitForFunction(()=>window.context.loopShortcut.detectedLoop);
    await page.waitForTimeout(150);
    await page.evaluate(()=>window.context.loopShortcut.proposeDetectedLoop());
    await page.waitForFunction(()=>window.context.loopShortcut.proposalDraft?.summary);
    // From the second repetition on, the engine reports a different change.
    await page.evaluate(()=>{let replayed=0;window.offerSummary=()=>{replayed+=1;return replayed>=2?'+1 red mana (Alice), -1 life (Alice)':'+1 red mana (Alice)';};});
    await page.waitForTimeout(150);
    await page.evaluate(()=>window.context.loopShortcut.propose(10));
    await page.waitForFunction(()=>window.submissions.some(c=>c.type==='cancel_loop_shortcut'),null,{timeout:10000});
    assert.deepEqual(await page.evaluate(()=>window.submissions.map(c=>c.type)),[
      'priority_action','propose_loop_shortcut','priority_action','priority_action','cancel_loop_shortcut',
    ], "two repetitions ran; the third never started");
  });
});
