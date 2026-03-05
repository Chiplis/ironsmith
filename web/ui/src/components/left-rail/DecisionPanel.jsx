import { useState, useCallback, useMemo, useRef, useEffect } from "react";
import { useGame } from "@/context/GameContext";
import { useHoveredObjectId } from "@/context/HoverContext";
import DecisionRouter from "@/components/decisions/DecisionRouter";
import { normalizeDecisionText } from "@/components/decisions/decisionText";
import { SymbolText } from "@/lib/mana-symbols";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Undo2 } from "lucide-react";

const PRIORITY_ACTION_GROUPS = [
  { key: "play", label: "Play", kinds: ["play_land"] },
  { key: "cast", label: "Cast", kinds: ["cast_spell"] },
  { key: "mana", label: "Mana", kinds: ["activate_mana_ability"] },
  { key: "activate", label: "Activate", kinds: ["activate_ability"] },
];
const BATTLEFIELD_HOVER_SUPPRESSED_KINDS = new Set(["activate_mana_ability", "activate_ability"]);

function zoneLabelFromAction(zone) {
  if (!zone) return "Unknown";
  switch (String(zone).toLowerCase()) {
    case "library": return "Library";
    case "hand": return "Hand";
    case "battlefield": return "Battlefield";
    case "graveyard": return "Graveyard";
    case "exile": return "Exile";
    case "stack": return "Stack";
    case "command": return "Command Zone";
    default:
      return String(zone)
        .split(/[_\s]+/)
        .filter(Boolean)
        .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
        .join(" ");
  }
}

