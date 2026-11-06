import { Clock3 } from "lucide-react";
import { useGame, useMatchClock } from "@/context/GameContext";
import { samePlayerId } from "@/lib/player-display";
import { cn } from "@/lib/utils";

function formatClock(ms) {
  const totalSeconds = Math.max(0, Math.ceil(Number(ms || 0) / 1000));
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;
  const pad = (value) => String(value).padStart(2, "0");
  return hours > 0 ? `${hours}:${pad(minutes)}:${pad(seconds)}` : `${minutes}:${pad(seconds)}`;
}

// One player's match clock. Renders nothing outside a clocked multiplayer
// match. Subscribes to the clock store directly so ticking never republishes
// board state.
export default function PlayerClock({ player, className }) {
  const { multiplayer } = useGame();
  const clock = useMatchClock();
  if (!multiplayer?.matchStarted || !clock?.enabled || !Array.isArray(clock.remainingMsByPlayer)) return null;
  const seat = Number(player?.id ?? player?.index);
  if (!Number.isInteger(seat) && !samePlayerId(player?.id, player?.index)) return null;
  const remainingMs = clock.remainingMsByPlayer[seat];
  if (remainingMs == null) return null;
  const active = Number(clock.activePlayerIndex ?? clock.currentPlayerIndex) === seat;
  const expired = Number(remainingMs) <= 0;
  return (
    <span className={cn("player-clock", active && "player-clock--active", expired && "player-clock--expired", className)}
      data-player-clock={seat} aria-live="off">
      <Clock3 className="player-clock__icon" aria-hidden="true" />
      <span className="player-clock__time">{formatClock(remainingMs)}</span>
    </span>
  );
}
