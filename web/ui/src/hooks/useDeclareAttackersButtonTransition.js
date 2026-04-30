import { useEffect, useRef, useState } from "react";

export const DECLARE_ATTACKERS_BUTTON_TRANSITION_MS = 500;

export default function useDeclareAttackersButtonTransition(decision) {
  const [locked, setLocked] = useState(() => decision?.kind === "attackers");
  const wasAttackersRef = useRef(false);
  const timerRef = useRef(null);
  const startFrameRef = useRef(null);

  useEffect(() => {
    const isAttackers = decision?.kind === "attackers";
    const enteredAttackers = isAttackers && !wasAttackersRef.current;
    wasAttackersRef.current = isAttackers;

    if (!enteredAttackers) {
      if (!isAttackers && locked) {
        startFrameRef.current = window.requestAnimationFrame(() => {
          setLocked(false);
          startFrameRef.current = null;
        });
      }
      return undefined;
    }

    if (timerRef.current) {
      clearTimeout(timerRef.current);
      timerRef.current = null;
    }
    if (startFrameRef.current) {
      window.cancelAnimationFrame(startFrameRef.current);
      startFrameRef.current = null;
    }

    if (!locked) {
      startFrameRef.current = window.requestAnimationFrame(() => {
        setLocked(true);
        startFrameRef.current = null;
      });
    }
    timerRef.current = window.setTimeout(() => {
      setLocked(false);
      timerRef.current = null;
    }, DECLARE_ATTACKERS_BUTTON_TRANSITION_MS);

    return undefined;
  }, [decision?.kind, locked]);

  useEffect(() => () => {
    if (timerRef.current) {
      clearTimeout(timerRef.current);
      timerRef.current = null;
    }
    if (startFrameRef.current) {
      window.cancelAnimationFrame(startFrameRef.current);
      startFrameRef.current = null;
    }
  }, []);

  return {
    locked,
    transitioning: locked,
  };
}
