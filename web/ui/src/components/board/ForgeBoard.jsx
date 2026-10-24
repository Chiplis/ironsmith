import useGraphicsQuality from '../../hooks/useGraphicsQuality';
import { useEffect, useRef } from 'react';
import { getPlayerAccent } from '../../lib/player-colors.js';
import { forgeSignal } from '../../lib/forge-board-layout.js';
import { mountForgeBoard } from './forge/forge-controller.js';
import './forge/forge-board.css';

export default function ForgeBoard({ state, playerAccentOverrides = null, interactionLocked = false }) {
  const [quality] = useGraphicsQuality();
  const hostRef = useRef(null);
  const controllerRef = useRef(null);
  const signal = { ...forgeSignal(state), perspective: state?.perspective,
    players: (state?.players || []).map(player => ({ id: player.id ?? player.index,
      color: getPlayerAccent(state.players, player.id ?? player.index, state.perspective, playerAccentOverrides).hex,
    })),
  };
  const latest = useRef({ signal, interactionLocked });
  useEffect(() => { latest.current = { signal, interactionLocked }; });
  useEffect(() => {
    const host = hostRef.current, root = host.parentElement;
    let cancelled = false;
    root.dataset.forgeBoard = 'true';
    root.dataset.graphicsQuality = quality;
    if (quality === 'off') {
      host.dataset.renderer = 'fallback';
      return () => { delete root.dataset.forgeBoard; delete root.dataset.graphicsQuality; };
    }
    // Keep the CSS board available throughout loading, GPU failure, and loss
    // of the graphics context. Three.js lives in its own lazy-loaded chunk.
    import('./forge/forge-scene.js').then(({ createForgeScene }) => {
      if (cancelled) return;
      let scene;
      try { scene = createForgeScene(host, quality); }
      catch { host.dataset.renderer = 'fallback'; }
      controllerRef.current = mountForgeBoard(root, host, scene, latest.current.signal, quality);
      controllerRef.current.update(latest.current.signal, latest.current.interactionLocked);
    }).catch(() => {
      if (!cancelled) {
        controllerRef.current = mountForgeBoard(root, host, null, latest.current.signal, quality);
        controllerRef.current.update(latest.current.signal, latest.current.interactionLocked);
      }
    });
    return () => {
      cancelled = true;
      controllerRef.current?.dispose(); controllerRef.current = null;
      delete root.dataset.graphicsQuality;
      delete root.dataset.forgeBoard;
    };
  }, [quality]);
  useEffect(() => {
    controllerRef.current?.update(signal, interactionLocked);
  });
  return <div ref={hostRef} className="forge-board" aria-hidden="true" data-renderer="fallback" />;
}
