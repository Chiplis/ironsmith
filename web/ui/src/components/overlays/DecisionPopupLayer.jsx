import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { useGame } from "@/context/GameContext";
import { useHover } from "@/context/HoverContext";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import DecisionRouter from "@/components/decisions/DecisionRouter";
import { normalizeDecisionText } from "@/components/decisions/decisionText";
import { animate, cancelMotion, snappySpring, stagger } from "@/lib/motion/anime";
import { SymbolText } from "@/lib/mana-symbols";
import { cn } from "@/lib/utils";

function clamp(value, min, max) {
  return Math.min(max, Math.max(min, value));
}

function priorityAnchorStyle(anchor) {
  if (!anchor || !Number.isFinite(anchor.x) || !Number.isFinite(anchor.y)) return null;
  const viewportWidth = typeof window !== "undefined" ? window.innerWidth : 1280;
  const viewportHeight = typeof window !== "undefined" ? window.innerHeight : 720;
  const width = Math.min(348, viewportWidth - 16);
  const left = clamp(anchor.x - (width * 0.5), 8, viewportWidth - width - 8);
  const top = clamp(anchor.y - 124, 74, viewportHeight - 102);
  return { left: `${left}px`, top: `${top}px`, width: `${width}px` };
}

function nextStepLabel(phase, step, stackSize) {
  if (stackSize > 0) return "Resolve";
  switch (step) {
    case "Untap": return "Upkeep";
    case "Upkeep": return "Draw";
    case "Draw": return "Main Phase";
    case "BeginCombat": return "Attackers";
    case "DeclareAttackers": return "Blockers";
    case "DeclareBlockers": return "Damage";
    case "CombatDamage": return "End Combat";
    case "EndCombat": return "Main 2";
    case "End": return "Cleanup";
    case "Cleanup": return "Next Turn";
    default: break;
  }
  switch (phase) {
    case "FirstMain": return "Combat";
    case "NextMain": return "End Step";
    case "Ending": return "Cleanup";
    default: return "Next";
  }
}

function formatPriorityInlineActionLabel(action) {
  const label = String(action?.label || "").trim();
  if (!label) return "Action";

  if (action?.kind === "activate_ability" || action?.kind === "activate_mana_ability") {
    const activateMatch = label.match(/^Activate\s+.+?:\s*(.+)$/i);
    if (activateMatch) return activateMatch[1];
    const tapMatch = label.match(/^Tap\s+.+?:\s*(.+)$/i);
    if (tapMatch) return tapMatch[1];
  }

  return label;
}

function actionTargetsObjectName(action, lowerName) {
  if (!lowerName) return false;
  const label = String(action?.label || "").trim().toLowerCase();
  if (!label) return false;
  return (
    label.startsWith(`activate ${lowerName}:`)
    || label.startsWith(`cast ${lowerName}`)
    || label.startsWith(`play ${lowerName}`)
    || label.startsWith(`tap ${lowerName}:`)
  );
}

function buildObjectNameById(players, stackObjects) {
  const map = new Map();
  const register = (id, name) => {
    if (id == null) return;
    const key = String(id);
    if (!key) return;
    const text = String(name || "").trim();
    if (!text) return;
    map.set(key, text);
  };

  for (const player of players || []) {
    for (const card of player?.hand_cards || []) {
      register(card?.id, card?.name);
    }
    for (const card of player?.graveyard_cards || []) {
      register(card?.id, card?.name);
    }
    for (const card of player?.exile_cards || []) {
      register(card?.id, card?.name);
    }
    for (const card of player?.command_cards || []) {
      register(card?.id, card?.name);
    }
    for (const card of player?.battlefield || []) {
      register(card?.id, card?.name);
      if (Array.isArray(card?.member_ids)) {
        for (const memberId of card.member_ids) {
          register(memberId, card?.name);
        }
      }
    }
  }

  for (const stackObject of stackObjects || []) {
    register(stackObject?.id, stackObject?.name);
  }

  return map;
}

function buildBattlefieldFamilies(players) {
  const familyIdByObjectId = new Map();
  const familyMembersByFamilyId = new Map();

  for (const player of players || []) {
    for (const card of player?.battlefield || []) {
      const rootId = card?.id != null ? String(card.id) : null;
      if (!rootId) continue;

      const memberIds = Array.isArray(card?.member_ids)
        ? card.member_ids.map((memberId) => String(memberId))
        : [];
      const familyMembers = Array.from(new Set([rootId, ...memberIds]));

      for (const id of familyMembers) {
        familyIdByObjectId.set(id, rootId);
      }
      familyMembersByFamilyId.set(rootId, familyMembers);
    }
  }

  return { familyIdByObjectId, familyMembersByFamilyId };
}

