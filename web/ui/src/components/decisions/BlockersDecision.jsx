import { useState, useMemo, useEffect, useCallback } from "react";
import { useGame } from "@/context/GameContext";
import { useCombatArrows } from "@/context/CombatArrowContext";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";

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
      color: "#3b82f6",
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
      color: "#3b82f6",
      key: `blk-${d.blocker}-${d.blocking}`,
    }));
    updateArrows(arrowData);
  }, [declarations, updateArrows]);

  useEffect(() => clearArrows, [clearArrows]);

  return (
    <div className="flex h-full min-h-0 flex-col gap-2">
      <Button
        variant="outline"
        size="sm"
        className="h-7 text-[11px] mb-0.5 shrink-0"
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
      <ScrollArea className="flex-1 min-h-0">
        <div className="flex flex-col gap-1.5 pr-1">
          <div className="text-[12px] text-muted-foreground">Declare blockers</div>
          {blockerOptions.map((opt) => {
            const blockerId = opt.blocker;
            const name = opt.name;
            const currentDecls = getBlockerDeclarations(blockerId);
            const validAttackers = opt.valid_attackers || [];

            return (
              <div key={blockerId} className="border border-game-line-2 p-1 rounded-sm">
                <div className={cn(
                  "text-[11px] font-bold mb-0.5",
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
                          "h-5 text-[10px] px-1.5",
                          blocking && "border-[rgba(174,118,255,0.95)] bg-[rgba(174,118,255,0.08)]"
                        )}
                        disabled={!canAct}
                        onClick={() => toggleBlocker(blockerId, attackerId)}
                      >
                        {blocking ? "\u2694 " : ""}Block {attackerName}
                      </Button>
                    );
                  })}
                </div>
              </div>
            );
          })}
        </div>
      </ScrollArea>
    </div>
  );
}
