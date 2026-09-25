import { useLayoutEffect, useRef, useState } from 'react';
import './card-frame-stage.css';

// Rendering history follows the cached asset bundle without retaining evicted
// frames. Live DOM/layout and game actions are still recomputed for each view.
const renderedFrames = new WeakSet();

export default function CardFrameStage({ preparation, assets = preparation, showLoadingFrame = false, onReadyChange, children, style, ...props }) {
  const ref = useRef(null);
  const [finished, setFinished] = useState(null);
  const [presentation, setPresentation] = useState(() => ({ assets, reuse: Boolean(assets && renderedFrames.has(assets)) }));
  if (presentation.assets !== assets) {
    setPresentation({ assets, reuse: Boolean(assets && renderedFrames.has(assets)) });
  }
  const ready = Boolean(preparation && finished === preparation);
  // The live placeholder frame (the art in our own frame, with the live
  // text) has no assets to wait for, and the bare printing or art crop is
  // never shown on its own. Once the placeholder has been up, the stage stays
  // visible while the prepared frame settles: hiding it would leave nothing.
  const loadingFrameVisible = showLoadingFrame;
  const [placeholderShown, setPlaceholderShown] = useState(loadingFrameVisible);
  if (loadingFrameVisible && !placeholderShown) setPlaceholderShown(true);
  const frameVisible = ready || loadingFrameVisible || placeholderShown;

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

  useLayoutEffect(() => { onReadyChange?.(frameVisible); }, [onReadyChange, frameVisible]);
  return <div className="card-frame-preview-shell">
    <div {...props} ref={ref} data-render-ready={ready ? 'true' : 'false'} data-frame-reused={presentation.reuse ? 'true' : 'false'}
    data-loading-frame={loadingFrameVisible ? 'true' : undefined}
    aria-hidden={!frameVisible} inert={!frameVisible}
    style={{...style, opacity: frameVisible ? 1 : 0, ...(presentation.reuse ? {transition: 'none'} : {}), ...(!frameVisible ? {pointerEvents: 'none'} : {})}}>
    {children}
  </div></div>;
}
