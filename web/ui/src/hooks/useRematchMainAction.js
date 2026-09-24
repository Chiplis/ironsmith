import { useCallback, useState } from "react";
import { useGame } from "@/context/GameContext";
import { useI18n } from "@/i18n/I18nContext";
import { rematchPlayersReady } from "@/hooks/peer-lobby/shared";

// What the main decision button does once a multiplayer game is over:
// open deck selection for the next game, mark the chosen deck ready, and --
// for the host alone, once every seat is ready -- start it.
export default function useRematchMainAction() {
  const {
    state,
    multiplayer,
    startRematchSideboarding,
    readyForRematch,
    startRematch,
  } = useGame();
  const { t } = useI18n();
  const [pending, setPending] = useState(false);
  const rematch = multiplayer?.rematch || null;
  const phase = rematch?.phase || null;
  const available = Boolean(
    (state?.game_over || phase)
    && (multiplayer?.matchStarted || multiplayer?.mode === "in_match" || phase)
    && typeof startRematchSideboarding === "function"
  );
  const isHost = multiplayer?.role === "host";
  const localReady = Boolean(rematch?.localReady);
  const allReady = rematchPlayersReady(rematch?.players);

  let label = t("game.playAgain");
  let run = startRematchSideboarding;
  let disabled = false;
  if (phase === "starting" || multiplayer?.mode === "starting") {
    label = t("game.starting");
    run = null;
    disabled = true;
  } else if (phase === "sideboarding") {
    if (!localReady) {
      label = pending ? t("game.submittingDeck") : t("game.ready");
      run = readyForRematch;
    } else if (isHost && allReady) {
      label = t("game.startNextGame");
      run = startRematch;
    } else {
      label = isHost ? t("game.waitingPlayers") : t("game.waitingHost");
      run = null;
    }
    disabled = !run || pending;
  }

  const press = useCallback(async () => {
    if (disabled || typeof run !== "function") return;
    setPending(true);
    try {
      await run();
    } finally {
      setPending(false);
    }
  }, [disabled, run]);

  return { available, phase, label, disabled, press };
}
