import { useState, useMemo, useEffect, useCallback } from "react";
import { useGame } from "@/context/GameContext";
import { useCombatArrows } from "@/context/CombatArrowContext";
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
  const { updateArrows, clearArrows, setCombatMode } = useCombatArrows();
  const attackerOptions = decision.blocker_options || [];
  const blockerOptions = useMemo(
    () => pivotToBlockerCentric(attackerOptions),
    [attackerOptions]
  );

  const [declarations, setDeclarations] = useState([]);

  const getBlockerDeclarations = (blockerId) =>
    declarations.filter((d) => d.blocker === Number(blockerId));

  const isBlockingAttacker = (blockerId, attackerId) =>
    declarations.some(
      (d) => d.blocker === Number(blockerId) && d.blocking === Number(attackerId)
    );

  const toggleBlocker = useCallback((blockerId, attackerId) => {
    blockerId = Number(blockerId);
    attackerId = Number(attackerId);
    if (declarations.some((d) => d.blocker === blockerId && d.blocking === attackerId)) {
      setDeclarations((prev) =>
        prev.filter((d) => !(d.blocker === blockerId && d.blocking === attackerId))
      );
    } else {
      setDeclarations((prev) => [
        ...prev.filter((d) => d.blocker !== blockerId),
        { blocker: blockerId, blocking: attackerId },
      ]);
    }
  }, [declarations]);

  // Handle drop from battlefield drag — blocker dragged to attacker
  const handleDrop = useCallback((fromId, x, y) => {
    const opt = blockerOptions.find((o) => o.blocker === Number(fromId));
    if (!opt) return;

    const el = document.elementFromPoint(x, y);
    if (!el) return;

    const cardEl = el.closest("[data-object-id]");
    if (!cardEl) return;

    const targetId = Number(cardEl.dataset.objectId);
    const validAttacker = opt.valid_attackers.find((a) => a.attacker === targetId);
    if (validAttacker) {
      toggleBlocker(Number(fromId), targetId);
    }
  }, [blockerOptions, toggleBlocker]);

  // Register combat mode for battlefield interaction
  useEffect(() => {
    if (!canAct) {
      setCombatMode(null);
      return;
    }
    const candidateIds = new Set(blockerOptions.map((o) => o.blocker));
    setCombatMode({
      mode: "blockers",
      candidates: candidateIds,
      color: "#ae76ff",
      onDrop: handleDrop,
      onClick: null, // clicks handled via buttons
    });
    return () => setCombatMode(null);
  }, [canAct, blockerOptions, handleDrop, setCombatMode]);

  // Update combat arrows when declarations change
  useEffect(() => {
    const arrowData = declarations.map((d) => ({
      fromId: d.blocker,
      toId: d.blocking,
      toPlayerId: null,
      color: "#ae76ff",
      key: `blk-${d.blocker}-${d.blocking}`,
    }));
    updateArrows(arrowData);
  }, [declarations, updateArrows]);

  useEffect(() => clearArrows, [clearArrows]);

  return (
    <div className="flex flex-col gap-2 overflow-visible">
      <div className="text-[13px] text-muted-foreground">
        Declare blockers — drag creatures to attackers
      </div>
      <div className="flex flex-wrap gap-2 overflow-visible py-1 -mx-1 px-1">
        {blockerOptions.map((opt) => {
          const blockerId = opt.blocker;
          const name = opt.name;
          const currentDecls = getBlockerDeclarations(blockerId);
          const validAttackers = opt.valid_attackers || [];

          return (
            <div key={blockerId} className="flex flex-col gap-1 overflow-visible">
              <div className={cn(
                "text-[14px] font-bold mb-0.5 px-1",
                currentDecls.length > 0 ? "text-[rgba(174,118,255,0.95)]" : "text-muted-foreground"
              )}>
                {name}
              </div>
              <div className="flex flex-wrap gap-1 overflow-visible">
                {validAttackers.map((attacker) => {
                  const attackerId = Number(attacker.attacker);
                  const attackerName = attacker.name;
                  const blocking = isBlockingAttacker(blockerId, attackerId);
                  return (
                    <Button
                      key={attackerId}
                      variant="ghost"
                      size="sm"
                      className={cn(
                        "combat-btn h-5 text-[12px] px-2 text-muted-foreground",
                        blocking && "combat-active text-[rgba(174,118,255,0.95)]"
                      )}
                      disabled={!canAct}
                      onClick={() => toggleBlocker(blockerId, attackerId)}
                    >
                      {blocking ? <Swords className="size-3.5 inline mr-1" /> : ""}{attackerName}
                    </Button>
                  );
                })}
              </div>
            </div>
          );
        })}
      </div>
      <Button
        variant="ghost"
        size="sm"
        className="combat-btn combat-active h-7 text-[14px] px-4 text-[rgba(174,118,255,0.95)] self-start"
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