function buildPriorityActionGroups(actions, families) {
  const { familyIdByObjectId, familyMembersByFamilyId } = families;
  const groups = [];
  const byKey = new Map();

  for (const action of actions || []) {
    const label = formatPriorityInlineActionLabel(action);
    const objectId = action?.object_id != null ? String(action.object_id) : null;
    const familyId = objectId != null ? (familyIdByObjectId.get(objectId) || objectId) : "";
    const key = `${action.kind || ""}|${action.from_zone || ""}|${familyId}|${label}`;

    let group = byKey.get(key);
    if (!group) {
      group = {
        key,
        label,
        count: 0,
        firstAction: action,
        actionIndices: new Set(),
        hoverObjectId: objectId != null ? (familyIdByObjectId.get(objectId) || objectId) : null,
        linkedObjectIds: new Set(),
      };
      byKey.set(key, group);
      groups.push(group);
    }

    group.count += 1;
    group.actionIndices.add(action.index);

    if (objectId != null) {
      const actionFamilyId = familyIdByObjectId.get(objectId);
      if (actionFamilyId && familyMembersByFamilyId.has(actionFamilyId)) {
        for (const id of familyMembersByFamilyId.get(actionFamilyId)) {
          group.linkedObjectIds.add(id);
        }
      } else {
        group.linkedObjectIds.add(objectId);
      }
    }
  }

  return groups;
}

function buildObjectFamilyIds(players, objectId) {
  const ids = new Set();
  if (objectId == null) return ids;

  const objectKey = String(objectId);
  ids.add(objectKey);

  for (const player of players || []) {
    for (const card of player?.battlefield || []) {
      const rootId = card?.id != null ? String(card.id) : null;
      const memberIds = Array.isArray(card?.member_ids)
        ? card.member_ids.map((memberId) => String(memberId))
        : [];
      const familyIds = rootId ? [rootId, ...memberIds] : memberIds;
      if (!familyIds.includes(objectKey)) continue;
      for (const id of familyIds) ids.add(id);
      return ids;
    }
  }

  return ids;
}

function PriorityActionPillLabel({ text, viewportRef, carouselResetVersion = 0 }) {
  const displayText = useMemo(() => normalizeDecisionText(text), [text]);
  const containerRef = useRef(null);
  const measureRef = useRef(null);
  const marqueeRef = useRef(null);
  const marqueeAnimationRef = useRef(null);
  const [isOverflowing, setIsOverflowing] = useState(false);
  const [isVisible, setIsVisible] = useState(true);
  const [travelDistance, setTravelDistance] = useState(0);
  const [travelDuration, setTravelDuration] = useState(0);

  const recomputeOverflow = useCallback(() => {
    const containerEl = containerRef.current;
    const measureEl = measureRef.current;
    if (!containerEl || !measureEl) return;

    const textWidth = Math.ceil(measureEl.scrollWidth);
    const containerWidth = Math.ceil(containerEl.clientWidth);
    const overflowPx = textWidth - containerWidth;
    if (overflowPx > 8) {
      const gapPx = 28;
      const distancePx = textWidth + gapPx;
      const speedPxPerSec = 40;
      setTravelDistance(distancePx);
      setTravelDuration(Math.max(5, distancePx / speedPxPerSec));
      setIsOverflowing(true);
    } else {
      setIsOverflowing(false);
      setTravelDistance(0);
      setTravelDuration(0);
    }
  }, []);

  const recomputeVisibility = useCallback(() => {
    const viewportEl = viewportRef.current;
    const containerEl = containerRef.current;
    if (!viewportEl || !containerEl) {
      setIsVisible(true);
      return;
    }

    const viewportRect = viewportEl.getBoundingClientRect();
    const containerRect = containerEl.getBoundingClientRect();
    const visible = (
      containerRect.right > (viewportRect.left + 6)
      && containerRect.left < (viewportRect.right - 6)
    );
    setIsVisible(visible);
  }, [viewportRef]);

  useLayoutEffect(() => {
    recomputeOverflow();
    if (typeof ResizeObserver === "undefined") return undefined;

    const observer = new ResizeObserver(() => recomputeOverflow());
    if (containerRef.current) observer.observe(containerRef.current);
    if (measureRef.current) observer.observe(measureRef.current);
    return () => observer.disconnect();
  }, [recomputeOverflow, text]);

  useEffect(() => {
    const viewportEl = viewportRef.current;
    if (!viewportEl) return undefined;

    const rafId = window.requestAnimationFrame(() => {
      recomputeVisibility();
    });
    const handleScroll = () => recomputeVisibility();
    viewportEl.addEventListener("scroll", handleScroll, { passive: true });
    window.addEventListener("resize", handleScroll);
    return () => {
      window.cancelAnimationFrame(rafId);
      viewportEl.removeEventListener("scroll", handleScroll);
      window.removeEventListener("resize", handleScroll);
    };
  }, [recomputeVisibility, text, viewportRef]);

  const shouldAnimate = isOverflowing && isVisible;

  useEffect(() => {
    const marqueeEl = marqueeRef.current;
    cancelMotion(marqueeAnimationRef.current);
    marqueeAnimationRef.current = null;

    if (!marqueeEl) return undefined;
    marqueeEl.style.transform = "translateX(0px)";

    if (!shouldAnimate || travelDistance <= 0 || travelDuration <= 0) {
      return undefined;
    }

    const animation = animate(marqueeEl, {
      x: -travelDistance,
      ease: "linear",
      duration: travelDuration * 1000,
      delay: 750,
      loop: true,
    });
    marqueeAnimationRef.current = animation;

    return () => {
      cancelMotion(animation);
    };
  }, [carouselResetVersion, shouldAnimate, travelDistance, travelDuration]);

  if (!shouldAnimate) {
    return (
      <span ref={containerRef} className="relative block min-w-0 overflow-hidden" style={{ textOverflow: "clip" }}>
        <span ref={measureRef} className="absolute left-0 top-0 invisible inline-block whitespace-nowrap pointer-events-none">
          <SymbolText text={displayText} style={{ whiteSpace: "nowrap" }} />
        </span>
        <span className="block min-w-0 overflow-hidden whitespace-nowrap" style={{ textOverflow: "clip" }}>
          <SymbolText text={displayText} style={{ whiteSpace: "nowrap" }} />
        </span>
      </span>
    );
  }

  return (
    <span ref={containerRef} className="relative block min-w-0 overflow-hidden" style={{ textOverflow: "clip" }}>
      <span ref={measureRef} className="absolute left-0 top-0 invisible inline-block whitespace-nowrap pointer-events-none">
        <SymbolText text={displayText} style={{ whiteSpace: "nowrap" }} />
      </span>
      <span
        ref={marqueeRef}
        className="inline-flex whitespace-nowrap will-change-transform"
      >
        <span className="pr-7">
          <SymbolText text={displayText} style={{ whiteSpace: "nowrap" }} />
        </span>
        <span aria-hidden="true" className="pr-7">
          <SymbolText text={displayText} style={{ whiteSpace: "nowrap" }} />
        </span>
      </span>
    </span>
  );
}

