import { useState, useEffect, useCallback } from "react";
import { useGame } from "@/context/GameContext";
import { useCombatArrows } from "@/context/CombatArrowContext";
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

/**
 * Given a drop point, try to resolve it to a valid attack target.
 * Checks for data-player-target (player) or data-object-id (planeswalker).
 */
function resolveDropTarget(x, y, validTargets) {
  const el = document.elementFromPoint(x, y);
  if (!el) return null;

  // Check player target
  const playerEl = el.closest("[data-player-target]");
  if (playerEl) {
    const playerIdx = Number(playerEl.dataset.playerTarget);
    // Verify it's a valid target
    for (const t of validTargets) {
      const decoded = decodeAttackTargetChoice(t);
      if (decoded.kind === "player" && decoded.player === playerIdx) return decoded;
    }
  }

  // Check planeswalker target
  const cardEl = el.closest("[data-object-id]");
  if (cardEl) {
    const objId = Number(cardEl.dataset.objectId);
    for (const t of validTargets) {
      const decoded = decodeAttackTargetChoice(t);
      if (decoded.kind === "planeswalker" && decoded.object === objId) return decoded;
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

  const [choosingTarget, setChoosingTarget] = useState(null);

  const getDeclaration = (creatureId) =>
    declarations.find((d) => d.creature === Number(creatureId));

  const isAttacking = (creatureId) =>
    declarations.some((d) => d.creature === Number(creatureId));

  const toggleAttacker = (opt) => {
    const creatureId = Number(opt.creature);
    const validTargets = opt.valid_targets || [];

    if (isAttacking(creatureId)) {
      if (opt.must_attack) return;
      setDeclarations((prev) => prev.filter((d) => d.creature !== creatureId));
      setChoosingTarget(null);
    } else if (validTargets.length <= 1) {
      const target = validTargets[0];
      if (!target) return;
      setDeclarations((prev) => [
        ...prev,
        { creature: creatureId, target: decodeAttackTargetChoice(target) },
      ]);
    } else {
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
      color: "#f0ce61",
      onDrop: handleDrop,
      onClick: (creatureId) => {
        const opt = options.find((o) => Number(o.creature) === Number(creatureId));
        if (opt) toggleAttacker(opt);
      },
    });
    return () => setCombatMode(null);
  }, [canAct, options, handleDrop, setCombatMode]);

  // Update combat arrows when declarations change
  useEffect(() => {
    const arrowData = declarations.map((d) => ({
      fromId: d.creature,
      toId: d.target.kind === "planeswalker" ? d.target.object : null,
      toPlayerId: d.target.kind === "player" ? d.target.player : null,
      color: "#f0ce61",
      key: `atk-${d.creature}`,
    }));
    updateArrows(arrowData);
  }, [declarations, updateArrows]);

  useEffect(() => clearArrows, [clearArrows]);

  return (
    <div className="flex flex-col gap-2 overflow-visible">
      <div className="text-[13px] text-muted-foreground">
        Declare attackers — drag creatures to targets
      </div>
      <div className="flex flex-wrap gap-2 overflow-visible py-1 -mx-1 px-1">
        {options.map((opt) => {
          const creatureId = Number(opt.creature);
          const attacking = isAttacking(creatureId);
          const name = opt.creature_name || opt.name || `Creature ${creatureId}`;
          const decl = getDeclaration(creatureId);
          const validTargets = opt.valid_targets || [];
          const isChoosingTarget = choosingTarget === creatureId;

          return (
            <div key={creatureId} className="flex flex-col gap-1 min-w-0 overflow-visible">
              <Button
                variant="ghost"
                size="sm"
                className={cn(
                  "combat-btn h-7 text-[14px] justify-start px-3 text-muted-foreground",
                  attacking && "combat-active text-[rgba(174,118,255,0.95)]",
                  opt.must_attack && "italic"
                )}
                disabled={!canAct}
                onClick={() => toggleAttacker(opt)}
              >
                {attacking ? <Swords className="size-3.5 inline mr-1" /> : ""}{name}
                {opt.must_attack && " (must)"}
                {attacking && decl && validTargets.length > 1 && (
                  <span className="ml-1 text-[13px] text-muted-foreground inline-flex items-center gap-0.5">
                    <ArrowRight className="size-3" /> {attackTargetLabel(decl.target, players)}
                  </span>
                )}
              </Button>

              {isChoosingTarget && (
                <div className="ml-2 flex flex-wrap gap-1 items-center overflow-visible">
                  <div className="text-[12px] text-muted-foreground">Target:</div>
                  {validTargets.map((target, tIdx) => {
                    const decoded = decodeAttackTargetChoice(target);
                    const label = attackTargetLabel(decoded, players);
                    return (
                      <Button
                        key={tIdx}
                        variant="ghost"
                        size="sm"
                        className="combat-btn h-5 text-[12px] justify-start px-2 text-muted-foreground"
                        disabled={!canAct}
                        onClick={() => selectTarget(creatureId, target)}
                      >
                        {label}
                      </Button>
                    );
                  })}
                </div>
              )}
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
