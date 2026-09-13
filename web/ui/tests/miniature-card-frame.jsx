import React from 'react';
import { createRoot } from 'react-dom/client';
import { GameContext } from '../src/context/GameContext.shared';
import { I18nProvider } from '../src/i18n/I18nContext';
import { HoverProvider } from '../src/context/HoverContext';
import { DragProvider } from '../src/context/DragContext';
import GameCard from '../src/components/cards/GameCard';
import OriginalCardFallback from '../src/components/right-rail/OriginalCardFallback';
import '../src/index.css';

const cards = window.__miniatureCards.map((printing, index) => ({
  ...printing, id: index + 1, owner: 0, controller: 0,
  power_toughness: printing.power != null ? `${printing.power}/${printing.toughness}` : null,
  sourceImageUrl: printing.image_uris.normal,
}));
const state = {players: [{id: 0, battlefield: cards}], perspective: 0};
window.__miniatureDetailsRequests = 0;
const game = {objectDetails: async id => {
  window.__miniatureDetailsRequests++;
  return cards.find(card => card.id === Number(id));
}};
const root = createRoot(document.getElementById('root'));
const render = state => root.render(
  <I18nProvider><GameContext.Provider value={{state, game}}><HoverProvider><DragProvider>
    <div id="miniatures" style={{display: 'flex', gap: 24, padding: 24, alignItems: 'start'}}>
      {cards.map(card => <GameCard key={card.id} card={card} variant="battlefield" battlefieldVisualMode="portrait"
        sourceImageUrl={card.sourceImageUrl} style={{width: 96, minWidth: 96, height: 134, minHeight: 134}} />)}
      <div id="fallback" style={{position: 'relative', width: 96, height: 134}}>
        <OriginalCardFallback imageUrl={cards[2].sourceImageUrl} name="Fallback land" showDetails={false} />
      </div>
    </div>
  </DragProvider></HoverProvider></GameContext.Provider></I18nProvider>
);
render(state);
window.__updateMiniatureSnapshot = () => render({...state, turn: 2});
