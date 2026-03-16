import { useState, useMemo, useEffect, useRef, useCallback } from "react";
import { useGame } from "@/context/GameContext";
import { useHoveredObjectId } from "@/context/HoverContext";
import { useCombatArrows } from "@/context/useCombatArrows";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { getCardRect, centerOf } from "@/hooks/useCardPositions";
import { getPlayerAccent } from "@/lib/player-colors";
import { usePointerClickGuard } from "@/lib/usePointerClickGuard";
import { X, ArrowRight } from "lucide-react";
import DecisionSummary from "./DecisionSummary";
import { getVisibleStackObjects, getVisibleTopStackObject } from "@/lib/stack-targets";

const STRIP_ITEM_BASE_CLASS = "decision-option-row decision-option-row--strip h-8 max-w-[360px] min-w-[120px] justify-start self-stretch px-2.5 text-[12px] font-semibold";
const STRIP_ITEM_ACTIVE_CLASS = "is-selected";
const STRIP_ITEM_DISABLED_CLASS = "is-disabled";
const STRIP_META_ITEM_CLASS = "decision-target-meta inline-flex h-8 max-w-[460px] min-w-[220px] items-center self-stretch px-2.5 text-[12px] font-semibold whitespace-nowrap";

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

  for (const stackObject of getVisibleStackObjects(state)) {
    const stackId = Number(stackObject?.id);
    if (Number.isFinite(stackId) && stackObject?.name) {
      objectNames.set(stackId, stackObject.name);
    }
  }

  return { objectNames, playerNames };
}

function normalizeNumericId(value) {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : null;
}

function stackObjectMatchesDecisionSource(stackObject, decision) {
  if (!stackObject || !decision) return false;

  const decisionSourceId = normalizeNumericId(decision?.source_id);
  const decisionSourceName = String(decision?.source_name || "").trim().toLowerCase();
  const topStackId = normalizeNumericId(stackObject?.id);
  const topInspectId = normalizeNumericId(stackObject?.inspect_object_id);
  const topName = String(stackObject?.name || "").trim().toLowerCase();

  if (
    decisionSourceId != null
    && (decisionSourceId === topStackId || decisionSourceId === topInspectId)
  ) {
    return true;
  }

  return Boolean(decisionSourceName && topName && decisionSourceName === topName);
}

function resolveTargetDecisionSourceId(state, decision) {
  const topStackObject = getVisibleTopStackObject(state);
  const decisionSourceId = normalizeNumericId(decision?.source_id);

  if (topStackObject && stackObjectMatchesDecisionSource(topStackObject, decision)) {
    return normalizeNumericId(topStackObject?.id);
  }

  return decisionSourceId;
}

