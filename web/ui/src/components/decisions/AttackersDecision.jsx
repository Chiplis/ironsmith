import { useState, useEffect, useCallback, useRef } from "react";
import { useGame } from "@/context/GameContext";
import { useCombatArrows } from "@/context/CombatArrowContext";
import { getCardRect, centerOf } from "@/hooks/useCardPositions";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";

const ATTACKER_COLOR = "#ff3b30";

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

/**
 * Given a drop point, try to resolve it to a valid attack target.
 * Checks planeswalker (exact card hit) first, then opponent zone (anywhere), then player target.
 */
function resolveDropTarget(x, y, validTargets) {
  const el = document.elementFromPoint(x, y);
  if (!el) return null;

  // Check planeswalker target (exact card hit only)
  const cardEl = el.closest(".game-card[data-object-id]");
  if (cardEl) {
    const objId = Number(cardEl.dataset.objectId);
    for (const t of validTargets) {
      const decoded = decodeAttackTargetChoice(t);
      if (decoded.kind === "planeswalker" && decoded.object === objId) return decoded;
    }
  }

  // Check opponent zone (anywhere on their area)
  const opponentZone = el.closest("[data-opponent-zone]");
  if (opponentZone) {
    const playerIdx = Number(opponentZone.dataset.opponentZone);
    for (const t of validTargets) {
      const decoded = decodeAttackTargetChoice(t);
      if (decoded.kind === "player" && decoded.player === playerIdx) return decoded;
    }
  }

  // Legacy: Check player target (life total / name)
  const playerEl = el.closest("[data-player-target]");
  if (playerEl) {
    const playerIdx = Number(playerEl.dataset.playerTarget);
    for (const t of validTargets) {
      const decoded = decodeAttackTargetChoice(t);
      if (decoded.kind === "player" && decoded.player === playerIdx) return decoded;
    }
  }

  return null;
}

