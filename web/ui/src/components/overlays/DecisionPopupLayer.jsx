import { useGame } from "@/context/GameContext";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { cn } from "@/lib/utils";

function clamp(value, min, max) {
  return Math.min(max, Math.max(min, value));
}

function priorityAnchorStyle(anchor) {
  if (!anchor || !Number.isFinite(anchor.x) || !Number.isFinite(anchor.y)) return null;
  const viewportWidth = typeof window !== "undefined" ? window.innerWidth : 1280;
  const viewportHeight = typeof window !== "undefined" ? window.innerHeight : 720;
  const width = Math.min(348, viewportWidth - 16);
  const left = clamp(anchor.x - (width * 0.5), 8, viewportWidth - width - 8);
  const top = clamp(anchor.y - 124, 74, viewportHeight - 102);
  return { left: `${left}px`, top: `${top}px`, width: `${width}px` };
}

function nextStepLabel(phase, step, stackSize) {
  if (stackSize > 0) return "Resolve";
  switch (step) {
    case "Untap": return "Upkeep";
    case "Upkeep": return "Draw";
    case "Draw": return "Main Phase";
    case "BeginCombat": return "Attackers";
    case "DeclareAttackers": return "Blockers";
    case "DeclareBlockers": return "Damage";
    case "CombatDamage": return "End Combat";
    case "EndCombat": return "Main 2";
    case "End": return "Cleanup";
    case "Cleanup": return "Next Turn";
    default: break;
  }
  switch (phase) {
    case "FirstMain": return "Combat";
    case "NextMain": return "End Step";
    case "Ending": return "Cleanup";
    default: return "Next";
  }
}

function PriorityBar({ anchor = null, inline = false }) {
  const { state, dispatch, holdRule, setHoldRule } = useGame();
  const decision = state?.decision;
  const canAct = decision && state?.perspective === decision.player;
  const passAction = (decision?.actions || []).find((action) => action.kind === "pass_priority");
  const otherActions = (decision?.actions || []).filter((action) => action.kind !== "pass_priority");
  if (!decision || decision.kind !== "priority" || !passAction) return null;

  const anchoredStyle = inline ? null : priorityAnchorStyle(anchor);
  const stackSize = Number(state?.stack_size || 0);
  const passLabel = holdRule === "always"
    ? (passAction.label || "Pass priority")
    : `→ ${nextStepLabel(state?.phase, state?.step, stackSize)}`;

  if (inline) {
    return (
      <div className="pointer-events-none absolute inset-0 z-[120] flex items-center px-2">
        <div
          className="priority-inline-panel pointer-events-auto flex w-full items-center gap-2 rounded border border-[#305071] bg-[rgba(7,15,23,0.97)] px-2 py-1.5 shadow-[0_12px_28px_rgba(0,0,0,0.45)] backdrop-blur-[2px]"
        >
          <div className="shrink-0 min-w-[110px]">
            <div className="text-[11px] font-bold uppercase tracking-[0.14em] text-[#93c7ff]">
              {canAct ? "Your Action" : "Opponent Priority"}
            </div>
            {otherActions.length > 0 && canAct && (
              <div className="mt-0.5 text-[11px] text-[#d2e5fb]">
                {otherActions.length} available action{otherActions.length === 1 ? "" : "s"}
              </div>
            )}
          </div>
          <Button
            variant="ghost"
            size="sm"
            className="h-8 flex-1 justify-start rounded border border-[#546c86] bg-[rgba(15,27,40,0.92)] px-3 text-[14px] font-bold text-[#f7b869] transition-all hover:border-[#8ca8c7] hover:bg-[rgba(28,43,58,0.95)] hover:text-[#ffd49d]"
            disabled={!canAct}
            onClick={() =>
              dispatch(
                { type: "priority_action", action_index: passAction.index },
                passAction.label
              )
            }
          >
            {passLabel}
          </Button>
          <label className="flex items-center gap-1 shrink-0 text-[11px] uppercase tracking-wider cursor-pointer text-[#9db7d5] hover:text-[#d7e8fb] transition-colors">
            <Checkbox
              checked={holdRule === "always"}
              onCheckedChange={(v) => setHoldRule(v ? "always" : "never")}
              className="h-3 w-3"
            />
            Hold
          </label>
        </div>
      </div>
    );
  }

  return (
    <div
      className={cn(
        "pointer-events-auto z-[120] rounded border border-[#305071] bg-[rgba(7,15,23,0.97)] shadow-[0_16px_36px_rgba(0,0,0,0.55)] backdrop-blur-[2px]",
        anchoredStyle
          ? "fixed"
          : "fixed left-2 bottom-[148px] w-[min(92vw,348px)]"
      )}
      style={anchoredStyle || undefined}
    >
      <div className="border-b border-[#2f4662]/85 bg-[rgba(10,22,34,0.88)] px-2 py-1.5">
        <div className="text-[11px] font-bold uppercase tracking-[0.14em] text-[#93c7ff]">
          {canAct ? "Your Action" : "Opponent Priority"}
        </div>
        {otherActions.length > 0 && canAct && (
          <div className="mt-0.5 text-[12px] text-[#d2e5fb]">
            {otherActions.length} available action{otherActions.length === 1 ? "" : "s"}
          </div>
        )}
      </div>
      <div className="flex items-center gap-2 px-2 py-2">
        <Button
          variant="ghost"
          size="sm"
          className="h-8 flex-1 justify-start rounded border border-[#546c86] bg-[rgba(15,27,40,0.92)] px-3 text-[14px] font-bold text-[#f7b869] transition-all hover:border-[#8ca8c7] hover:bg-[rgba(28,43,58,0.95)] hover:text-[#ffd49d]"
          disabled={!canAct}
          onClick={() =>
            dispatch(
              { type: "priority_action", action_index: passAction.index },
              passAction.label
            )
          }
        >
          {passLabel}
        </Button>
        <label className="flex items-center gap-1 shrink-0 text-[11px] uppercase tracking-wider cursor-pointer text-[#9db7d5] hover:text-[#d7e8fb] transition-colors">
          <Checkbox
            checked={holdRule === "always"}
            onCheckedChange={(v) => setHoldRule(v ? "always" : "never")}
            className="h-3 w-3"
          />
          Hold
        </label>
      </div>
    </div>
  );
}

export default function DecisionPopupLayer({ anchor = null, priorityInline = false }) {
  const { state } = useGame();
  const decision = state?.decision || null;

  if (!decision) return null;
  if (decision.kind !== "priority") return null;
  return <PriorityBar anchor={anchor} inline={priorityInline} />;
}
