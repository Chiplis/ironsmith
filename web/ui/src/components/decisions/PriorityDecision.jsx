import { useState, useEffect, useRef, useMemo, useCallback } from "react";
import { useGame } from "@/context/GameContext";
import { useHover } from "@/context/HoverContext";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { SymbolText } from "@/lib/mana-symbols";

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

function ActionButton({ action, canAct, dispatch, label, isHighlighted, onMouseEnter, onMouseLeave }) {
  return (
    <Button
      variant="ghost"
      size="sm"
      className={
        "h-auto min-h-7 py-1 text-[13px] justify-start px-2 whitespace-normal text-left text-muted-foreground transition-all hover:text-foreground hover:bg-[rgba(100,169,255,0.1)] hover:shadow-[0_0_8px_rgba(100,169,255,0.15)]" +
        (isHighlighted ? " text-foreground bg-[rgba(240,206,97,0.08)] shadow-[0_0_10px_rgba(240,206,97,0.25)]" : "")
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
      {label ?? action.label}
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
  if (isActivate) return activateSourceName(action.label);
  return stripPrefix(action.label, catLabel);
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
  const sourceName = activateSourceName(action.label);
  const manaOutput = extractManaOutput(action.label);

  return (
    <Button
      variant="ghost"
      size="sm"
      className={
        "h-auto min-h-7 py-1 text-[13px] justify-start px-2 whitespace-normal text-left text-muted-foreground transition-all hover:text-foreground hover:bg-[rgba(180,220,80,0.1)] hover:shadow-[0_0_8px_rgba(180,220,80,0.15)]" +
        (isHighlighted ? " text-foreground bg-[rgba(180,220,80,0.08)] shadow-[0_0_10px_rgba(180,220,80,0.25)]" : "")
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

function CollapsibleSection({ catKey, label, actions, canAct, dispatch, isActivate, hoveredObjectId, hoverCard, clearHover, isOpen, onToggle, actionRefs }) {
  const isMana = catKey === "mana";

  return (
    <div className="flex flex-col">
      {/* Section header */}
      <button
        className="flex items-center gap-1 px-1 py-1 text-[12px] uppercase tracking-wider text-muted-foreground font-bold hover:text-foreground transition-colors cursor-pointer select-none border-none bg-transparent text-left"
        onClick={onToggle}
        type="button"
      >
        <span
          className="text-[10px] transition-transform duration-200 inline-block"
          style={{ transform: isOpen ? "rotate(0deg)" : "rotate(-90deg)" }}
        >
          ▼
        </span>
        <span>{label}</span>
        <span className="text-[11px] text-muted-foreground/60">({actions.length})</span>
      </button>

      {/* Collapsible body — CSS grid row trick for smooth height animation */}
      <div
        className="grid transition-[grid-template-rows] duration-300 ease-in-out"
        style={{ gridTemplateRows: isOpen ? "1fr" : "0fr" }}
      >
        <div className="overflow-hidden">
          <div className="flex flex-col gap-0.5 pb-1">
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
        </div>
      </div>
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

  // Build object_id → catKey lookup for auto-expand
  const objectToCat = useMemo(() => {
    const lookup = new Map();
    for (const [catKey, catActions] of catMap) {
      for (const action of catActions) {
        if (action.object_id != null) {
          const objId = String(action.object_id);
          if (!lookup.has(objId)) lookup.set(objId, catKey);
        }
      }
    }
    return lookup;
  }, [catMap]);

  // Track manually collapsed categories
  const [manualClosed, setManualClosed] = useState(new Set());

  // Which category is auto-focused by hover?
  const focusedCatKey = hoveredObjectId ? objectToCat.get(hoveredObjectId) : null;

  // A category is open if:
  // - Hover focus active → only the focused category is open
  // - No hover focus → all open unless manually collapsed
  const isCatOpen = useCallback(
    (catKey) => {
      if (focusedCatKey) return catKey === focusedCatKey;
      return !manualClosed.has(catKey);
    },
    [focusedCatKey, manualClosed]
  );

  const toggleCategory = useCallback((catKey) => {
    setManualClosed((prev) => {
      const next = new Set(prev);
      if (next.has(catKey)) next.delete(catKey);
      else next.add(catKey);
      return next;
    });
  }, []);

  // Auto-scroll to highlighted action when hover changes
  useEffect(() => {
    if (!hoveredObjectId || !focusedCatKey) return;

    const catActions = catMap.get(focusedCatKey) || [];
    const matchingAction = catActions.find(
      (a) => a.object_id != null && String(a.object_id) === hoveredObjectId
    );
    if (!matchingAction) return;

    // Small delay so the expand animation starts before we scroll
    const timer = setTimeout(() => {
      const el = actionRefs.current.get(matchingAction.index);
      if (el) {
        el.scrollIntoView({ behavior: "smooth", block: "nearest" });
      }
    }, 80);

    return () => clearTimeout(timer);
  }, [hoveredObjectId, focusedCatKey, catMap]);

  return (
    <div className="flex flex-col gap-1 h-full min-h-0">
      {/* Pass / advance button */}
      {passAction && (
        <Button
          variant="ghost"
          size="sm"
          className="group h-auto min-h-7 py-1.5 text-[15px] font-bold justify-start px-3 whitespace-normal text-left transition-all duration-200 shrink-0 pass-priority-btn"
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

      {/* Collapsible category sections — scrollable */}
      {activeCategories.length > 0 && (
        <ScrollArea className="flex-1 min-h-0">
          <div className="flex flex-col gap-0.5 pr-1">
            {activeCategories.map((cat) => (
              <CollapsibleSection
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
                isOpen={isCatOpen(cat.key)}
                onToggle={() => toggleCategory(cat.key)}
                actionRefs={actionRefs}
              />
            ))}
          </div>
        </ScrollArea>
      )}

      {/* Ungrouped actions */}
      {ungrouped.length > 0 && (
        <div className="flex flex-col gap-0.5 shrink-0">
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
  );
}
