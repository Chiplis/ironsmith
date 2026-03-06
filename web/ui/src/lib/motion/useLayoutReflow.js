import { useEffect, useLayoutEffect, useRef } from "react";
import { createLayout, uiSpring, cancelMotion } from "@/lib/motion/anime";

export default function useLayoutReflow(rootRef, signature, options = {}) {
  const {
    children = ".game-card",
    disabled = false,
    duration = 360,
    bounce = 0.14,
    delay,
    enterFrom,
    leaveTo,
  } = options;
  const layoutRef = useRef(null);
  const motionRef = useRef(null);
  const hasRecordedRef = useRef(false);
  const settleFrameRef = useRef(0);
  const paramsRef = useRef({ delay, enterFrom, leaveTo });

  useLayoutEffect(() => {
    paramsRef.current = { delay, enterFrom, leaveTo };
  }, [delay, enterFrom, leaveTo]);

  useLayoutEffect(() => {
    const root = rootRef.current;
    if (!root) return undefined;

    if (!layoutRef.current) {
      layoutRef.current = createLayout(root, { children });
    }

    const layout = layoutRef.current;
    if (disabled) {
      cancelMotion(motionRef.current);
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
    cancelMotion(motionRef.current);
    const params = paramsRef.current;
    motionRef.current = layout.animate({
      delay: params.delay,
      duration,
      ease: uiSpring({ duration, bounce }),
      enterFrom: params.enterFrom,
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
        cancelMotion(currentMotion);
        motionRef.current = null;
      }
    };
  }, [bounce, children, disabled, duration, rootRef, signature]);

  useEffect(() => () => {
    cancelAnimationFrame(settleFrameRef.current);
    cancelMotion(motionRef.current);
    layoutRef.current?.revert();
    layoutRef.current = null;
    motionRef.current = null;
    hasRecordedRef.current = false;
  }, []);
}
