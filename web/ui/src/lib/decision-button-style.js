import { useEffect, useRef, useState } from "react";
import { getPlayerAccent, hexToRgbString } from "./player-colors.js";

const LOCAL_DECISION_ACCENT = "#731bde";

// How long a new accent must stay put before the button adopts it. Multiplayer
// echoes can bounce the pending decision across players several times within a
// frame or two (auto-passes, action round-trips); committing instantly makes
// the main decision button strobe through every seat color.
const ACCENT_COMMIT_DELAY_MS = 140;

export function decisionButtonPlayerId(state, decision = state?.decision) {
  return (
    decision?.player
    ?? state?.priority_player
    ?? state?.perspective
    ?? null
  );
}

export function decisionButtonAccentVars(state, decision = state?.decision, accentOverrides = null) {
  const playerId = decisionButtonPlayerId(state, decision);
  if (Number(playerId) === Number(state?.perspective)) {
    return {
      "--decision-main-accent": LOCAL_DECISION_ACCENT,
      "--decision-main-rgb": hexToRgbString(LOCAL_DECISION_ACCENT),
    };
  }
  const accent = getPlayerAccent(state?.players || [], playerId, state?.perspective, accentOverrides);
  if (!accent) return undefined;

  return {
    "--decision-main-accent": accent.hex,
    "--decision-main-rgb": accent.rgb,
  };
}

// Debounced accent for the main decision button: holds the last committed
// accent while none is resolvable (instead of flashing the CSS default), and
// only adopts a different accent once it has been stable for a beat.
export function useDecisionButtonAccent(state, decision = state?.decision, accentOverrides = null) {
  const style = decisionButtonAccentVars(state, decision, accentOverrides);
  const local = isLocalDecisionButton(state, decision);
  const key = style ? `${style["--decision-main-accent"]}|${local ? "1" : "0"}` : null;

  const [committed, setCommitted] = useState(() => ({ key, style, local }));
  const latestRef = useRef(null);
  latestRef.current = { key, style, local };
  const timerRef = useRef(null);

  useEffect(() => {
    // Unresolvable accent (no state yet, players missing): keep showing the
    // last committed colors rather than snapping to the stylesheet fallback.
    if (key === null || key === committed.key) {
      if (timerRef.current) {
        clearTimeout(timerRef.current);
        timerRef.current = null;
      }
      return undefined;
    }
    if (committed.key === null) {
      setCommitted(latestRef.current);
      return undefined;
    }
    if (timerRef.current) clearTimeout(timerRef.current);
    timerRef.current = setTimeout(() => {
      timerRef.current = null;
      setCommitted(latestRef.current);
    }, ACCENT_COMMIT_DELAY_MS);
    return undefined;
  }, [key, committed.key]);

  useEffect(() => () => {
    if (timerRef.current) clearTimeout(timerRef.current);
  }, []);

  return { style: committed.style, isLocal: committed.local };
}

export function isLocalDecisionButton(state, decision = state?.decision) {
  const perspective = Number(state?.perspective);
  if (!Number.isFinite(perspective)) return false;

  const decisionPlayer = Number(decision?.player);
  if (Number.isFinite(decisionPlayer) && decisionPlayer === perspective) return true;
  if (Number.isFinite(decisionPlayer)) return false;

  const priorityPlayer = Number(state?.priority_player);
  return Number.isFinite(priorityPlayer) && priorityPlayer === perspective;
}
