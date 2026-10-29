import React, { useState } from 'react';
import { createRoot } from 'react-dom/client';
import { GameContext } from '../src/context/GameContext.shared';
import { I18nProvider } from '../src/i18n/I18nContext';
import AddCardSheet from '../src/components/layout/AddCardSheet';
import HoverArtOverlay from '../src/components/right-rail/HoverArtOverlay';
import { saveCustomCard } from '../src/lib/custom-card-library.js';
import { createWasmInteractionGate } from '../src/lib/wasmInteractionGate.js';
import init, { WasmGame } from '../../wasm_demo/pkg/engine.js';
import '../src/index.css';

await init();
const game = new WasmGame();
game.resetEmpty(['Alice', 'Bob'], 20);
// Synthetic definition: the report does not include the real plunder rules.
await saveCustomCard({ faces: [{ name: 'Custom action regression', cardTypes: ['Creature'],
  manaCost: '{1}{R}', power: '2', toughness: '2',
  oracleText: 'Whenever this creature attacks, plunder. (Create a Treasure token.)',
}] }, { compile: input => game.previewCustomCard(input) });
const gate = createWasmInteractionGate();
window.customAddGame = game;
window.customAddGate = gate;
function Fixture() {
  const [state, setState] = useState(() => game.uiState());
  const [status, setStatus] = useState('Ready');
  const [selected, setSelected] = useState(null);
  const refresh = async message => { setState(game.uiState()); setStatus(message); };
  return <GameContext.Provider value={{ game, state, refresh, setStatus,
    runWasmInteraction: gate.run, multiplayer: { mode: 'idle', matchStarted: false } }}>
    <AddCardSheet embedded />
    <p role="status">{status}</p>
    {state.players.flatMap(player => player.battlefield || []).map(card =>
      <button key={card.id} onClick={() => setSelected(card.id)}>Inspect {card.name}</button>)}
    <button onClick={() => setSelected(null)}>Close inspector</button>
    <div style={{ position: 'relative', width: 700, height: 600 }}>
      {selected != null && <HoverArtOverlay objectId={selected} displayMode="inspector" />}
    </div>
  </GameContext.Provider>;
}
createRoot(document.getElementById('root')).render(<I18nProvider><Fixture /></I18nProvider>);
