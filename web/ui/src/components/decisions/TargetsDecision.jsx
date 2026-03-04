import { useState, useMemo, useEffect, useRef } from "react";
import { useGame } from "@/context/GameContext";
import { useHoveredObjectId } from "@/context/HoverContext";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";
import { X, ArrowRight } from "lucide-react";

function targetObjectId(target) {
  if (!target || target.kind === "player") return null;
  if (target.object != null) return String(target.object);
  if (target.id != null) return String(target.id);
  return null;
}

function targetListKey(target) {
  if (!target) return "unknown";
  if (target.kind === "player") return `player:${target.player}`;
  const objectId = targetObjectId(target);
  if (objectId != null) return `object:${objectId}`;
  return "object:unknown";
}

function targetsMatch(left, right) {
  if (!left || !right) return false;
  if (left.kind !== right.kind) return false;
  if (left.kind === "player") return Number(left.player) === Number(right.player);
  return Number(targetObjectId(left)) === Number(targetObjectId(right));
}

function toDispatchTarget(target) {
  if (target.kind === "player") {
    return { kind: "player", player: Number(target.player) };
  }
  return { kind: "object", object: Number(targetObjectId(target)) };
}

function buildTargetNameMaps(state) {
  const objectNames = new Map();
  const playerNames = new Map();
  const players = state?.players || [];
  const zones = ["battlefield", "hand_cards", "graveyard_cards", "exile_cards"];

  for (const player of players) {
    const pid = Number(player?.id);
    const pidx = Number(player?.index);
    if (Number.isFinite(pid)) playerNames.set(pid, player?.name);
    if (Number.isFinite(pidx)) playerNames.set(pidx, player?.name);

    for (const zone of zones) {
      for (const card of player?.[zone] || []) {
        const cardId = Number(card?.id);
        if (Number.isFinite(cardId) && card?.name) {
          objectNames.set(cardId, card.name);
        }
        if (Array.isArray(card?.member_ids)) {
          for (let i = 0; i < card.member_ids.length; i += 1) {
            const memberId = Number(card.member_ids[i]);
            if (!Number.isFinite(memberId)) continue;
            const memberName = Array.isArray(card.member_names) ? card.member_names[i] : null;
            objectNames.set(memberId, memberName || card?.name || objectNames.get(memberId));
          }
        }
      }
    }
  }

  for (const stackObject of state?.stack_objects || []) {
    const stackId = Number(stackObject?.id);
    if (Number.isFinite(stackId) && stackObject?.name) {
      objectNames.set(stackId, stackObject.name);
    }
  }

  return { objectNames, playerNames };
}

function isGenericObjectName(name, objectId = null) {
  if (!name) return true;
  const trimmed = String(name).trim();
  if (!trimmed) return true;
  if (objectId != null && trimmed === `Object #${objectId}`) return true;
  return /^Object\s+#?\d+$/i.test(trimmed);
}

function pickBestTargetName({ target, legalName, targetName, objectNames, playerNames }) {
  if (!target) return null;
  if (target.kind === "player") {
    const playerId = Number(target.player);
    return legalName || targetName || playerNames.get(playerId) || null;
  }

  const objectId = Number(targetObjectId(target));
  const fromState = objectNames.get(objectId) || null;
  if (legalName && !isGenericObjectName(legalName, objectId)) return legalName;
  if (targetName && !isGenericObjectName(targetName, objectId)) return targetName;
  return fromState || legalName || targetName || null;
}

