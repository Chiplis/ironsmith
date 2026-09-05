import { useEffect, useRef } from "react";

const FOCUSABLE = 'button:not(:disabled), a[href], input:not(:disabled), select:not(:disabled), textarea:not(:disabled), [tabindex]:not([tabindex="-1"])';

/** Keyboard behavior for game overlays that must remain inside the scene DOM. */
export default function useModalFocus(onClose, enabled = true) {
  const rootRef = useRef(null);
  const onCloseRef = useRef(onClose);
  useEffect(() => { onCloseRef.current = onClose; }, [onClose]);
  useEffect(() => {
    const root = rootRef.current;
    if (!enabled || !root) return undefined;
    const previous = document.activeElement;
    const controls = () => Array.from(root.querySelectorAll(FOCUSABLE)).filter(node => node.getClientRects().length > 0);
    const isTopmost = () => {
      const dialogs = Array.from(document.querySelectorAll('[role="dialog"][aria-modal="true"]')).filter(node => node.getClientRects().length > 0);
      return dialogs.at(-1) === root;
    };
    const focusFirst = () => (controls()[0] || root).focus({preventScroll: true});
    const frame = requestAnimationFrame(() => {
      if (isTopmost()) (root.querySelector('[aria-label^="Close"]') || controls()[0] || root).focus({preventScroll: true});
    });
    const onKeyDown = (event) => {
      if (!isTopmost()) return;
      if (event.key === "Escape") {
        event.preventDefault();
        event.stopPropagation();
        onCloseRef.current?.();
      } else if (event.key === "Tab") {
        const nodes = controls();
        const first = nodes[0];
        const last = nodes.at(-1);
        if (!first) { event.preventDefault(); root.focus(); }
        else if (event.shiftKey && (document.activeElement === first || document.activeElement === root)) {
          event.preventDefault(); last.focus();
        } else if (!event.shiftKey && document.activeElement === last) {
          event.preventDefault(); first.focus();
        }
      }
    };
    const onFocus = (event) => {
      if (isTopmost() && !root.contains(event.target)) focusFirst();
    };
    document.addEventListener("keydown", onKeyDown, true);
    document.addEventListener("focusin", onFocus);
    return () => {
      cancelAnimationFrame(frame);
      document.removeEventListener("keydown", onKeyDown, true);
      document.removeEventListener("focusin", onFocus);
      if (previous?.isConnected && (root.contains(document.activeElement) || document.activeElement === document.body)) previous.focus({preventScroll: true});
    };
  }, [enabled]);
  return rootRef;
}
