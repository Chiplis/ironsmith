import { useEffect, useState } from 'react';
import { cachedCardFrame, cardFramePreparationKey, prepareCardFrame } from '@/lib/card-frame-preparation';

export default function usePreparedCardFrame(imageUrl, typeLine, enabled) {
  const key = cardFramePreparationKey(imageUrl, typeLine);
  const [result, setResult] = useState(null);
  useEffect(() => {
    if (!enabled || !imageUrl) return undefined;
    let active = true;
    prepareCardFrame(imageUrl, typeLine).then(value => {
      if (active) setResult(value);
    }).catch(() => {});
    return () => { active = false; };
  }, [enabled, imageUrl, typeLine]);
  if (!enabled || !imageUrl) return null;
  return cachedCardFrame(imageUrl, typeLine) || (result?.key === key ? result : null);
}
