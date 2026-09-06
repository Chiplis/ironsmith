import React from 'react';
import { createRoot } from 'react-dom/client';
import { GameContext } from '../src/context/GameContext.shared';
import { I18nProvider } from '../src/i18n/I18nContext';
import HoverArtOverlay from '../src/components/right-rail/HoverArtOverlay';
import '../src/index.css';
const cards = window.__comparisonCards || [];
createRoot(document.getElementById('root')).render(<I18nProvider>
  <div style={{ display: 'flex', gap: 28, padding: 30, background: '#101216', width: 'max-content' }}>
    {cards.map((card, index) => <GameContext.Provider key={card.name} value={{ state: { players: [{ id: 0, battlefield: [card] }], perspective: 0 } }}>
      <div data-comparison-card={index} style={{ position: 'relative', width: 420, height: 600 }}>
        <HoverArtOverlay objectId={card.id} displayMode="card-frame" transientPreview={{card}} />
      </div>
    </GameContext.Provider>)}
  </div>
</I18nProvider>);
