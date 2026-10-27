import {useLayoutEffect,useRef} from 'react';
import {ABILITY_INK} from '@/lib/ability-action-state';

// Keep the renderer's tap-arrow geometry, including historical printings.
// Only dark glyph pixels become ink; the light symbol disk stays untouched.
const tapMasks = new WeakMap();
function paintTapInk(context, symbol, x, y, size, source, ink) {
  if (symbol !== 'T') return;
  let masks = tapMasks.get(source);
  if (!masks) { masks = new Map(); tapMasks.set(source, masks); }
  const transform = context.getTransform();
  const left = Math.round(x * transform.a + transform.e);
  const top = Math.round(y * transform.d + transform.f);
  const width = Math.max(1, Math.round(size * transform.a));
  const height = Math.max(1, Math.round(size * transform.d));
  const key = `${left}:${top}:${width}:${height}:${ink}`;
  let mask = masks.get(key);
  if (!mask) {
    mask = document.createElement('canvas');
    mask.width = width; mask.height = height;
    const painter = mask.getContext('2d');
    painter.drawImage(source, left, top, width, height, 0, 0, width, height);
    const pixels = painter.getImageData(0, 0, width, height);
    for (let i = 0; i < pixels.data.length; i += 4) {
      const lightness = Math.max(pixels.data[i], pixels.data[i + 1], pixels.data[i + 2]);
      pixels.data[i + 3] *= Math.max(0, Math.min(1, (150 - lightness) / 100));
    }
    painter.putImageData(pixels, 0, 0);
    painter.globalCompositeOperation = 'source-in';
    painter.fillStyle = ink;
    painter.fillRect(0, 0, width, height);
    masks.set(key, mask);
  }
  context.drawImage(mask, x, y, size, size);
}

// Repaint measured text and tap glyphs, leaving the frame and mana unchanged.
export default function AbilityTextInk({result,regions,states}) {
  const ref=useRef(null);
  useLayoutEffect(()=>{
    const canvas=ref.current, {api,face}=result;
    canvas.width=result.canvas.width;canvas.height=result.canvas.height;
    const context=canvas.getContext('2d');
    context.scale(canvas.width/488,canvas.height/680);
    const kind=api.faceFrame(face);
    const layouts=kind.startsWith('planeswalker')?api.planeswalkerRuleRows(context,face).map(row=>row.layout)
      :kind==='saga'?(()=>{const saga=api.sagaRuleLayout(context,face);return [saga.reminder,...saga.rows.map(row=>row.layout)];})()
      :kind==='normal'?[api.isRetro(face)?api.retroTextLayout(context,face):api.ordinaryTextLayout(context,face)]:[];
    for(const [key,state] of states){
      const ink=ABILITY_INK[state];if(!ink)continue;
      context.save();context.beginPath();
      for(const region of regions)if(`${region.lineIndex}:${region.option||''}`===key)
        context.rect(region.left-1,region.top-1,region.width+2,region.height+2);
      context.clip();
      const paintSymbol = (ctx, symbol, x, y, size) => paintTapInk(ctx, symbol, x, y, size, result.canvas, ink);
      for(const layout of layouts.filter(Boolean))api.paintRulesLayout(context,layout,{},paintSymbol,0,ink);
      context.restore();
    }
  },[result,regions,states]);
  return <canvas ref={ref} className="ability-text-ink" aria-hidden="true"/>;
}
