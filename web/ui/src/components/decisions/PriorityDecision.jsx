import { useState } from "react";
import { useGame } from "@/context/GameContext";
import { Button } from "@/components/ui/button";
import { ChevronDown, ChevronRight } from "lucide-react";

const CATEGORIES = [
  { key: "play", label: "Play", kinds: ["play_land"] },
  { key: "cast", label: "Cast", kinds: ["cast_spell"] },
  { key: "activate", label: "Activate", kinds: ["activate_ability", "activate_mana_ability"] },
];

function ActionButton({ action, canAct, dispatch, label }) {
  return (
    <Button
      key={action.index}
      variant="outline"
      size="sm"
      className="h-auto min-h-7 py-1 text-[14px] justify-start px-2.5 whitespace-normal text-left"
      disabled={!canAct}
      onClick={() =>
        dispatch(
          { type: "priority_action", action_index: action.index },
          action.label
        )
      }
    >
      {label ?? action.label}
    </Button>
  );
}

function stripPrefix(label, prefix) {
  const re = new RegExp(`^${prefix}\\s+`, "i");
  return label.replace(re, "");
}

function ActionGroup({ label, actions, canAct, dispatch }) {
  const [open, setOpen] = useState(true);

  if (actions.length === 1) {
    return <ActionButton action={actions[0]} canAct={canAct} dispatch={dispatch} />;
  }

  return (
    <div className="flex flex-col gap-0.5">
      <button
        className="flex items-center gap-1 text-[13px] uppercase tracking-wider text-muted-foreground font-bold cursor-pointer hover:text-foreground py-0.5 px-1 bg-transparent border-none"
        onClick={() => setOpen((o) => !o)}
      >
        {open ? <ChevronDown className="size-3" /> : <ChevronRight className="size-3" />}
        {label} ({actions.length})
      </button>
      {open && (
        <div className="flex flex-col gap-0.5 pl-3">
          {actions.map((action) => (
            <ActionButton
              key={action.index}
              action={action}
              canAct={canAct}
              dispatch={dispatch}
              label={stripPrefix(action.label, label)}
            />
          ))}
        </div>
      )}
    </div>
  );
}

export default function PriorityDecision({ decision, canAct }) {
  const { dispatch } = useGame();
  const actions = decision.actions || [];

  // Separate pass priority from other actions
  const passAction = actions.find((a) => a.kind === "pass_priority");
  const rest = actions.filter((a) => a.kind !== "pass_priority");

  // Categorize remaining actions
  const categorized = new Map();
  const ungrouped = [];

  for (const action of rest) {
    const cat = CATEGORIES.find((c) => c.kinds.includes(action.kind));
    if (cat) {
      if (!categorized.has(cat.key)) categorized.set(cat.key, []);
      categorized.get(cat.key).push(action);
    } else {
      ungrouped.push(action);
    }
  }

  return (
    <div className="flex flex-col gap-1">
      {passAction && (
        <Button
          variant="outline"
          size="sm"
          className="h-auto min-h-7 py-1.5 mb-1.5 text-[14px] justify-start px-3 whitespace-normal text-left border-[#f7b869]/50 text-[#f7b869] hover:bg-[#f7b869]/10 hover:text-[#f7b869]"
          disabled={!canAct}
          onClick={() =>
            dispatch(
              { type: "priority_action", action_index: passAction.index },
              passAction.label
            )
          }
        >
          {passAction.label}
        </Button>
      )}
      {CATEGORIES.map((cat) => {
        const catActions = categorized.get(cat.key);
        if (!catActions || catActions.length === 0) return null;
        if (catActions.length === 1) {
          return <ActionButton key={cat.key} action={catActions[0]} canAct={canAct} dispatch={dispatch} />;
        }
        return (
          <ActionGroup
            key={cat.key}
            label={cat.label}
            actions={catActions}
            canAct={canAct}
            dispatch={dispatch}
          />
        );
      })}
      {ungrouped.map((action) => (
        <ActionButton key={action.index} action={action} canAct={canAct} dispatch={dispatch} />
      ))}
    </div>
  );
}
