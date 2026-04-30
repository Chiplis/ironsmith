import { useCallback, useEffect, useRef } from "react";
import { MOBILE_LONG_PRESS_MS, MOBILE_TAP_DISTANCE_SQ, pointerDistanceSq } from "@/lib/mobile-gestures";

export default function useMobileLongPress({
  onLongPress,
  ms = MOBILE_LONG_PRESS_MS,
  distanceSq = MOBILE_TAP_DISTANCE_SQ,
} = {}) {
  const timerRef = useRef(null);
  const startRef = useRef(null);
  const triggeredRef = useRef(false);

  const clear = useCallback(() => {
    if (timerRef.current != null) {
      window.clearTimeout(timerRef.current);
      timerRef.current = null;
    }
  }, []);

  useEffect(() => () => clear(), [clear]);

  const onPointerDown = useCallback((event) => {
    if (event.pointerType === "mouse" && event.button !== 0) return;
    clear();
    triggeredRef.current = false;
    startRef.current = {
      pointerId: event.pointerId,
      clientX: event.clientX,
      clientY: event.clientY,
    };
    timerRef.current = window.setTimeout(() => {
      triggeredRef.current = true;
      timerRef.current = null;
      onLongPress?.(event);
    }, ms);
  }, [clear, ms, onLongPress]);

  const onPointerMove = useCallback((event) => {
    if (!startRef.current) return;
    if (pointerDistanceSq(event, startRef.current) > distanceSq) {
      clear();
    }
  }, [clear, distanceSq]);

  const onPointerUp = useCallback(() => {
    clear();
  }, [clear]);

  const onPointerCancel = useCallback(() => {
    clear();
  }, [clear]);

  const onPointerLeave = useCallback(() => {
    clear();
  }, [clear]);

  const consumeTrigger = useCallback(() => {
    const fired = triggeredRef.current;
    triggeredRef.current = false;
    return fired;
  }, []);

  return {
    onPointerDown,
    onPointerMove,
    onPointerUp,
    onPointerCancel,
    onPointerLeave,
    consumeTrigger,
  };
}
