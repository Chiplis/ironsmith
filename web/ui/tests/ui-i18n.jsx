/* eslint-disable react-refresh/only-export-components -- Standalone browser fixture. */
import React from 'react';
import { createRoot } from 'react-dom/client';
import { I18nProvider, useI18n } from '../src/i18n/I18nContext';
import { GameContext } from '../src/context/GameContext.shared';
import RandomGameSheet from '../src/components/layout/RandomGameSheet';
import AddCardSheet from '../src/components/layout/AddCardSheet';
import PhaseHelpPopover from '../src/components/decisions/PhaseHelpPopover';
import PhaseTrack from '../src/components/board/PhaseTrack';
import { SymbolText } from '../src/lib/mana-symbols';
import '../src/index.css';
const state = {players:[{id:0,name:'Main'}], perspective:0, phase:'Beginning',step:'Upkeep',stack_size:0};
const decision = {kind:'priority',actions:[{kind:'activate_ability',legal:true}]};
const context = {state,game:{},multiplayer:{mode:'idle'},setStatus:()=>{}};
function Probe() {
  const {setLocale} = useI18n();
  // Allows switching even while a modal sheet owns keyboard focus.
  React.useEffect(() => { window.__setTestLocale = setLocale; }, [setLocale]);
  return <main style={{padding:30}}>
    <PhaseTrack />
    <PhaseHelpPopover state={state} decision={decision} advanceLabel="Draw" />
    <div data-keyword-probe><SymbolText text="Flying" /></div>
    <div data-localized-keyword-probe><SymbolText text="Vuela" /></div>
    <div data-mana-probe><SymbolText text="{W/U}" /></div>
    <RandomGameSheet trigger={<button>Random fixture</button>} />
    <AddCardSheet trigger={<button>Add fixture</button>} />
  </main>;
}
createRoot(document.getElementById('root')).render(<I18nProvider><GameContext.Provider value={context}><Probe /></GameContext.Provider></I18nProvider>);
