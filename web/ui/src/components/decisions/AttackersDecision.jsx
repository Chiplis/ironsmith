import { useState, useEffect, useCallback, useRef, useMemo } from "react";
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
    const p = players.find((pl) => Number(pl.id ?? pl.index) === target.player);
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
  const options = useMemo(() => decision.attacker_options || [], [decision.attacker_options]);
  const players = state?.players || [];
  const optionsRef = useRef(options);

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
  const declarationsRef = useRef(declarations);

  useEffect(() => {
    optionsRef.current = options;
  }, [options]);

  useEffect(() => {
    selectedAttackerRef.current = selectedAttackerId;
  }, [selectedAttackerId]);

  useEffect(() => {
    declarationsRef.current = declarations;
  }, [declarations]);

  const getDeclaration = (creatureId) =>
    declarations.find((d) => d.creature === Number(creatureId));

  const isAttacking = (creatureId) =>
    declarations.some((d) => d.creature === Number(creatureId));

  const toggleAttacker = useCallback((opt) => {
    const creatureId = Number(opt.creature);
    const currentDeclarations = declarationsRef.current || [];

    if (currentDeclarations.some((d) => d.creature === creatureId)) {
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
  }, []);

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
    const opt = (optionsRef.current || []).find((o) => Number(o.creature) === selId);
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
  }, [selectTarget]);

  // Handle drop from battlefield drag
  const handleDrop = useCallback((fromId, x, y) => {
    const opt = (optionsRef.current || []).find((o) => Number(o.creature) === Number(fromId));
    if (!opt) return;

    const validTargets = opt.valid_targets || [];
    if (validTargets.length === 0) return;

    // If only one target, declare immediately on any drag release
    if (validTargets.length === 1) {
      const creatureId = Number(fromId);
      setDeclarations((prev) => (
        prev.some((d) => d.creature === creatureId)
          ? prev
          : [...prev, { creature: creatureId, target: decodeAttackTargetChoice(validTargets[0]) }]
      ));
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
  }, []);

  const combatOptionsKey = options
    .map((o) => {
      const targets = (o.valid_targets || [])
        .map((t) => JSON.stringify(t))
        .join(",");
      return `${Number(o.creature)}:${o.must_attack ? 1 : 0}:${targets}`;
    })
    .join("|");

  // Register combat mode for battlefield interaction
  useEffect(() => {
    if (!canAct) {
      setCombatMode(null);
      return;
    }
    const currentOptions = optionsRef.current || [];
    const candidateIds = new Set(currentOptions.map((o) => Number(o.creature)));
    const validTargetObjectsByAttacker = {};
    const validTargetPlayersByAttacker = {};
    for (const opt of currentOptions) {
      const creatureId = Number(opt.creature);
      const objectTargets = new Set();
      const playerTargets = new Set();
      for (const target of opt.valid_targets || []) {
        const decoded = decodeAttackTargetChoice(target);
        if (decoded.kind === "planeswalker" && Number.isFinite(decoded.object)) {
          objectTargets.add(Number(decoded.object));
        } else if (decoded.kind === "player" && Number.isFinite(decoded.player)) {
          playerTargets.add(Number(decoded.player));
        }
      }
      validTargetObjectsByAttacker[creatureId] = objectTargets;
      validTargetPlayersByAttacker[creatureId] = playerTargets;
    }
    const activeAttackerId = selectedAttackerId != null ? Number(selectedAttackerId) : null;
    const validTargetObjects = (
      activeAttackerId != null
        ? (validTargetObjectsByAttacker[activeAttackerId] || new Set())
        : new Set()
    );
    const validTargetPlayers = (
      activeAttackerId != null
        ? (validTargetPlayersByAttacker[activeAttackerId] || new Set())
        : new Set()
    );
    setCombatMode({
      mode: "attackers",
      candidates: candidateIds,
      validTargetObjectsByAttacker,
      validTargetPlayersByAttacker,
      validTargetObjects,
      validTargetPlayers,
      color: ATTACKER_COLOR,
      selectedAttacker: selectedAttackerId,
      onDrop: handleDrop,
      onClick: (creatureId) => {
        const opt = (optionsRef.current || []).find((o) => Number(o.creature) === Number(creatureId));
        if (opt) toggleAttacker(opt);
      },
      onTargetAreaClick: handleTargetAreaClick,
    });
    return () => setCombatMode(null);
  }, [canAct, combatOptionsKey, handleDrop, selectedAttackerId, handleTargetAreaClick, setCombatMode, toggleAttacker]);

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
    <div className="flex h-full min-h-0 w-full flex-col gap-2 overflow-x-hidden">
      <ScrollArea className="flex-1 min-h-0 w-full overflow-x-hidden">
        <div className="flex flex-col gap-2 pr-1 overflow-x-hidden">
          <div className="px-0.5 text-[13px] font-bold uppercase tracking-wider text-[#a4c2e2]">Declare attackers</div>
          {options.map((opt) => {
            const creatureId = Number(opt.creature);
            const attacking = isAttacking(creatureId);
            const name = opt.creature_name || opt.name || `Creature ${creatureId}`;
            const decl = getDeclaration(creatureId);
            const validTargets = opt.valid_targets || [];
            const isSelected = selectedAttackerId === creatureId;

            return (
              <div
                key={creatureId}
                className={cn(
                  "min-w-0 rounded-sm px-2 py-1.5 border-l-[3px] border-[#2a3b4d] bg-[rgba(7,15,23,0.35)]",
                  attacking && "border-[rgba(174,118,255,0.85)] bg-[rgba(38,24,58,0.48)]",
                  isSelected && "border-[rgba(255,59,48,0.85)] bg-[rgba(52,20,24,0.5)] shadow-[inset_0_0_0_1px_rgba(255,59,48,0.25)]"
                )}
              >
                <Button
                  variant="ghost"
                  size="sm"
                  className={cn(
                    "h-auto min-h-10 w-full min-w-0 overflow-hidden justify-start rounded-sm border px-3 py-2 text-left text-[15px] font-semibold leading-snug whitespace-normal",
                    "border-[#2f4f70] bg-[rgba(15,27,40,0.9)] text-[#d6e7fb] hover:border-[#4f7cad] hover:bg-[rgba(24,43,64,0.95)]",
                    attacking && "border-[rgba(174,118,255,0.95)] bg-[rgba(65,38,102,0.5)] text-[#eadbff]",
                    isSelected && "border-[rgba(255,59,48,0.85)] bg-[rgba(78,28,33,0.52)] text-[#ffd2cf]",
                    opt.must_attack && "italic"
                  )}
                  disabled={!canAct}
                  onClick={() => toggleAttacker(opt)}
                >
                  <span className="block min-w-0 truncate">
                    {attacking ? "[ATK] " : ""}{name}
                    {opt.must_attack && " (must attack)"}
                  </span>
                </Button>

                {attacking && decl && (
                  <div className="mt-1.5 px-1 text-[14px] text-[#bcd0e8] min-w-0 truncate">
                    -&gt; {attackTargetLabel(decl.target, players)}
                  </div>
                )}

                {isSelected && validTargets.length > 1 && (
                  <div className="mt-1.5 flex flex-wrap gap-1.5">
                    {validTargets.map((target, i) => (
                      <Button
                        key={`${creatureId}-target-${i}`}
                        variant="ghost"
                        size="sm"
                        className="h-8 max-w-full min-w-0 overflow-hidden rounded-full border border-[#325474] bg-[rgba(15,27,40,0.9)] px-3 text-[13px] font-semibold text-[#c7dbf2] transition-all hover:border-[#4f7cad] hover:bg-[rgba(25,44,66,0.95)] hover:text-[#eaf3ff]"
                        disabled={!canAct}
                        onClick={() => selectTarget(creatureId, target)}
                      >
                        <span className="min-w-0 truncate">
                          {attackTargetLabel(decodeAttackTargetChoice(target), players)}
                        </span>
                      </Button>
                    ))}
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
          className="w-full h-10 rounded-sm border border-[#315274] bg-[rgba(15,27,40,0.88)] px-3 text-[16px] font-bold text-[#8ec4ff] transition-all hover:border-[#4f7cad] hover:bg-[rgba(24,43,64,0.95)] hover:text-[#d7ebff]"
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
    </div>
  );
}
