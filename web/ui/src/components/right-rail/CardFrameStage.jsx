import useUiText from "@/i18n/useUiText";
import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import './card-frame-stage.css';

// Rendering history follows the cached asset bundle without retaining evicted
// frames. Live DOM/layout and game actions are still recomputed for each view.
const renderedFrames = new WeakSet();

export default function CardFrameStage({ preparation, assets = preparation, previewUrl, previewName, onReadyChange, children, style, ...props }) {
  const ui = useUiText();
  const ref = useRef(null);
  const [decodedPreview, setDecodedPreview] = useState(null);
  const [finished, setFinished] = useState(null);
  const [presentation, setPresentation] = useState(() => ({ assets, reuse: Boolean(assets && renderedFrames.has(assets)) }));
  if (presentation.assets !== assets) {
    setPresentation({ assets, reuse: Boolean(assets && renderedFrames.has(assets)) });
  }
  const ready = Boolean(preparation && finished === preparation);
  const previewReady = Boolean(!presentation.reuse && previewUrl && decodedPreview === previewUrl);

  useEffect(() => {
    if (!previewUrl || presentation.reuse) return undefined;
    let active = true;
    const image = new Image();
    image.referrerPolicy = 'no-referrer';
    image.src = previewUrl;
    image.decode().then(() => {
      if (active) setDecodedPreview(previewUrl);
    }).catch(() => {});
    return () => { active = false; };
  }, [previewUrl, presentation.reuse]);

  useLayoutEffect(() => {
    if (!preparation) return undefined;
    let active = true;
    let frame = 0;
    Promise.allSettled([...ref.current.querySelectorAll('img')].map(image => image.decode())).then(() => {
      if (!active) return;
      // The hidden frame still participates in layout. Let rules fitting and
      // its ResizeObserver finish with the final fonts, textures, and flavor.
      frame = requestAnimationFrame(() => {
        frame = requestAnimationFrame(() => {
          if (active) {
            renderedFrames.add(preparation);
            setFinished(preparation);
          }
        });
      });
    });
    return () => { active = false; cancelAnimationFrame(frame); };
  }, [preparation]);

  useLayoutEffect(() => { onReadyChange?.(ready || previewReady); }, [onReadyChange, ready, previewReady]);
  return <div className="card-frame-preview-shell">
    {previewReady && <img className="card-frame-art-preview" src={previewUrl}
      alt={previewName || ui('Card artwork')} referrerPolicy="no-referrer"
      data-frame-ready={ready ? 'true' : 'false'} aria-hidden={ready} />}
    <div {...props} ref={ref} data-render-ready={ready ? 'true' : 'false'} data-frame-reused={presentation.reuse ? 'true' : 'false'}
    aria-hidden={!ready} inert={!ready}
    style={{...style, opacity: ready ? 1 : 0, ...(presentation.reuse ? {transition: 'none'} : {}), ...(!ready ? {pointerEvents: 'none'} : {})}}>
    {children}
  </div></div>;
}
