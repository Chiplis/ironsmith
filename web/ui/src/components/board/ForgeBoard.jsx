import { useEffect, useRef } from 'react';
import { getPlayerAccent } from '../../lib/player-colors.js';
import { forgeSignal } from '../../lib/forge-board-layout.js';
import { mountForgeBoard } from './forge/forge-controller.js';
import './forge/forge-board.css';

export default function ForgeBoard({ state, playerAccentOverrides = null, interactionLocked = false }) {
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
    // Keep the CSS board available throughout loading, GPU failure, and loss
    // of the graphics context. Three.js lives in its own lazy-loaded chunk.
    import('./forge/forge-scene.js').then(({ createForgeScene }) => {
      if (cancelled) return;
      let scene;
      try { scene = createForgeScene(host); }
      catch { host.dataset.renderer = 'fallback'; }
      controllerRef.current = mountForgeBoard(root, host, scene, latest.current.signal);
      controllerRef.current.update(latest.current.signal, latest.current.interactionLocked);
    }).catch(() => {
      if (!cancelled) {
        controllerRef.current = mountForgeBoard(root, host, null, latest.current.signal);
        controllerRef.current.update(latest.current.signal, latest.current.interactionLocked);
      }
    });
    return () => {
      cancelled = true;
      controllerRef.current?.dispose(); controllerRef.current = null;
      delete root.dataset.forgeBoard;
    };
  }, []);
  useEffect(() => {
    controllerRef.current?.update(signal, interactionLocked);
  });
  return <div ref={hostRef} className="forge-board" aria-hidden="true" data-renderer="fallback" />;
}
