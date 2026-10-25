import {useEffect} from 'react';
import {mergeHandManaCosts} from '../lib/hand-mana-cost-analysis.js';

// Let printed cards paint first. WASM analysis runs in the game worker, and
// late or failed results never block drawing or replace a newer game view.
export function useHandManaCosts({game, state, stateRef, setState}) {
  const snapshotId = state?.snapshot_id, perspective = state?.perspective, generation = game?.runtimeGeneration;
  useEffect(() => {
    if (snapshotId == null || !game?.previewHandManaCosts) return;
    let disposed = false, timer;
    const frame = requestAnimationFrame(() => {
      timer = setTimeout(() => {
        Promise.resolve().then(() => game.previewHandManaCosts(BigInt(snapshotId))).then(preview => {
          if (disposed) return;
          const next = mergeHandManaCosts(stateRef.current, preview);
          if (next !== stateRef.current) {
            game.adoptSnapshotVersion?.(next, stateRef.current);
            setState(next);
          }
        }).catch(() => { /* Printed costs remain available if analysis fails. */ });
      }, 0);
    });
    return () => {disposed = true; cancelAnimationFrame(frame); clearTimeout(timer);};
  }, [game, generation, snapshotId, perspective, stateRef, setState]);
}