function PriorityActionStrip({
  groups,
  canAct,
  hoveredObjectFamilyIds,
  selectedObjectFamilyIds,
  selectedActionIndices,
  onActionClick,
  onActionHoverStart,
  onActionHoverEnd,
}) {
  const BASE_LOOP_CYCLES = 5;
  const viewportRef = useRef(null);
  const groupNodeRefs = useRef(new Map());
  const displayNodeRefs = useRef(new Map());
  const previousHoveredGroupKeysRef = useRef(new Set());
  const previousSelectedGroupKeysRef = useRef(new Set());
  const stripMotionRef = useRef(null);
  const [carouselResetByGroupKey, setCarouselResetByGroupKey] = useState({});
  const [isPointerInStrip, setIsPointerInStrip] = useState(false);
  const [loopEnabled, setLoopEnabled] = useState(false);
  const effectiveLoopCycles = loopEnabled ? BASE_LOOP_CYCLES : 1;
  const middleLoopIndex = Math.floor(effectiveLoopCycles / 2);
  const groupKeysSignature = useMemo(
    () => groups.map((group) => group.key).join("|"),
    [groups]
  );
  const displayGroups = useMemo(() => {
    if (!groups.length) return [];
    const entries = [];
    for (let cycle = 0; cycle < effectiveLoopCycles; cycle += 1) {
      for (const group of groups) {
        entries.push({
          cycle,
          group,
          key: `${group.key}::${cycle}`,
        });
      }
    }
    return entries;
  }, [effectiveLoopCycles, groups]);

  const isGroupHoveredLinked = useCallback((group) => {
    for (const linkedObjectId of group.linkedObjectIds) {
      if (hoveredObjectFamilyIds.has(linkedObjectId)) return true;
    }
    return false;
  }, [hoveredObjectFamilyIds]);

  const isGroupSelectedLinked = useCallback((group) => {
    for (const linkedObjectId of group.linkedObjectIds) {
      if (selectedObjectFamilyIds.has(linkedObjectId)) return true;
    }
    for (const actionIndex of group.actionIndices) {
      if (selectedActionIndices.has(actionIndex)) return true;
    }
    return false;
  }, [selectedObjectFamilyIds, selectedActionIndices]);

  const hoveredGroupKeys = useMemo(
    () => groups.filter((group) => isGroupHoveredLinked(group)).map((group) => group.key),
    [groups, isGroupHoveredLinked]
  );
  const selectedGroupKeys = useMemo(
    () => groups.filter((group) => isGroupSelectedLinked(group)).map((group) => group.key),
    [groups, isGroupSelectedLinked]
  );

  useEffect(() => {
    const previousHovered = previousHoveredGroupKeysRef.current;
    const currentHovered = new Set(hoveredGroupKeys);
    const newlyHovered = hoveredGroupKeys.filter((key) => !previousHovered.has(key));
    if (newlyHovered.length > 0) {
      setCarouselResetByGroupKey((prev) => {
        const next = { ...prev };
        for (const key of newlyHovered) {
          next[key] = (next[key] || 0) + 1;
        }
        return next;
      });
    }
    previousHoveredGroupKeysRef.current = currentHovered;
  }, [hoveredGroupKeys]);

  useEffect(() => {
    const previousSelected = previousSelectedGroupKeysRef.current;
    const currentSelected = new Set(selectedGroupKeys);
    const newlySelected = selectedGroupKeys.filter((key) => !previousSelected.has(key));
    if (newlySelected.length > 0) {
      setCarouselResetByGroupKey((prev) => {
        const next = { ...prev };
        for (const key of newlySelected) {
          next[key] = (next[key] || 0) + 1;
        }
        return next;
      });
    }
    previousSelectedGroupKeysRef.current = currentSelected;
  }, [selectedGroupKeys]);

  useEffect(() => {
    groupNodeRefs.current = new Map();
    displayNodeRefs.current = new Map();
  }, [groupKeysSignature, effectiveLoopCycles]);

  useLayoutEffect(() => {
    const nodes = displayGroups
      .map(({ key }) => displayNodeRefs.current.get(key))
      .filter(Boolean);
    if (nodes.length === 0) return undefined;

    cancelMotion(stripMotionRef.current);
    stripMotionRef.current = animate(nodes, {
      opacity: [0, 1],
      y: [12, 0],
      scale: [0.982, 1],
      delay: stagger(18),
      duration: 260,
      ease: snappySpring({ duration: 260, bounce: 0.08 }),
    });

    return () => {
      cancelMotion(stripMotionRef.current);
      stripMotionRef.current = null;
    };
  }, [displayGroups, effectiveLoopCycles, groupKeysSignature]);

  useLayoutEffect(() => {
    const viewport = viewportRef.current;
    if (!viewport || !groups.length) return undefined;

    const recomputeLoopEnabled = () => {
      const cycleWidth = viewport.scrollWidth / effectiveLoopCycles;
      if (!(cycleWidth > 0)) return;
      const shouldLoop = cycleWidth > (viewport.clientWidth + 1);
      setLoopEnabled((prev) => (prev === shouldLoop ? prev : shouldLoop));
    };

    const raf = window.requestAnimationFrame(recomputeLoopEnabled);
    if (typeof ResizeObserver === "undefined") {
      return () => window.cancelAnimationFrame(raf);
    }

    const observer = new ResizeObserver(() => recomputeLoopEnabled());
    observer.observe(viewport);
    return () => {
      window.cancelAnimationFrame(raf);
      observer.disconnect();
    };
  }, [effectiveLoopCycles, groupKeysSignature, groups.length]);

  useEffect(() => {
    const viewport = viewportRef.current;
    if (!viewport || !groups.length || !loopEnabled) return undefined;

    const placeAtMiddleSegment = () => {
      if (viewport.scrollWidth <= viewport.clientWidth + 1) return;
      const cycleWidth = viewport.scrollWidth / effectiveLoopCycles;
      if (!(cycleWidth > 0)) return;
      const target = cycleWidth * middleLoopIndex;
      if (Math.abs(viewport.scrollLeft - target) > 1) {
        viewport.scrollLeft = target;
      }
    };

    const raf = window.requestAnimationFrame(placeAtMiddleSegment);
    return () => window.cancelAnimationFrame(raf);
  }, [effectiveLoopCycles, groupKeysSignature, groups.length, loopEnabled, middleLoopIndex]);

  useEffect(() => {
    const viewport = viewportRef.current;
    if (!viewport || !groups.length || !loopEnabled) return undefined;

    let raf = 0;
    const normalizeScroll = () => {
      raf = 0;
      if (viewport.scrollWidth <= viewport.clientWidth + 1) return;
      const cycleWidth = viewport.scrollWidth / effectiveLoopCycles;
      if (!(cycleWidth > 0)) return;

      const minBound = cycleWidth * 0.15;
      const maxBound = Math.min(
        Math.max(0, viewport.scrollWidth - viewport.clientWidth),
        cycleWidth * (effectiveLoopCycles - 0.85)
      );
      if (viewport.scrollLeft < minBound) {
        viewport.scrollLeft += cycleWidth;
      } else if (viewport.scrollLeft > maxBound) {
        viewport.scrollLeft -= cycleWidth;
      }
    };

    const handleScroll = () => {
      if (raf) return;
      raf = window.requestAnimationFrame(normalizeScroll);
    };

    viewport.addEventListener("scroll", handleScroll, { passive: true });
    normalizeScroll();
    return () => {
      if (raf) window.cancelAnimationFrame(raf);
      viewport.removeEventListener("scroll", handleScroll);
    };
  }, [effectiveLoopCycles, groupKeysSignature, groups.length, loopEnabled]);

  useEffect(() => {
    const viewport = viewportRef.current;
    if (!viewport) return;

    const objectHoverActive = typeof document !== "undefined"
      && Boolean(document.querySelector("[data-object-id]:hover"));
    const focusKind = (!isPointerInStrip && objectHoverActive && hoveredGroupKeys.length > 0)
      ? "hover"
      : (selectedGroupKeys.length > 0 ? "selected" : null);
    const focusKeys = focusKind === "hover" ? hoveredGroupKeys : selectedGroupKeys;

    if (!focusKind || focusKeys.length === 0) return;

    const scrollFocusedGroupsIntoView = () => {
      const interactiveNodes = focusKeys
        .map((key) => groupNodeRefs.current.get(key)?.[middleLoopIndex] || null)
        .filter(Boolean);
      if (interactiveNodes.length === 0) return false;

      const viewportRect = viewport.getBoundingClientRect();
      let minDeltaLeft = Number.POSITIVE_INFINITY;
      for (const node of interactiveNodes) {
        const nodeRect = node.getBoundingClientRect();
        minDeltaLeft = Math.min(minDeltaLeft, nodeRect.left - viewportRect.left);
      }
      if (!Number.isFinite(minDeltaLeft)) return false;

      const maxScrollLeft = Math.max(0, viewport.scrollWidth - viewport.clientWidth);
      const leftAnchorPadding = 0;
      let targetLeft = viewport.scrollLeft + minDeltaLeft - leftAnchorPadding;
      targetLeft = Math.min(maxScrollLeft, Math.max(0, targetLeft));
      viewport.scrollTo({ left: targetLeft, behavior: "smooth" });
      return true;
    };

    let raf = 0;
    const tryScroll = (attempt = 0) => {
      if (scrollFocusedGroupsIntoView()) return;
      if (attempt >= 4) return;
      raf = window.requestAnimationFrame(() => {
        tryScroll(attempt + 1);
      });
    };
    tryScroll(0);
    return () => {
      if (raf) window.cancelAnimationFrame(raf);
    };
  }, [hoveredGroupKeys, selectedGroupKeys, groupKeysSignature, isPointerInStrip, middleLoopIndex]);

  if (!canAct) {
    return (
      <span className="text-[12px] text-[#b8d2ef] whitespace-nowrap">
        Waiting for opponent
      </span>
    );
  }

  if (!groups.length) {
    return (
      <span className="text-[12px] text-[#b8d2ef] whitespace-nowrap">
        No actions available
      </span>
    );
  }

  return (
    <div
      ref={viewportRef}
      className="min-w-0 flex-1 overflow-x-auto overflow-y-hidden whitespace-nowrap"
      onMouseEnter={() => setIsPointerInStrip(true)}
      onMouseLeave={() => setIsPointerInStrip(false)}
    >
      <div className="flex w-max min-w-full min-h-[32px] items-stretch gap-1.5 pr-2">
        {displayGroups.map(({ key, cycle, group }) => {
          const isInteractiveCycle = cycle === middleLoopIndex;
          const linkedActive = isGroupHoveredLinked(group) || isGroupSelectedLinked(group);
          const setNodeRef = (node) => {
            const existing = groupNodeRefs.current.get(group.key) || [];
            if (node) {
              existing[cycle] = node;
              groupNodeRefs.current.set(group.key, existing);
              displayNodeRefs.current.set(key, node);
            } else if (existing.length > cycle) {
              existing[cycle] = undefined;
              if (existing.some(Boolean)) {
                groupNodeRefs.current.set(group.key, existing);
              } else {
                groupNodeRefs.current.delete(group.key);
              }
              displayNodeRefs.current.delete(key);
            }
          };
          const pillClassName = cn(
            "inline-flex max-w-[360px] min-w-0 items-center self-stretch border-0 border-l-2 px-2.5 text-[12px] font-semibold transition-all",
            linkedActive
              ? "border-l-[rgba(236,245,255,0.9)] bg-[rgba(220,236,255,0.16)] text-[#f4f9ff] shadow-[0_0_12px_rgba(236,245,255,0.3)]"
              : "border-l-[rgba(116,139,164,0.42)] bg-[rgba(12,22,34,0.58)] text-[rgba(206,223,242,0.52)]",
            isInteractiveCycle && "hover:border-l-[rgba(236,245,255,0.92)] hover:bg-[rgba(220,236,255,0.16)] hover:text-[#f4f9ff] hover:shadow-[0_0_12px_rgba(236,245,255,0.3)]"
          );
          const pillContent = (
            <>
              {group.count > 1 && (
                <span className="mr-1.5 inline-flex h-4 min-w-4 items-center justify-center rounded-sm bg-[rgba(12,20,31,0.86)] px-1 text-[10px] font-bold leading-none tracking-wide text-[#f5d08b]">
                  x{group.count}
                </span>
              )}
              <PriorityActionPillLabel
                text={group.label}
                viewportRef={viewportRef}
                carouselResetVersion={carouselResetByGroupKey[group.key] || 0}
              />
            </>
          );

          if (!isInteractiveCycle) {
            return (
              <div
                key={key}
                aria-hidden="true"
                ref={setNodeRef}
                className={pillClassName}
                style={{ textOverflow: "clip" }}
              >
                {pillContent}
              </div>
            );
          }

          return (
            <button
              key={key}
              type="button"
              ref={setNodeRef}
              className={pillClassName}
              style={{ textOverflow: "clip" }}
              onClick={() => onActionClick(group.firstAction)}
              onMouseEnter={() => onActionHoverStart(group)}
              onMouseLeave={onActionHoverEnd}
            >
              {pillContent}
            </button>
          );
        })}
      </div>
    </div>
  );
}

