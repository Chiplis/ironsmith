import { useEffect, useRef } from "react";
import { useGame } from "@/context/GameContext";
import { useI18n } from "@/i18n/I18nContext";
import { samePlayerId } from "@/lib/player-display";

const FLASH_INTERVAL_MS = 1000;

// Flashes the document title while the tab is hidden (or the window is
// unfocused) and the local player holds the pending decision, so a player who
// browsed away notices it is their turn to act. Browsers keep rendering tab
// titles for background tabs, so this works after switching tabs; it cannot
// reach a fully minimized browser (that would need the Notifications API).
export default function useTabAttention() {
  const { state, multiplayer } = useGame();
  const { t } = useI18n();
  const baseTitleRef = useRef(typeof document !== "undefined" ? document.title : "");

  const decision = state?.decision;
  const localPlayerIndex = multiplayer?.localPlayerIndex ?? state?.perspective;
  const needsAttention = Boolean(
    multiplayer?.matchStarted
    && !state?.game_over
    && decision
    && samePlayerId(decision.player, localPlayerIndex)
    && !multiplayer?.peerWait
  );

  useEffect(() => {
    if (typeof document === "undefined" || !needsAttention) return undefined;

    const baseTitle = baseTitleRef.current || document.title;
    const alertTitle = `⚡ ${t("tab.yourMove")}`;
    let flashTimer = null;

    const stopFlashing = () => {
      if (flashTimer) {
        clearInterval(flashTimer);
        flashTimer = null;
      }
      document.title = baseTitle;
    };

    const startFlashing = () => {
      if (flashTimer) return;
      document.title = alertTitle;
      flashTimer = setInterval(() => {
        document.title = document.title === alertTitle ? baseTitle : alertTitle;
      }, FLASH_INTERVAL_MS);
    };

    const sync = () => {
      if (document.hidden || !document.hasFocus()) {
        startFlashing();
      } else {
        stopFlashing();
      }
    };

    sync();
    document.addEventListener("visibilitychange", sync);
    window.addEventListener("focus", sync);
    window.addEventListener("blur", sync);
    return () => {
      document.removeEventListener("visibilitychange", sync);
      window.removeEventListener("focus", sync);
      window.removeEventListener("blur", sync);
      stopFlashing();
    };
  }, [needsAttention, t]);
}
