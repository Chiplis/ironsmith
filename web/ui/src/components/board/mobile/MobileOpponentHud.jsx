import useUiText from "@/i18n/useUiText";
import { useCastPlayerHovered } from "@/context/DragContext";
import { useCallback } from "react";
import { ChevronLeft, ChevronRight } from "lucide-react";
import { useGame } from "@/context/GameContext";
import { getPlayerAccent } from "@/lib/player-colors";
import useMobileLongPress from "@/hooks/useMobileLongPress";
import { cn } from "@/lib/utils";
import { playerDisplayName } from "@/lib/player-display";

function PlayerAvatar({ player, accentHex, isActiveTurn }) {
  const initial = String(player?.name || "?").charAt(0).toUpperCase();
  return (
    <span
      className={cn(
        "mobile-mtga-avatar",
        isActiveTurn && "mobile-mtga-avatar--active-turn"
      )}
      style={accentHex ? { "--player-accent": accentHex } : undefined}
      aria-hidden="true"
    >
      <span className="mobile-mtga-avatar-glyph">{initial}</span>
    </span>
  );
}

export default function MobileOpponentHud({
  opponent,
  cycleEnabled = false,
  previousOpponent,
  nextOpponent,
  onCyclePrev,
  onCycleNext,
  onTap,
  onLongPress,
  targetable = false,
  trailing = null,
  className,
}) {
  const ui = useUiText();
  const { state, playerAccentOverrides } = useGame();
  const castPlayerHovered = useCastPlayerHovered(opponent?.index ?? opponent?.id);
  const accent = getPlayerAccent(state?.players || [], opponent?.id, state?.perspective, playerAccentOverrides);
  const isActiveTurn = opponent?.id === state?.active_player;

  const handleLongPress = useCallback(() => {
    onLongPress?.(opponent);
  }, [onLongPress, opponent]);
  const longPress = useMobileLongPress({ onLongPress: handleLongPress });

  const handleTap = useCallback(() => {
    if (longPress.consumeTrigger()) return;
    onTap?.(opponent);
  }, [longPress, onTap, opponent]);

  if (!opponent) {
    return (
      <header className={cn("mobile-mtga-opponent-hud", className)}>
        <div className="mobile-mtga-opponent-hud-spacer" aria-hidden="true" />
        {trailing}
      </header>
    );
  }

  return (
    <header
      className={cn("mobile-mtga-opponent-hud", className)}
      data-active-turn={isActiveTurn ? "true" : "false"}
    >
      {cycleEnabled ? (
        <button
          type="button"
          className="mobile-mtga-opponent-cycle"
          data-player-nav-target={previousOpponent?.index ?? previousOpponent?.id}
          data-player-nav-target-name={previousOpponent?.id ?? previousOpponent?.index}
          aria-label={ui("Show previous opponent")}
          onClick={onCyclePrev}
        >
          <ChevronLeft className="size-3.5" aria-hidden="true" />
        </button>
      ) : null}

      <button
        type="button"
        className={cn(
          "mobile-mtga-opponent-hud-body",
          targetable && "mobile-mtga-opponent-hud-body--targetable player-target-box"
        )}
        data-cast-hovered={targetable && castPlayerHovered ? "true" : undefined}
        data-player-target={opponent.index ?? opponent.id}
        data-player-target-name={opponent.id ?? opponent.index}
        style={accent ? { "--player-accent": accent.hex } : undefined}
        onPointerDown={onLongPress ? longPress.onPointerDown : undefined}
        onPointerMove={longPress.onPointerMove}
        onPointerUp={longPress.onPointerUp}
        onPointerCancel={longPress.onPointerCancel}
        onPointerLeave={longPress.onPointerLeave}
        onClick={handleTap}
      >
        <PlayerAvatar player={opponent} accentHex={accent?.hex} isActiveTurn={isActiveTurn} />
        <span className="mobile-mtga-hud-identity">
          <span className="mobile-mtga-hud-name">{playerDisplayName(state?.players || [], opponent)}</span>
          <span className="mobile-mtga-hud-life" aria-label={ui("Life {0}", { 0: opponent.life })}>
            {opponent.life}
          </span>
        </span>
      </button>

      {cycleEnabled ? (
        <button
          type="button"
          className="mobile-mtga-opponent-cycle"
          data-player-nav-target={nextOpponent?.index ?? nextOpponent?.id}
          data-player-nav-target-name={nextOpponent?.id ?? nextOpponent?.index}
          aria-label={ui("Show next opponent")}
          onClick={onCycleNext}
        >
          <ChevronRight className="size-3.5" aria-hidden="true" />
        </button>
      ) : null}

      {trailing ? (
        <div className="mobile-mtga-opponent-hud-trailing">{trailing}</div>
      ) : null}
    </header>
  );
}