function resolveDecisionTitle(decision) {
  if (!decision) return "Decision";
  switch (decision.kind) {
    case "targets":
      return "Choose Targets";
    case "select_objects":
      return "Choose Objects";
    case "select_options":
      return "Choose Option";
    case "number":
      return "Choose Number";
    default:
      return "Decision";
  }
}

function PriorityControlStack({
  actionCount = 0,
  holdEnabled = false,
  confirmEnabled = false,
  onHoldChange,
  onConfirmChange,
  className = "",
}) {
  const checkboxLabelClass =
    "flex items-center gap-1.5 text-[11px] uppercase tracking-wider cursor-pointer text-[#9db7d5] hover:text-[#d7e8fb] transition-colors";

  return (
    <div className={cn("flex shrink-0 flex-col items-start justify-center py-1.5", className)}>
      <div className="pointer-events-none pl-[18px] text-[11px] font-bold uppercase tracking-[0.14em] text-[#93c7ff]">
          {actionCount} actions
      </div>
      <div className="flex items-center gap-3">
        <label className={checkboxLabelClass}>
          <Checkbox
            checked={holdEnabled}
            onCheckedChange={(value) => onHoldChange?.(Boolean(value))}
            className="h-3 w-3"
          />
          Hold
        </label>
        <label className={checkboxLabelClass}>
          <Checkbox
            checked={confirmEnabled}
            onCheckedChange={(value) => onConfirmChange?.(Boolean(value))}
            className="h-3 w-3"
          />
          Confirm
        </label>
      </div>
    </div>
  );
}

