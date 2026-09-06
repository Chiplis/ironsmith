import { useEffect, useState } from 'react';
import { fullCardImageUrl, sampleCardFrameColors } from '@/lib/card-frame-colors';

export default function useCardFrameColors(artUrl, enabled, textures = true) {
  const key = enabled ? fullCardImageUrl(artUrl) : '';
  const [result, setResult] = useState(null);
  useEffect(() => {
    if (!key) return undefined;
    let active = true;
    sampleCardFrameColors(key, { textures }).then(style => {
      if (active) setResult({ key, textures, style });
    });
    return () => { active = false; };
  }, [key, textures]);
  return result?.key === key && result?.textures === textures ? result.style : null;
}
