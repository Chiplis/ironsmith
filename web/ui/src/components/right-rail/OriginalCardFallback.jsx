import useUiText from "@/i18n/useUiText";
import { SymbolText } from '@/lib/mana-symbols';
import { useState } from 'react';
import GroupedManaAbility from './GroupedManaAbility';
import './original-card-fallback.css';

function repairPrintingCorners(image) {
  const width = image.naturalWidth;
  const height = image.naturalHeight;
  if (!width || !height) return null;
  const canvas = document.createElement('canvas');
  canvas.width = width;
  canvas.height = height;
  const context = canvas.getContext('2d', { willReadFrequently: true });
  if (!context) return null;
  context.drawImage(image, 0, 0);
  const pixels = context.getImageData(0, 0, width, height);
  const { data } = pixels;
  const radius = Math.max(10, Math.round(Math.min(width, height) * 0.026));
  const corners = [[0, 0, 1, 1], [width - 1, 0, -1, 1], [0, height - 1, 1, -1], [width - 1, height - 1, -1, -1]];
  let repaired = 0;

  for (const [originX, originY, xDirection, yDirection] of corners) {
    for (let y = 0; y < radius; y += 1) for (let x = 0; x < radius; x += 1) {
      const distance = Math.hypot(x - radius, y - radius);
      const targetX = originX + x * xDirection;
      const targetY = originY + y * yDirection;
      const target = (targetY * width + targetX) * 4;
      const needsRepair = data[target + 3] < 250 || distance > radius - 0.5;
      if (!needsRepair) continue;
      const scale = Math.min(1, (radius - 1) / Math.max(distance, 1));
      const sourceX = Math.round(radius + (x - radius) * scale);
      const sourceY = Math.round(radius + (y - radius) * scale);
      const sourceXAbsolute = originX + sourceX * xDirection;
      const sourceYAbsolute = originY + sourceY * yDirection;
      const source = (sourceYAbsolute * width + sourceXAbsolute) * 4;
      if (data[source + 3] < 250) continue;
      const difference = Math.abs(data[target] - data[source]) + Math.abs(data[target + 1] - data[source + 1]) + Math.abs(data[target + 2] - data[source + 2]);
      if (data[target + 3] >= 250 && difference < 42) continue;
      data[target] = data[source]; data[target + 1] = data[source + 1]; data[target + 2] = data[source + 2]; data[target + 3] = 255;
      repaired += 1;
    }
  }
  if (!repaired) return null;
  context.putImageData(pixels, 0, 0);
  return canvas.toDataURL('image/webp', 0.98);
}

function PrintingImage({ imageUrl, name }) {
  const ui = useUiText();
  const [repaired, setRepaired] = useState(null);
  const displayedUrl = repaired?.source === imageUrl ? repaired.url || imageUrl : imageUrl;
  return <img
    key={`${imageUrl}-corner-repair-v1`}
    src={displayedUrl}
    alt={name || ui('Card')}
    crossOrigin="anonymous"
    referrerPolicy="no-referrer"
    decoding="async"
    onLoad={(event) => {
      if (displayedUrl !== imageUrl) return;
      try { setRepaired({ source: imageUrl, url: repairPrintingCorners(event.currentTarget) }); }
      catch { setRepaired({ source: imageUrl, url: null }); }
    }}
  />;
}

// When text regions cannot be masked, preserve the printing. Live rules and
// actions remain available in an explicit details panel, not a fake card frame.
export default function OriginalCardFallback({ imageUrl, name, rulesView, onActivate, highlighted, flavorText, stats, counters, detailsLabel }) {
  const ui = useUiText();
  return <article className="original-card-fallback" aria-label={name || ui('Card details')}>
    {imageUrl && <PrintingImage imageUrl={imageUrl} name={name} />}
    <details className="original-card-details" open={!imageUrl || undefined}>
      <summary>{ui(detailsLabel)}</summary>
      <div className="original-card-details__body">
        <strong>{name}</strong>
        {(stats || counters) && <p>{[stats, counters].filter(Boolean).join(' · ')}</p>}
        {rulesView.lines.map((line, index) => {
          const actions = rulesView.actions.get(index) || [];
          const action = actions.find(action => !action.payment_pending && action.mana_payment_available !== false);
          const available = Boolean(action && onActivate);
          return <div key={index} className="inspector-ability-section" data-stack-highlighted={highlighted.has(index) ? 'true' : undefined}>
            {rulesView.manaGroups.has(index) ? <GroupedManaAbility group={rulesView.manaGroups.get(index)} name={name} onActivate={onActivate} />
              : actions.length || /[:：]/u.test(line) ? <button type="button" className="inspector-oracle-line-action"
                data-available={available ? 'true' : 'false'} disabled={!available}
                aria-label={ui("{0}: {1}", { 0: name || 'Card', 1: line })}
                onPointerDown={event => event.stopPropagation()}
                onClick={event => { event.stopPropagation(); if (available) onActivate(action); }}>
                <SymbolText text={line} />
              </button> : <SymbolText text={line} />}
          </div>;
        })}
        {flavorText && <p aria-label={ui("Flavor text")}><em>{flavorText}</em></p>}
      </div>
    </details>
  </article>;
}
