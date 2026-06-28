import { useGame } from "@/context/GameContext";
import { formatPhase, formatStep } from "@/lib/constants";
import { playerDisplayName, samePlayerId } from "@/lib/player-display";

export default function TurnPane() {
  const { state } = useGame();
  if (!state) return null;

  const players = state.players || [];
  const activePlayer = players.find((p) => samePlayerId(p.id, state.active_player));
  const priorityPlayer =
    state.priority_player != null
      ? players.find((p) => samePlayerId(p.id, state.priority_player))
      : null;
  const decisionPlayer =
    state.decision?.player != null
      ? players.find((p) => samePlayerId(p.id, state.decision.player))
      : null;
  const decisionOwnerDiffersFromPriority = decisionPlayer
    && (!priorityPlayer || !samePlayerId(decisionPlayer.id, priorityPlayer.id));

  return (
    <section className="mt-auto border-t border-game-line-2 bg-[#0b121a] p-2 grid gap-1.5 content-start shrink-0">
      <h4 className="m-0 uppercase text-[12px] tracking-wider text-muted-foreground font-bold">
        Turn Summary
      </h4>
      <div className="border border-[#203247] bg-[#0a1118] p-1.5 flex flex-wrap gap-1.5 text-[12px] text-[#d3e5fb]">
        <span className="border border-[#1e3044] bg-[#0c151f] px-1.5 rounded-none">
          Turn {state.turn_number}
        </span>
        <span className="border border-[#1e3044] bg-[#0c151f] px-1.5 rounded-none">
          {formatPhase(state.phase)}
        </span>
        <span className="border border-[#1e3044] bg-[#0c151f] px-1.5 rounded-none">
          {formatStep(state.step)}
        </span>
        <span className="border border-[#1e3044] bg-[#0c151f] px-1.5 rounded-none">
          Active: {playerDisplayName(players, activePlayer)}
        </span>
        {decisionOwnerDiffersFromPriority ? (
          <span className="border border-[#1e3044] bg-[#0c151f] px-1.5 rounded-none">
            Decision: {playerDisplayName(players, decisionPlayer)}
          </span>
        ) : priorityPlayer && (
          <span className="border border-[#1e3044] bg-[#0c151f] px-1.5 rounded-none">
            Priority: {playerDisplayName(players, priorityPlayer)}
          </span>
        )}
        <span className="border border-[#1e3044] bg-[#0c151f] px-1.5 rounded-none">
          Stack: {state.stack_size}
        </span>
      </div>
    </section>
  );
}