function resolveTargetDecisionColor(state, decision) {
  const topStackObject = getVisibleTopStackObject(state);
  const controllerId = stackObjectMatchesDecisionSource(topStackObject, decision)
    ? normalizeNumericId(topStackObject?.controller)
    : null;
  const fallbackControllerId = normalizeNumericId(decision?.player);
  const accent = getPlayerAccent(state?.players || [], controllerId);
  if (accent?.hex) return accent.hex;
  return getPlayerAccent(state?.players || [], fallbackControllerId)?.hex || "#ff3b30";
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
  header,
  optionsMaxHeight = 360,
  canAct,
  isActive,
  canSelectMore,
  selectedTargets = [],
  hoveredObjectId,
  onSelectTarget,
  onSkipRequirement,
  showSkip,
  skipLabel,
  horizontal = false,
  showTargetButtons = true,
  interactionHint = null,
}) {
  const { registerPointerDown, shouldHandleClick } = usePointerClickGuard();
  const legalTargets = req.legal_targets || [];
  const objectTargets = legalTargets.filter((target) => targetObjectId(target) != null);
  const hasHoverMatch = hoveredObjectId != null
    && objectTargets.some((target) => targetObjectId(target) === String(hoveredObjectId));
  const scopedTargets = legalTargets;

  const hideTimerRef = useRef(null);
  const targetButtonRefs = useRef(new Map());
  const panelContentRef = useRef(null);
  const heightAnimationFrameRef = useRef(null);
  const [visibleTargets, setVisibleTargets] = useState(scopedTargets);
  const [panelMaxHeight, setPanelMaxHeight] = useState(68);
  const showRows = scopedTargets.length > 0;
  const MIN_OPTIONS_PANEL_HEIGHT = 68;

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
      if (heightAnimationFrameRef.current != null) {
        cancelAnimationFrame(heightAnimationFrameRef.current);
        heightAnimationFrameRef.current = null;
      }
    },
    []
  );

  useEffect(() => {
    if (horizontal) return undefined;
    const contentNode = panelContentRef.current;
    if (!contentNode) return undefined;

    const publishPanelHeight = () => {
      const measured = Math.ceil(contentNode.scrollHeight);
      const nextHeight = Math.min(
        optionsMaxHeight,
        Math.max(MIN_OPTIONS_PANEL_HEIGHT, measured)
      );
      setPanelMaxHeight((prev) => (Math.abs(prev - nextHeight) > 1 ? nextHeight : prev));
    };

    const schedulePanelHeight = () => {
      if (heightAnimationFrameRef.current != null) {
        cancelAnimationFrame(heightAnimationFrameRef.current);
      }
      heightAnimationFrameRef.current = requestAnimationFrame(() => {
        publishPanelHeight();
        heightAnimationFrameRef.current = null;
      });
    };

    schedulePanelHeight();
    const observer = new ResizeObserver(schedulePanelHeight);
    observer.observe(contentNode);
    window.addEventListener("resize", schedulePanelHeight);

    return () => {
      observer.disconnect();
      window.removeEventListener("resize", schedulePanelHeight);
      if (heightAnimationFrameRef.current != null) {
        cancelAnimationFrame(heightAnimationFrameRef.current);
        heightAnimationFrameRef.current = null;
      }
    };
  }, [horizontal, showRows, showSkip, visibleTargets.length, optionsMaxHeight]);

  useEffect(() => {
    if (!hasHoverMatch || hoveredObjectId == null) return;
    const key = `object:${String(hoveredObjectId)}`;
    if (!visibleTargets.some((target) => targetListKey(target) === key)) return;
    const node = targetButtonRefs.current.get(key);
    if (!node) return;
    node.scrollIntoView({ block: "nearest", inline: "nearest", behavior: "smooth" });
  }, [hasHoverMatch, hoveredObjectId, visibleTargets]);

  const targetButtons = visibleTargets.map((target, tIdx) => {
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
          horizontal
            ? STRIP_ITEM_BASE_CLASS
            : "decision-option-row h-7 w-full justify-start rounded-none border-0 bg-[linear-gradient(180deg,rgba(49,42,36,0.94),rgba(21,18,17,0.98))] px-2.5 text-[13px] text-[#d8cbb0] transition-all hover:bg-[linear-gradient(180deg,rgba(82,66,45,0.98),rgba(33,25,19,0.98))] hover:text-[#fff1cb]",
          horizontal && isSelected && STRIP_ITEM_ACTIVE_CLASS,
          !horizontal && isSelected && "bg-[linear-gradient(180deg,rgba(95,75,50,0.98),rgba(42,32,21,0.98))] text-[#fff0cf]",
          horizontal && !isSelected && isHoveredTarget && STRIP_ITEM_ACTIVE_CLASS,
          !horizontal && !isSelected && isHoveredTarget && "bg-[linear-gradient(180deg,rgba(84,68,47,0.98),rgba(34,27,20,0.98))] text-[#f5e7c7]",
          isUnavailable
            && (horizontal
              ? STRIP_ITEM_DISABLED_CLASS
              : "bg-[linear-gradient(180deg,rgba(38,33,29,0.94),rgba(18,16,15,0.98))] text-[#897b66] hover:bg-[linear-gradient(180deg,rgba(38,33,29,0.94),rgba(18,16,15,0.98))] hover:text-[#897b66]")
        )}
        disabled={!canAct || isUnavailable}
        onPointerDown={(event) => {
          if (!canAct || isUnavailable || !registerPointerDown(event)) return;
          event.preventDefault();
          onSelectTarget(target, reqIdx, { toggleExisting: true, strictRequirement: true });
        }}
        onClick={(event) => {
          if (!canAct || isUnavailable || !shouldHandleClick(event)) return;
          onSelectTarget(target, reqIdx, { toggleExisting: true, strictRequirement: true });
        }}
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
  });

  if (horizontal) {
    return (
      <div
        className={cn(
          "transition-all duration-200",
          showRows ? "opacity-100 translate-y-0" : "opacity-0 -translate-y-1 pointer-events-none"
        )}
      >
          <div className="flex min-w-max items-center gap-1.5 py-0.5">
          <div className={cn(STRIP_META_ITEM_CLASS, !isActive && "opacity-80")}>
            {header}
          </div>
          {showTargetButtons ? targetButtons : (
            <div className="decision-empty-note px-2 text-[12px] italic whitespace-nowrap">
              {interactionHint || "Click a highlighted card or player to target it directly."}
            </div>
          )}
          {!showRows && showTargetButtons && (
            <div className="decision-empty-note px-2 text-[12px] italic whitespace-nowrap">
              No legal targets.
            </div>
          )}
          {showSkip && (
            <Button
              variant="ghost"
              size="sm"
              className={cn(STRIP_ITEM_BASE_CLASS, "h-8 min-w-[140px]")}
              disabled={!canAct}
              onPointerDown={(event) => {
                if (!canAct || !registerPointerDown(event)) return;
                event.preventDefault();
                onSkipRequirement();
              }}
              onClick={(event) => {
                if (!canAct || !shouldHandleClick(event)) return;
                onSkipRequirement();
              }}
            >
              {skipLabel}
            </Button>
          )}
        </div>
      </div>
    );
  }

  return (
    <div className="w-full">
      <div
        className={cn(
          "-mx-1.5 transition-all duration-200",
          showRows ? "opacity-100 translate-y-0" : "opacity-0 -translate-y-1 pointer-events-none"
        )}
      >
        <div className="decision-panel-header sticky top-0 z-10 px-1.5 py-1">
          {header}
        </div>
        <div
          className="w-full overflow-y-auto overflow-x-hidden transition-[max-height] duration-300 ease-out"
          style={{ maxHeight: `${panelMaxHeight}px` }}
        >
          <div ref={panelContentRef} className="w-full">
            <div className="w-full divide-y divide-[rgba(128,107,78,0.28)]">
              {targetButtons}
            </div>
          </div>
        </div>
      </div>
      {showSkip && (
        <Button
          variant="ghost"
          size="sm"
          className="decision-option-row decision-option-row--panel mt-1 h-6 w-full justify-start border-y border-x-0 px-2.5 text-[12px]"
          disabled={!canAct}
          onPointerDown={(event) => {
            if (!canAct || !registerPointerDown(event)) return;
            event.preventDefault();
            onSkipRequirement();
          }}
          onClick={(event) => {
            if (!canAct || !shouldHandleClick(event)) return;
            onSkipRequirement();
          }}
        >
          {skipLabel}
        </Button>
      )}
    </div>
  );
}

