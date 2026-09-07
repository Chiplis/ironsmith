import { useLayoutEffect, useRef, useState } from 'react';

export default function CardFrameStage({ preparation, onReadyChange, children, style, ...props }) {
  const ref = useRef(null);
  const [finished, setFinished] = useState(null);
  const ready = Boolean(preparation && finished === preparation);

  useLayoutEffect(() => {
    if (!preparation) return undefined;
    let active = true;
    let frame = 0;
    Promise.allSettled([...ref.current.querySelectorAll('img')].map(image => image.decode())).then(() => {
      if (!active) return;
      // The hidden frame still participates in layout. Let rules fitting and
      // its ResizeObserver finish with the final fonts, textures, and flavor.
      frame = requestAnimationFrame(() => {
        frame = requestAnimationFrame(() => { if (active) setFinished(preparation); });
      });
    });
    return () => { active = false; cancelAnimationFrame(frame); };
  }, [preparation]);

  useLayoutEffect(() => { onReadyChange?.(ready); }, [onReadyChange, ready]);

  return <div {...props} ref={ref} data-render-ready={ready ? 'true' : 'false'}
    aria-hidden={!ready} inert={!ready}
    style={{...style, ...(!ready ? {visibility: 'hidden', pointerEvents: 'none'} : {})}}>
    {children}
  </div>;
}
