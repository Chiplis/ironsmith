import { useLayoutEffect, useRef, useState } from 'react';
import CardFramePresentation from './CardFramePresentation';

// Lay out the composition at inspector size. Only its finished presentation
// scales with the battlefield slot, including pixel-sized icons and borders.
// The hand exports these so a hovered card can grow to the very same size the
// table renders its frames at.
export const CARD_FRAME_RENDER_WIDTH = 380;
export const CARD_FRAME_RENDER_HEIGHT = 531;

const WIDTH = CARD_FRAME_RENDER_WIDTH;
const HEIGHT = CARD_FRAME_RENDER_HEIGHT;

export default function MiniatureCardFrame({ card, imageUrl, faceIndex=0 }) {
  const host = useRef(null);
  const [scale, setScale] = useState(0);
  useLayoutEffect(() => {
    const observer = new ResizeObserver(([entry]) => {
      setScale(Math.min(entry.contentRect.width / WIDTH, entry.contentRect.height / HEIGHT));
    });
    observer.observe(host.current);
    return () => observer.disconnect();
  }, []);
  return <div ref={host} className="battlefield-prepared-frame" style={{position:'absolute',inset:0,zIndex:1,pointerEvents:'none',overflow:'hidden'}} aria-hidden="true" inert>
    <div className="battlefield-prepared-frame__composition" style={{
      width: WIDTH, height: HEIGHT,
      position:'absolute',left:'50%',top:'50%',
      transform: `translate(-50%, -50%) scale(${scale})`, transformOrigin:'center', visibility: scale > 0 ? 'visible' : 'hidden',
    }}>
      <CardFramePresentation card={card} imageUrl={imageUrl} faceIndex={faceIndex}/>
    </div>
  </div>;
}