function ActiveRequirementTargets({
  req,
  reqIdx,
  canAct,
  isActive,
  canSelectMore,
  selectedTargets = [],
  hoveredObjectId,
  onSelectTarget,
  onSkipRequirement,
  showSkip,
  skipLabel,
}) {
  const legalTargets = req.legal_targets || [];
  const objectTargets = legalTargets.filter((target) => targetObjectId(target) != null);
  const hasHoverMatch = hoveredObjectId != null
    && objectTargets.some((target) => targetObjectId(target) === String(hoveredObjectId));
  const scopedTargets = legalTargets;

  const hideTimerRef = useRef(null);
  const targetButtonRefs = useRef(new Map());
  const [visibleTargets, setVisibleTargets] = useState(scopedTargets);
  const showRows = scopedTargets.length > 0;

  useEffect(() => {
    if (hideTimerRef.current) {
      clearTimeout(hideTimerRef.current);
      hideTimerRef.current = null;
    }
    hideTimerRef.current = setTimeout(() => {
      setVisibleTargets(showRows ? scopedTargets : []);
      hideTimerRef.current = null;
    }, showRows ? 0 : 180);
  }, [scopedTargets, showRows]);

  useEffect(
    () => () => {
      if (hideTimerRef.current) {
        clearTimeout(hideTimerRef.current);
        hideTimerRef.current = null;
      }
    },
    []
  );

  useEffect(() => {
    if (!hasHoverMatch || hoveredObjectId == null) return;
    const key = `object:${String(hoveredObjectId)}`;
    if (!visibleTargets.some((target) => targetListKey(target) === key)) return;
    const node = targetButtonRefs.current.get(key);
    if (!node) return;
    node.scrollIntoView({ block: "nearest", inline: "nearest", behavior: "smooth" });
  }, [hasHoverMatch, hoveredObjectId, visibleTargets]);

  return (
    <div className="flex flex-wrap gap-1">
      <div
        className={`flex flex-wrap gap-1 transition-all duration-200 ${
          showRows ? "opacity-100 translate-y-0" : "opacity-0 -translate-y-1 pointer-events-none"
        }`}
      >
        {visibleTargets.map((target, tIdx) => {
          const listKey = targetListKey(target);
          const isSelected = selectedTargets.some((selection) => targetsMatch(selection, target));
          const isHoveredTarget =
            hoveredObjectId != null && listKey === `object:${String(hoveredObjectId)}`;
          const isUnavailable = !isSelected && (!isActive || !canSelectMore);
          const label =
            target.kind === "player"
              ? target.name || `Player ${target.player}`
              : target.name || `Object ${target.object}`;
          return (
            <Button
              key={`${listKey}:${tIdx}`}
              variant="ghost"
              size="sm"
              className={cn(
                "h-6 rounded-full border border-[#325474] bg-[rgba(15,27,40,0.9)] text-[13px] px-2.5 text-[#c7dbf2] transition-all hover:border-[#4f7cad] hover:bg-[rgba(25,44,66,0.95)] hover:text-[#eaf3ff]",
                isSelected
                  && "border-[#6cb0ff] bg-[rgba(29,56,84,0.95)] text-[#eaf4ff] shadow-[0_0_10px_rgba(100,169,255,0.45),inset_0_0_0_1px_rgba(170,215,255,0.35)]",
                !isSelected && isHoveredTarget
                  && "border-[#5e97d1] bg-[rgba(25,47,71,0.94)] text-[#d9ecff]",
                isUnavailable
                  && "border-[#2e445a] bg-[rgba(12,20,30,0.72)] text-[#647f99] hover:border-[#2e445a] hover:bg-[rgba(12,20,30,0.72)] hover:text-[#647f99]"
              )}
              disabled={!canAct || isUnavailable}
              onClick={() => onSelectTarget(target, reqIdx, { toggleExisting: true, strictRequirement: true })}
              ref={(node) => {
                if (node) {
                  targetButtonRefs.current.set(listKey, node);
                } else {
                  targetButtonRefs.current.delete(listKey);
                }
              }}
            >
              {label}
            </Button>
          );
        })}
      </div>
      {showSkip && (
        <Button
          variant="ghost"
          size="sm"
          className="h-5 rounded-full border border-[#2a3d52] px-2 text-[12px] !text-[#9ab6d3] hover:border-[#3f5f83] hover:bg-[rgba(17,30,46,0.92)] hover:!text-[#ddecff]"
          disabled={!canAct}
          onClick={onSkipRequirement}
        >
          {skipLabel}
        </Button>
      )}
    </div>
  );
}

