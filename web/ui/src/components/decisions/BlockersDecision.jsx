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

export default function BlockersDecision({ decision, canAct, compact = false }) {
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

    const onMouseMove = (event) => {
      updateDragArrow(event.clientX, event.clientY);
    };
    document.addEventListener("mousemove", onMouseMove);
    return () => {
      document.removeEventListener("mousemove", onMouseMove);
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

  const blockerNameById = useMemo(() => {
    const map = new Map();
    for (const opt of blockerOptions) {
      map.set(Number(opt.blocker), opt.name || `Creature ${Number(opt.blocker)}`);
    }
    return map;
  }, [blockerOptions]);

  const attackerNameById = useMemo(() => {
    const map = new Map();
    for (const opt of attackerOptions) {
      const attackerId = Number(opt.attacker);
      map.set(attackerId, opt.attacker_name || `Attacker ${attackerId}`);
    }
    return map;
  }, [attackerOptions]);

  if (compact) {
    const pendingOnlySelection = (
      selectedBlockerId != null
      && !declarations.some((d) => d.blocker === Number(selectedBlockerId))
    );

    return (
      <div className="flex h-full min-w-0 items-center gap-2">
        <div className="shrink-0 flex min-w-[308px] min-h-[34px] items-stretch gap-2">
          <div className="min-w-[110px] flex flex-col justify-center">
            <div className="text-[11px] font-bold uppercase tracking-[0.14em] text-[#93c7ff]">
              {canAct ? "Your Action" : "Opponent Action"}
            </div>
            <div className="text-[10px] text-[#b8d2ef]">
              Blockers
            </div>
          </div>
          <Button
            variant="ghost"
            size="sm"
            className="decision-neon-button decision-submit-button w-[176px] shrink-0 self-stretch rounded-none px-2 py-1 text-[13px] font-bold uppercase"
            disabled={!canAct}
            onClick={() =>
              dispatch(
                { type: "declare_blockers", declarations },
                `Declared ${declarations.length} blocker(s)`
              )
            }
          >
            Confirm ({declarations.length})
          </Button>
        </div>

        <div className="min-w-0 flex-1 overflow-x-auto overflow-y-hidden whitespace-nowrap">
          <div className="flex w-max min-w-full items-center gap-1.5 pr-2">
            {declarations.length === 0 && !pendingOnlySelection && (
              <span className="text-[12px] text-[#b8d2ef]">
                Select a blocker, then point to the attacker it should block.
              </span>
            )}
            {pendingOnlySelection && (
              <button
                type="button"
                className="inline-flex h-7 items-center rounded border border-[rgba(59,130,246,0.75)] bg-[rgba(16,41,76,0.72)] px-2.5 text-[12px] font-semibold text-[#dbeafe]"
                disabled={!canAct}
                onClick={() => setSelectedBlockerId(null)}
              >
                {(blockerNameById.get(Number(selectedBlockerId)) || `Creature ${Number(selectedBlockerId)}`)} -&gt; ?
              </button>
            )}
            {declarations.map((decl) => {
              const blockerName = blockerNameById.get(Number(decl.blocker)) || `Creature ${Number(decl.blocker)}`;
              const attackerName = attackerNameById.get(Number(decl.blocking)) || `Attacker ${Number(decl.blocking)}`;
              return (
                <button
                  type="button"
                  key={`compact-blk-${decl.blocker}-${decl.blocking}`}
                  className="inline-flex h-7 items-center rounded border border-[#4f7cad] bg-[rgba(24,43,64,0.78)] px-2.5 text-[12px] font-semibold text-[#d7ebff] transition-colors hover:border-[#7eb1e5] hover:bg-[rgba(34,58,84,0.9)]"
                  disabled={!canAct}
                  onClick={() => setSelectedBlockerId(Number(decl.blocker))}
                >
                  {blockerName} -&gt; {attackerName}
                </button>
              );
            })}
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="flex h-full min-h-0 w-full flex-col gap-2 overflow-x-hidden">
      <ScrollArea className="flex-1 min-h-0 w-full overflow-x-hidden">
        <div className="flex flex-col gap-2 pr-1 overflow-x-hidden">
          <div className="px-0.5 text-[13px] font-bold uppercase tracking-wider text-[#a4c2e2]">Declare blockers</div>
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
                  "min-w-0 rounded-sm px-2 py-1.5 border-l-[3px] border-[#2a3b4d] bg-[rgba(7,15,23,0.35)]",
                  currentDecls.length > 0 && "border-[rgba(105,181,247,0.9)] bg-[rgba(20,39,58,0.52)]",
                  isSelected && "border-[rgba(59,130,246,0.92)] bg-[rgba(17,32,60,0.58)] shadow-[inset_0_0_0_1px_rgba(96,165,250,0.24)]"
                )}
              >
                <Button
                  variant="ghost"
                  size="sm"
                  className={cn(
                    "h-auto min-h-10 w-full min-w-0 overflow-hidden justify-start rounded-sm border px-3 py-2 text-left text-[15px] font-semibold leading-snug whitespace-normal",
                    "border-[#2f4f70] bg-[rgba(15,27,40,0.9)] text-[#d6e7fb] hover:border-[#4f7cad] hover:bg-[rgba(24,43,64,0.95)]",
                    currentDecls.length > 0 && "border-[rgba(105,181,247,0.95)] bg-[rgba(36,58,84,0.5)] text-[#dff1ff]",
                    isSelected && "border-[rgba(59,130,246,0.92)] bg-[rgba(19,42,78,0.58)] text-[#dbeafe]"
                  )}
                  disabled={!canAct}
                  onClick={() => toggleBlockerSelection(opt)}
                >
                  <span className="block min-w-0 truncate">
                    {currentDecls.length > 0 ? "[BLK] " : ""}{name}
                  </span>
                </Button>

                {decl && (
                  <div className="mt-1.5 px-1 text-[14px] text-[#bcd0e8] min-w-0 truncate">
                    -&gt; {attackerNameById.get(Number(decl.blocking)) || `Attacker ${Number(decl.blocking)}`}
                  </div>
                )}

                {isSelected && validAttackers.length > 0 && (
                  <div className="-mx-2 mt-1.5 border-y border-[#2f4b67] bg-[rgba(10,20,30,0.45)]">
                    <div className="w-full divide-y divide-[#2f4b67]">
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
                              "h-8 w-full justify-start rounded-none border-0 bg-[rgba(15,27,40,0.9)] px-2.5 text-[13px] text-[#c7dbf2] transition-all hover:bg-[rgba(25,44,66,0.95)] hover:text-[#eaf3ff]",
                              blocking && "bg-[rgba(36,58,84,0.72)] text-[#eaf4ff]"
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
