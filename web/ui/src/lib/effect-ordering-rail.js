import { useLayoutEffect, useSyncExternalStore } from "react";

// Trigger ordering is normally done on the pending stack cards, so the
// decision panel shows only a hint. Some layouts (landscape phones, collapsed
// or zero-width rails) never show those cards; the decision falls back to its
// own rows unless a rail reports that it is visibly showing the entries.
const visibleRails = new Map();
const listeners = new Set();
let version = 0;

function publish() {
  version += 1;
  listeners.forEach(listener => listener());
}

function subscribe(listener) {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

function railShowsKey(key) {
  for (const railKey of visibleRails.values()) if (railKey === key) return true;
  return false;
}

export function useEffectOrderingRailVisible(key) {
  useSyncExternalStore(subscribe, () => version, () => version);
  return key != null && railShowsKey(key);
}

const MIN_VISIBLE_WIDTH = 48;
const MIN_VISIBLE_HEIGHT = 24;

// The pending cards must survive every clipping ancestor (collapsed rails and
// squeezed full-card HUDs clip with overflow rather than shrinking the body).
function isVisiblyRendered(container) {
  if (!container?.isConnected) return false;
  const card = container.querySelector('[data-pending-trigger="true"], .stack-timeline-entry') || container;
  let { left, top, right, bottom } = card.getBoundingClientRect();
  const view = card.ownerDocument.defaultView;
  for (let node = card; node && node !== card.ownerDocument.documentElement; node = node.parentElement) {
    const style = view.getComputedStyle(node);
    if (style.display === "none" || style.visibility === "hidden") return false;
    if (node !== card && style.overflow !== "visible") {
      const clip = node.getBoundingClientRect();
      left = Math.max(left, clip.left); top = Math.max(top, clip.top);
      right = Math.min(right, clip.right); bottom = Math.min(bottom, clip.bottom);
    }
  }
  right = Math.min(right, view.innerWidth); bottom = Math.min(bottom, view.innerHeight);
  return right - Math.max(left, 0) >= MIN_VISIBLE_WIDTH && bottom - Math.max(top, 0) >= MIN_VISIBLE_HEIGHT;
}

// Registers `ref` as showing the pending entries for `key` while it is
// actually on screen with a usable size.
export function useReportEffectOrderingRail(ref, key) {
  useLayoutEffect(() => {
    const element = ref.current;
    if (!element || key == null) return undefined;
    const token = {};
    const sync = () => {
      const showing = isVisiblyRendered(element);
      if (showing === (visibleRails.get(token) === key)) return;
      if (showing) visibleRails.set(token, key);
      else visibleRails.delete(token);
      publish();
    };
    sync();
    const observer = typeof ResizeObserver === "function" ? new ResizeObserver(sync) : null;
    for (let node = element; node; node = node.parentElement) observer?.observe(node);
    // Layout can still be settling (rail panels roll open); check again.
    const settle = setTimeout(sync, 600);
    window.addEventListener("resize", sync);
    return () => {
      clearTimeout(settle);
      window.removeEventListener("resize", sync);
      observer?.disconnect();
      if (visibleRails.delete(token)) publish();
    };
  }, [ref, key]);
}
