import React, { useEffect, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { GameContext } from '../src/context/GameContext.shared';
import { I18nProvider } from '../src/i18n/I18nContext';
import { HoverProvider, useHoverActions } from '../src/context/HoverContext';
import { DragProvider } from '../src/context/DragContext';
import FloatingCardPreview from '../src/components/right-rail/FloatingCardPreview';
import { cachedCardFrame } from '../src/lib/card-frame-preparation';
import { cardArtCropUrl } from '../src/lib/card-image-variants';
import '../src/index.css';
const normal = 'https://cards.scryfall.io/normal/back/a/b/aaaaaaaa-bbbb-cccc-dddd-000000000077.jpg?printing=selected';
const custom = 'data:image/svg+xml,' + encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" width="488" height="684"><rect width="488" height="684" fill="purple"/></svg>');
const cards = [1, 2].map(id => ({ id, name: 'Same name, different field images', type_line: 'Artifact', oracle_text: 'Flying', zone: 'Battlefield' }));
window.detailRequests = 0;
const game = { objectDetails: async id => {
  window.detailRequests++;
  return cards.find(card => card.id === Number(id));
} };
export default function Fixture() {
  const { hoverCard, clearHover } = useHoverActions();
  const [firstUrl, setFirstUrl] = useState(normal);
  useEffect(() => {
    window.readCachedFrame = () => cachedCardFrame(cardArtCropUrl(firstUrl), 'Artifact');
    return () => { delete window.readCachedFrame; };
  }, [firstUrl]);
  return <GameContext.Provider value={{ game, state: { perspective: 0, players: [{ id: 0, battlefield: cards }] } }}>
    <button onClick={() => setFirstUrl(custom)}>Change field image</button>
    <button onClick={clearHover}>Clear hover</button>
    {cards.map(card => <div key={card.id} className="game-card battlefield-row-card"
      data-object-id={card.id} data-card-image-url={card.id === 1 ? firstUrl : custom}
      onMouseEnter={() => hoverCard(card.id)} style={{ width: 80, height: 112, marginTop: 40 }}>
      <img alt={`Field card ${card.id}`} src={card.id === 1 ? firstUrl : custom} style={{ width: '100%', height: '100%' }} referrerPolicy="no-referrer" />
    </div>)}
    <FloatingCardPreview />
  </GameContext.Provider>;
}
createRoot(document.getElementById('root')).render(<I18nProvider><HoverProvider><DragProvider><Fixture /></DragProvider></HoverProvider></I18nProvider>);