export default function TargetsDecision({
  decision,
  canAct,
  inspectorOracleTextHeight = 0,
  inlineSubmit = true,
  onSubmitActionChange = null,
  hideDescription = false,
  layout = "panel",
  showStripSummary = true,
}) {
  const { dispatch, state } = useGame();
  const { startDragArrow, updateDragArrow, endDragArrow } = useCombatArrows();
  const { registerPointerDown, shouldHandleClick } = usePointerClickGuard();
  const stripLayout = layout === "strip";
  const hoveredObjectId = useHoveredObjectId();
  const requirements = useMemo(() => decision.requirements || [], [decision.requirements]);
  const { objectNames: objectNamesById, playerNames: playerNamesById } = useMemo(
    () => buildTargetNameMaps(state),
    [state]
  );
  const [currentReqIdx, setCurrentReqIdx] = useState(0);
  // Per-requirement selections: array of arrays
  const [selectionsByReq, setSelectionsByReq] = useState(() =>
    requirements.map(() => [])
  );
  const liveTargetSourceId = useMemo(
    () => resolveTargetDecisionSourceId(state, decision),
    [state, decision]
  );
  const liveTargetColor = useMemo(
    () => resolveTargetDecisionColor(state, decision),
    [state, decision]
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
  const optionsMaxHeight = useMemo(() => {
    const oracleHeight = Number(inspectorOracleTextHeight);
    if (!Number.isFinite(oracleHeight) || oracleHeight <= 0) return 360;
    const dynamicMax = Math.round(420 - (oracleHeight * 0.55));
    return Math.max(180, Math.min(360, dynamicMax));
  }, [inspectorOracleTextHeight]);

  const handleSelectTarget = useCallback((
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
  }, [
    currentReqIdx,
    objectNamesById,
    playerNamesById,
    requirements,
  ]);

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

  useEffect(() => {
    if (!canAct || requirements.length === 0 || allDone || liveTargetSourceId == null) {
      endDragArrow();
      return undefined;
    }

    const sourceRect = getCardRect(liveTargetSourceId);
    const sourceCenter = sourceRect
      ? centerOf(sourceRect)
      : { x: window.innerWidth * 0.5, y: window.innerHeight * 0.5 };

    startDragArrow(liveTargetSourceId, sourceCenter.x, sourceCenter.y, liveTargetColor);

    const onPointerMove = (event) => {
      updateDragArrow(event.clientX, event.clientY);
    };

    document.addEventListener("pointermove", onPointerMove);
    return () => {
      document.removeEventListener("pointermove", onPointerMove);
      endDragArrow();
    };
  }, [
    allDone,
    canAct,
    endDragArrow,
    liveTargetColor,
    liveTargetSourceId,
    requirements.length,
    startDragArrow,
    updateDragArrow,
  ]);

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

  const handleSubmit = useCallback(() => {
    dispatch(
      { type: "select_targets", targets: allSelections.map(toDispatchTarget) },
      "Targets selected"
    );
  }, [dispatch, allSelections]);

  useEffect(() => {
    if (!onSubmitActionChange) return undefined;
    onSubmitActionChange({
      label: `Submit Targets (${allSelections.length})`,
      disabled: !canAct || !canSubmit,
      onSubmit: handleSubmit,
    });
    return () => onSubmitActionChange(null);
  }, [onSubmitActionChange, allSelections.length, canAct, canSubmit, handleSubmit]);

  if (requirements.length === 0) return null;

  return (
    <div className={cn(
      "flex w-full min-w-0 flex-col gap-1.5",
      stripLayout && "gap-1"
    )}>
      {(!stripLayout || showStripSummary) && (
        <DecisionSummary
          decision={decision}
          hideDescription={hideDescription}
          layout={layout}
          className={stripLayout ? "w-full" : ""}
        />
      )}
      <div className={cn(
        stripLayout
          ? "min-w-0 overflow-x-auto overflow-y-hidden pb-1"
          : "grid gap-1.5"
      )}>
        <div className={cn(
          stripLayout
            ? "flex min-w-max items-center gap-1.5"
            : "grid gap-1.5"
        )}>
          {requirements.map((req, reqIdx) => {
            const isActive = reqIdx === currentReqIdx && !allDone;
            const reqSelections = selectionsByReq[reqIdx] || [];
            const reqMin = req.min_targets ?? 1;
            const reqMax = req.max_targets ?? req.legal_targets?.length ?? 1;
            const isOptional = reqMin === 0;
            const canSelectMore = reqSelections.length < reqMax;
            const showCompletedOptions = allDone && reqSelections.length > 0;
            const shouldShowSelectedChips = reqSelections.length > 0 && !isActive && !showCompletedOptions;
            const shouldShowTargetOptions = isActive || showCompletedOptions;
            const interactionHint = !canSelectMore
              ? "Target selected. Submit or move to the next requirement."
              : "Click a highlighted card or player to target it directly.";
            const requirementHeader = (
              <div className={cn(
                "leading-snug",
                stripLayout
                  ? "text-[12px] whitespace-nowrap text-[#d5c7ab]"
                  : "text-[13px] text-[#d9ccb1]"
              )}>
                <span className={cn(
                  "font-semibold",
                  stripLayout ? "text-[#f0e0bf]" : "text-[#f0e0bf]"
                )}>
                  Target {reqIdx + 1}:
                </span>{" "}
                {req.description || "Choose a target"}
                <span className={cn(
                  "ml-1 text-[12px]",
                  stripLayout ? "text-[#bca887]" : "text-[#bca887]"
                )}>
                  ({reqMin}-{req.max_targets ?? req.legal_targets?.length ?? "?"}{isOptional ? ", optional" : ""})
                </span>
              </div>
            );

            return (
              <div
                key={reqIdx}
                className={cn(
                  stripLayout
                    ? "flex min-w-max items-center gap-1.5"
                    : "decision-target-requirement px-1.5 py-1",
                  !stripLayout && isActive && "is-active"
                )}
              >
                {!shouldShowTargetOptions && !stripLayout && <div className="mb-1">{requirementHeader}</div>}

                {/* Show current selections for this requirement */}
                {shouldShowSelectedChips && (
                  <div className={cn(
                    "mb-1 flex",
                    stripLayout ? "items-center gap-1.5 mb-0" : "flex-wrap gap-0.5"
                  )}>
                    {stripLayout && (
                      <div className={STRIP_META_ITEM_CLASS}>
                        {requirementHeader}
                      </div>
                    )}
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
                          className={cn(
                            stripLayout
                              ? cn(STRIP_ITEM_BASE_CLASS, STRIP_ITEM_ACTIVE_CLASS)
                              : "decision-selected-chip h-5 px-1.5 text-[12px]"
                          )}
                          disabled={!canAct}
                          onPointerDown={(event) => {
                            if (!canAct || !registerPointerDown(event)) return;
                            event.preventDefault();
                            handleRemoveTarget(reqIdx, selIdx);
                          }}
                          onClick={(event) => {
                            if (!canAct || !shouldHandleClick(event)) return;
                            handleRemoveTarget(reqIdx, selIdx);
                          }}
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
                    header={requirementHeader}
                    optionsMaxHeight={optionsMaxHeight}
                    canAct={canAct}
                    isActive={isActive}
                    canSelectMore={canSelectMore}
                    selectedTargets={reqSelections}
                    hoveredObjectId={hoveredObjectId}
                    onSelectTarget={handleSelectTarget}
                    onSkipRequirement={handleSkipRequirement}
                    showSkip={isActive && (isOptional || currentMet) && !allDone}
                    skipLabel={isOptional ? "Skip (optional)" : <>Next requirement <ArrowRight className="size-3 inline" /></>}
                    horizontal={stripLayout}
                    showTargetButtons={!stripLayout}
                    interactionHint={interactionHint}
                  />
                )}
              </div>
            );
          })}
        </div>
      </div>

      {inlineSubmit && (
        <div className={cn("w-full shrink-0", stripLayout ? "pt-0" : "pt-1")}>
          <Button
            variant="ghost"
            size="sm"
            className={cn(
              "decision-neon-button decision-submit-button h-6 rounded-none px-2 text-[13px] font-semibold uppercase",
              stripLayout ? "w-auto ml-1" : "w-full"
            )}
            disabled={!canAct || !canSubmit}
            onPointerDown={(event) => {
              if (!canAct || !canSubmit || !registerPointerDown(event)) return;
              event.preventDefault();
              handleSubmit();
            }}
            onClick={(event) => {
              if (!canAct || !canSubmit || !shouldHandleClick(event)) return;
              handleSubmit();
            }}
          >
            Submit Targets ({allSelections.length})
          </Button>
        </div>
      )}
    </div>
  );
}
