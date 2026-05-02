import { useMemo } from "react";
import { useGame } from "@/context/GameContext";
import { MANA_SYMBOLS } from "@/lib/constants";
import { ManaSymbol } from "@/lib/mana-symbols";
import { cn } from "@/lib/utils";

function buildManaActivationMap(decision) {
  const map = new Map();
  if (decision?.kind !== "priority" || !Array.isArray(decision.actions)) return map;
  for (const action of decision.actions) {
    if (action?.kind !== "activate_mana_ability") continue;
    const label = String(action?.label || "").toUpperCase();
    for (const { key, symbol } of MANA_SYMBOLS) {
      if (label.includes(`{${symbol}}`) || label.includes(`: ${symbol}`)) {
        if (!map.has(key)) map.set(key, []);
        map.get(key).push(action);
        break;
      }
    }
  }
  return map;
}

export default function MobileManaPool({ pool, interactive = false, side = "self", className }) {
  const { state, dispatch } = useGame();
  const activationsByColor = useMemo(
    () => (interactive ? buildManaActivationMap(state?.decision) : new Map()),
    [interactive, state?.decision]
  );

  const chips = MANA_SYMBOLS.map(({ key, symbol, label }) => {
    const amount = Math.max(0, Math.floor(Number(pool?.[key]) || 0));
    const actions = activationsByColor.get(key) || [];
    const canActivate = interactive && actions.length > 0;
    const empty = amount <= 0 && !canActivate;
    return (
      <button
        key={key}
        type="button"
        className={cn(
          "mobile-mtga-mana-pool-chip",
          empty && "mobile-mtga-mana-pool-chip--empty",
          canActivate && "mobile-mtga-mana-pool-chip--activatable",
        )}
        disabled={!canActivate || amount > 0 ? !canActivate : false}
        aria-label={`${amount} ${label} mana${canActivate ? ", tap to add mana" : ""}`}
        onClick={() => {
          if (!canActivate) return;
          const action = actions[0];
          dispatch(
            { type: "priority_action", action_index: action.index, action_ref: action.action_ref },
            action.label
          );
        }}
      >
        <ManaSymbol sym={symbol} size={14} />
        <span className="mobile-mtga-mana-pool-chip-amount">{amount}</span>
      </button>
    );
  });

  return (
    <div
      className={cn("mobile-mtga-mana-pool", `mobile-mtga-mana-pool--${side}`, className)}
      data-mobile-mana-pool-side={side}
      aria-label={`${side === "self" ? "Your" : "Opponent"} mana pool`}
    >
      {chips}
    </div>
  );
}
