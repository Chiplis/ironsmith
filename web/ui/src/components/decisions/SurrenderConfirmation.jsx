import { useState } from "react";
import { useGame } from "@/context/GameContext";
import { useI18n } from "@/i18n/I18nContext";
import { Button } from "@/components/ui/button";
import { playerDisplayName, samePlayerId } from "@/lib/player-display";

export default function SurrenderConfirmation() {
  const { state, multiplayer, submitMultiplayerCommand, setSurrenderRequested, setStatus } = useGame();
  const { t } = useI18n();
  const [submitting, setSubmitting] = useState(false);
  const surrender = async () => {
    if (submitting) return;
    const seat = multiplayer?.localPlayerIndex;
    const player = state?.players?.find(entry => samePlayerId(entry.id ?? entry.index, seat));
    if (seat == null || state?.game_over || player?.has_lost || player?.has_left_game) {
      setSurrenderRequested(false);
      return;
    }
    setSubmitting(true);
    try {
      await submitMultiplayerCommand({ type: "forfeit_player", player: Number(seat), reason: "surrender" },
        `${playerDisplayName(state?.players, player)} surrendered`);
    } catch (error) {
      setStatus?.(`Surrender failed: ${error?.message || error}`, true);
    } finally {
      setSubmitting(false);
      setSurrenderRequested(false);
    }
  };
  return (
    <div className="surrender-confirmation flex h-full min-h-10 w-full items-center gap-2 p-1" role="group" aria-label={t("action.surrenderConfirm")}>
      <span className="min-w-0 flex-1 text-[13px] font-bold text-[#ffaaaa]">{t("action.surrenderConfirm")}</span>
      <Button className="decision-neon-button surrender-confirm-yes min-h-10 min-w-16" disabled={submitting} onClick={surrender}>{t("decision.yes")}</Button>
      <Button className="decision-neon-button surrender-confirm-no min-h-10 min-w-16" disabled={submitting} onClick={() => setSurrenderRequested(false)}>{t("decision.no")}</Button>
    </div>
  );
}
