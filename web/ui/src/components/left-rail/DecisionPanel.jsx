import { useState, useCallback } from "react";
import { useGame } from "@/context/GameContext";
import DecisionRouter from "@/components/decisions/DecisionRouter";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { scryfallImageUrl } from "@/lib/scryfall";
import { SymbolText } from "@/lib/mana-symbols";

export default function DecisionPanel() {
  const { state, cancelDecision, holdRule, setHoldRule } = useGame();
  const [cancelling, setCancelling] = useState(false);
  const decision = state?.decision;
  const players = state?.players || [];
  const perspective = state?.perspective;
  const canAct = decision && decision.player === perspective;

  const decisionPlayer = decision
    ? players.find((p) => p.id === decision.player)
    : null;

  const metaText = decision
    ? `${decisionPlayer?.name || "?"} · ${decision.kind}`
    : "No pending action";

  const showCancel = canAct && state?.cancelable;
  const stackObjects = state?.stack_objects || [];
  const topOfStack = stackObjects.length > 0 ? stackObjects[0] : null;

  const handleCancel = useCallback(() => {
    setCancelling(true);
    setTimeout(() => {
      cancelDecision();
      setCancelling(false);
    }, 350);
  }, [cancelDecision]);

  return (
    <section className="relative shrink-0 overflow-hidden flex flex-col gap-1 p-1.5">
      {/* Cancel flash overlay */}
      {cancelling && (
        <div
          className="absolute inset-0 z-10 pointer-events-none rounded"
          style={{ animation: "cancel-flash 350ms ease-out forwards" }}
        />
      )}

      <div className="flex items-center gap-1 shrink-0 flex-wrap">
        <h3 className="m-0 text-[12px] font-bold whitespace-nowrap uppercase tracking-wider text-[#8ec4ff]">Action</h3>
        <span className="text-muted-foreground text-[11px] truncate flex-1 min-w-0">{metaText}</span>
        <div className="flex items-center gap-1">
          {showCancel && (
            <Button
              variant="ghost"
              size="sm"
              className="h-5 py-0 text-[11px] px-1.5 shrink-0 text-[#f76969]/60 hover:text-[#f76969] hover:bg-[#f76969]/10 hover:shadow-[0_0_8px_rgba(247,105,105,0.15)] transition-all"
              disabled={cancelling}
              onClick={handleCancel}
            >
              Cancel
            </Button>
          )}
          <label className="flex items-center gap-1 shrink-0 text-[11px] uppercase tracking-wider cursor-pointer text-muted-foreground hover:text-foreground transition-colors">
            <Checkbox
              checked={holdRule === "always"}
              onCheckedChange={(v) => setHoldRule(v ? "always" : "never")}
              className="h-3 w-3"
            />
            Hold
          </label>
        </div>
      </div>

      {/* Top of stack */}
      {topOfStack && (
        <div className="shrink-0 relative rounded overflow-hidden" style={{ minHeight: 48 }}>
          {scryfallImageUrl(topOfStack.name, "art_crop") && (
            <img
              className="absolute inset-0 w-full h-full object-cover opacity-60 z-0 pointer-events-none"
              src={scryfallImageUrl(topOfStack.name, "art_crop")}
              alt=""
              loading="lazy"
              referrerPolicy="no-referrer"
            />
          )}
          <div className="relative z-[1] px-1.5 py-1 flex flex-col gap-0.5" style={{ background: "linear-gradient(to right, rgba(10,15,22,0.88) 0%, rgba(10,15,22,0.6) 100%)" }}>
            <span className="text-[13px] font-bold text-[#d8e8ff] leading-tight text-shadow-[0_1px_2px_rgba(0,0,0,0.9)]">
              {topOfStack.name}
            </span>
            {topOfStack.effect_text && (
              <span className="text-[11px] text-[#8ab4e0] leading-tight overflow-hidden text-ellipsis" style={{ display: "-webkit-box", WebkitLineClamp: 2, WebkitBoxOrient: "vertical" }}>
                <SymbolText text={topOfStack.effect_text} />
              </span>
            )}
            {topOfStack.ability_kind && !topOfStack.effect_text && (
              <span className="text-[11px] italic text-[#c0a060] leading-tight">
                {topOfStack.ability_kind} ability
              </span>
            )}
          </div>
        </div>
      )}

      <ScrollArea className="min-h-0" style={{ maxHeight: "40vh" }}>
        <div
          className="flex flex-col gap-1 pr-0.5"
          style={cancelling ? { animation: "cancel-slide-out 350ms ease-in forwards" } : undefined}
        >
          {decision ? (
            <DecisionRouter decision={decision} canAct={canAct} />
          ) : (
            <div className="text-muted-foreground text-[13px] italic px-1">
              Waiting...
            </div>
          )}
        </div>
      </ScrollArea>
    </section>
  );
}
