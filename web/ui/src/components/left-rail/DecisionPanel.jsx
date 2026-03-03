import { useGame } from "@/context/GameContext";
import DecisionRouter from "@/components/decisions/DecisionRouter";
import { ScrollArea } from "@/components/ui/scroll-area";

export default function DecisionPanel() {
  const { state, status } = useGame();
  const decision = state?.decision;
  const players = state?.players || [];
  const perspective = state?.perspective;
  const canAct = decision && decision.player === perspective;

  const decisionPlayer = decision
    ? players.find((p) => p.id === decision.player)
    : null;

  const metaText = decision
    ? `${decisionPlayer?.name || "?"} · ${decision.kind}`
    : "No pending decision";

  return (
    <section className="flex-1 min-h-0 overflow-hidden flex flex-col gap-1.5">
      <div className="flex items-baseline gap-1.5 shrink-0">
        <h3 className="m-0 text-sm font-bold whitespace-nowrap">Decision</h3>
        <span className="text-muted-foreground text-[14px] truncate">{metaText}</span>
      </div>

      <ScrollArea className="flex-1 min-h-0">
        <div className="flex flex-col gap-1.5 p-1.5">
          {decision ? (
            <DecisionRouter decision={decision} canAct={canAct} />
          ) : (
            <div className="text-muted-foreground text-[16px] italic p-2">
              Waiting for decision...
            </div>
          )}
        </div>
      </ScrollArea>

      <div
        className="text-[14px] shrink-0 px-1 py-0.5 truncate"
        style={{ color: status.isError ? "#ffb5c5" : "#d5e4f8" }}
      >
        {status.msg}
      </div>
    </section>
  );
}
