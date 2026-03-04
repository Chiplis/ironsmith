import { useEffect, useMemo, useRef } from "react";
import { useGame } from "@/context/GameContext";
import { useHover } from "@/context/HoverContext";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { SymbolText } from "@/lib/mana-symbols";
import { normalizeDecisionText } from "./decisionText";

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

// Color scheme for pass-priority button based on what phase/step comes next
function passButtonColor(phase, step, stackSize) {
  if (stackSize > 0) return "yellow"; // resolving stack
  // Going to combat from main phase → red
  if (phase === "FirstMain" && !step) return "red";
  switch (step) {
    case "BeginCombat":       // → declare attackers
    case "DeclareAttackers":  // → declare blockers
      return "blue";
    case "DeclareBlockers":   // → combat damage
    case "CombatDamage":      // → end combat
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

const CATEGORIES = [
  { key: "play", label: "Play", kinds: ["play_land"] },
  { key: "cast", label: "Cast", kinds: ["cast_spell"] },
  { key: "mana", label: "Mana", kinds: ["activate_mana_ability"] },
  { key: "activate", label: "Activate", kinds: ["activate_ability"] },
];

// Neon blue highlight style for hovered actions
const HIGHLIGHT_STYLE = "text-foreground bg-[rgba(100,169,255,0.12)] shadow-[0_0_12px_rgba(100,169,255,0.35)]";

function ActionButton({ action, canAct, dispatch, label, isHighlighted, onMouseEnter, onMouseLeave }) {
  return (
    <Button
      variant="ghost"
      size="sm"
      className={
        "h-auto min-h-7 py-1 text-[13px] justify-start px-2 whitespace-normal text-left text-muted-foreground transition-all hover:text-foreground hover:bg-[rgba(100,169,255,0.1)] hover:shadow-[0_0_8px_rgba(100,169,255,0.15)]" +
        (isHighlighted ? ` ${HIGHLIGHT_STYLE}` : "")
      }
      disabled={!canAct}
      onClick={() =>
        dispatch(
          { type: "priority_action", action_index: action.index },
          action.label
        )
      }
      onMouseEnter={onMouseEnter}
      onMouseLeave={onMouseLeave}
    >
      {normalizeDecisionText(label ?? action.label)}
    </Button>
  );
}

function stripPrefix(label, prefix) {
  const re = new RegExp(`^${prefix}\\s+`, "i");
  return label.replace(re, "");
}

function activateSourceName(label) {
  const stripped = stripPrefix(label, "Activate");
  const colonIdx = stripped.indexOf(": ");
  return colonIdx > 0 ? stripped.slice(0, colonIdx) : stripped.replace(/ ability #\d+$/, "");
}

function actionLabel(action, catLabel, isActivate) {
  if (isActivate) return normalizeDecisionText(activateSourceName(action.label));
  return normalizeDecisionText(stripPrefix(action.label, catLabel));
}

/**
 * Extract the mana output portion from a mana ability label.
 * e.g. "Activate Forest: {T}: Add {G}" → "{G}"
 */
function extractManaOutput(label) {
  const addMatch = label.match(/Add\s+(.+)$/i);
  if (addMatch) {
    const output = addMatch[1].trim();
    const symbols = output.match(/\{[^}]+\}/g);
    if (symbols && symbols.length > 0) return symbols.join("");
    return "Add " + output;
  }
  return activateSourceName(label);
}

/** Render a mana ability as source name + mana pips */
function ManaActionButton({ action, canAct, dispatch, isHighlighted, onMouseEnter, onMouseLeave }) {
  const sourceName = normalizeDecisionText(activateSourceName(action.label));
  const manaOutput = extractManaOutput(action.label);

  return (
    <Button
      variant="ghost"
      size="sm"
      className={
        "h-auto min-h-7 py-1 text-[13px] justify-start px-2 whitespace-normal text-left text-muted-foreground transition-all hover:text-foreground hover:bg-[rgba(180,220,80,0.1)] hover:shadow-[0_0_8px_rgba(180,220,80,0.15)]" +
        (isHighlighted ? ` ${HIGHLIGHT_STYLE}` : "")
      }
      disabled={!canAct}
      onClick={() =>
        dispatch(
          { type: "priority_action", action_index: action.index },
          action.label
        )
      }
      onMouseEnter={onMouseEnter}
      onMouseLeave={onMouseLeave}
    >
      <span className="flex items-center gap-1.5">
        <span className="truncate">{sourceName}</span>
        <span className="inline-flex items-center shrink-0"><SymbolText text={manaOutput} /></span>
      </span>
    </Button>
  );
}

function CategorySection({ catKey, label, actions, canAct, dispatch, isActivate, hoveredObjectId, hoverCard, clearHover, actionRefs }) {
  const isMana = catKey === "mana";

  return (
    <div className="flex flex-col gap-0.5 min-w-0">
      <h4 className="text-[12px] uppercase tracking-wider text-muted-foreground font-bold px-1 pb-0.5">
        {label} ({actions.length})
      </h4>
      {actions.map((action) => {
        const objId = action.object_id != null ? String(action.object_id) : null;
        const isHighlighted = objId != null && hoveredObjectId === objId;

        const setRef = (el) => {
          if (el) actionRefs.current.set(action.index, el);
          else actionRefs.current.delete(action.index);
        };

        if (isMana) {
          return (
            <div key={action.index} ref={setRef}>
              <ManaActionButton
                action={action}
                canAct={canAct}
                dispatch={dispatch}
                isHighlighted={isHighlighted}
                onMouseEnter={() => objId && hoverCard(objId)}
                onMouseLeave={clearHover}
              />
            </div>
          );
        }
        return (
          <div key={action.index} ref={setRef}>
            <ActionButton
              action={action}
              canAct={canAct}
              dispatch={dispatch}
              label={actionLabel(action, label, isActivate)}
              isHighlighted={isHighlighted}
              onMouseEnter={() => objId && hoverCard(objId)}
              onMouseLeave={clearHover}
            />
          </div>
        );
      })}
    </div>
  );
}

export default function PriorityDecision({ decision, canAct }) {
  const { state, dispatch, holdRule } = useGame();
  const { hoveredObjectId, hoverCard, clearHover } = useHover();
  const actions = decision.actions || [];
  const actionRefs = useRef(new Map());

  const passAction = actions.find((a) => a.kind === "pass_priority");
  const rest = actions.filter((a) => a.kind !== "pass_priority");

  const holdingPriority = holdRule === "always";
  const stackSize = state?.stack_size || 0;
  const isResolvingStack = stackSize > 0;
  const topOfStack = (state?.stack_objects || [])[0];
  const resolvingAbilityText = isResolvingStack && topOfStack?.ability_kind
    ? (topOfStack.ability_text || topOfStack.effect_text || null)
    : null;
  const passLabel = holdingPriority
    ? passAction?.label || "Pass priority"
    : `→ ${nextStepLabel(state?.phase, state?.step, stackSize)}`;
  const colorKey = passButtonColor(state?.phase, state?.step, stackSize);
  const pc = PASS_COLORS[colorKey];

  // Categorize actions
  const { catMap, ungrouped } = useMemo(() => {
    const map = new Map();
    const ung = [];
    for (const action of rest) {
      const cat = CATEGORIES.find((c) => c.kinds.includes(action.kind));
      if (cat) {
        if (!map.has(cat.key)) map.set(cat.key, []);
        map.get(cat.key).push(action);
      } else {
        ung.push(action);
      }
    }
    return { catMap: map, ungrouped: ung };
  }, [actions]);

  const activeCategories = CATEGORIES.filter((cat) => catMap.has(cat.key));

  // Auto-scroll to highlighted action when hover changes
  useEffect(() => {
    if (!hoveredObjectId) return;

    // Find the matching action across all categories
    for (const [, catActions] of catMap) {
      const match = catActions.find(
        (a) => a.object_id != null && String(a.object_id) === hoveredObjectId
      );
      if (match) {
        const el = actionRefs.current.get(match.index);
        if (el) {
          el.scrollIntoView({ behavior: "smooth", block: "nearest" });
        }
        break;
      }
    }
  }, [hoveredObjectId, catMap]);

  return (
    <div className="flex flex-col gap-1 h-full min-h-0">
      {/* Pass / advance button */}
      {passAction && (
        <Button
          variant="ghost"
          size="sm"
          className="group mb-1.5 h-auto min-h-7 py-1.5 text-[15px] font-bold justify-start px-3 whitespace-normal text-left transition-all duration-200 shrink-0 pass-priority-btn"
          style={{
            color: pc.text,
            border: `1px solid ${pc.border}`,
            boxShadow: `0 0 8px 2px ${pc.glow}, 0 0 18px 5px ${pc.glowOuter}, inset 0 0 6px 2px ${pc.glowInner}`,
            "--pass-text": pc.text,
            "--pass-border": pc.border,
            "--pass-glow": pc.glow,
            "--pass-glow-outer": pc.glowOuter,
            "--pass-glow-inner": pc.glowInner,
          }}
          disabled={!canAct}
          onClick={() =>
            dispatch(
              { type: "priority_action", action_index: passAction.index },
              passAction.label
            )
          }
        >
          <span className="inline-block transition-transform duration-200 group-hover:translate-x-0.5">{passLabel}</span>
        </Button>
      )}

      {resolvingAbilityText && (
        <div className="px-1 pb-1">
          <SymbolText
            text={resolvingAbilityText}
            className="text-[12px] text-[#9fc2e4] leading-snug font-[inherit] block"
          />
        </div>
      )}

      {!isResolvingStack && (
        <ScrollArea className="flex-1 min-h-0">
          <div className="flex flex-col gap-2 pr-1">
            {activeCategories.map((cat) => (
              <CategorySection
                key={cat.key}
                catKey={cat.key}
                label={cat.label}
                actions={catMap.get(cat.key)}
                canAct={canAct}
                dispatch={dispatch}
                isActivate={cat.key === "activate"}
                hoveredObjectId={hoveredObjectId}
                hoverCard={hoverCard}
                clearHover={clearHover}
                actionRefs={actionRefs}
              />
            ))}

            {ungrouped.length > 0 && (
              <div className="flex flex-col gap-0.5">
                {ungrouped.map((action) => {
                  const objId = action.object_id != null ? String(action.object_id) : null;
                  return (
                    <ActionButton
                      key={action.index}
                      action={action}
                      canAct={canAct}
                      dispatch={dispatch}
                      isHighlighted={objId != null && hoveredObjectId === objId}
                      onMouseEnter={() => objId && hoverCard(objId)}
                      onMouseLeave={clearHover}
                    />
                  );
                })}
              </div>
            )}
          </div>
        </ScrollArea>
      )}
    </div>
  );
}
