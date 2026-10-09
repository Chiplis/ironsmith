import { useEffect, useRef } from 'react';
import { automaticHandRevealCommand } from '@/lib/automatic-hand-reveal';
import { decisionKey } from '@/lib/decision-key';

export function useAutomaticHandReveal({ state, stateRef, dispatch, isBlocked }) {
  const sent = useRef(null);
  const inFlight = useRef(false);
  useEffect(() => {
    const command = automaticHandRevealCommand(state);
    if (!command) {
      sent.current = null;
      return;
    }
    const key = decisionKey(state.decision);
    let timer;
    let cancelled = false;
    const submit = () => {
      if (cancelled || stateRef.current !== state || sent.current === key) return;
      if (inFlight.current || isBlocked()) {
        timer = setTimeout(submit, 50);
        return;
      }
      sent.current = key;
      inFlight.current = true;
      void (async () => {
        try {
          await dispatch(command, 'Reveal hand');
        } catch {
          // Dispatch reports errors through the existing UI. Keep the manual
          // prompt available rather than repeatedly retrying a failure.
        } finally {
          inFlight.current = false;
        }
      })();
    };
    timer = setTimeout(submit, 0);
    return () => { cancelled = true; clearTimeout(timer); };
  }, [state, stateRef, dispatch, isBlocked]);
}