export default function AttackersDecision({ decision, canAct }) {
  const { dispatch, state } = useGame();
  const { updateArrows, clearArrows, startDragArrow, updateDragArrow, endDragArrow, setCombatMode } = useCombatArrows();
  const options = decision.attacker_options || [];
  const players = state?.players || [];

  const [declarations, setDeclarations] = useState(() => {
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

  // Selected attacker awaiting target click (for multi-target creatures)
  const [selectedAttackerId, setSelectedAttackerId] = useState(null);
  const selectedAttackerRef = useRef(null);
  selectedAttackerRef.current = selectedAttackerId;

  const getDeclaration = (creatureId) =>
    declarations.find((d) => d.creature === Number(creatureId));

  const isAttacking = (creatureId) =>
    declarations.some((d) => d.creature === Number(creatureId));

  const toggleAttacker = useCallback((opt) => {
    const creatureId = Number(opt.creature);
    const validTargets = opt.valid_targets || [];

    if (isAttacking(creatureId)) {
      if (opt.must_attack) return;
      setDeclarations((prev) => prev.filter((d) => d.creature !== creatureId));
      setSelectedAttackerId(null);
    } else if (selectedAttackerRef.current === creatureId) {
      // Already selected for targeting — deselect
      setSelectedAttackerId(null);
    } else {
      // Select this creature — arrow will follow mouse via useEffect below
      setSelectedAttackerId(creatureId);
    }
  }, [isAttacking]);

  const selectTarget = useCallback((creatureId, target) => {
    creatureId = Number(creatureId);
    const decoded = decodeAttackTargetChoice(target);
    setDeclarations((prev) => [
      ...prev.filter((d) => d.creature !== creatureId),
      { creature: creatureId, target: decoded },
    ]);
    setSelectedAttackerId(null);
  }, []);

  // When selectedAttackerId is set, start a drag arrow from the creature
  // and track mouse movement so the arrow follows the cursor
  useEffect(() => {
    if (selectedAttackerId == null) {
      endDragArrow();
      return;
    }

    const rect = getCardRect(selectedAttackerId);
    if (rect) {
      const center = centerOf(rect);
      startDragArrow(selectedAttackerId, center.x, center.y, ATTACKER_COLOR);
    }

    const onMouseMove = (e) => {
      updateDragArrow(e.clientX, e.clientY);
    };
    document.addEventListener("mousemove", onMouseMove);
    return () => {
      document.removeEventListener("mousemove", onMouseMove);
    };
  }, [selectedAttackerId, startDragArrow, updateDragArrow, endDragArrow]);

  // Handle target area click from opponent zone
  const handleTargetAreaClick = useCallback((playerIdx, planeswalkerObjId) => {
    const selId = selectedAttackerRef.current;
    if (selId == null) return;
    const opt = options.find((o) => Number(o.creature) === selId);
    if (!opt) return;
    const validTargets = opt.valid_targets || [];

    // Check planeswalker target first (only if click was exactly on a planeswalker)
    if (planeswalkerObjId != null) {
      for (const t of validTargets) {
        const decoded = decodeAttackTargetChoice(t);
        if (decoded.kind === "planeswalker" && decoded.object === planeswalkerObjId) {
          selectTarget(selId, t);
          return;
        }
      }
    }

    // Fall back to player target
    for (const t of validTargets) {
      const decoded = decodeAttackTargetChoice(t);
      if (decoded.kind === "player" && decoded.player === playerIdx) {
        selectTarget(selId, t);
        return;
      }
    }
  }, [options, selectTarget]);

  // Handle drop from battlefield drag
  const handleDrop = useCallback((fromId, x, y) => {
    const opt = options.find((o) => Number(o.creature) === Number(fromId));
    if (!opt) return;

    const validTargets = opt.valid_targets || [];
    if (validTargets.length === 0) return;

    // If only one target, declare immediately on any drag release
    if (validTargets.length === 1) {
      const creatureId = Number(fromId);
      if (!declarations.some((d) => d.creature === creatureId)) {
        setDeclarations((prev) => [
          ...prev,
          { creature: creatureId, target: decodeAttackTargetChoice(validTargets[0]) },
        ]);
      }
      return;
    }

    // Multiple targets — resolve drop position
    const target = resolveDropTarget(x, y, validTargets);
    if (target) {
      const creatureId = Number(fromId);
      setDeclarations((prev) => [
        ...prev.filter((d) => d.creature !== creatureId),
        { creature: creatureId, target },
      ]);
      setSelectedAttackerId(null);
    }
  }, [options, declarations]);

  // Register combat mode for battlefield interaction
  useEffect(() => {
    if (!canAct) {
      setCombatMode(null);
      return;
    }
    const candidateIds = new Set(options.map((o) => Number(o.creature)));
    setCombatMode({
      mode: "attackers",
      candidates: candidateIds,
      color: ATTACKER_COLOR,
      selectedAttacker: selectedAttackerId,
      onDrop: handleDrop,
      onClick: (creatureId) => {
        const opt = options.find((o) => Number(o.creature) === Number(creatureId));
        if (opt) toggleAttacker(opt);
      },
      onTargetAreaClick: handleTargetAreaClick,
    });
    return () => setCombatMode(null);
  }, [canAct, options, handleDrop, selectedAttackerId, handleTargetAreaClick, setCombatMode, toggleAttacker]);

  // Update combat arrows when declarations change
  useEffect(() => {
    const arrowData = declarations.map((d) => ({
      fromId: d.creature,
      toId: d.target.kind === "planeswalker" ? d.target.object : null,
      toPlayerId: d.target.kind === "player" ? d.target.player : null,
      color: ATTACKER_COLOR,
      key: `atk-${d.creature}`,
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
            { type: "declare_attackers", declarations },
            `Declared ${declarations.length} attacker(s)`
          )
        }
      >
        Confirm Attackers ({declarations.length})
      </Button>
      <ScrollArea className="flex-1 min-h-0">
        <div className="flex flex-col gap-1 pr-1">
          <div className="text-[12px] text-muted-foreground">Declare attackers</div>
          {options.map((opt) => {
            const creatureId = Number(opt.creature);
            const attacking = isAttacking(creatureId);
            const name = opt.creature_name || opt.name || `Creature ${creatureId}`;
            const decl = getDeclaration(creatureId);
            const validTargets = opt.valid_targets || [];
            const isSelected = selectedAttackerId === creatureId;

            return (
              <div key={creatureId} className="flex flex-col gap-0.5">
                <Button
                  variant="outline"
                  size="sm"
                  className={cn(
                    "h-7 text-[11px] justify-start px-2",
                    attacking && "border-[rgba(174,118,255,0.95)] bg-[rgba(174,118,255,0.08)]",
                    isSelected && "border-[rgba(255,59,48,0.8)] bg-[rgba(255,59,48,0.08)]",
                    opt.must_attack && "italic"
                  )}
                  disabled={!canAct}
                  onClick={() => toggleAttacker(opt)}
                >
                  {attacking ? "\u2694 " : ""}{name}
                  {opt.must_attack && " (must attack)"}
                  {isSelected && " (select target)"}
                  {attacking && decl && validTargets.length > 1 && (
                    <span className="ml-1 text-[10px] text-muted-foreground">
                      → {attackTargetLabel(decl.target, players)}
                    </span>
                  )}
                </Button>
              </div>
            );
          })}
        </div>
      </ScrollArea>
    </div>
  );
}
