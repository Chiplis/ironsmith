import {useEffect,useState} from 'react';

// Keep the existing fallback above the completed canvas for a single fade.
// Cache hits never mount it. No duplicate renderer or canvas snapshot is needed.
export default function CardFrameFallbackTransition({ready,children}) {
  const [retired,setRetired] = useState(ready);
  useEffect(() => {
    if (!ready || retired) return;
    const delay = window.matchMedia('(prefers-reduced-motion: reduce)').matches ? 0 : 160;
    const timer = setTimeout(() => setRetired(true),delay);
    return () => clearTimeout(timer);
  }, [ready,retired]);
  if (ready && retired) return null;
  return <div className="card-frame-fallback" data-leaving={ready ? 'true' : undefined}
    aria-hidden={ready || undefined} inert={ready || undefined}>{children}</div>;
}