function PriorityBar({ anchor = null, inline = false, selectedObjectId = null }) {
  const {
    state,
    dispatch,
    holdRule,
    setHoldRule,
    confirmEnabled,
    setConfirmEnabled,
    cancelDecision,
  } = useGame();
  const {
    hoveredObjectId,
    hoverCard,
    clearHover,
    setHoverLinkedObjects,
    clearHoverLinkedObjects,
  } = useHover();
  const decision = state?.decision || null;
  const canAct = !!decision && state?.perspective === decision.player;
  const isPriorityDecision = decision?.kind === "priority";
  const isCombatDecision = decision?.kind === "attackers" || decision?.kind === "blockers";
  const decisionActions = useMemo(() => decision?.actions || [], [decision]);
  const passAction = useMemo(
    () => decisionActions.find((action) => action.kind === "pass_priority"),
    [decisionActions]
  );
  const otherActions = useMemo(
    () => decisionActions.filter((action) => action.kind !== "pass_priority"),
    [decisionActions]
  );

  const anchoredStyle = inline ? null : priorityAnchorStyle(anchor);
  const stackSize = Number(state?.stack_size || 0);
  const passLabel = holdRule === "always"
    ? (passAction?.label || "Pass priority")
    : `→ ${nextStepLabel(state?.phase, state?.step, stackSize)}`;
  const battlefieldFamilies = useMemo(
    () => buildBattlefieldFamilies(state?.players),
    [state?.players]
  );
  const actionGroups = useMemo(
    () => buildPriorityActionGroups(otherActions, battlefieldFamilies),
    [otherActions, battlefieldFamilies]
  );
  const priorityActionCount = otherActions.length;
  const objectNameById = useMemo(
    () => buildObjectNameById(state?.players, state?.stack_objects),
    [state?.players, state?.stack_objects]
  );
  const hoveredObjectFamilyIds = useMemo(
    () => buildObjectFamilyIds(state?.players, hoveredObjectId),
    [state?.players, hoveredObjectId]
  );
  const selectedObjectFamilyIds = useMemo(
    () => buildObjectFamilyIds(state?.players, selectedObjectId),
    [state?.players, selectedObjectId]
  );
  const selectedObjectNameLower = useMemo(() => {
    if (selectedObjectId == null) return "";
    return String(objectNameById.get(String(selectedObjectId)) || "").trim().toLowerCase();
  }, [selectedObjectId, objectNameById]);
  const selectedActionIndices = useMemo(() => {
    const ids = new Set();
    if (selectedObjectId == null && !selectedObjectNameLower) return ids;
    for (const action of otherActions) {
      const actionObjectId = action?.object_id != null ? String(action.object_id) : null;
      if (actionObjectId != null && selectedObjectFamilyIds.has(actionObjectId)) {
        ids.add(action.index);
        continue;
      }
      if (actionTargetsObjectName(action, selectedObjectNameLower)) {
        ids.add(action.index);
      }
    }
    return ids;
  }, [otherActions, selectedObjectFamilyIds, selectedObjectId, selectedObjectNameLower]);
  const triggerPriorityAction = useCallback(
    (action) => {
      if (!canAct || !action) return;
      clearHover();
      dispatch(
        { type: "priority_action", action_index: action.index },
        action.label
      );
    },
    [canAct, clearHover, dispatch]
  );
  const handleActionHoverStart = useCallback(
    (group) => {
      if (!canAct || !group) return;
      setHoverLinkedObjects(group.linkedObjectIds || []);
      if (group.hoverObjectId != null) hoverCard(group.hoverObjectId);
    },
    [canAct, setHoverLinkedObjects, hoverCard]
  );
  const handleActionHoverEnd = useCallback(() => {
    if (!canAct) return;
    clearHoverLinkedObjects();
    clearHover();
  }, [canAct, clearHoverLinkedObjects, clearHover]);
  const decisionIdentity = `${decision?.kind || ""}|${decision?.source_name || ""}|${decision?.description || ""}`;
  const [submitState, setSubmitState] = useState({ key: "", action: null });
  const handleSubmitActionChange = useCallback(
    (nextAction) => {
      setSubmitState({ key: decisionIdentity, action: nextAction || null });
    },
    [decisionIdentity]
  );
  const submitAction = submitState.key === decisionIdentity ? submitState.action : null;
  const canSubmitFocused = canAct
    && !!submitAction
    && !submitAction.disabled
    && typeof submitAction.onSubmit === "function";

  if (!decision || isCombatDecision) return null;
  if (isPriorityDecision && !passAction) return null;

  if (inline) {
    return (
      <div className="pointer-events-none absolute inset-0 z-[120] flex items-center px-2">
        <div
          className="priority-inline-panel pointer-events-auto flex min-h-[46px] w-full items-stretch gap-2 rounded bg-[rgba(7,15,23,0.97)] px-2 py-0 shadow-[0_12px_28px_rgba(0,0,0,0.45)] backdrop-blur-[2px]"
        >
          {isPriorityDecision ? (
            <>
              <div className="shrink-0 flex min-w-[308px] self-stretch items-stretch gap-2">
                <Button
                  variant="ghost"
                  size="sm"
                  className="decision-neon-button decision-submit-button h-full w-[176px] shrink-0 self-stretch rounded-none px-3 text-[14px] font-bold uppercase"
                  disabled={!canAct}
                  onClick={() => triggerPriorityAction(passAction)}
                >
                  {passLabel}
                </Button>
                <PriorityControlStack
                  actionCount={priorityActionCount}
                  holdEnabled={holdRule === "always"}
                  confirmEnabled={confirmEnabled}
                  onHoldChange={(value) => setHoldRule(value ? "always" : "never")}
                  onConfirmChange={setConfirmEnabled}
                  className="min-w-[104px]"
                />
              </div>
              <PriorityActionStrip
                groups={actionGroups}
                canAct={canAct}
                hoveredObjectFamilyIds={hoveredObjectFamilyIds}
                selectedObjectFamilyIds={selectedObjectFamilyIds}
                selectedActionIndices={selectedActionIndices}
                onActionClick={triggerPriorityAction}
                onActionHoverStart={handleActionHoverStart}
                onActionHoverEnd={handleActionHoverEnd}
              />
            </>
          ) : (
            <>
              <div className="shrink-0 flex min-w-[308px] self-stretch items-stretch gap-2">
                <Button
                  variant="ghost"
                  size="sm"
                  className="decision-neon-button decision-submit-button h-full w-[176px] shrink-0 self-stretch rounded-none px-3 text-[14px] font-bold uppercase"
                  disabled={!canSubmitFocused}
                  onClick={() => {
                    if (!canSubmitFocused) return;
                    submitAction.onSubmit();
                  }}
                >
                  {submitAction?.label || "Submit"}
                </Button>
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  className="decision-neon-button decision-neon-button--danger decision-cancel-button h-full w-[96px] shrink-0 self-stretch rounded-none px-2 text-[13px] font-bold uppercase tracking-wide"
                  disabled={!canAct}
                  onClick={() => {
                    if (!canAct) return;
                    cancelDecision();
                  }}
                >
                  Cancel
                </Button>
                <div className="min-w-[86px] self-stretch flex flex-col justify-center py-1.5">
                  <div className="text-[11px] font-bold uppercase tracking-[0.14em] text-[#93c7ff]">
                    {resolveDecisionTitle(decision)}
                  </div>
                  {decision?.source_name && (
                    <div className="mt-0.5 text-[11px] text-[#d2e5fb]">
                      {normalizeDecisionText(decision.source_name)}
                    </div>
                  )}
                </div>
              </div>
              <div className="min-w-0 flex-1 overflow-hidden">
                {canAct ? (
                  <DecisionRouter
                    decision={decision}
                    canAct={canAct}
                    inlineSubmit={false}
                    onSubmitActionChange={handleSubmitActionChange}
                    hideDescription
                    layout="strip"
                  />
                ) : (
                  <span className="text-[12px] text-[#b8d2ef] whitespace-nowrap">
                    Waiting for opponent
                  </span>
                )}
              </div>
            </>
          )}
        </div>
      </div>
    );
  }

  return (
    <div
      className={cn(
        "pointer-events-auto z-[120] rounded bg-[rgba(7,15,23,0.97)] shadow-[0_16px_36px_rgba(0,0,0,0.55)] backdrop-blur-[2px]",
        anchoredStyle
          ? "fixed"
          : "fixed left-2 bottom-[148px] w-[min(92vw,348px)]"
      )}
      style={anchoredStyle || undefined}
    >
      <div className="border-b border-[#2f4662]/85 bg-[rgba(10,22,34,0.88)] px-2 py-0">
        <div className="flex min-h-[46px] items-stretch gap-2">
          {isPriorityDecision ? (
            <>
              <Button
                variant="ghost"
                size="sm"
                className="decision-neon-button h-full w-[176px] shrink-0 self-stretch rounded-none px-3 text-[14px] font-bold uppercase"
                disabled={!canAct}
                onClick={() => triggerPriorityAction(passAction)}
              >
                {passLabel}
              </Button>
              <PriorityControlStack
                actionCount={priorityActionCount}
                holdEnabled={holdRule === "always"}
                confirmEnabled={confirmEnabled}
                onHoldChange={(value) => setHoldRule(value ? "always" : "never")}
                onConfirmChange={setConfirmEnabled}
                className="min-w-[104px]"
              />
            </>
          ) : (
            <>
              <Button
                variant="ghost"
                size="sm"
                className="decision-neon-button decision-submit-button h-full w-[176px] shrink-0 self-stretch rounded-none px-3 text-[14px] font-bold uppercase"
                disabled={!canSubmitFocused}
                onClick={() => {
                  if (!canSubmitFocused) return;
                  submitAction.onSubmit();
                }}
              >
                {submitAction?.label || "Submit"}
              </Button>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                className="decision-neon-button decision-neon-button--danger decision-cancel-button h-full w-[96px] shrink-0 self-stretch rounded-none px-2 text-[13px] font-bold uppercase tracking-wide"
                disabled={!canAct}
                onClick={() => {
                  if (!canAct) return;
                  cancelDecision();
                }}
              >
                Cancel
              </Button>
              <div className="self-stretch flex flex-col justify-center py-1.5 text-[11px] font-bold uppercase tracking-[0.14em] text-[#93c7ff]">
                {resolveDecisionTitle(decision)}
              </div>
            </>
          )}
        </div>
      </div>
      <div className="border-b border-[#2f4662]/70 px-2 py-1.5">
        {isPriorityDecision ? (
          <PriorityActionStrip
            groups={actionGroups}
            canAct={canAct}
            hoveredObjectFamilyIds={hoveredObjectFamilyIds}
            selectedObjectFamilyIds={selectedObjectFamilyIds}
            selectedActionIndices={selectedActionIndices}
            onActionClick={triggerPriorityAction}
            onActionHoverStart={handleActionHoverStart}
            onActionHoverEnd={handleActionHoverEnd}
          />
        ) : (
          <div className="min-w-0">
            <DecisionRouter
              decision={decision}
              canAct={canAct}
              inlineSubmit={false}
              onSubmitActionChange={handleSubmitActionChange}
              hideDescription
              layout="strip"
            />
          </div>
        )}
      </div>
    </div>
  );
}

