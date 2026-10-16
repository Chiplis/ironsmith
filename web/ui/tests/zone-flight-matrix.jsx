import { HoverProvider } from '../src/context/HoverContext';
import React, { useState } from 'react';
import { createRoot } from 'react-dom/client';
import { GameContext } from '../src/context/GameContext.shared';
import { I18nProvider } from '../src/i18n/I18nContext';
import GameEffectAnimations from '../src/components/overlays/GameEffectAnimations';
import LibraryZonePile from '../src/components/board/LibraryZonePile';
import { ANIMATION_ZONES, ZONE_COLORS } from '../src/lib/game-effect-events';
import '../src/index.css';
function Fixture() {
  const [state, setState] = useState({ perspective: 0, snapshot_id: 1, players: [{ id: 0, life: 20, library_size: 40, hand_size: 0, battlefield: [{ stable_id: 1, abilities: [] }] }] });
  return <I18nProvider><GameContext.Provider value={{ state }}><HoverProvider>
    <main style={{ padding: 40, background: '#101723', minHeight: '100vh', color: '#eef' }}>
      <h1>Zone movement palette</h1>
      <div style={{ display: 'flex', gap: 48, marginTop: 80 }}>
        {ANIMATION_ZONES.map(zone => <div key={zone}>
          <div data-zone-pile={zone} data-zone-owner="0" style={{ width: 86, height: 120, border: `2px solid ${ZONE_COLORS[zone]}`, borderRadius: 8, display: 'grid', placeItems: 'center', color: ZONE_COLORS[zone] }}>{zone}</div>
        </div>)}
      </div>
      <div style={{ marginTop: 90, display: 'flex', gap: 50 }}>
        <LibraryZonePile player={{ id: 1, name: 'Opponent', library_size: 40 }} />
        <div className="game-card" data-stable-id="1" style={{ width: 100, height: 140, border: '1px solid #859', borderRadius: 8 }}>Creature</div>
        <button onClick={() => setState(current => ({ ...current, snapshot_id: current.snapshot_id + 1,
          players: [{ ...current.players[0], battlefield: [{ stable_id: 1, abilities: ['Hexproof'] }] }] }))}>Grant hexproof</button>
      </div>
      <button onClick={() => setState(current => ({ ...current, snapshot_id: current.snapshot_id + 1,
        zone_transitions: [{ id: current.snapshot_id, identity_hidden: true, owner: 0, controller: 0,
          from_zone: 'library', to_zone: 'hand' }],
        players: [{ ...current.players[0], library_size: current.players[0].library_size - 1,
          hand_size: current.players[0].hand_size + 1 }] }))}>Hidden draw</button>
      <GameEffectAnimations />
    </main>
  </HoverProvider></GameContext.Provider></I18nProvider>;
}
createRoot(document.getElementById('root')).render(<Fixture />);
