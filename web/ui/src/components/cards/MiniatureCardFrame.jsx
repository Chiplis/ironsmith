import { memo, useLayoutEffect, useMemo, useRef, useState } from 'react';
import HoverArtOverlay from '../right-rail/HoverArtOverlay';
import { GameContext } from '@/context/GameContext.shared';

// Lay out the composition at inspector size. Only its finished presentation
// scales with the battlefield slot, including pixel-sized icons and borders.
const WIDTH = 380;
const HEIGHT = 531;

const FRAME_FIELDS = ['id', 'stable_id', 'name', 'oracle_id', 'oracleId', 'oracle_text', 'effect_text', 'ability_text', 'compiled_text', 'abilities', 'mana_cost', 'type_line', 'owner', 'controller'];

const MiniatureComposition = memo(function MiniatureComposition({snapshot, imageUrl}) {
  const card = useMemo(() => JSON.parse(snapshot), [snapshot]);
  // A miniature has no game actions. Publish only its own printable snapshot
  // so unrelated game/priority updates cannot refit every card on the table.
  const context = useMemo(() => ({game: null, state: {
    players: [{id: card.controller ?? card.owner, battlefield: [card]}],
    perspective: card.controller ?? card.owner,
  }}), [card]);
  return <GameContext.Provider value={context}>
    <HoverArtOverlay objectId={card.id} sourceImageUrl={imageUrl} displayMode="miniature-frame" showFramePreview={false} />
  </GameContext.Provider>;
});

export default function MiniatureCardFrame({ card, imageUrl }) {
  const snapshot = JSON.stringify(Object.fromEntries(FRAME_FIELDS.map(key => [key, card[key]])));
  const host = useRef(null);
  const [scale, setScale] = useState(0);
  useLayoutEffect(() => {
    const observer = new ResizeObserver(([entry]) => {
      setScale(Math.min(entry.contentRect.width / WIDTH, entry.contentRect.height / HEIGHT));
    });
    observer.observe(host.current);
    return () => observer.disconnect();
  }, []);
  return <div ref={host} className="battlefield-prepared-frame" aria-hidden="true" inert>
    <div className="battlefield-prepared-frame__composition" style={{
      width: WIDTH, height: HEIGHT,
      transform: `scale(${scale})`, visibility: scale > 0 ? 'visible' : 'hidden',
    }}>
      <MiniatureComposition snapshot={snapshot} imageUrl={imageUrl} />
    </div>
  </div>;
}
