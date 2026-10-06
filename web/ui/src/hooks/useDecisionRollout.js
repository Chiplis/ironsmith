import { useLayoutEffect, useRef } from "react";

// Only mana payment rolls in; ordinary decision and phase changes stay still.
// Animate the container without changing its layout or anchored transform.
export default function useDecisionRollout(identity, skipDocked = false) {
  const ref = useRef(null);
  useLayoutEffect(() => {
    const node = ref.current;
    if (!node || (skipDocked && node.closest('[data-human-action-dock], .mobile-decision-sheet, .mobile-decision-dock'))) return undefined;
    if (!node.matches('.mana-payment-editor') && !node.querySelector('.mana-payment-editor')) return undefined;
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return undefined;
    let frame;
    let motion;
    let attempts = 0;
    const start = () => {
      if (getComputedStyle(node).visibility === 'hidden' && attempts++ < 12) {
        frame = requestAnimationFrame(start);
        return;
      }
      const distance = Math.max(24, window.innerHeight - node.getBoundingClientRect().top + 16);
      motion = node.animate([
        { translate: `0 ${distance}px`, opacity: 0, offset: 0, easing: 'cubic-bezier(.16,1,.3,1)' },
        { translate: '0 -5px', opacity: 1, offset: .76, easing: 'ease-in-out' },
        { translate: '0 2px', opacity: 1, offset: .9, easing: 'ease-out' },
        { translate: '0 0', opacity: 1, offset: 1 },
      ], { duration: 540 });
    };
    frame = requestAnimationFrame(start);
    return () => { cancelAnimationFrame(frame); motion?.cancel(); };
  }, [identity, skipDocked]);
  return ref;
}
