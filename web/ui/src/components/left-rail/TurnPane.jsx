import { useGame } from "@/context/GameContext";
import { formatStep } from "@/lib/constants";

export default function TurnPane() {
  const { state } = useGame();
  if (!state) return null;

  const players = state.players || [];
  const activePlayer = players.find((p) => p.id === state.active_player);
  const priorityPlayer =
    state.priority_player != null
      ? players.find((p) => p.id === state.priority_player)
      : null;

  return (
    <section className="mt-auto border-t border-game-line-2 bg-[#0b121a] p-2 grid gap-1.5 content-start shrink-0">
      <h4 className="m-0 uppercase text-[12px] tracking-wider text-muted-foreground font-bold">
        Turn Summary
      </h4>
      <div className="border border-[#203247] bg-[#0a1118] p-1.5 flex flex-wrap gap-1.5 text-[12px] text-[#d3e5fb]">
        <span className="border border-[#1e3044] bg-[#0c151f] px-1.5 rounded-sm">
          Turn {state.turn_number}
        </span>
        <span className="border border-[#1e3044] bg-[#0c151f] px-1.5 rounded-sm">
          {state.phase}
        </span>
        <span className="border border-[#1e3044] bg-[#0c151f] px-1.5 rounded-sm">
          {formatStep(state.step)}
        </span>
        <span className="border border-[#1e3044] bg-[#0c151f] px-1.5 rounded-sm">
          Active: {activePlayer?.name || "?"}
        </span>
        {priorityPlayer && (
          <span className="border border-[#1e3044] bg-[#0c151f] px-1.5 rounded-sm">
            Priority: {priorityPlayer.name}
          </span>
        )}
        <span className="border border-[#1e3044] bg-[#0c151f] px-1.5 rounded-sm">
          Stack: {state.stack_size}
        </span>
      </div>
    </section>
  );
}
