import { useGame } from "@/context/GameContext";
import PlayerCard from "./PlayerCard";

export default function PlayersPane() {
  const { state } = useGame();
  if (!state?.players?.length) return null;

  const players = state.players;
  const perspective = state.perspective;
  const me = players.find((p) => p.id === perspective) || players[0];

  // Order: perspective player first, then others in seat order
  const meIndex = players.findIndex((p) => p.id === me.id);
  const ordered =
    meIndex >= 0
      ? [...players.slice(meIndex), ...players.slice(0, meIndex)]
      : players;

  return (
    <section className="grid gap-2 content-start overflow-auto min-h-0 shrink-0">
      {ordered.map((player, idx) => (
        <PlayerCard
          key={player.id}
          player={player}
          isActive={state.active_player === player.id}
          isPerspective={player.id === me.id}
          label={player.id === me.id ? "You" : `Opponent ${idx}`}
        />
      ))}
    </section>
  );
}
