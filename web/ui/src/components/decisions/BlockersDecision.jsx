import { useState } from "react";
import { useGame } from "@/context/GameContext";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

export default function BlockersDecision({ decision, canAct }) {
  const { dispatch } = useGame();
  const options = decision.blocker_options || [];
  const [declarations, setDeclarations] = useState([]);

  const getBlockerDeclaration = (blockerId) =>
    declarations.find((d) => d.blocker === Number(blockerId));

  const toggleBlocker = (opt, attackerId) => {
    const blockerId = Number(opt.blocker);
    const existing = getBlockerDeclaration(blockerId);
    if (existing && existing.attacker === Number(attackerId)) {
      // Remove
      setDeclarations((prev) => prev.filter((d) => d.blocker !== blockerId));
    } else {
      // Add or change target
      setDeclarations((prev) => [
        ...prev.filter((d) => d.blocker !== blockerId),
        { blocker: blockerId, attacker: Number(attackerId) },
      ]);
    }
  };

  return (
    <div className="flex flex-col gap-2">
      <div className="text-[12px] text-muted-foreground">Declare blockers</div>
      <div className="flex flex-col gap-1.5">
        {options.map((opt) => {
          const blockerId = Number(opt.blocker);
          const name = opt.name || `Creature ${blockerId}`;
          const currentDecl = getBlockerDeclaration(blockerId);
          const validAttackers = opt.valid_attackers || [];

          return (
            <div key={blockerId} className="border border-game-line-2 p-1 rounded-sm">
              <div className={cn(
                "text-[11px] font-bold mb-0.5",
                currentDecl && "text-[rgba(174,118,255,0.95)]"
              )}>
                {name}
              </div>
              <div className="flex flex-wrap gap-0.5">
                {validAttackers.map((attacker) => {
                  const attackerId = Number(attacker.attacker || attacker.id || attacker);
                  const attackerName = attacker.name || `Attacker ${attackerId}`;
                  const isBlocking = currentDecl?.attacker === attackerId;
                  return (
                    <Button
                      key={attackerId}
                      variant="outline"
                      size="sm"
                      className={cn(
                        "h-5 text-[10px] px-1.5",
                        isBlocking && "border-[rgba(174,118,255,0.95)] bg-[rgba(174,118,255,0.08)]"
                      )}
                      disabled={!canAct}
                      onClick={() => toggleBlocker(opt, attackerId)}
                    >
                      {isBlocking ? "\u2694 " : ""}Block {attackerName}
                    </Button>
                  );
                })}
              </div>
            </div>
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
            { type: "declare_blockers", declarations },
            `Declared ${declarations.length} blocker(s)`
          )
        }
      >
        Confirm Blockers ({declarations.length})
      </Button>
    </div>
  );
}
