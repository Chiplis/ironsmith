import { useState } from "react";
import { useGame } from "@/context/GameContext";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { Swords, ArrowRight } from "lucide-react";

function decodeAttackTargetChoice(choice) {
  if (choice && typeof choice === "object") {
    if ("Player" in choice) return { kind: "player", player: Number(choice.Player) };
    if ("Planeswalker" in choice) return { kind: "planeswalker", object: Number(choice.Planeswalker) };
    if (choice.kind === "player") return { kind: "player", player: Number(choice.player) };
    if (choice.kind === "planeswalker") return { kind: "planeswalker", object: Number(choice.object) };
  }
  return { kind: "player", player: Number(choice) };
}

function attackTargetLabel(target, players) {
  if (target.kind === "player") {
    const p = players.find((pl) => pl.index === target.player);
    return p ? p.name : `Player ${target.player}`;
  }
  return target.name || `Planeswalker ${target.object}`;
}

export default function AttackersDecision({ decision, canAct }) {
  const { dispatch, state } = useGame();
  const options = decision.attacker_options || [];
  const players = state?.players || [];

  const [declarations, setDeclarations] = useState(() => {
    // Pre-populate must-attack creatures with first valid target
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

  // Track which creature is in "choose target" mode
  const [choosingTarget, setChoosingTarget] = useState(null);

  const getDeclaration = (creatureId) =>
    declarations.find((d) => d.creature === Number(creatureId));

  const isAttacking = (creatureId) =>
    declarations.some((d) => d.creature === Number(creatureId));

  const toggleAttacker = (opt) => {
    const creatureId = Number(opt.creature);
    const validTargets = opt.valid_targets || [];

    if (isAttacking(creatureId)) {
      if (opt.must_attack) return; // Can't un-declare must-attack
      setDeclarations((prev) => prev.filter((d) => d.creature !== creatureId));
      setChoosingTarget(null);
    } else if (validTargets.length <= 1) {
      // Only one target - auto-select
      const target = validTargets[0];
      if (!target) return;
      setDeclarations((prev) => [
        ...prev,
        { creature: creatureId, target: decodeAttackTargetChoice(target) },
      ]);
    } else {
      // Multiple targets - show target picker
      setChoosingTarget(creatureId);
    }
  };

  const selectTarget = (creatureId, target) => {
    creatureId = Number(creatureId);
    const decoded = decodeAttackTargetChoice(target);
    setDeclarations((prev) => [
      ...prev.filter((d) => d.creature !== creatureId),
      { creature: creatureId, target: decoded },
    ]);
    setChoosingTarget(null);
  };

  const changeTarget = (opt) => {
    const validTargets = opt.valid_targets || [];
    if (validTargets.length <= 1) return;
    setChoosingTarget(Number(opt.creature));
  };

  return (
    <div className="flex flex-col gap-2">
      <div className="text-[16px] text-muted-foreground">Declare attackers</div>
      <div className="flex flex-col gap-1">
        {options.map((opt) => {
          const creatureId = Number(opt.creature);
          const attacking = isAttacking(creatureId);
          const name = opt.creature_name || opt.name || `Creature ${creatureId}`;
          const decl = getDeclaration(creatureId);
          const validTargets = opt.valid_targets || [];
          const isChoosingTarget = choosingTarget === creatureId;

          return (
            <div key={creatureId} className="flex flex-col gap-0.5">
              <Button
                variant="outline"
                size="sm"
                className={cn(
                  "h-7 text-[14px] justify-start px-2",
                  attacking && "border-[rgba(174,118,255,0.95)] bg-[rgba(174,118,255,0.08)]",
                  opt.must_attack && "italic"
                )}
                disabled={!canAct}
                onClick={() => toggleAttacker(opt)}
              >
                {attacking ? <Swords className="size-3.5 inline mr-1" /> : ""}{name}
                {opt.must_attack && " (must attack)"}
                {attacking && decl && validTargets.length > 1 && (
                  <span className="ml-1 text-[13px] text-muted-foreground inline-flex items-center gap-0.5">
                    <ArrowRight className="size-3" /> {attackTargetLabel(decl.target, players)}
                  </span>
                )}
              </Button>

              {/* Target picker when multiple valid targets */}
              {isChoosingTarget && (
                <div className="ml-4 flex flex-col gap-0.5">
                  <div className="text-[13px] text-muted-foreground">Choose attack target:</div>
                  {validTargets.map((target, tIdx) => {
                    const decoded = decodeAttackTargetChoice(target);
                    const label = attackTargetLabel(decoded, players);
                    return (
                      <Button
                        key={tIdx}
                        variant="outline"
                        size="sm"
                        className="h-5 text-[13px] justify-start px-2"
                        disabled={!canAct}
                        onClick={() => selectTarget(creatureId, target)}
                      >
                        {label}
                      </Button>
                    );
                  })}
                </div>
              )}

              {/* Change target button for already-attacking creatures with multiple targets */}
              {attacking && !isChoosingTarget && validTargets.length > 1 && (
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-4 text-[12px] ml-4 text-muted-foreground"
                  disabled={!canAct}
                  onClick={() => changeTarget(opt)}
                >
                  Change target
                </Button>
              )}
            </div>
          );
        })}
      </div>
      <Button
        variant="outline"
        size="sm"
        className="h-7 text-[14px]"
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
