import React, { useState } from 'react';
import { createRoot } from 'react-dom/client';
import { GameContext } from '../src/context/GameContext.shared';
import { I18nProvider } from '../src/i18n/I18nContext';
import { HoverProvider } from '../src/context/HoverContext';
import { DragProvider } from '../src/context/DragContext';
import GameCard from '../src/components/cards/GameCard';
import '../src/index.css';

const card = { id: 42, name: 'Network Independence Creature', type_line: 'Creature — Construct',
  oracle_text: 'Vigilance', mana_cost: '{2}', power: 2, toughness: 3, controller: 0, owner: 0 };
export default function Fixture() {
  const [resolved, setResolved] = useState(false);
  const state = { players: [{ id: 0, battlefield: resolved ? [card] : [] }], perspective: 0,
    stack_size: resolved ? 0 : 1, stack: resolved ? [] : [card] };
  return <GameContext.Provider value={{ state, game: null }}>
    <button onClick={() => setResolved(true)}>Resolve into battlefield</button>
    {resolved && <GameCard card={card} variant="battlefield" style={{ width: 190, height: 266 }} />}
  </GameContext.Provider>;
}
createRoot(document.getElementById('root')).render(
  <I18nProvider><HoverProvider><DragProvider><Fixture /></DragProvider></HoverProvider></I18nProvider>
);