function CombatBar({ anchor = null, inline = false, decision, canAct }) {
  if (!decision || (decision.kind !== "attackers" && decision.kind !== "blockers")) return null;

  const anchoredStyle = inline ? null : priorityAnchorStyle(anchor);
  const panelClass = inline
    ? "pointer-events-none absolute inset-0 z-[120] flex items-center px-2"
    : "pointer-events-none fixed left-2 bottom-[148px] z-[120] w-[min(96vw,740px)]";

  const innerClass = cn(
    "priority-inline-panel pointer-events-auto flex w-full items-center gap-2 rounded border border-[#305071] bg-[rgba(7,15,23,0.97)] px-2 py-1.5 shadow-[0_12px_28px_rgba(0,0,0,0.45)] backdrop-blur-[2px]",
    !inline && anchoredStyle ? "fixed" : ""
  );

  return (
    <div className={panelClass}>
      <div className={innerClass} style={anchoredStyle || undefined}>
        <DecisionRouter
          decision={decision}
          canAct={canAct}
          combatInline
        />
      </div>
    </div>
  );
}

export default function DecisionPopupLayer({ anchor = null, priorityInline = false, selectedObjectId = null }) {
  const { state } = useGame();
  const decision = state?.decision || null;
  const canAct = !!decision && state?.perspective === decision.player;

  if (!decision) return null;
  if (decision?.kind === "priority") {
    return <PriorityBar anchor={anchor} inline={priorityInline} selectedObjectId={selectedObjectId} />;
  }
  if (decision?.kind === "attackers" || decision?.kind === "blockers") {
    return <CombatBar anchor={anchor} inline={priorityInline} decision={decision} canAct={canAct} />;
  }
  return <PriorityBar anchor={anchor} inline={priorityInline} selectedObjectId={selectedObjectId} />;
}
