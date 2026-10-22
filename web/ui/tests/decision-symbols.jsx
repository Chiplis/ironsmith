import React from 'react';
import { createRoot } from 'react-dom/client';
import { GameContext } from '../src/context/GameContext.shared';
import { HoverProvider } from '../src/context/HoverContext';
import { I18nProvider } from '../src/i18n/I18nContext';
import { TooltipProvider } from '../src/components/ui/tooltip';
import DecisionPopupLayer from '../src/components/overlays/DecisionPopupLayer';
import SelectCountersDecision from '../src/components/decisions/SelectCountersDecision';
import '../src/index.css';

const mobile = new URLSearchParams(location.search).has('mobile');
const decision = { kind: 'priority', player: 0, actions: [
  { index: 1, kind: 'pass_priority', label: 'Pay {X}{E}{T}{W/U}{B/P}' },
  { index: 0, kind: 'activate_ability', label: 'Pay {X}{E}{T}{W/U}{B/P}' },
] };
const state = { snapshot_id: 1, players: [0, 1].map(id => ({ id, name: `Player ${id}`, battlefield: [], hand_cards: [], mana_pool: {} })), perspective: 0, active_player: 0, priority_player: 0, phase: 'Main', step: 'Main', stack_objects: [], decision };
const counters = { kind: 'select_counters', player: 0, description: 'Remove {E}', min_total: 0, max_total: 2, options: [{ index: 0, description: 'Energy {E}', max: 2 }] };
createRoot(document.getElementById('root')).render(
  <I18nProvider><GameContext.Provider value={{ state, dispatch: () => {}, game: null, multiplayer: { mode: 'idle' }, holdRule: 'never', setHoldRule: () => {}, playerAccentOverrides: {} }}><HoverProvider><TooltipProvider>
    <div id="menu" style={{ position: 'relative', margin: 40, width: 800, height: 200 }}><DecisionPopupLayer priorityInline mobileBattle={mobile} mobileBattleDockInline /></div>
    <div id="counters"><SelectCountersDecision decision={counters} canAct /></div>
  </TooltipProvider></HoverProvider></GameContext.Provider></I18nProvider>
);
