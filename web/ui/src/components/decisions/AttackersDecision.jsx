import { useState } from "react";
import { useGame } from "@/context/GameContext";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

function decodeAttackTargetChoice(choice) {
  if (choice && typeof choice === "object") {
    if ("Player" in choice) return { kind: "player", player: Number(choice.Player) };
    if ("Planeswalker" in choice) return { kind: "planeswalker", object: Number(choice.Planeswalker) };
    if (choice.kind === "player") return { kind: "player", player: Number(choice.player) };
    if (choice.kind === "planeswalker") return { kind: "planeswalker", object: Number(choice.object) };
  }
  return { kind: "player", player: Number(choice) };
}

export default function AttackersDecision({ decision, canAct }) {
  const { dispatch, state } = useGame();
  const options = decision.attacker_options || [];
  const [declarations, setDeclarations] = useState(() => {
    // Pre-populate must-attack creatures
    const initial = [];
    for (const opt of options) {
      if (opt.must_attack) {
        const target = (opt.valid_targets || [])[0];
        if (target) {
          initial.push({
            creature: Number(opt.creature),
            target: decodeAttackTargetChoice(target),
          });
        }
      }
    }
    return initial;
  });

  const isAttacking = (creatureId) =>
    declarations.some((d) => d.creature === Number(creatureId));

  const toggleAttacker = (opt) => {
    const creatureId = Number(opt.creature);
    if (isAttacking(creatureId)) {
      if (opt.must_attack) return; // Can't un-declare must-attack
      setDeclarations((prev) => prev.filter((d) => d.creature !== creatureId));
    } else {
      const target = (opt.valid_targets || [])[0];
      if (!target) return;
      setDeclarations((prev) => [
        ...prev,
        { creature: creatureId, target: decodeAttackTargetChoice(target) },
      ]);
    }
  };

  const players = state?.players || [];

  return (
    <div className="flex flex-col gap-2">
      <div className="text-[12px] text-muted-foreground">Declare attackers</div>
      <div className="flex flex-col gap-1">
        {options.map((opt) => {
          const creatureId = Number(opt.creature);
          const attacking = isAttacking(creatureId);
          const name = opt.name || `Creature ${creatureId}`;
          return (
            <Button
              key={creatureId}
              variant="outline"
              size="sm"
              className={cn(
                "h-7 text-[11px] justify-start px-2",
                attacking && "border-[rgba(174,118,255,0.95)] bg-[rgba(174,118,255,0.08)]",
                opt.must_attack && "italic"
              )}
              disabled={!canAct}
              onClick={() => toggleAttacker(opt)}
            >
              {attacking ? "\u2694 " : ""}{name}
              {opt.must_attack && " (must attack)"}
            </Button>
          );
        })}
      </div>
      <Button
        variant="outline"
        size="sm"
        className="h-7 text-[11px]"
        disabled={!canAct}
        onClick={() =>
          dispatch(
            { type: "declare_attackers", declarations },
            `Declared ${declarations.length} attacker(s)`
          )
        }
      >
        Confirm Attackers ({declarations.length})
      </Button>
    </div>
  );
}
