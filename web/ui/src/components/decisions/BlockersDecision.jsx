import { useState, useMemo, useEffect, useCallback, useRef } from "react";
import { useGame } from "@/context/GameContext";
import { useCombatArrows } from "@/context/useCombatArrows";
import { getCardRect, centerOf } from "@/hooks/useCardPositions";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";

const BLOCKER_COLOR = "#3b82f6";

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

export default function BlockersDecision({
  decision,
  canAct,
  compact = false,
  onCompactActionChange = null,
}) {
  const { dispatch } = useGame();
  const {
    updateArrows,
    clearArrows,
    startDragArrow,
    updateDragArrow,
    endDragArrow,
    setCombatMode,
  } = useCombatArrows();
  const attackerOptions = useMemo(() => decision.blocker_options || [], [decision.blocker_options]);
  const blockerOptions = useMemo(
    () => pivotToBlockerCentric(attackerOptions),
    [attackerOptions]
  );
  const blockerOptionsRef = useRef(blockerOptions);
  const selectedBlockerRef = useRef(null);
  const declarationsRef = useRef([]);

  const [declarations, setDeclarations] = useState([]);
  const [selectedBlockerId, setSelectedBlockerId] = useState(null);

  useEffect(() => {
    blockerOptionsRef.current = blockerOptions;
  }, [blockerOptions]);

  useEffect(() => {
    selectedBlockerRef.current = selectedBlockerId;
  }, [selectedBlockerId]);

  useEffect(() => {
    declarationsRef.current = declarations;
  }, [declarations]);

  const getDeclaration = (blockerId) =>
    declarations.find((d) => d.blocker === Number(blockerId));

  const getBlockerDeclarations = (blockerId) =>
    declarations.filter((d) => d.blocker === Number(blockerId));

  const isBlockingAttacker = (blockerId, attackerId) =>
    declarations.some(
      (d) => d.blocker === Number(blockerId) && d.blocking === Number(attackerId)
    );

  const assignBlocker = useCallback((blockerId, attackerId) => {
    blockerId = Number(blockerId);
    attackerId = Number(attackerId);
    setDeclarations((prev) => [
      ...prev.filter((d) => d.blocker !== blockerId),
      { blocker: blockerId, blocking: attackerId },
    ]);
    setSelectedBlockerId(null);
  }, []);

  const toggleBlocker = useCallback((blockerId, attackerId) => {
    blockerId = Number(blockerId);
    attackerId = Number(attackerId);
    if (declarationsRef.current.some((d) => d.blocker === blockerId && d.blocking === attackerId)) {
      setDeclarations((prev) =>
        prev.filter((d) => !(d.blocker === blockerId && d.blocking === attackerId))
      );
      setSelectedBlockerId(null);
      return;
    }
    assignBlocker(blockerId, attackerId);
  }, [assignBlocker]);

  const toggleBlockerSelection = useCallback((opt) => {
    const blockerId = Number(opt.blocker);
    if (declarationsRef.current.some((d) => d.blocker === blockerId)) {
      setDeclarations((prev) => prev.filter((d) => d.blocker !== blockerId));
      setSelectedBlockerId(null);
      return;
    }
    if (selectedBlockerRef.current === blockerId) {
      setSelectedBlockerId(null);
      return;
    }
    setSelectedBlockerId(blockerId);
  }, []);

  useEffect(() => {
    if (selectedBlockerId == null) {
      endDragArrow();
      return;
    }

    const rect = getCardRect(selectedBlockerId);
    if (rect) {
      const center = centerOf(rect);
      startDragArrow(selectedBlockerId, center.x, center.y, BLOCKER_COLOR);
    }

    const onPointerMove = (event) => {
      updateDragArrow(event.clientX, event.clientY);
    };
    document.addEventListener("pointermove", onPointerMove, { passive: true });
    return () => {
      document.removeEventListener("pointermove", onPointerMove);
    };
  }, [endDragArrow, selectedBlockerId, startDragArrow, updateDragArrow]);

  const handleTargetCardClick = useCallback((attackerId) => {
    const blockerId = selectedBlockerRef.current;
    if (blockerId == null) return false;
    const opt = (blockerOptionsRef.current || []).find((entry) => entry.blocker === Number(blockerId));
    if (!opt) return false;
    const validAttacker = (opt.valid_attackers || []).find((entry) => entry.attacker === Number(attackerId));
    if (!validAttacker) return false;
    assignBlocker(blockerId, attackerId);
    return true;
  }, [assignBlocker]);

  const handleDrop = useCallback((fromId, x, y) => {
    const opt = (blockerOptionsRef.current || []).find((o) => o.blocker === Number(fromId));
    if (!opt) return;

    const el = document.elementFromPoint(x, y);
    if (!el) return;

    const cardEl = el.closest("[data-object-id]");
    if (!cardEl) return;

    const targetId = Number(cardEl.dataset.objectId);
    const validAttacker = opt.valid_attackers.find((a) => a.attacker === targetId);
    if (validAttacker) {
      assignBlocker(Number(fromId), targetId);
    }
  }, [assignBlocker]);

  const combatOptionsKey = blockerOptions
    .map((opt) => {
      const validAttackers = (opt.valid_attackers || [])
        .map((attacker) => `${Number(attacker.attacker)}`)
        .join(",");
      return `${Number(opt.blocker)}:${validAttackers}`;
    })
    .join("|");

  useEffect(() => {
    if (!canAct) {
      setCombatMode(null);
      return;
    }
    const currentOptions = blockerOptionsRef.current || [];
    const candidateIds = new Set(currentOptions.map((o) => o.blocker));
    const validTargetObjectsByBlocker = {};
    for (const opt of currentOptions) {
      validTargetObjectsByBlocker[Number(opt.blocker)] = new Set(
        (opt.valid_attackers || []).map((attacker) => Number(attacker.attacker))
      );
    }
    const activeBlockerId = selectedBlockerId != null ? Number(selectedBlockerId) : null;
    const validTargetObjects = (
      activeBlockerId != null
        ? (validTargetObjectsByBlocker[activeBlockerId] || new Set())
        : new Set()
    );
    setCombatMode({
      mode: "blockers",
      candidates: candidateIds,
      color: BLOCKER_COLOR,
      selectedBlocker: selectedBlockerId,
      validTargetObjectsByBlocker,
      validTargetObjects,
      onDrop: handleDrop,
      onClick: (blockerId) => {
        const opt = (blockerOptionsRef.current || []).find((entry) => entry.blocker === Number(blockerId));
        if (opt) toggleBlockerSelection(opt);
      },
      onTargetCardClick: handleTargetCardClick,
    });
    return () => setCombatMode(null);
  }, [
    canAct,
    combatOptionsKey,
    handleDrop,
    handleTargetCardClick,
    selectedBlockerId,
    setCombatMode,
    toggleBlockerSelection,
  ]);

  useEffect(() => {
    const arrowData = declarations.map((d) => ({
      fromId: d.blocker,
      toId: d.blocking,
      toPlayerId: null,
      color: BLOCKER_COLOR,
      key: `blk-${d.blocker}-${d.blocking}`,
    }));
    updateArrows(arrowData);
  }, [declarations, updateArrows]);

  useEffect(() => clearArrows, [clearArrows]);

  useEffect(() => {
    if (typeof onCompactActionChange !== "function") return;
    if (!compact) {
      onCompactActionChange(null);
      return;
    }

    onCompactActionChange({
      label: `Confirm Blockers (${declarations.length})`,
      disabled: !canAct,
      onSubmit: () =>
        dispatch(
          { type: "declare_blockers", declarations },
          `Declared ${declarations.length} blocker(s)`
        ),
    });
  }, [canAct, compact, declarations, dispatch, onCompactActionChange]);

  const attackerNameById = useMemo(() => {
    const map = new Map();
    for (const opt of attackerOptions) {
      const attackerId = Number(opt.attacker);
      map.set(attackerId, opt.attacker_name || `Attacker ${attackerId}`);
    }
    return map;
  }, [attackerOptions]);

  if (compact) {
    return null;
  }

  return (
    <div className="flex h-full min-h-0 w-full flex-col gap-2 overflow-x-hidden">
      <ScrollArea className="flex-1 min-h-0 w-full overflow-x-hidden">
        <div className="flex flex-col gap-2 pr-1 overflow-x-hidden">
          <div className="px-0.5 text-[13px] font-bold uppercase tracking-wider text-[#d8c18c]">Declare blockers</div>
          {blockerOptions.map((opt) => {
            const blockerId = opt.blocker;
            const name = opt.name;
            const decl = getDeclaration(blockerId);
            const currentDecls = getBlockerDeclarations(blockerId);
            const validAttackers = opt.valid_attackers || [];
            const isSelected = selectedBlockerId === blockerId;

            return (
              <div
                key={blockerId}
                className={cn(
                  "min-w-0 rounded-none px-2 py-1.5 border-l-[3px] border-[rgba(122,97,67,0.72)] bg-[rgba(31,25,21,0.35)]",
                  currentDecls.length > 0 && "border-[rgba(176,151,104,0.86)] bg-[rgba(56,42,24,0.48)]",
                  isSelected && "border-[rgba(158,92,74,0.86)] bg-[rgba(60,28,24,0.5)] shadow-[inset_0_0_0_1px_rgba(196,128,108,0.2)]"
                )}
              >
                <Button
                  variant="ghost"
                  size="sm"
                  className={cn(
                    "decision-option-row h-auto min-h-10 w-full min-w-0 overflow-hidden justify-start rounded-none border px-3 py-2 text-left text-[15px] font-semibold leading-snug whitespace-normal",
                    "border-[rgba(128,107,78,0.48)] bg-[linear-gradient(180deg,rgba(58,50,43,0.94),rgba(22,20,18,0.98))] text-[#d7c7a4] hover:border-[rgba(196,165,112,0.7)] hover:bg-[linear-gradient(180deg,rgba(84,68,47,0.98),rgba(34,27,20,0.98))]",
                    currentDecls.length > 0 && "border-[rgba(201,171,114,0.84)] bg-[linear-gradient(180deg,rgba(86,67,40,0.96),rgba(39,30,20,0.98))] text-[#f0e2bf]",
                    isSelected && "border-[rgba(165,101,82,0.85)] bg-[linear-gradient(180deg,rgba(84,45,34,0.96),rgba(43,25,20,0.98))] text-[#f0d1c4]"
                  )}
                  disabled={!canAct}
                  onClick={() => toggleBlockerSelection(opt)}
                >
                  <span className="block min-w-0 truncate">
                    {currentDecls.length > 0 ? "[BLK] " : ""}{name}
                  </span>
                </Button>

                {decl && (
                  <div className="mt-1.5 px-1 text-[14px] text-[#d6c8ac] min-w-0 truncate">
                    -&gt; {attackerNameById.get(Number(decl.blocking)) || `Attacker ${Number(decl.blocking)}`}
                  </div>
                )}

                {isSelected && validAttackers.length > 0 && (
                  <div className="-mx-2 mt-1.5 border-y border-[rgba(128,107,78,0.36)] bg-[rgba(27,22,19,0.54)]">
                    <div className="w-full divide-y divide-[rgba(128,107,78,0.28)]">
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
                              "decision-option-row h-8 w-full justify-start rounded-none border-0 bg-[linear-gradient(180deg,rgba(49,42,36,0.94),rgba(21,18,17,0.98))] px-2.5 text-[13px] text-[#d8cbb0] transition-all hover:bg-[linear-gradient(180deg,rgba(82,66,45,0.98),rgba(33,25,19,0.98))] hover:text-[#fff1cb]",
                              blocking && "bg-[linear-gradient(180deg,rgba(95,75,50,0.98),rgba(42,32,21,0.98))] text-[#fff0cf]"
                            )}
                            disabled={!canAct}
                            onClick={() => toggleBlocker(blockerId, attackerId)}
                          >
                            <span className="min-w-0 truncate">
                              {attackerName}
                            </span>
                          </Button>
                        );
                      })}
                    </div>
                  </div>
                )}
              </div>
            );
          })}
        </div>
      </ScrollArea>

      <div className="w-full shrink-0 pt-1">
        <Button
          variant="ghost"
          size="sm"
          className="decision-neon-button decision-submit-button h-9 w-full rounded-sm px-2 text-[16px] font-bold uppercase"
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
