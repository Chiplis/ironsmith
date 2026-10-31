import PriorityShortcutHint from '../src/components/decisions/PriorityShortcutHint';
import SelectOptionsDecision from '../src/components/decisions/SelectOptionsDecision';
import {HoverProvider} from '../src/context/HoverContext';
import React, {useState,useSyncExternalStore,useEffect} from 'react';
import {createRoot} from 'react-dom/client';
import {GameContext} from '../src/context/GameContext.shared';
import {I18nProvider} from '../src/i18n/I18nContext';
import {createPriorityStops} from '../src/lib/priority-stops';
import {createPriorityPolicy} from '../src/lib/priority-policy';
import {installPriorityShortcuts,handleTriggerShortcut} from '../src/lib/priority-shortcuts';
import PriorityPreferences from '../src/components/priority/PriorityPreferences';
import PriorityHoldControl from '../src/components/decisions/PriorityHoldControl';
import PhaseTrack from '../src/components/board/PhaseTrack';
import '../src/index.css';
const entry={id:10,inspect_object_id:6,source_stable_id:25,name:'Soul Warden',controller:1,ability_kind:'Triggered',
 automation_identity:'trigger:123',source_ability_text:'Whenever another creature enters, you gain 1 life.',targets:[]};
const initial={perspective:0,active_player:1,priority_player:0,turn_number:3,phase:'FirstMain',step:null,stack_size:1,stack_objects:[entry],
 players:[{id:0,name:'Alice',life:20},{id:1,name:'Bob',life:20}],decision:{kind:'priority',player:0,actions:[{kind:'pass_priority',index:0,action_ref:{kind:'pass_priority'}}],can_act:true}};
function App(){
 const [state,setState]=useState(initial);
 const [stops]=useState(()=>createPriorityStops({resolveMode:'manual'}));
 const [policy]=useState(()=>createPriorityPolicy({stops}));
 const revision=useSyncExternalStore(policy.subscribe,policy.getSnapshot);
 const stopState=useSyncExternalStore(stops.subscribe,stops.getSnapshot);
 const start=(mode,target)=>policy.start(mode,state,target);
 const clear=()=>policy.clear(state);
 const status=policy.status(state);
 useEffect(()=>installPriorityShortcuts(document,{state:()=>state,maintain:value=>{window.__held=value;},start,clear,
  group:()=>handleTriggerShortcut(policy,state),answer:command=>{window.__answer=command;}}),[state]);
 const context={dispatch:command=>{window.__answer=command;},state,priorityPolicy:policy,priorityPolicyRevision:revision,priorityStatus:status,
  priorityStops:stopState.stops,cyclePriorityStop:(key,player)=>stops.cycleStop(key,state,player),
  setPriorityStop:(key,mode,player)=>stops.setStop(key,mode,state,player),startPriorityYield:start,clearPriorityYields:clear};
 window.__priorityFixture={state:()=>state,policy,status:()=>policy.status(state),setState,entry,initial};
 return <I18nProvider><GameContext.Provider value={context}>
  <HoverProvider><main style={{minHeight:'100vh',background:'radial-gradient(ellipse at 50% 25%,#23364b,#0b1322 80%)',padding:'36px',color:'#e6ecf5'}}>
   <div style={{display:'flex',justifyContent:'space-between',alignItems:'center',marginBottom:50}}><strong style={{fontSize:24}}>IronSmith</strong><span>Turn 3 · Bob’s main phase</span></div>
   <div style={{maxWidth:1050,margin:'auto'}}><PhaseTrack reserveCombatSpace/>
    <div style={{marginTop:60,display:'flex',gap:12,alignItems:'center'}}><PriorityHoldControl/>
    <button className="decision-main-button" onClick={()=>{window.__passed=true;}}>Pass priority<PriorityShortcutHint number="1" /></button></div>
    <div style={{marginTop:40,padding:20,border:'1px solid #596a7c',borderRadius:8,maxWidth:360,background:'#182b40'}}>
     <small>BOB’S TRIGGERED ABILITY</small><h2 style={{fontSize:24,margin:'8px 0'}}>Soul Warden</h2><p>{entry.source_ability_text}</p>
     <button style={{marginTop:18,textDecoration:'underline'}} onClick={()=>window.dispatchEvent(new CustomEvent('ironsmith:priority-menu',{detail:{entry}}))}>Ability preferences</button>
    </div>
   {state.decision.kind === 'select_options' && <SelectOptionsDecision decision={state.decision} canAct />}
   </div>
  </main></HoverProvider><PriorityPreferences/>
 </GameContext.Provider></I18nProvider>;
}
createRoot(document.getElementById('root')).render(<App/>);
