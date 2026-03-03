import { useGame } from "@/context/GameContext";
import DecisionRouter from "@/components/decisions/DecisionRouter";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Button } from "@/components/ui/button";

export default function DecisionPanel() {
  const { state, cancelDecision } = useGame();
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

  const showCancel = canAct && state?.cancelable;

  return (
    <section className="flex-1 min-h-0 overflow-hidden flex flex-col gap-1.5">
      <div className="flex items-baseline gap-1.5 shrink-0">
        <h3 className="m-0 text-sm font-bold whitespace-nowrap">Decision</h3>
        <span className="text-muted-foreground text-[14px] truncate">{metaText}</span>
      </div>

      {showCancel && (
        <Button
          variant="outline"
          size="sm"
          className="h-auto min-h-7 py-1.5 text-[14px] px-3 shrink-0 border-[#f76969]/50 text-[#f76969] hover:bg-[#f76969]/10 hover:text-[#f76969]"
          onClick={cancelDecision}
        >
          Cancel
        </Button>
      )}

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

    </section>
  );
}
