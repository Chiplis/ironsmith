import { useState, useMemo } from "react";
import { useGame } from "@/context/GameContext";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { Swords } from "lucide-react";

/**
 * The engine emits attacker-centric blocker options:
 *   { attacker, attacker_name, valid_blockers: [{ id, name }], min_blockers }
 *
 * We pivot to blocker-centric UI: each blocker shows which attackers it can block.
 * Declarations are sent as { blocker, blocking } (blocker creature, attacker it blocks).
 */
function pivotToBlockerCentric(attackerOptions) {
  const blockerMap = new Map();
  for (const opt of attackerOptions) {
    const attackerId = Number(opt.attacker);
    const attackerName = opt.attacker_name || `Attacker ${attackerId}`;
    for (const b of opt.valid_blockers || []) {
      const bid = Number(b.id);
      if (!blockerMap.has(bid)) {
        blockerMap.set(bid, {
          blocker: bid,
          name: b.name || `Creature ${bid}`,
          valid_attackers: [],
        });
      }
      blockerMap.get(bid).valid_attackers.push({
        attacker: attackerId,
        name: attackerName,
        min_blockers: opt.min_blockers || 0,
      });
    }
  }
  return Array.from(blockerMap.values());
}

export default function BlockersDecision({ decision, canAct }) {
  const { dispatch } = useGame();
  const attackerOptions = decision.blocker_options || [];
  const blockerOptions = useMemo(
    () => pivotToBlockerCentric(attackerOptions),
    [attackerOptions]
  );
  // declarations: array of { blocker, blocking } — can have multiple entries
  // per blocker if it can block multiple attackers
  const [declarations, setDeclarations] = useState([]);

  const getBlockerDeclarations = (blockerId) =>
    declarations.filter((d) => d.blocker === Number(blockerId));

  const isBlockingAttacker = (blockerId, attackerId) =>
    declarations.some(
      (d) => d.blocker === Number(blockerId) && d.blocking === Number(attackerId)
    );

  const toggleBlocker = (blockerId, attackerId) => {
    blockerId = Number(blockerId);
    attackerId = Number(attackerId);
    if (isBlockingAttacker(blockerId, attackerId)) {
      // Remove this specific assignment
      setDeclarations((prev) =>
        prev.filter((d) => !(d.blocker === blockerId && d.blocking === attackerId))
      );
    } else {
      // For single-block creatures, replace existing assignment; for multi-block, add
      setDeclarations((prev) => [
        ...prev.filter((d) => d.blocker !== blockerId),
        { blocker: blockerId, blocking: attackerId },
      ]);
    }
  };

  return (
    <div className="flex flex-col gap-2">
      <div className="text-[16px] text-muted-foreground">Declare blockers</div>
      <div className="flex flex-col gap-1.5">
        {blockerOptions.map((opt) => {
          const blockerId = opt.blocker;
          const name = opt.name;
          const currentDecls = getBlockerDeclarations(blockerId);
          const validAttackers = opt.valid_attackers || [];

          return (
            <div key={blockerId} className="border border-game-line-2 p-1 rounded-sm">
              <div className={cn(
                "text-[14px] font-bold mb-0.5",
                currentDecls.length > 0 && "text-[rgba(174,118,255,0.95)]"
              )}>
                {name}
              </div>
              <div className="flex flex-wrap gap-0.5">
                {validAttackers.map((attacker) => {
                  const attackerId = Number(attacker.attacker);
                  const attackerName = attacker.name;
                  const blocking = isBlockingAttacker(blockerId, attackerId);
                  return (
                    <Button
                      key={attackerId}
                      variant="outline"
                      size="sm"
                      className={cn(
                        "h-5 text-[13px] px-1.5",
                        blocking && "border-[rgba(174,118,255,0.95)] bg-[rgba(174,118,255,0.08)]"
                      )}
                      disabled={!canAct}
                      onClick={() => toggleBlocker(blockerId, attackerId)}
                    >
                      {blocking ? <Swords className="size-3.5 inline mr-1" /> : ""}Block {attackerName}
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
        className="h-7 text-[14px]"
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
