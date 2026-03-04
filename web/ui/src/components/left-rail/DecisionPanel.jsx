import { useState, useCallback } from "react";
import { useGame } from "@/context/GameContext";
import DecisionRouter from "@/components/decisions/DecisionRouter";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { scryfallImageUrl } from "@/lib/scryfall";
import { ManaCostIcons, SymbolText } from "@/lib/mana-symbols";

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
  const topName = topOfStack?.name || (topOfStack ? `Object#${topOfStack.id}` : "");
  const isCastStackEntry = !!topOfStack && !topOfStack.ability_kind;
  const topPt = topOfStack?.power_toughness
    || (topOfStack?.power != null && topOfStack?.toughness != null
      ? `${topOfStack.power}/${topOfStack.toughness}`
      : null);

  const handleCancel = useCallback(() => {
    setCancelling(true);
    setTimeout(() => {
      cancelDecision();
      setCancelling(false);
    }, 350);
  }, [cancelDecision]);

  return (
    <section className="relative h-full min-h-0 overflow-hidden flex flex-col gap-1 p-1.5">
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
        <div className="shrink-0 relative rounded overflow-hidden min-h-[80px]">
          {scryfallImageUrl(topOfStack.name, "art_crop") && (
            <img
              className="absolute inset-0 w-full h-full object-cover opacity-72 saturate-[1.05] contrast-[1.04] z-0 pointer-events-none"
              src={scryfallImageUrl(topOfStack.name, "art_crop")}
              alt=""
              loading="lazy"
              referrerPolicy="no-referrer"
            />
          )}
          <div
            className="absolute inset-x-0 bottom-0 z-[1] p-1.5 pt-3"
            style={{
              background: "linear-gradient(to bottom, transparent 0%, rgba(0,0,0,0.7) 30%, rgba(0,0,0,0.88) 100%)",
            }}
          >
            <div className="flex items-start gap-1">
              <span className="block text-[13px] font-bold text-[#d8e8ff] leading-[1.12] text-shadow-[0_1px_2px_rgba(0,0,0,0.95)] flex-1 min-w-0 break-words">
                {topName}
              </span>
              {isCastStackEntry && topOfStack.mana_cost && (
                <span className="shrink-0 mt-px">
                  <ManaCostIcons cost={topOfStack.mana_cost} />
                </span>
              )}
            </div>
            {isCastStackEntry && topPt && (
              <div className="flex items-center mt-0.5">
                <span className="ml-auto text-[#f5d08b] text-[13px] font-bold tracking-wide shrink-0">
                  {topPt}
                </span>
              </div>
            )}
            {topOfStack.ability_kind && (
              <span className="block text-[12px] italic text-[#c0a060] pt-0.5 leading-tight">
                {topOfStack.ability_kind} ability
              </span>
            )}
          </div>
        </div>
      )}

      {/* Effect text between image and decision buttons */}
      {topOfStack?.effect_text && (
        <div className="text-[12px] text-[#8ab4e0] leading-snug px-1.5 shrink-0">
          <SymbolText text={topOfStack.effect_text} />
        </div>
      )}

      <div className="flex-1 min-h-0 overflow-hidden">
        <div
          className="flex h-full min-h-0 flex-col gap-1 pr-0.5"
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
      </div>
    </section>
  );
}
