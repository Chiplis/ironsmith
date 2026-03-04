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
  const attackerOptions = useMemo(() => decision.blocker_options || [], [decision.blocker_options]);
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
    <div className="flex h-full min-h-0 w-full flex-col gap-2 overflow-x-hidden">
      <ScrollArea className="flex-1 min-h-0 w-full overflow-x-hidden">
        <div className="flex flex-col gap-2 pr-1 overflow-x-hidden">
          <div className="px-0.5 text-[13px] font-bold uppercase tracking-wider text-[#a4c2e2]">Declare blockers</div>
          {blockerOptions.map((opt) => {
            const blockerId = opt.blocker;
            const name = opt.name;
            const currentDecls = getBlockerDeclarations(blockerId);
            const validAttackers = opt.valid_attackers || [];

            return (
              <div
                key={blockerId}
                className={cn(
                  "min-w-0 rounded-sm px-2 py-1.5 border-l-[3px] border-[#2a3b4d] bg-[rgba(7,15,23,0.35)]",
                  currentDecls.length > 0 && "border-[rgba(105,181,247,0.9)] bg-[rgba(20,39,58,0.52)]"
                )}
              >
                <div className={cn(
                  "mb-1.5 text-[15px] font-semibold text-[#d6e7fb]",
                  currentDecls.length > 0 && "text-[#bfe1ff]"
                )}>
                  {name}
                </div>
                <div className="flex flex-wrap gap-1.5">
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
                          "h-8 max-w-full min-w-0 overflow-hidden rounded-full border border-[#325474] bg-[rgba(15,27,40,0.9)] px-3 text-[13px] font-semibold text-[#c7dbf2] transition-all hover:border-[#4f7cad] hover:bg-[rgba(25,44,66,0.95)] hover:text-[#eaf3ff]",
                          blocking && "border-[rgba(105,181,247,0.95)] bg-[rgba(41,73,105,0.7)] text-[#e1f1ff]"
                        )}
                        disabled={!canAct}
                        onClick={() => toggleBlocker(blockerId, attackerId)}
                      >
                        <span className="min-w-0 truncate">
                          {blocking ? "[BLK] " : ""}Block {attackerName}
                        </span>
                      </Button>
                    );
                  })}
                </div>
              </div>
            );
          })}
        </div>
      </ScrollArea>

      <div className="w-full shrink-0 pt-1">
        <Button
          variant="ghost"
          size="sm"
          className="w-full h-10 rounded-sm border border-[#315274] bg-[rgba(15,27,40,0.88)] px-3 text-[16px] font-bold text-[#8ec4ff] transition-all hover:border-[#4f7cad] hover:bg-[rgba(24,43,64,0.95)] hover:text-[#d7ebff]"
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
    </div>
  );
}
