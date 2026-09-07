import { useEffect, useMemo, useState } from 'react';
import { resolveScryfallPrintingMetadata } from '@/lib/scryfall';
import { cardTypography } from '@/lib/card-typography';
export default function useCardTypography(imageUrl) {
  const [result, setResult] = useState(null);
  useEffect(() => {
    let active = true;
    if (imageUrl) resolveScryfallPrintingMetadata(imageUrl).then(printing => {
      if (active) setResult({ imageUrl, printing });
    });
    return () => { active = false; };
  }, [imageUrl]);
  const typography = useMemo(() => cardTypography(result?.imageUrl === imageUrl ? result?.printing || {} : {}), [result, imageUrl]);
  const [metrics, setMetrics] = useState(null);
  useEffect(() => {
    let active = true;
    const sections = ['title', 'type', 'rules', 'stats'];
    const weight = name => name === 'rules' ? 400 : name === 'stats' ? typography.style['--card-stats-weight'] : typography.titleWeight;
    Promise.all(sections.map(name => document.fonts.load(`${weight(name)} 100px ${typography[name]}`))).then(() => {
      if (!active) return;
      const ctx = document.createElement('canvas').getContext('2d');
      const style = {};
      for (const name of sections) {
        ctx.font = `${weight(name)} 100px ${typography[name]}`;
        const measure = ctx.measureText(name === 'stats' ? '0' : 'x');
        style[`--card-${name}-glyph-ratio`] = Math.max(.3, (measure.actualBoundingBoxAscent + measure.actualBoundingBoxDescent) / 100);
      }
      setMetrics({ typography, style });
    }).catch(() => {});
    return () => { active = false; };
  }, [typography]);
  const printingReady = result?.imageUrl === imageUrl;
  return useMemo(() => ({ ...typography, printingReady, style: { ...typography.style, ...(metrics?.typography === typography ? metrics.style : {}) } }), [typography, metrics, printingReady]);
}
