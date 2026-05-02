import { useCallback } from "react";
import { useGame } from "@/context/GameContext";
import { getPlayerAccent } from "@/lib/player-colors";
import { usePointerClickGuard } from "@/lib/usePointerClickGuard";
import useMobileLongPress from "@/hooks/useMobileLongPress";
import { cn } from "@/lib/utils";
import MobileZoneTray from "./MobileZoneTray";

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

export default function MobileSelfHud({
  me,
  onTap,
  onLongPress,
  onOpenZone,
  targetable = false,
  manaPool = null,
  className,
}) {
  const { state } = useGame();
  const { registerPointerDown, shouldHandleClick } = usePointerClickGuard();
  const accent = getPlayerAccent(state?.players || [], me?.id, state?.perspective);
  const isActiveTurn = me?.id === state?.active_player;

  const handleLongPress = useCallback(() => {
    onLongPress?.(me);
  }, [onLongPress, me]);
  const longPress = useMobileLongPress({ onLongPress: handleLongPress });

  const handleTap = useCallback((event) => {
    if (longPress.consumeTrigger()) return;
    if (!shouldHandleClick(event)) return;
    onTap?.(me);
  }, [longPress, me, onTap, shouldHandleClick]);

  if (!me) {
    return <footer className={cn("mobile-mtga-self-hud", className)} aria-hidden="true" />;
  }

  return (
    <footer
      className={cn("mobile-mtga-self-hud", className)}
      data-active-turn={isActiveTurn ? "true" : "false"}
    >
      <button
        type="button"
        className={cn(
          "mobile-mtga-self-hud-body",
          targetable && "mobile-mtga-self-hud-body--targetable"
        )}
        data-player-target={me.index ?? me.id}
        data-player-target-name={me.id ?? me.index}
        style={accent ? { "--player-accent": accent.hex } : undefined}
        onPointerDown={(event) => {
          longPress.onPointerDown(event);
          registerPointerDown(event);
        }}
        onPointerMove={longPress.onPointerMove}
        onPointerUp={longPress.onPointerUp}
        onPointerCancel={longPress.onPointerCancel}
        onPointerLeave={longPress.onPointerLeave}
        onClick={handleTap}
      >
        <PlayerAvatar player={me} accentHex={accent?.hex} isActiveTurn={isActiveTurn} />
        <span className="mobile-mtga-hud-identity">
          <span className="mobile-mtga-hud-name">{me.name || "You"}</span>
          <span className="mobile-mtga-hud-life" aria-label={`Life ${me.life}`}>
            {me.life ?? 0}
          </span>
        </span>
      </button>

      {manaPool ? (
        <div className="mobile-mtga-hud-mana">
          {manaPool}
        </div>
      ) : null}

      <MobileZoneTray player={me} onOpenZone={onOpenZone} />
    </footer>
  );
}
