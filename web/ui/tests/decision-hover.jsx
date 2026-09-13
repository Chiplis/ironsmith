import React, {useLayoutEffect} from 'react';
import {createRoot} from 'react-dom/client';
import {useGameSnapshot} from '../src/hooks/useGameSnapshot';
import {GameContext} from '../src/context/GameContext.shared';
import {HoverProvider} from '../src/context/HoverContext';
import {I18nProvider} from '../src/i18n/I18nContext';
import {TooltipProvider} from '../src/components/ui/tooltip';
import DecisionPopupLayer from '../src/components/overlays/DecisionPopupLayer';
import '../src/index.css';
const initialState={snapshot_id:1,players:[0,1].map(id=>({id,name:`Player ${id}`,battlefield:[],hand_cards:[],mana_pool:{}})),perspective:0,active_player:0,priority_player:0,phase:'Combat',step:'EndCombat',stack_size:0,decision:{kind:'priority',player:0,actions:[{index:0,kind:'pass_priority',label:'Pass priority'}]}};
const stress=new URLSearchParams(location.search).has('stress');
// Many individually small card renders model a busy table, without engine,
// image-download or machine-specific game-state timing affecting the test.
function BusyCard({phase}) {
  if(stress && phase==='NextMain') {
    const until=performance.now()+3;
    while(performance.now()<until) { /* deterministic rendering load */ }
  }
  return <span>{phase}</span>;
}
function Fixture() {
  const {state,setState,stateRef,subscribeState,isSnapshotRendered}=useGameSnapshot(initialState);
  useLayoutEffect(()=>{
    window.__publishSnapshot=setState;
    window.__readSnapshot=()=>stateRef.current;
    window.__isSnapshotRendered=isSnapshotRendered;
    return subscribeState(snapshot=>{window.__sequencerSnapshot=snapshot;});
  },[setState,stateRef,subscribeState,isSnapshotRendered]);
  useLayoutEffect(()=>{if(state.phase==='NextMain') window.__phaseCommittedAt=performance.now();},[state]);
  const dispatch=async()=>{
    await new Promise(resolve=>setTimeout(resolve,100));
    setState({...stateRef.current,snapshot_id:2,phase:'NextMain',step:null});
    window.__authoritativePhase=stateRef.current.phase;
    window.__snapshotQueuedAt=performance.now();
  };
  return <I18nProvider><GameContext.Provider value={{state,dispatch,game:null,multiplayer:{mode:'idle'},holdRule:'never',setHoldRule:()=>{},playerAccentOverrides:{}}}><HoverProvider><TooltipProvider>
    <div className="topbar-main-decision-host" style={{position:'relative',margin:40,width:180,height:60}}><DecisionPopupLayer priorityInline /></div>
    <output>{state.phase}</output>
    <div>{Array.from({length:100},(_,i)=><BusyCard key={i} phase={state.phase}/>)}</div>
  </TooltipProvider></HoverProvider></GameContext.Provider></I18nProvider>;
}
createRoot(document.getElementById('root')).render(<Fixture/>);
