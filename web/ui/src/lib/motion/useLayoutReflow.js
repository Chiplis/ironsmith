import { useEffect, useLayoutEffect, useRef } from "react";
import { createLayout, uiSpring, cancelMotion } from "@/lib/motion/anime";

// A layout (FLIP) timeline writes `position: absolute` plus a frozen
// width/height onto every child it animates, and only restores those inline
// styles from its completion callback. Cancelling one leaves the children out
// of flow at whatever size the interrupted step had measured, so an
// interrupted reflow can empty a list visually while its count still reads
// full. Finish the timeline instead of cancelling it.
function settleMotion(motion) {
  if (!motion) return;
  if (typeof motion.complete === "function") {
    motion.complete();
    return;
  }
  cancelMotion(motion);
}

export default function useLayoutReflow(rootRef, signature, options = {}) {
  const {
    children = ".game-card",
    disabled = false,
    duration = 360,
    bounce = 0.14,
    delay,
    enterFrom,
    swapAt,
    leaveTo,
  } = options;
  const layoutRef = useRef(null);
  const motionRef = useRef(null);
  const hasRecordedRef = useRef(false);
  const settleFrameRef = useRef(0);
  const paramsRef = useRef({ delay, enterFrom, swapAt, leaveTo });

  useLayoutEffect(() => {
    paramsRef.current = { delay, enterFrom, swapAt, leaveTo };
  }, [delay, enterFrom, swapAt, leaveTo]);

  useLayoutEffect(() => {
    const root = rootRef.current;
    if (!root) return undefined;

    if (!layoutRef.current) {
      layoutRef.current = createLayout(root, { children });
    }

    const layout = layoutRef.current;
    if (disabled) {
      settleMotion(motionRef.current);
      motionRef.current = null;
      layout.record();
      hasRecordedRef.current = true;
      return undefined;
    }

    if (!hasRecordedRef.current) {
      layout.record();
      hasRecordedRef.current = true;
      return undefined;
    }

    cancelAnimationFrame(settleFrameRef.current);
    settleMotion(motionRef.current);
    motionRef.current = null;
    const params = paramsRef.current;
    motionRef.current = layout.animate({
      delay: params.delay,
      duration,
      ease: uiSpring({ duration, bounce }),
      enterFrom: params.enterFrom,
      swapAt: params.swapAt,
      leaveTo: params.leaveTo,
    });

    const currentMotion = motionRef.current;
    if (typeof currentMotion?.then === "function") {
      currentMotion.then(() => {
        if (motionRef.current !== currentMotion) return;
        settleFrameRef.current = window.requestAnimationFrame(() => {
          layout.record();
        });
      });
    }

    return () => {
      cancelAnimationFrame(settleFrameRef.current);
      if (motionRef.current === currentMotion) {
        settleMotion(currentMotion);
        motionRef.current = null;
      }
    };
  }, [bounce, children, disabled, duration, rootRef, signature]);

  useEffect(() => () => {
    cancelAnimationFrame(settleFrameRef.current);
    settleMotion(motionRef.current);
    layoutRef.current?.revert();
    layoutRef.current = null;
    motionRef.current = null;
    hasRecordedRef.current = false;
  }, []);
}
