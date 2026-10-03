import React, { useLayoutEffect, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { GameContext } from '../src/context/GameContext.shared';
import { HoverProvider } from '../src/context/HoverContext';
import { I18nProvider } from '../src/i18n/I18nContext';
import { TooltipProvider } from '../src/components/ui/tooltip';
import DecisionPopupLayer from '../src/components/overlays/DecisionPopupLayer';
import '../src/index.css';
const state = { snapshot_id: 1, perspective: 0, active_player: 0, priority_player: 0,
  phase: 'first main phase', stack_size: 3, stack_preview: ['Trigger', 'Trigger', 'Trigger'],
  players: [0, 1].map(id => ({ id, name: `Player ${id}`, battlefield: [], hand_cards: [], mana_pool: {} })),
  decision: { kind: 'priority', player: 0, actions: [{ index: 0, kind: 'pass_priority', label: 'Pass priority',
    action_ref: { kind: 'pass_priority' } }] } };
function Fixture() {
  const [busy, setBusy] = useState(false);
  useLayoutEffect(() => {
    window.__setSubmissionBusy = setBusy;
    window.__decisionCalls ||= [];
  }, []);
  return <I18nProvider><GameContext.Provider value={{ state, game: null,
    multiplayer: { mode: 'in_match', matchStarted: true, localPlayerIndex: 0,
      submittingAction: busy, verifyingAction: true, pendingVerification: 3 },
    dispatch: command => window.__decisionCalls.push(command),
    startResolveAll: () => window.__decisionCalls.push('resolve all'),
    holdRule: 'never', setHoldRule: () => {}, playerAccentOverrides: {} }}>
    <HoverProvider><TooltipProvider>
      <div className="topbar-main-decision-host" style={{ position: 'relative', margin: 40, width: 500, height: 60 }}>
        <DecisionPopupLayer priorityInline />
      </div>
    </TooltipProvider></HoverProvider>
  </GameContext.Provider></I18nProvider>;
}
createRoot(document.getElementById('root')).render(<Fixture />);
