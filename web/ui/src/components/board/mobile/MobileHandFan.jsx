import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import HandZone from "@/components/board/HandZone";
import { cn } from "@/lib/utils";

// Half-hidden hand at the bottom of the battlefield. Tap the fan surface to
// toggle it, while drag gestures still use HandZone's existing pointer-down
// flow that Workspace recognizes via `[data-mobile-hand-drop-target]`.
export default function MobileHandFan({
  me,
  selectedObjectId,
  onInspect,
  className,
}) {
  const fanRef = useRef(null);
  const [availableWidth, setAvailableWidth] = useState(0);
  useLayoutEffect(() => {
    const update = () => setAvailableWidth(fanRef.current?.clientWidth || 0);
    update();
    const observer = new ResizeObserver(update);
    observer.observe(fanRef.current);
    return () => observer.disconnect();
  }, []);
  const [fanned, setFanned] = useState(false);
  const pendingTapRef = useRef(null);
  const suppressNextClickRef = useRef(false);
  const suppressClickTimerRef = useRef(null);

  useEffect(() => {
    const collapseOutside = (event) => {
      if (fanRef.current?.contains(event.target)) return;
      setFanned(false);
      pendingTapRef.current = null;
      suppressNextClickRef.current = false;
      window.clearTimeout(suppressClickTimerRef.current);
      suppressClickTimerRef.current = null;
    };
    // Capture before battlefield controls and portaled menus stop propagation.
    window.addEventListener("pointerdown", collapseOutside, true);
    return () => window.removeEventListener("pointerdown", collapseOutside, true);
  }, []);

  useEffect(() => () => {
    if (suppressClickTimerRef.current != null) {
      window.clearTimeout(suppressClickTimerRef.current);
      suppressClickTimerRef.current = null;
    }
  }, []);

  const suppressNextClick = useCallback(() => {
    suppressNextClickRef.current = true;
    if (suppressClickTimerRef.current != null) {
      window.clearTimeout(suppressClickTimerRef.current);
    }
    suppressClickTimerRef.current = window.setTimeout(() => {
      suppressNextClickRef.current = false;
      suppressClickTimerRef.current = null;
    }, 250);
  }, []);

  const handlePointerDown = useCallback((event) => {
    if (event.button != null && event.button !== 0) {
      pendingTapRef.current = null;
      return;
    }
    pendingTapRef.current = {
      pointerId: event.pointerId,
      startX: event.clientX,
      startY: event.clientY,
    };
  }, []);

  const handlePointerUp = useCallback((event) => {
    const pending = pendingTapRef.current;
    pendingTapRef.current = null;
    if (!pending) return;
    if (pending.pointerId != null && event.pointerId !== pending.pointerId) return;
    const dx = event.clientX - pending.startX;
    const dy = event.clientY - pending.startY;
    if ((dx * dx + dy * dy) > 16 * 16) return;
    if (!fanned && event.target instanceof Element && event.target.closest(".game-card.hand-card")) {
      return;
    }
    if (fanned) {
      suppressNextClick();
    }
    setFanned((current) => !current);
  }, [fanned, suppressNextClick]);

  const handlePointerCancel = useCallback(() => {
    pendingTapRef.current = null;
  }, []);

  const handleClickCapture = useCallback((event) => {
    if (!suppressNextClickRef.current) return;
    suppressNextClickRef.current = false;
    if (suppressClickTimerRef.current != null) {
      window.clearTimeout(suppressClickTimerRef.current);
      suppressClickTimerRef.current = null;
    }
    event.preventDefault();
    event.stopPropagation();
  }, []);

  return (
    <div
      ref={fanRef}
      className={cn(
        "mobile-mtga-hand-fan",
        fanned && "mobile-mtga-hand-fan--fanned",
        className,
      )}
      data-fanned={fanned ? "true" : "false"}
      aria-expanded={fanned}
      onPointerDown={handlePointerDown}
      onPointerUp={handlePointerUp}
      onPointerCancel={handlePointerCancel}
      onPointerLeave={handlePointerCancel}
      onClickCapture={handleClickCapture}
    >
      <div className="mobile-mtga-hand-fan-viewport">
        <HandZone
          player={me}
          selectedObjectId={selectedObjectId}
          onInspect={onInspect}
          isExpanded
          layout="mobile-fan"
          availableWidth={availableWidth}
        />
      </div>
    </div>
  );
}