function formatPriorityActionLabel(action) {
  const label = action?.label || "";
  if (action?.kind === "play_land") {
    return `Play ${zoneLabelFromAction(action.from_zone)}`;
  }
  if (action?.kind === "cast_spell") {
    return `From ${zoneLabelFromAction(action.from_zone)}`;
  }
  if (action?.kind === "activate_ability" || action?.kind === "activate_mana_ability") {
    // "Activate Black Lotus: Add {R}{R}{R}." -> "Add {R}{R}{R}."
    const match = label.match(/^Activate\s+.+?:\s*(.+)$/i);
    if (match) return match[1];
  }
  return label;
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

function passButtonColor(phase, step, stackSize) {
  if (stackSize > 0) return "yellow";
  if (phase === "FirstMain" && !step) return "red";
  switch (step) {
    case "BeginCombat":
    case "DeclareAttackers":
      return "blue";
    case "DeclareBlockers":
    case "CombatDamage":
      return "orange";
    default:
      return "yellow";
  }
}

const PASS_COLORS = {
  yellow: {
    text: "#f7b869",
    border: "rgba(247,184,105,0.45)",
    glow: "rgba(247,184,105,0.3)",
    glowOuter: "rgba(247,184,105,0.12)",
    glowInner: "rgba(247,184,105,0.1)",
  },
  red: {
    text: "#f76969",
    border: "rgba(247,105,105,0.45)",
    glow: "rgba(247,105,105,0.3)",
    glowOuter: "rgba(247,105,105,0.12)",
    glowInner: "rgba(247,105,105,0.1)",
  },
  blue: {
    text: "#69b5f7",
    border: "rgba(105,181,247,0.45)",
    glow: "rgba(105,181,247,0.3)",
    glowOuter: "rgba(105,181,247,0.12)",
    glowInner: "rgba(105,181,247,0.1)",
  },
  orange: {
    text: "#f7a040",
    border: "rgba(247,160,64,0.45)",
    glow: "rgba(247,160,64,0.3)",
    glowOuter: "rgba(247,160,64,0.12)",
    glowInner: "rgba(247,160,64,0.1)",
  },
};

function isBattlefieldObject(players, hoveredObjectId) {
  if (hoveredObjectId == null) return false;
  const hoveredId = String(hoveredObjectId);

  for (const player of players || []) {
    for (const card of player?.battlefield || []) {
      if (String(card?.id) === hoveredId) return true;
      if (Array.isArray(card?.member_ids)) {
        for (const memberId of card.member_ids) {
          if (String(memberId) === hoveredId) return true;
        }
      }
    }
  }
  return false;
}

function hoveredPriorityActionGroups(decision, hoveredObjectId, suppressBattlefieldAbilityOptions) {
  if (!decision || decision.kind !== "priority" || !hoveredObjectId) return [];

  const filtered = (decision.actions || []).filter(
    (action) =>
      action.kind !== "pass_priority"
      && action.object_id != null
      && String(action.object_id) === String(hoveredObjectId)
  );
  if (filtered.length === 0) return [];

  const visibleActions = suppressBattlefieldAbilityOptions
    ? filtered.filter((action) => !BATTLEFIELD_HOVER_SUPPRESSED_KINDS.has(action.kind))
    : filtered;
  if (visibleActions.length === 0) return [];

  const grouped = PRIORITY_ACTION_GROUPS
    .map((group) => ({
      key: group.key,
      label: group.label,
      actions: visibleActions.filter((action) => group.kinds.includes(action.kind)),
    }))
    .filter((group) => group.actions.length > 0);

  const groupedKinds = new Set(PRIORITY_ACTION_GROUPS.flatMap((group) => group.kinds));
  const otherActions = visibleActions.filter((action) => !groupedKinds.has(action.kind));
  if (otherActions.length > 0) {
    grouped.push({
      key: "other",
      label: "Other",
      actions: otherActions,
    });
  }

  return grouped;
}

export default function DecisionPanel({ inspectorOracleTextHeight = 0 }) {
  const { state, dispatch, cancelDecision, holdRule, setHoldRule } = useGame();
  const hoveredObjectId = useHoveredObjectId();
  const [cancelling, setCancelling] = useState(false);
  const [visibleHoverGroups, setVisibleHoverGroups] = useState([]);
  const hideHoverGroupsTimerRef = useRef(null);
  const decision = state?.decision;
  const players = useMemo(() => state?.players || [], [state?.players]);
  const perspective = state?.perspective;
  const canAct = decision && decision.player === perspective;

  const decisionPlayer = decision
    ? players.find((p) => p.id === decision.player)
    : null;

  const metaText = decision
    ? `${decisionPlayer?.name || "?"} · ${decision.kind}`
    : "No pending action";

  const isPriorityDecision = decision?.kind === "priority";
  const passAction = isPriorityDecision
    ? (decision.actions || []).find((action) => action.kind === "pass_priority")
    : null;
  const stackSize = state?.stack_size || 0;
  const holdingPriority = holdRule === "always";
  const passLabel = holdingPriority
    ? passAction?.label || "Pass priority"
    : `→ ${nextStepLabel(state?.phase, state?.step, stackSize)}`;
  const passColorKey = passButtonColor(state?.phase, state?.step, stackSize);
  const passColors = PASS_COLORS[passColorKey];

  const undoAvailable = !!state?.cancelable && (!decision || canAct);
  const undoDisabled = cancelling || !undoAvailable;
  const suppressBattlefieldAbilityOptions = useMemo(
    () => isBattlefieldObject(players, hoveredObjectId),
    [players, hoveredObjectId]
  );
  const hoverGroups = useMemo(
    () => hoveredPriorityActionGroups(
      decision,
      hoveredObjectId,
      suppressBattlefieldAbilityOptions
    ),
    [decision, hoveredObjectId, suppressBattlefieldAbilityOptions]
  );
  const showHoverOptions = hoverGroups.length > 0;

  useEffect(() => {
    if (hideHoverGroupsTimerRef.current) {
      clearTimeout(hideHoverGroupsTimerRef.current);
      hideHoverGroupsTimerRef.current = null;
    }

    if (showHoverOptions) {
      hideHoverGroupsTimerRef.current = setTimeout(() => {
        setVisibleHoverGroups(hoverGroups);
        hideHoverGroupsTimerRef.current = null;
      }, 0);
      return;
    }

    hideHoverGroupsTimerRef.current = setTimeout(() => {
      setVisibleHoverGroups([]);
      hideHoverGroupsTimerRef.current = null;
    }, 220);
  }, [hoverGroups, showHoverOptions]);

  useEffect(() => {
    return () => {
      if (hideHoverGroupsTimerRef.current) {
        clearTimeout(hideHoverGroupsTimerRef.current);
        hideHoverGroupsTimerRef.current = null;
      }
    };
  }, []);

  const handleCancel = useCallback(() => {
    setCancelling(true);
    setTimeout(() => {
      cancelDecision();
      setCancelling(false);
    }, 350);
  }, [cancelDecision]);

  return (
    <section className="relative z-30 flex h-full min-h-0 flex-1 flex-col overflow-visible border-t border-[#223247]/70 bg-[rgba(7,15,23,0.98)] backdrop-blur-[1.5px]">
      {/* Cancel flash overlay */}
      {cancelling && (
        <div
          className="absolute inset-0 z-10 pointer-events-none rounded"
          style={{ animation: "cancel-flash 350ms ease-out forwards" }}
        />
      )}

      <div className="relative z-20 flex h-full min-h-0 flex-1 flex-col overflow-visible">
        <div
          className="w-full min-h-0 flex-1 overflow-visible px-1.5 pt-1.5"
          style={cancelling ? { animation: "cancel-slide-out 350ms ease-in forwards" } : undefined}
        >
          {decision ? (
            <DecisionRouter
              decision={decision}
              canAct={canAct}
              inspectorOracleTextHeight={inspectorOracleTextHeight}
            />
          ) : (
            <div className="text-muted-foreground text-[13px] italic">
              Waiting...
            </div>
          )}
        </div>

        <div className="relative shrink-0 px-1.5 py-1 border-t border-[#203247]/70">
          {isPriorityDecision && (
            <div
              className={`overflow-hidden pointer-events-none transition-all duration-200 ease-out ${
                showHoverOptions
                  ? "max-h-[280px] opacity-100 translate-y-0 pb-1"
                  : "max-h-0 opacity-0 translate-y-1 pb-0"
              }`}
            >
              <div
                className="px-0 py-1 bg-[#070f17]"
              >
                {visibleHoverGroups.length > 0 && (
                  <div className="grid gap-1.5 max-h-[280px] overflow-y-auto">
                    {visibleHoverGroups.map((group, groupIndex) => (
                      <div
                        key={group.key}
                        className={groupIndex > 0 ? "pt-1 border-t border-[#2a3647]" : ""}
                      >
                        <h4 className="text-[11px] uppercase tracking-wider font-bold text-[#c6ddff]">
                          {group.label}
                        </h4>
                        <div className="grid gap-0.5 mt-0.5">
                          {group.actions.map((action) => (
                            <div key={action.index} className="text-[13px] leading-snug text-[#d6e6fb]">
                              <SymbolText text={normalizeDecisionText(formatPriorityActionLabel(action))} />
                            </div>
                          ))}
                        </div>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            </div>
          )}

          {isPriorityDecision && passAction && (
            <div className="pb-1">
              <Button
                variant="ghost"
                size="sm"
                className="group w-full h-auto min-h-7 py-1.5 text-[15px] font-bold justify-start px-3 whitespace-normal text-left transition-all duration-200 shrink-0 pass-priority-btn"
                style={{
                  color: passColors.text,
                  border: `1px solid ${passColors.border}`,
                  boxShadow: `0 0 8px 2px ${passColors.glow}, 0 0 18px 5px ${passColors.glowOuter}, inset 0 0 6px 2px ${passColors.glowInner}`,
                  "--pass-text": passColors.text,
                  "--pass-border": passColors.border,
                  "--pass-glow": passColors.glow,
                  "--pass-glow-outer": passColors.glowOuter,
                  "--pass-glow-inner": passColors.glowInner,
                }}
                disabled={!canAct}
                onClick={() =>
                  dispatch(
                    { type: "priority_action", action_index: passAction.index },
                    passAction.label
                  )
                }
              >
                <span className="inline-block transition-transform duration-200 group-hover:translate-x-0.5">
                  {passLabel}
                </span>
              </Button>
            </div>
          )}

          <div className="flex items-center gap-1 shrink-0 flex-wrap">
            <h3 className="m-0 text-[12px] font-bold whitespace-nowrap uppercase tracking-wider text-[#8ec4ff]">Action</h3>
            <span className="text-muted-foreground text-[11px] truncate flex-1 min-w-0">{metaText}</span>
            <div className="flex items-center gap-1">
              <Button
                variant="ghost"
                size="sm"
                className={`h-5 w-5 p-0 shrink-0 transition-all ${
                  undoAvailable
                    ? "text-[#f76969]/60 hover:text-[#f76969] hover:bg-[#f76969]/10 hover:shadow-[0_0_8px_rgba(247,105,105,0.15)]"
                    : "text-muted-foreground/35 opacity-65"
                }`}
                disabled={undoDisabled}
                onClick={handleCancel}
                title={undoAvailable ? "Undo" : "Undo unavailable"}
                aria-label={undoAvailable ? "Undo" : "Undo unavailable"}
              >
                <Undo2 className="h-3.5 w-3.5" />
              </Button>
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
        </div>
      </div>
    </section>
  );
}