export default function TargetsDecision({ decision, canAct }) {
  const { dispatch, state } = useGame();
  const hoveredObjectId = useHoveredObjectId();
  const requirements = decision.requirements || [];
  const { objectNames: objectNamesById, playerNames: playerNamesById } = useMemo(
    () => buildTargetNameMaps(state),
    [state]
  );
  const [currentReqIdx, setCurrentReqIdx] = useState(0);
  // Per-requirement selections: array of arrays
  const [selectionsByReq, setSelectionsByReq] = useState(() =>
    requirements.map(() => [])
  );

  const currentReq = requirements[currentReqIdx];
  const allDone = currentReqIdx >= requirements.length;

  // Flat list of all selections for dispatch
  const allSelections = useMemo(
    () => selectionsByReq.flat(),
    [selectionsByReq]
  );

  // Check if current requirement has met its minimum
  const currentReqSelections = selectionsByReq[currentReqIdx] || [];
  const currentMin = currentReq?.min_targets ?? 1;
  const currentMet = currentReqSelections.length >= currentMin;

  // Overall: all requirements met their minimums
  const allMinsMet = requirements.every(
    (req, idx) => (selectionsByReq[idx] || []).length >= (req.min_targets ?? 1)
  );

  // Can submit: either all done cycling through, or all mins are met
  const canSubmit = allDone || allMinsMet;

  const handleSelectTarget = (
    target,
    preferredReqIdx = currentReqIdx,
    { toggleExisting = false, strictRequirement = false } = {}
  ) => {
    const targetInput = toDispatchTarget(target);
    if (targetInput.kind === "player" && !Number.isFinite(targetInput.player)) return;
    if (targetInput.kind === "object" && !Number.isFinite(targetInput.object)) return;

    setSelectionsByReq((prev) => {
      const next = prev.map((arr) => [...arr]);
      const findReqSelectionIndex = (reqIdx) =>
        (next[reqIdx] || []).findIndex((selection) => targetsMatch(selection, targetInput));

      if (toggleExisting) {
        if (strictRequirement) {
          const removeIdx = findReqSelectionIndex(preferredReqIdx);
          if (removeIdx >= 0) {
            next[preferredReqIdx] = next[preferredReqIdx].filter((_, idx) => idx !== removeIdx);
            setTimeout(() => setCurrentReqIdx(preferredReqIdx), 0);
            return next;
          }
        } else {
          const selectedReqIdx = next.findIndex((_, reqIdx) => findReqSelectionIndex(reqIdx) >= 0);
          if (selectedReqIdx >= 0) {
            const removeIdx = findReqSelectionIndex(selectedReqIdx);
            next[selectedReqIdx] = next[selectedReqIdx].filter((_, idx) => idx !== removeIdx);
            setTimeout(() => setCurrentReqIdx(selectedReqIdx), 0);
            return next;
          }
        }
      }

      const reqCanAcceptTarget = (reqIdx) => {
        const req = requirements[reqIdx];
        if (!req) return false;
        const legal = (req.legal_targets || []).some((candidate) =>
          targetsMatch(candidate, targetInput)
        );
        if (!legal) return false;
        const reqMax = req?.max_targets ?? req?.legal_targets?.length ?? 1;
        return (next[reqIdx] || []).length < reqMax;
      };

      let reqIdx = preferredReqIdx;
      if (!reqCanAcceptTarget(reqIdx)) {
        if (strictRequirement) return prev;
        reqIdx = requirements.findIndex((_, idx) => reqCanAcceptTarget(idx));
      }
      if (reqIdx < 0) return prev;

      if (findReqSelectionIndex(reqIdx) >= 0) return prev;

      const req = requirements[reqIdx];
      const legalMatch = (req?.legal_targets || []).find((candidate) =>
        targetsMatch(candidate, targetInput)
      );
      const selectedName = pickBestTargetName({
        target: targetInput,
        legalName: legalMatch?.name,
        targetName: target?.name,
        objectNames: objectNamesById,
        playerNames: playerNamesById,
      });
      const selectedTarget = {
        ...targetInput,
        name: selectedName,
      };
      next[reqIdx] = [...(next[reqIdx] || []), selectedTarget];

      // Auto-advance if we've hit max for this requirement
      const reqMax =
        requirements[reqIdx]?.max_targets
          ?? requirements[reqIdx]?.legal_targets?.length
          ?? 1;
      if (next[reqIdx].length >= reqMax) {
        // Find next unfilled requirement
        let nextIdx = reqIdx + 1;
        while (nextIdx < requirements.length) {
          const reqMin = requirements[nextIdx].min_targets ?? 1;
          if ((next[nextIdx] || []).length < reqMin) break;
          nextIdx++;
        }
        // Use setTimeout to batch with the state update
        setTimeout(() => setCurrentReqIdx(nextIdx), 0);
      } else if (reqIdx !== currentReqIdx) {
        setTimeout(() => setCurrentReqIdx(reqIdx), 0);
      }
      return next;
    });
  };

  useEffect(() => {
    const onExternalTargetChoice = (event) => {
      if (!canAct) return;
      const target = event?.detail?.target;
      if (!target || (target.kind !== "player" && target.kind !== "object")) return;
      handleSelectTarget(target, currentReqIdx, { toggleExisting: true });
    };

    window.addEventListener("ironsmith:target-choice", onExternalTargetChoice);
    return () => {
      window.removeEventListener("ironsmith:target-choice", onExternalTargetChoice);
    };
  }, [canAct, currentReqIdx, handleSelectTarget]);

  const handleRemoveTarget = (reqIdx, selIdx) => {
    setSelectionsByReq((prev) => {
      const next = prev.map((arr) => [...arr]);
      next[reqIdx] = next[reqIdx].filter((_, i) => i !== selIdx);
      return next;
    });
    // Jump back to this requirement if needed
    if (reqIdx < currentReqIdx) {
      setCurrentReqIdx(reqIdx);
    }
  };

  const handleSkipRequirement = () => {
    if (currentReqIdx + 1 <= requirements.length) {
      setCurrentReqIdx(currentReqIdx + 1);
    }
  };

  const handleSubmit = () => {
    dispatch(
      { type: "select_targets", targets: allSelections.map(toDispatchTarget) },
      "Targets selected"
    );
  };

  if (requirements.length === 0) return null;

  return (
    <div className="flex h-full min-h-0 w-full flex-col gap-1.5">
      <ScrollArea
        className="min-h-0 w-full"
        style={{ maxHeight: "calc(100% - 3.25rem)" }}
      >
        <div className="flex flex-col gap-1.5 pr-1">
          <div className="grid gap-1.5">
          {requirements.map((req, reqIdx) => {
            const isActive = reqIdx === currentReqIdx && !allDone;
            const reqSelections = selectionsByReq[reqIdx] || [];
            const reqMin = req.min_targets ?? 1;
            const reqMax = req.max_targets ?? req.legal_targets?.length ?? 1;
            const isOptional = reqMin === 0;
            const canSelectMore = reqSelections.length < reqMax;
            const shouldShowSelectedChips = reqSelections.length > 0 && !isActive;
            const shouldShowTargetOptions = isActive;

            return (
              <div
                key={reqIdx}
                className={cn(
                  "rounded-sm px-1.5 py-1 border-l-2 border-[#2a3b4d] bg-[rgba(7,15,23,0.35)]",
                  isActive && "border-[#5f9ad6] bg-[rgba(18,34,52,0.56)] shadow-[inset_0_0_0_1px_rgba(95,154,214,0.2)]"
                )}
              >
                <div className="mb-1 text-[13px] text-[#b6cae1] leading-snug">
                  <span className="font-semibold text-[#d6e7fa]">Target {reqIdx + 1}:</span>{" "}
                  {req.description || "Choose a target"}
                  <span className="ml-1 text-[12px] text-[#8ba4c1]">
                    ({reqMin}-{req.max_targets ?? req.legal_targets?.length ?? "?"}{isOptional ? ", optional" : ""})
                  </span>
                </div>

                {/* Show current selections for this requirement */}
                {shouldShowSelectedChips && (
                  <div className="mb-1 flex flex-wrap gap-0.5">
                    {reqSelections.map((sel, selIdx) => {
                      const selectedName = pickBestTargetName({
                        target: sel,
                        legalName: sel.name,
                        targetName: sel.name,
                        objectNames: objectNamesById,
                        playerNames: playerNamesById,
                      });
                      const label =
                        selectedName
                          || (sel.kind === "player"
                            ? `Player ${sel.player}`
                            : `Object ${sel.object}`);
                      return (
                        <Button
                          key={selIdx}
                          variant="ghost"
                          size="sm"
                          className="h-5 rounded-full border border-[#4a6f94] bg-[rgba(22,40,60,0.9)] px-1.5 text-[12px] text-[#d7e8fa] hover:border-[#6993bf] hover:bg-[rgba(29,52,78,0.95)]"
                          disabled={!canAct}
                          onClick={() => handleRemoveTarget(reqIdx, selIdx)}
                        >
                          {label} <X className="size-3 inline ml-1" />
                        </Button>
                      );
                    })}
                  </div>
                )}

                {shouldShowTargetOptions && (
                  <ActiveRequirementTargets
                    req={req}
                    reqIdx={reqIdx}
                    canAct={canAct}
                    isActive={isActive}
                    canSelectMore={canSelectMore}
                    selectedTargets={reqSelections}
                    hoveredObjectId={hoveredObjectId}
                    onSelectTarget={handleSelectTarget}
                    onSkipRequirement={handleSkipRequirement}
                    showSkip={isActive && (isOptional || currentMet) && !allDone}
                    skipLabel={isOptional ? "Skip (optional)" : <>Next requirement <ArrowRight className="size-3 inline" /></>}
                  />
                )}
              </div>
            );
          })}
          </div>
        </div>
      </ScrollArea>

      <div className="w-full shrink-0 pt-1">
        <Button
          variant="ghost"
          size="sm"
          className="w-full h-7 rounded-sm border border-[#315274] bg-[rgba(15,27,40,0.88)] px-3 text-[13px] font-semibold text-[#8ec4ff] transition-all hover:border-[#4f7cad] hover:bg-[rgba(24,43,64,0.95)] hover:text-[#d7ebff]"
          disabled={!canAct || !canSubmit}
          onClick={handleSubmit}
        >
          Submit Targets ({allSelections.length})
        </Button>
      </div>
    </div>
  );
}
