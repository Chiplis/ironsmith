import { Flag } from "lucide-react";
import { useGame } from "@/context/GameContext";
import { useI18n } from "@/i18n/I18nContext";
import { samePlayerId } from "@/lib/player-display";

export default function SurrenderButton({ className = "stone-pill px-3 py-2", onClick }) {
  const { state, multiplayer, setSurrenderRequested } = useGame();
  const { t } = useI18n();
  const seat = multiplayer?.localPlayerIndex;
  const player = state?.players?.find((entry) => samePlayerId(entry.id ?? entry.index, seat));
  const unavailable = seat == null || state?.game_over
    || player?.has_lost || player?.hasLost || player?.has_left_game || player?.hasLeftGame;
  return (
    <button
      type="button"
      className={`${className} inline-flex items-center justify-center gap-2 surrender-button`}
      disabled={Boolean(unavailable)}
      onClick={() => {
        setSurrenderRequested(true);
        onClick?.();
      }}
    >
      <Flag className="size-3.5" />
      {t("action.surrender")}
    </button>
  );
}
