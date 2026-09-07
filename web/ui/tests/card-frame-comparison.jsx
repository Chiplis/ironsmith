import React from 'react';
import { createRoot } from 'react-dom/client';
import { GameContext } from '../src/context/GameContext.shared';
import { I18nProvider } from '../src/i18n/I18nContext';
import HoverArtOverlay from '../src/components/right-rail/HoverArtOverlay';
import '../src/index.css';
const cards = window.__comparisonCards || [];
const showOriginals = window.__comparisonShowOriginals;
createRoot(document.getElementById('root')).render(<I18nProvider>
  <div style={{ display: 'grid', gridTemplateColumns: showOriginals ? 'repeat(2, 840px)' : 'repeat(4, 420px)', gap: 28, padding: 30, background: '#101216', width: 'max-content' }}>
    {cards.map((card, index) => <GameContext.Provider key={card.name} value={{ state: { players: [{ id: 0, battlefield: [card] }], perspective: 0 } }}>
      <div>
      {showOriginals && <div style={{color:'white',font:'16px sans-serif',padding:8}}>{card.name} — original / interactive preview</div>}
      <div style={{display:'flex'}}>
      {showOriginals && <img src={card.sourceImageUrl} alt="Original printing" style={{width:420,height:600,objectFit:'contain'}} />}
      <div data-comparison-card={index} style={{ position: 'relative', width: 420, height: 600 }}>
        <HoverArtOverlay objectId={card.id} sourceImageUrl={card.sourceImageUrl} displayMode="card-frame" transientPreview={{card}} />
      </div>
      </div>
      </div>
    </GameContext.Provider>)}
  </div>
</I18nProvider>);
