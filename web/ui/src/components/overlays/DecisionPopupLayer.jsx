import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { useGame } from "@/context/GameContext";
import { useHover } from "@/context/HoverContext";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import DecisionRouter from "@/components/decisions/DecisionRouter";
import DecisionSummary from "@/components/decisions/DecisionSummary";
import { normalizeDecisionText } from "@/components/decisions/decisionText";
import { animate, cancelMotion, snappySpring, stagger } from "@/lib/motion/anime";
import { ManaSymbol, SymbolText } from "@/lib/mana-symbols";
import { nextPriorityAdvanceLabel } from "@/lib/constants";
import HighlightedDecisionText from "@/components/decisions/HighlightedDecisionText";
import { getPlayerAccent } from "@/lib/player-colors";
import {
  buildObjectControllerById,
  buildObjectNameById,
} from "@/lib/decision-object-meta";
import {
  defaultTriggerOrderingOrder,
  isTriggerOrderingDecision,
  normalizeTriggerOrderingOrder,
} from "@/lib/trigger-ordering";
import { useHoverSuppressedWhileScrolling } from "@/lib/useHoverSuppressedWhileScrolling";
import { cn } from "@/lib/utils";
import { X } from "lucide-react";

const ACTION_STRIP_BODY_CLASS = "min-h-0 h-full";
const MANA_PAYMENT_TAB_EXIT_MS = 320;

function clamp(value, min, max) {
  return Math.min(max, Math.max(min, value));
}

function renderMobileBattlePortal(content, target = null) {
  if (typeof document === "undefined") return content;
  const candidateTarget = target?.current || target;
  const resolvedTarget = candidateTarget && typeof candidateTarget.nodeType === "number"
    ? candidateTarget
    : document.body;
  return createPortal(content, resolvedTarget);
}

function isSingleGenericPip(symbols) {
  return Array.isArray(symbols) && symbols.length === 1 && String(symbols[0]) === "1";
}

function manaPaymentDisplayCode(symbols) {
  const normalized = Array.isArray(symbols)
    ? symbols
      .map((symbol) => String(symbol || "").trim().toUpperCase())
      .filter(Boolean)
    : [];
  return normalized.join("/") || "0";
}

function manaPaymentEndsOnGeneric(payment) {
  const pips = Array.isArray(payment?.pips) ? payment.pips : [];
  return pips.length > 0 && isSingleGenericPip(pips[pips.length - 1]);
}

function buildManaPaymentGroups(payment) {
  const pips = Array.isArray(payment?.pips) ? payment.pips : [];
  const currentIndex = clamp(Number(payment?.current_pip_index || 0), 0, pips.length);
  const groups = [];

  for (let index = 0; index < pips.length; index += 1) {
    const pip = pips[index];

    if (isSingleGenericPip(pip)) {
      let count = 1;
      while (index + count < pips.length && isSingleGenericPip(pips[index + count])) {
        count += 1;
      }

      const paidCount = clamp(currentIndex - index, 0, count);
      groups.push({
        key: `generic-${index}`,
        start: index,
        end: index + count,
        kind: "generic",
        displayCount: Math.max(0, count - paidCount),
        isActive: currentIndex >= index && currentIndex < index + count,
        isPaid: currentIndex >= index + count,
      });
      index += count - 1;
      continue;
    }

    groups.push({
      key: `pip-${index}`,
      start: index,
      end: index + 1,
      kind: "symbol",
      displayCode: manaPaymentDisplayCode(pip),
      isActive: currentIndex === index,
      isPaid: currentIndex > index,
    });
  }

  return groups;
}

function ManaPaymentTab({ manaPayment = null, anchorRect = null }) {
  const [renderedPayment, setRenderedPayment] = useState(manaPayment);
  const [visible, setVisible] = useState(Boolean(manaPayment));
  const renderedPaymentRef = useRef(renderedPayment);
  const exitTimerRef = useRef(null);
  const frameRef = useRef(null);
  const shellRef = useRef(null);
  const indicatorRef = useRef(null);
  const groupNodeRefs = useRef(new Map());

  useEffect(() => {
    renderedPaymentRef.current = renderedPayment;
  }, [renderedPayment]);

  useEffect(() => {
    if (exitTimerRef.current) {
      clearTimeout(exitTimerRef.current);
      exitTimerRef.current = null;
    }
    if (frameRef.current) {
      cancelAnimationFrame(frameRef.current);
      frameRef.current = null;
    }

    if (manaPayment) {
      frameRef.current = requestAnimationFrame(() => {
        setRenderedPayment(manaPayment);
        setVisible(true);
        frameRef.current = null;
      });
      return undefined;
    }

    if (!renderedPaymentRef.current) return undefined;

    frameRef.current = requestAnimationFrame(() => {
      setRenderedPayment((current) => {
        const totalPips = Array.isArray(current?.pips) ? current.pips.length : 0;
        if (!current || !manaPaymentEndsOnGeneric(current) || current.current_pip_index >= totalPips) {
          return current;
        }
        return {
          ...current,
          current_pip_index: totalPips,
        };
      });
      setVisible(false);
      frameRef.current = null;
    });
    exitTimerRef.current = setTimeout(() => {
      setRenderedPayment(null);
      exitTimerRef.current = null;
    }, MANA_PAYMENT_TAB_EXIT_MS);

    return undefined;
  }, [manaPayment]);

  useEffect(() => () => {
    if (exitTimerRef.current) {
      clearTimeout(exitTimerRef.current);
      exitTimerRef.current = null;
    }
    if (frameRef.current) {
      cancelAnimationFrame(frameRef.current);
      frameRef.current = null;
    }
  }, []);

  const groups = useMemo(
    () => (renderedPayment ? buildManaPaymentGroups(renderedPayment) : []),
    [renderedPayment]
  );

  useLayoutEffect(() => {
    const shellEl = shellRef.current;
    const indicatorEl = indicatorRef.current;
    const activeGroup = groups.find((group) => group.isActive);
    if (!shellEl || !indicatorEl) {
      return;
    }
    if (!activeGroup) {
      indicatorEl.style.opacity = "0";
      return;
    }

    const activeEl = groupNodeRefs.current.get(activeGroup.key);
    if (!activeEl) {
      indicatorEl.style.opacity = "0";
      return;
    }

    const shellRect = shellEl.getBoundingClientRect();
    const activeRect = activeEl.getBoundingClientRect();
    indicatorEl.style.opacity = "1";
    indicatorEl.style.transform = `translate(${activeRect.left - shellRect.left}px, ${activeRect.top - shellRect.top}px)`;
    indicatorEl.style.width = `${activeRect.width}px`;
    indicatorEl.style.height = `${activeRect.height}px`;
  }, [groups, visible]);

  if (!renderedPayment || groups.length === 0) return null;

  const tabContent = (
    <div
      className={cn(
        anchorRect
          ? "pointer-events-none fixed z-[140] h-0 overflow-visible transition-all duration-300 ease-[cubic-bezier(0.22,1,0.36,1)]"
          : "pointer-events-none absolute inset-x-0 top-0 z-[140] h-0 overflow-visible transition-all duration-300 ease-[cubic-bezier(0.22,1,0.36,1)]",
        visible ? "opacity-100" : "opacity-0"
      )}
      style={anchorRect
        ? {
          left: `${anchorRect.left}px`,
          top: `${anchorRect.top}px`,
          width: `${anchorRect.width}px`,
        }
        : undefined}
      aria-hidden="true"
    >
      <div
        className={cn(
          "absolute left-1/2 top-0 w-max max-w-[min(52vw,380px)] origin-bottom transition-all duration-300 ease-[cubic-bezier(0.22,1,0.36,1)]",
          anchorRect
            ? (visible
              ? "-translate-x-1/2 translate-y-[-82%]"
              : "-translate-x-1/2 translate-y-[-98%]")
            : (visible
              ? "-translate-x-1/2 translate-y-[-118%]"
              : "-translate-x-1/2 translate-y-[-134%]")
      )}
      >
        <div
        ref={shellRef}
        className="mana-payment-shell relative overflow-visible rounded-none border px-2.5 py-1.5"
        >
          <div className="mana-payment-shell-glow absolute inset-0" />
          <div className="absolute inset-x-0 top-0 h-px bg-[linear-gradient(90deg,transparent,rgba(255,220,176,0.85),transparent)]" />
          <div className="mana-payment-tail absolute left-1/2 top-full h-3.5 w-14 -translate-x-1/2 -translate-y-px overflow-hidden rounded-none border-x border-b" />
          <div
            ref={indicatorRef}
            className="mana-payment-indicator absolute left-0 top-0 rounded-none border opacity-0 transition-all duration-300 ease-[cubic-bezier(0.22,1,0.36,1)]"
          />
          <div className="mana-payment-track relative rounded-none border px-1.5 py-0.5">
            <div className="relative flex items-center gap-1.5">
              {groups.map((group) => {
                const toneClass = group.isPaid
                  ? "opacity-45 saturate-[0.12] grayscale"
                  : group.isActive
                    ? "opacity-100"
                    : "opacity-88";
                return (
                  <span
                    key={group.key}
                    ref={(node) => {
                      if (node) groupNodeRefs.current.set(group.key, node);
                      else groupNodeRefs.current.delete(group.key);
                    }}
                    className={cn(
                      "mana-payment-group relative inline-flex min-w-[28px] items-center justify-center rounded-none px-1 py-0.5 transition-all duration-300 ease-[cubic-bezier(0.22,1,0.36,1)]",
                      toneClass
                    )}
                    style={group.isActive ? { filter: "drop-shadow(0 0 10px rgba(247,160,64,0.44))" } : undefined}
                  >
                    {group.kind === "generic" ? (
                      <ManaSymbol sym={String(group.displayCount)} size={18} />
                    ) : (
                      <ManaSymbol sym={group.displayCode} size={18} />
                    )}
                  </span>
                );
              })}
            </div>
          </div>
        </div>
      </div>
    </div>
  );

  if (anchorRect && typeof document !== "undefined") {
    return createPortal(tabContent, document.body);
  }

  return tabContent;
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

function resolveObjectAccent(players, perspective, controllerById, objectId, explicitControllerId = null) {
  const controllerId = explicitControllerId != null
    ? Number(explicitControllerId)
    : controllerById.get(String(objectId));
  if (controllerId == null || Number(controllerId) === Number(perspective)) {
    return null;
  }
  return getPlayerAccent(players || [], controllerId);
}

function PriorityActionPillLabel({
  text,
  viewportRef,
  carouselResetVersion = 0,
  highlightText = "",
  highlightColor = null,
}) {
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
          <HighlightedDecisionText
            text={displayText}
            highlightText={highlightText}
            highlightColor={highlightColor}
          />
        </span>
        <span className="block min-w-0 overflow-hidden whitespace-nowrap" style={{ textOverflow: "clip" }}>
          <HighlightedDecisionText
            text={displayText}
            highlightText={highlightText}
            highlightColor={highlightColor}
          />
        </span>
      </span>
    );
  }

  return (
    <span ref={containerRef} className="relative block min-w-0 overflow-hidden" style={{ textOverflow: "clip" }}>
      <span ref={measureRef} className="absolute left-0 top-0 invisible inline-block whitespace-nowrap pointer-events-none">
        <HighlightedDecisionText
          text={displayText}
          highlightText={highlightText}
          highlightColor={highlightColor}
        />
      </span>
      <span
        ref={marqueeRef}
        className="inline-flex whitespace-nowrap will-change-transform"
      >
        <span className="pr-7">
          <HighlightedDecisionText
            text={displayText}
            highlightText={highlightText}
            highlightColor={highlightColor}
          />
        </span>
        <span aria-hidden="true" className="pr-7">
          <HighlightedDecisionText
            text={displayText}
            highlightText={highlightText}
            highlightColor={highlightColor}
          />
        </span>
      </span>
    </span>
  );
}

function PriorityActionStrip({
  groups,
  canAct,
  players,
  perspective,
  hasPinnedSelection = false,
  objectNameById,
  objectControllerById,
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
  const { attachScrollableRef, hoverSuppressed } = useHoverSuppressedWhileScrolling({
    onScrollStart: onActionHoverEnd,
  });
  const compactLandscapeViewport = typeof window !== "undefined"
    && window.matchMedia("(max-height: 480px) and (orientation: landscape)").matches;
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
  const compactActionLabel = useCallback((label) => {
    const raw = String(label || "").trim();
    if (!compactLandscapeViewport) return raw;
    return raw
      .replace(/\s*\([^)]*\)\s*/g, " ")
      .replace(/\s{2,}/g, " ")
      .trim();
  }, [compactLandscapeViewport]);

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
      if (
        typeof window !== "undefined"
        && window.matchMedia("(max-width: 720px) and (orientation: portrait)").matches
      ) {
        setLoopEnabled((prev) => (prev ? false : prev));
        return;
      }
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

    const objectHoverActive = !hasPinnedSelection && typeof document !== "undefined"
      && Boolean(document.querySelector("[data-object-id]:hover"));
    const focusKind = hasPinnedSelection
      ? (selectedGroupKeys.length > 0 ? "selected" : null)
      : ((!isPointerInStrip && objectHoverActive && hoveredGroupKeys.length > 0)
          ? "hover"
          : (selectedGroupKeys.length > 0 ? "selected" : null));
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
  }, [groupKeysSignature, hasPinnedSelection, hoveredGroupKeys, isPointerInStrip, middleLoopIndex, selectedGroupKeys]);

  const handleViewportWheel = useCallback((event) => {
    const viewport = viewportRef.current;
    if (!viewport) return;
    if (viewport.scrollWidth <= viewport.clientWidth + 1) return;

    const primaryDelta = Math.abs(event.deltaX) > Math.abs(event.deltaY)
      ? event.deltaX
      : event.deltaY;
    if (Math.abs(primaryDelta) < 0.5) return;

    event.preventDefault();
    viewport.scrollBy({
      left: primaryDelta,
      behavior: "auto",
    });
  }, []);

  useEffect(() => {
    const viewport = viewportRef.current;
    if (!viewport) return undefined;

    viewport.addEventListener("wheel", handleViewportWheel, { passive: false });
    return () => {
      viewport.removeEventListener("wheel", handleViewportWheel);
    };
  }, [handleViewportWheel]);

  if (!canAct) {
    return (
      <div className="action-strip-empty-state action-strip-empty-state--waiting flex min-w-0 flex-1 items-center px-3 text-[12px] whitespace-nowrap">
        Waiting for opponent
      </div>
    );
  }

  if (!groups.length) {
    return (
      <div className="action-strip-empty-state action-strip-empty-state--empty flex min-w-0 flex-1 items-center px-3 text-[12px] whitespace-nowrap">
        No actions available
      </div>
    );
  }

  return (
    <div
      ref={(node) => {
        viewportRef.current = node;
        attachScrollableRef(node);
      }}
      className="action-strip-scroll min-w-0 flex-1 overflow-x-auto overflow-y-hidden whitespace-nowrap"
      onMouseEnter={() => setIsPointerInStrip(true)}
      onMouseLeave={() => setIsPointerInStrip(false)}
    >
      <div className="flex w-max min-w-full min-h-[32px] items-stretch gap-1.5 pr-2">
        {displayGroups.map(({ key, cycle, group }) => {
          const isPrimaryCycle = cycle === middleLoopIndex;
          const linkedActive = isGroupHoveredLinked(group) || isGroupSelectedLinked(group);
          const highlightName = group.hoverObjectId != null
            ? objectNameById.get(String(group.hoverObjectId)) || ""
            : "";
          const accent = resolveObjectAccent(
            players,
            perspective,
            objectControllerById,
            group.hoverObjectId
          );
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
            "action-strip-pill inline-flex max-w-[360px] min-w-0 items-center self-stretch px-2.5 text-[12px] font-semibold transition-all",
            linkedActive
              ? "is-linked-active text-[#fff5de]"
              : "text-[#d8ccb4]",
            "is-interactive"
          );
          const pillContent = (
            <>
              {group.count > 1 && (
                <span className="action-strip-pill-count mr-1.5 inline-flex h-4 min-w-4 items-center justify-center px-1 text-[10px] font-bold leading-none tracking-wide text-[#f5d08b]">
                  x{group.count}
                </span>
              )}
              <PriorityActionPillLabel
                text={compactActionLabel(group.label)}
                viewportRef={viewportRef}
                carouselResetVersion={carouselResetByGroupKey[group.key] || 0}
                highlightText={highlightName}
                highlightColor={accent?.hex || null}
              />
            </>
          );

          return (
            <button
              key={key}
              type="button"
              aria-hidden={isPrimaryCycle ? undefined : true}
              tabIndex={isPrimaryCycle ? undefined : -1}
              ref={setNodeRef}
              className={pillClassName}
              style={{ textOverflow: "clip" }}
              onPointerDown={(event) => {
                if (event.button !== 0) return;
                if (event.pointerType && event.pointerType !== "mouse") return;
                // Match decision option buttons so a pointer sequence that
                // started on a mana pip cannot finish as a click on a newly
                // rendered priority action under the cursor.
                event.preventDefault();
                onActionClick(group.firstAction);
              }}
              onClick={(event) => {
                if (event.detail !== 0) return;
                onActionClick(group.firstAction);
              }}
              onMouseEnter={() => {
                if (hoverSuppressed) return;
                onActionHoverStart(group);
              }}
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
  if (decision.reason) return decision.reason;
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

function buildBattlefieldObjectIdSet(players) {
  const ids = new Set();
  for (const player of players || []) {
    for (const card of player?.battlefield || []) {
      if (card?.id != null) ids.add(String(card.id));
      for (const memberId of card?.member_ids || []) {
        ids.add(String(memberId));
      }
    }
  }
  return ids;
}

function canUseMobileBoardSelection(state, decision) {
  if (!decision) return false;
  const battlefieldObjectIds = buildBattlefieldObjectIdSet(state?.players);

  if (decision.kind === "targets") {
    return (decision.requirements || []).every((req) =>
      (req.legal_targets || []).every((target) => (
        target?.kind === "player"
          || (target?.kind === "object" && target.object != null && battlefieldObjectIds.has(String(target.object)))
      ))
    );
  }

  if (decision.kind === "select_objects") {
    return (decision.candidates || [])
      .filter((candidate) => candidate?.legal !== false)
      .every((candidate) => candidate?.id != null && battlefieldObjectIds.has(String(candidate.id)));
  }

  return false;
}

function buildViewedCardsIdentity(viewedCards) {
  if (!viewedCards) return "";
  const cardIds = Array.isArray(viewedCards.card_ids) ? viewedCards.card_ids.join(",") : "";
  return [
    viewedCards.visibility || "",
    viewedCards.subject ?? "",
    viewedCards.zone || "",
    viewedCards.source ?? "",
    viewedCards.description || "",
    cardIds,
  ].join("|");
}

function normalizeMobileDecisionSummaryText(text) {
  if (typeof text !== "string") return "";
  return normalizeDecisionText(text)
    .replace(/^spell effects(?:\s+\d+)?\s*:\s*/i, "")
    .trim();
}

function decisionTextMatches(left, right) {
  return normalizeDecisionText(String(left || "")).trim().toLowerCase()
    === normalizeDecisionText(String(right || "")).trim().toLowerCase();
}

function buildMobileSelectOptionsSummary(decision) {
  const segments = [
    decision?.context_text,
    decision?.consequence_text,
  ]
    .map((value) => normalizeMobileDecisionSummaryText(value))
    .filter(Boolean);

  if (segments.length > 0) return segments.join(" ");
  return normalizeMobileDecisionSummaryText(decision?.description || "");
}

function ViewedCardsStrip({
  label,
  description = "",
  sourceName = "",
  cards = [],
  players = [],
  perspective = null,
  objectControllerById = new Map(),
  hoveredObjectId = null,
  selectedObjectId = null,
  onCardHoverStart,
  onCardHoverEnd,
  compact = false,
}) {
  const { attachScrollableRef, hoverSuppressed } = useHoverSuppressedWhileScrolling({
    onScrollStart: onCardHoverEnd,
  });

  const normalizedSourceName = String(sourceName || "").trim();
  const normalizedDescription = String(description || "").trim();
  const metadata = (
    <>
      <div className="flex min-w-0 items-center gap-2">
        <div className="shrink-0 text-[11px] font-bold uppercase tracking-[0.14em] text-[#d9c18b]">
          {label}
        </div>
        {normalizedSourceName && (
          <div className="min-w-0 truncate text-[11px] text-[#d8cdb6]">
            <SymbolText text={normalizeDecisionText(normalizedSourceName)} />
          </div>
        )}
      </div>
      {normalizedDescription && (
        <div className={cn(
          "text-[12px] leading-snug text-[#c7baa1]",
          compact && "truncate"
        )}>
          <SymbolText text={normalizeDecisionText(normalizedDescription)} />
        </div>
      )}
    </>
  );
  const cardScroller = (
    <div
      ref={attachScrollableRef}
      className={cn(
        "action-strip-scroll min-w-0 overflow-x-auto overflow-y-hidden",
        compact && "flex-1 self-stretch"
      )}
    >
      <div className="flex w-max min-w-full items-center gap-1.5 pb-0.5">
        {cards.length > 0 ? cards.map((card) => (
          <button
            key={card.id}
            type="button"
            className={cn(
              "action-strip-pill action-strip-view-card inline-flex max-w-[220px] items-center px-2 py-1 text-[12px] transition-all",
              String(hoveredObjectId) === String(card.id) || String(selectedObjectId) === String(card.id)
                ? "is-linked-active text-[#fff5de]"
                : "is-interactive text-[#decfae]"
            )}
            onMouseEnter={() => {
              if (hoverSuppressed) return;
              onCardHoverStart?.(card);
            }}
            onMouseLeave={() => onCardHoverEnd?.()}
          >
            <span className="truncate">
              <HighlightedDecisionText
                text={normalizeDecisionText(card.name)}
                highlightText={normalizeDecisionText(card.name)}
                highlightColor={
                  resolveObjectAccent(
                    players,
                    perspective,
                    objectControllerById,
                    card.id,
                    card.controller
                  )?.hex || null
                }
              />
            </span>
          </button>
        )) : (
          <div className="text-[12px] italic text-[#bda983]">
            No cards visible.
          </div>
        )}
      </div>
    </div>
  );

  return (
    <div className="viewed-cards-strip min-w-0 flex-1 overflow-hidden px-1 py-1">
      {compact ? (
        <div className="flex min-w-0 items-center gap-3">
          <div className="min-w-[200px] max-w-[360px] shrink-0">
            {metadata}
          </div>
          {cardScroller}
        </div>
      ) : (
        <div className="flex flex-col gap-1">
          {metadata}
          {cardScroller}
        </div>
      )}
    </div>
  );
}

function MobileDecisionHeader({
  eyebrow,
  title,
  subtitle = "",
  details = null,
  trailing = null,
  compact = false,
  className = "",
}) {
  if (compact) {
    return (
      <div className={cn("mobile-decision-header mobile-decision-header--compact", className)}>
        <div className="mobile-decision-header-copy">
          {eyebrow ? (
            <div className="mobile-decision-eyebrow">
              {eyebrow}
            </div>
          ) : null}
          <div className="mobile-decision-title">
            {normalizeDecisionText(title || "Decision")}
          </div>
          {subtitle ? (
            <div className="mobile-decision-subtitle">
              <SymbolText text={normalizeDecisionText(subtitle)} noWrap />
            </div>
          ) : null}
          {details ? (
            <div className="mobile-decision-header-details">
              {details}
            </div>
          ) : null}
        </div>
        {trailing ? (
          <div className="mobile-decision-header-trailing mobile-decision-header-trailing--compact">
            {trailing}
          </div>
        ) : null}
      </div>
    );
  }

  return (
    <div className={cn("mobile-decision-header", className)}>
      {trailing ? (
        <div className="mobile-decision-header-trailing">
          {trailing}
        </div>
      ) : null}
      {eyebrow ? (
        <div className="mobile-decision-eyebrow">
          {eyebrow}
        </div>
      ) : null}
      <div className="mobile-decision-title">
        {normalizeDecisionText(title || "Decision")}
      </div>
      {subtitle ? (
        <div className="mobile-decision-subtitle">
          <SymbolText text={normalizeDecisionText(subtitle)} />
        </div>
      ) : null}
      {details ? (
        <div className="mobile-decision-header-details">
          {details}
        </div>
      ) : null}
    </div>
  );
}

export function MobileDecisionCloseButton({
  label = "Close",
  onClick,
  className = "",
}) {
  return (
    <button
      type="button"
      className={cn("mobile-decision-close", className)}
      aria-label={label}
      onClick={onClick}
    >
      <X className="size-4" />
    </button>
  );
}

function MobileDecisionDock({
  subtitle = "",
  primaryLabel = "Continue",
  primaryDisabled = false,
  onPrimary,
  secondaryLabel = "",
  secondaryDisabled = false,
  onSecondary,
  inline = false,
}) {
  return (
    <div className={cn("mobile-decision-dock", inline && "mobile-decision-dock--inline")}>
      <div className="mobile-decision-dock-actions">
        {secondaryLabel ? (
          <Button
            type="button"
            variant="ghost"
            size="sm"
            className="mobile-decision-secondary-button"
            disabled={secondaryDisabled}
            onClick={onSecondary}
          >
            {secondaryLabel}
          </Button>
        ) : null}
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="mobile-decision-primary-button"
          disabled={primaryDisabled}
          onClick={onPrimary}
        >
          <span className="mobile-decision-primary-label">
            {primaryLabel}
          </span>
          {subtitle ? (
            <span className="mobile-decision-primary-subtitle">
              <SymbolText text={normalizeDecisionText(subtitle)} noWrap />
            </span>
          ) : null}
        </Button>
      </div>
    </div>
  );
}

export function MobileDecisionSheet({
  eyebrow = "",
  title = "",
  subtitle = "",
  headerDetails = null,
  headerTrailing = null,
  headerClassName = "",
  children,
  footer = null,
  onBackdropClick = null,
  onClose = null,
  closeLabel = "Close panel",
  inline = false,
  compactInline = false,
  className = "",
  bodyClassName = "",
}) {
  const resolvedHeaderTrailing = headerTrailing || (onClose ? (
    <MobileDecisionCloseButton
      label={closeLabel}
      onClick={onClose}
    />
  ) : null);

  return (
    <>
      {!inline ? (
        <div
          className="mobile-decision-sheet-backdrop"
          onClick={onBackdropClick || undefined}
          aria-hidden="true"
        />
      ) : null}
      <div className={cn("mobile-decision-sheet-shell", inline && "mobile-decision-sheet-shell--inline")}>
        <section
          className={cn(
            "mobile-decision-sheet",
            inline && "mobile-decision-sheet--inline",
            inline && compactInline && "mobile-decision-sheet--inline-compact",
            className
          )}
          aria-modal="true"
          role="dialog"
        >
          <MobileDecisionHeader
            eyebrow={eyebrow}
            title={title}
            subtitle={subtitle}
            details={headerDetails}
            trailing={resolvedHeaderTrailing}
            compact={inline && compactInline}
            className={headerClassName}
          />
          <div className={cn("mobile-decision-sheet-body", inline && compactInline && "mobile-decision-sheet-body--inline-compact", bodyClassName)}>
            {children}
          </div>
          {footer ? (
            <div className="mobile-decision-sheet-footer">
              {footer}
            </div>
          ) : null}
        </section>
      </div>
    </>
  );
}

function MobileDecisionOverlay({
  eyebrow = "",
  title = "",
  subtitle = "",
  headerDetails = null,
  headerTrailing = null,
  headerClassName = "",
  children,
  footer = null,
  onBackdropClick = null,
  className = "",
  bodyClassName = "",
}) {
  return (
    <>
      <div
        className="mobile-decision-overlay-backdrop"
        onClick={onBackdropClick || undefined}
        aria-hidden="true"
      />
      <div className="mobile-decision-overlay-shell">
        <section className={cn("mobile-decision-overlay", className)} aria-modal="true" role="dialog">
          <MobileDecisionHeader
            eyebrow={eyebrow}
            title={title}
            subtitle={subtitle}
            details={headerDetails}
            trailing={headerTrailing}
            className={headerClassName}
          />
          <div className={cn("mobile-decision-overlay-body", bodyClassName)}>
            {children}
          </div>
          {footer ? (
            <div className="mobile-decision-overlay-footer">
              {footer}
            </div>
          ) : null}
        </section>
      </div>
    </>
  );
}

export function MobileDecisionActionList({
  items = [],
  emptyText = "No additional actions.",
  horizontal = false,
}) {
  if (!items.length) {
    return (
      <div className={cn("mobile-decision-empty-state", horizontal && "mobile-decision-empty-state--inline-strip")}>
        {emptyText}
      </div>
    );
  }

  return (
    <div className={cn("mobile-decision-action-list", horizontal && "mobile-decision-action-list--inline-strip")}>
      {items.map((item) => (
        <button
          key={item.key}
          type="button"
          className={cn("mobile-decision-action-row", horizontal && "mobile-decision-action-row--inline-strip")}
          disabled={Boolean(item.disabled)}
          onClick={item.onClick}
          onMouseEnter={item.onMouseEnter}
          onMouseLeave={item.onMouseLeave}
        >
          <span className="mobile-decision-action-text">
            <SymbolText text={normalizeDecisionText(item.label || "Action")} />
          </span>
          {item.trailing || null}
        </button>
      ))}
    </div>
  );
}

function MobilePriorityActionList({
  groups,
  canAct,
  onActionClick,
  onActionHoverStart,
  onActionHoverEnd,
  horizontal = false,
}) {
  return (
    <MobileDecisionActionList
      horizontal={horizontal}
      items={groups.map((group) => ({
        key: group.key,
        label: group.label || group.firstAction?.label || "Action",
        disabled: !canAct,
        onClick: () => onActionClick(group.firstAction),
        onMouseEnter: () => onActionHoverStart?.(group),
        onMouseLeave: () => onActionHoverEnd?.(),
        trailing: group.count > 1 ? (
          <span className="mobile-decision-action-count">
            {group.count}
          </span>
        ) : null,
      }))}
    />
  );
}

function MobileBattleDecisionLayer({
  selectedObjectId = null,
  portalTarget = null,
  dockInline = false,
  dockHidden = false,
}) {
  const {
    state,
    dispatch,
    cancelDecision,
    triggerOrderingState,
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

  const [actionsSheetState, setActionsSheetState] = useState({ key: "", open: false });
  const [acknowledgedViewedCardsToken, setAcknowledgedViewedCardsToken] = useState("");
  const [submitState, setSubmitState] = useState({ key: "", action: null });
  const [combatActionState, setCombatActionState] = useState({ key: "", action: null });

  const decisionIdentity = [
    decision?.kind || "",
    decision?.player ?? "",
    decision?.source_id ?? "",
    decision?.source_name || "",
    decision?.reason || "",
    decision?.description || "",
    decision?.context_text || "",
    decision?.consequence_text || "",
  ].join("|");
  const viewedCards = state?.viewed_cards || null;
  const viewedCardsLabel = viewedCards?.visibility === "public" ? "Revealed" : "Look";
  const viewedCardsIdentity = useMemo(
    () => buildViewedCardsIdentity(viewedCards),
    [viewedCards]
  );
  const viewedCardsToken = viewedCardsIdentity ? `${decisionIdentity}|${viewedCardsIdentity}` : "";
  const showViewedCardsStep = Boolean(viewedCardsToken)
    && acknowledgedViewedCardsToken !== viewedCardsToken;
  const actionsSheetOpen = actionsSheetState.key === decisionIdentity
    ? actionsSheetState.open
    : false;
  const canCancelDecision = canAct && !!state?.cancelable;
  const isPriorityDecision = decision?.kind === "priority";
  const isCombatDecision = decision?.kind === "attackers" || decision?.kind === "blockers";
  const stackSize = Number(state?.stack_size || 0);
  const decisionActions = useMemo(() => decision?.actions || [], [decision]);
  const passAction = useMemo(
    () => decisionActions.find((action) => action.kind === "pass_priority"),
    [decisionActions]
  );
  const otherActions = useMemo(
    () => decisionActions.filter((action) => action.kind !== "pass_priority"),
    [decisionActions]
  );
  const battlefieldFamilies = useMemo(
    () => buildBattlefieldFamilies(state?.players),
    [state?.players]
  );
  const actionGroups = useMemo(
    () => buildPriorityActionGroups(otherActions, battlefieldFamilies),
    [otherActions, battlefieldFamilies]
  );
  const showPriorityAdvanceButton = !!passAction;
  const hasCustomPassLabel = !!passAction?.label && passAction.label !== "Pass priority";
  const passLabel = showPriorityAdvanceButton
    ? (
      hasCustomPassLabel
        ? passAction.label
        : nextPriorityAdvanceLabel(state?.phase, state?.step, stackSize)
    )
    : (actionGroups[0]?.label || "Continue");
  const objectNameById = useMemo(
    () => buildObjectNameById(state),
    [state]
  );
  const objectControllerById = useMemo(
    () => buildObjectControllerById(state),
    [state]
  );
  const viewedCardEntries = useMemo(
    () => {
      if (Array.isArray(viewedCards?.cards) && viewedCards.cards.length > 0) {
        return viewedCards.cards.map((card) => ({
          id: String(card.id),
          name: card.name || `Card #${card.id}`,
          controller: viewedCards?.subject,
        }));
      }
      return (viewedCards?.card_ids || []).map((id) => ({
        id: String(id),
        name: objectNameById.get(String(id)) || `Card #${id}`,
        controller: viewedCards?.subject,
      }));
    },
    [objectNameById, viewedCards]
  );
  const viewedCardsSourceName = (() => {
    if (viewedCards?.source != null) {
      const sourceName = objectNameById.get(String(viewedCards.source));
      if (sourceName) return sourceName;
    }
    return decision?.source_name || "";
  })();
  const toolbarDecisionSummary = useMemo(() => {
    const parts = [
      decision?.description,
      decision?.context_text,
    ]
      .map((value) => normalizeDecisionText(value))
      .filter(Boolean);
    return parts[0] || "";
  }, [decision?.context_text, decision?.description]);
  const mobileDockSubtitle = useMemo(() => {
    if (toolbarDecisionSummary) return toolbarDecisionSummary;
    if (stackSize > 0) {
      return `Resolve ${stackSize}`;
    }
    return nextPriorityAdvanceLabel(state?.phase, state?.step, stackSize);
  }, [stackSize, state?.phase, state?.step, toolbarDecisionSummary]);

  const triggerPriorityAction = useCallback(
    (action) => {
      if (!canAct || !action) return;
      clearHoverLinkedObjects();
      clearHover();
      dispatch(
        { type: "priority_action", action_index: action.index },
        action.label
      );
      setActionsSheetState({ key: decisionIdentity, open: false });
    },
    [canAct, clearHover, clearHoverLinkedObjects, decisionIdentity, dispatch]
  );
  const handleActionHoverStart = useCallback(
    (group) => {
      if (!canAct || !group) return;
      setHoverLinkedObjects(group.linkedObjectIds || []);
      if (group.hoverObjectId != null) hoverCard(group.hoverObjectId);
    },
    [canAct, hoverCard, setHoverLinkedObjects]
  );
  const handleActionHoverEnd = useCallback(() => {
    clearHoverLinkedObjects();
    clearHover();
  }, [clearHover, clearHoverLinkedObjects]);
  const handleViewedCardHoverStart = useCallback((card) => {
    if (!card?.id) return;
    clearHoverLinkedObjects();
    hoverCard(card.id);
  }, [clearHoverLinkedObjects, hoverCard]);
  const handleViewedCardHoverEnd = useCallback(() => {
    clearHoverLinkedObjects();
    clearHover();
  }, [clearHoverLinkedObjects, clearHover]);
  const handleSubmitActionChange = useCallback(
    (nextAction) => {
      setSubmitState({ key: decisionIdentity, action: nextAction || null });
    },
    [decisionIdentity]
  );
  const handleCombatActionChange = useCallback(
    (nextAction) => {
      setCombatActionState({ key: decisionIdentity, action: nextAction || null });
    },
    [decisionIdentity]
  );
  const submitAction = submitState.key === decisionIdentity ? submitState.action : null;
  const combatAction = combatActionState.key === decisionIdentity ? combatActionState.action : null;
  const triggerOrderingDecision = isTriggerOrderingDecision(decision);
  const triggerOrderingSubmitAction = useMemo(() => {
    if (!triggerOrderingDecision) return null;
    const order = triggerOrderingState?.order?.length
      ? normalizeTriggerOrderingOrder(triggerOrderingState.order, decision)
      : defaultTriggerOrderingOrder(decision);
    return {
      label: "Submit Order",
      disabled: !canAct,
      onSubmit: () => {
        clearHover();
        dispatch({ type: "select_options", option_indices: order }, "Order submitted");
      },
    };
  }, [canAct, clearHover, decision, dispatch, triggerOrderingDecision, triggerOrderingState]);
  const effectiveSubmitAction = triggerOrderingSubmitAction || submitAction;
  const canSubmitFocused = canAct
    && !!effectiveSubmitAction
    && !effectiveSubmitAction.disabled
    && typeof effectiveSubmitAction.onSubmit === "function";
  const boardSelectionDecision = (
    (decision?.kind === "targets" || decision?.kind === "select_objects")
    && canUseMobileBoardSelection(state, decision)
  );
  const completeViewedCardsStep = useCallback(() => {
    if (!viewedCardsToken) return;
    setAcknowledgedViewedCardsToken(viewedCardsToken);
  }, [viewedCardsToken]);

  if (!decision) return null;

  if (showViewedCardsStep) {
    return renderMobileBattlePortal(
      <MobileDecisionOverlay
        eyebrow={canAct ? "Your Action" : "Opponent Action"}
        title={viewedCardsLabel}
        subtitle={viewedCards?.description || viewedCardsSourceName}
      >
        <ViewedCardsStrip
          label={viewedCardsLabel}
          description={viewedCards?.description || ""}
          sourceName={viewedCardsSourceName}
          cards={viewedCardEntries}
          players={state?.players || []}
          perspective={state?.perspective}
          objectControllerById={objectControllerById}
          hoveredObjectId={hoveredObjectId}
          selectedObjectId={selectedObjectId}
          onCardHoverStart={handleViewedCardHoverStart}
          onCardHoverEnd={handleViewedCardHoverEnd}
        />
        <div className="mobile-decision-overlay-footer">
          <Button
            type="button"
            variant="ghost"
            size="sm"
            className="mobile-decision-primary-button mobile-decision-primary-button--full"
            disabled={!decision}
            onClick={completeViewedCardsStep}
          >
            <span className="mobile-decision-primary-label">Done</span>
          </Button>
        </div>
      </MobileDecisionOverlay>
    );
  }

  if (isPriorityDecision) {
    if (dockHidden) {
      return null;
    }

    const dockTitle = canAct ? "Your Action" : "Opponent Action";
    const singleActionGroup = actionGroups.length === 1 ? actionGroups[0] : null;
    const secondaryAction = showPriorityAdvanceButton
      ? (actionGroups.length > 1
        ? {
          label: "Actions",
          disabled: !canAct,
          onClick: () => setActionsSheetState({ key: decisionIdentity, open: true }),
        }
        : singleActionGroup?.firstAction
          ? {
            label: normalizeDecisionText(singleActionGroup.label || singleActionGroup.firstAction.label || "Action"),
            disabled: !canAct,
            onClick: () => triggerPriorityAction(singleActionGroup.firstAction),
          }
          : (canCancelDecision
            ? {
              label: "Cancel",
              disabled: !canCancelDecision,
              onClick: () => cancelDecision(),
            }
            : null))
      : (actionGroups.length > 1
        ? {
          label: "Actions",
          disabled: !canAct,
          onClick: () => setActionsSheetState({ key: decisionIdentity, open: true }),
        }
        : (canCancelDecision
          ? {
            label: "Cancel",
            disabled: !canCancelDecision,
            onClick: () => cancelDecision(),
          }
          : null));
    const primaryDisabled = !canAct || (!showPriorityAdvanceButton && actionGroups.length === 0);
    const resolvedDockSubtitle = decisionTextMatches(mobileDockSubtitle, passLabel)
      ? ""
      : mobileDockSubtitle;
    const handlePrimary = () => {
      if (showPriorityAdvanceButton) {
        triggerPriorityAction(passAction);
        return;
      }
      if (actionGroups[0]?.firstAction) {
        triggerPriorityAction(actionGroups[0].firstAction);
      }
    };

    return (
      <>
        {renderMobileBattlePortal(
          <MobileDecisionDock
            subtitle={resolvedDockSubtitle}
            primaryLabel={passLabel}
            primaryDisabled={primaryDisabled}
            onPrimary={handlePrimary}
            secondaryLabel={secondaryAction?.label || ""}
            secondaryDisabled={secondaryAction?.disabled || false}
            onSecondary={secondaryAction?.onClick}
            inline={dockInline}
          />,
          portalTarget
        )}
        {actionsSheetOpen ? (
          <MobileDecisionSheet
            eyebrow={dockTitle}
            title="Available Actions"
            subtitle={`${actionGroups.length} action${actionGroups.length === 1 ? "" : "s"}`}
            onBackdropClick={() => setActionsSheetState({ key: decisionIdentity, open: false })}
            onClose={() => setActionsSheetState({ key: decisionIdentity, open: false })}
            closeLabel="Close available actions"
            inline={false}
            className="mobile-decision-sheet--action-list"
            bodyClassName="mobile-decision-sheet-body--action-list"
            footer={canCancelDecision ? (
              <Button
                type="button"
                variant="ghost"
                size="sm"
                className="mobile-decision-secondary-button mobile-decision-secondary-button--wide"
                disabled={!canCancelDecision}
                onClick={() => {
                  cancelDecision();
                  setActionsSheetState({ key: decisionIdentity, open: false });
                }}
              >
                Cancel
              </Button>
            ) : null}
          >
            <MobilePriorityActionList
              groups={visibleActionGroups}
              canAct={canAct}
              onActionClick={triggerPriorityAction}
              onActionHoverStart={handleActionHoverStart}
              onActionHoverEnd={handleActionHoverEnd}
            />
          </MobileDecisionSheet>
        ) : null}
      </>
    );
  }

  if (decision.kind === "select_options") {
    const optionSummary = buildMobileSelectOptionsSummary(decision);
    const optionHeaderDetails = (
      optionSummary || effectiveSubmitAction
    ) ? (
      <div className="mobile-select-options-toolbar">
        {optionSummary ? (
          <div
            className={cn(
              "mobile-select-options-summary",
              optionSummary.length > 220 && "is-compact",
              optionSummary.length > 340 && "is-tight"
            )}
          >
            <SymbolText text={optionSummary} />
          </div>
        ) : (
          <div className="mobile-select-options-summary mobile-select-options-summary--empty" />
        )}
        {effectiveSubmitAction ? (
          <Button
            type="button"
            variant="ghost"
            size="sm"
            className="mobile-decision-primary-button mobile-select-options-submit"
            disabled={!canSubmitFocused}
            onClick={() => effectiveSubmitAction.onSubmit()}
          >
            <span className="mobile-decision-primary-label">
              {effectiveSubmitAction.label || "Submit"}
            </span>
          </Button>
        ) : null}
      </div>
    ) : null;

    return renderMobileBattlePortal(
      <MobileDecisionSheet
        eyebrow={canAct ? "Your Action" : "Opponent Action"}
        title={resolveDecisionTitle(decision)}
        subtitle={decision?.source_name || ""}
        headerClassName="mobile-select-options-header"
        headerDetails={optionHeaderDetails}
        className="mobile-decision-sheet--select-options"
        bodyClassName="mobile-decision-sheet-body--select-options"
        onClose={canCancelDecision ? () => cancelDecision() : null}
        closeLabel="Close option picker"
        inline={false}
        onBackdropClick={canCancelDecision ? () => cancelDecision() : null}
      >
        <DecisionRouter
          decision={decision}
          canAct={canAct}
          selectedObjectId={selectedObjectId}
          inlineSubmit={false}
          onSubmitActionChange={handleSubmitActionChange}
          hideDescription
          combatInline={false}
          layout="mobile-overlay"
          showStripSummary={false}
        />
      </MobileDecisionSheet>,
      null
    );
  }

  if (boardSelectionDecision) {
    const boardSelectionSubtitle = decision.kind === "targets"
      ? "Tap a highlighted card or player on the battlefield."
      : "Tap highlighted permanents on the battlefield.";

    return renderMobileBattlePortal(
      <>
        <MobileDecisionDock
          title={canAct ? "Your Action" : "Opponent Action"}
          subtitle={boardSelectionSubtitle}
          primaryLabel={effectiveSubmitAction?.label || "Submit"}
          primaryDisabled={!canSubmitFocused}
          onPrimary={() => effectiveSubmitAction?.onSubmit?.()}
          secondaryLabel={canCancelDecision ? "Cancel" : ""}
          secondaryDisabled={!canCancelDecision}
          onSecondary={canCancelDecision ? () => cancelDecision() : null}
          inline={dockInline}
        />
        <div className="hidden" aria-hidden="true">
          <DecisionRouter
            decision={decision}
            canAct={canAct}
            selectedObjectId={selectedObjectId}
            inlineSubmit={false}
            onSubmitActionChange={handleSubmitActionChange}
            hideDescription={false}
            combatInline={false}
            layout="panel"
            showStripSummary={false}
          />
        </div>
      </>,
      portalTarget
    );
  }

  if (isCombatDecision) {
    if (dockHidden) return null;

    return renderMobileBattlePortal(
      <>
        <MobileDecisionDock
          primaryLabel={
            combatAction?.label
            || (decision.kind === "attackers" ? "Confirm Attackers (0)" : "Confirm Blockers (0)")
          }
          primaryDisabled={combatAction?.disabled ?? !canAct}
          onPrimary={combatAction?.onSubmit}
          secondaryLabel={canCancelDecision ? "Cancel" : ""}
          secondaryDisabled={!canCancelDecision}
          onSecondary={canCancelDecision ? () => cancelDecision() : null}
          inline={dockInline}
        />
        <DecisionRouter
          decision={decision}
          canAct={canAct}
          selectedObjectId={selectedObjectId}
          inlineSubmit={false}
          onSubmitActionChange={null}
          onCombatActionChange={handleCombatActionChange}
          hideDescription
          combatInline
          layout="strip"
          showStripSummary={false}
        />
      </>,
      portalTarget
    );
  }

  const footer = (
    <>
      {canCancelDecision ? (
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="mobile-decision-secondary-button"
          disabled={!canCancelDecision}
          onClick={() => cancelDecision()}
        >
          Cancel
        </Button>
      ) : null}
      {effectiveSubmitAction ? (
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="mobile-decision-primary-button"
          disabled={!canSubmitFocused}
          onClick={() => effectiveSubmitAction.onSubmit()}
        >
          <span className="mobile-decision-primary-label">
            {effectiveSubmitAction.label || "Submit"}
          </span>
        </Button>
      ) : null}
    </>
  );

  return renderMobileBattlePortal(
    <MobileDecisionSheet
      eyebrow={canAct ? "Your Action" : "Opponent Action"}
      title={
        decision.kind === "attackers"
          ? "Declare Attackers"
          : decision.kind === "blockers"
            ? "Declare Blockers"
            : resolveDecisionTitle(decision)
      }
      subtitle={decision?.source_name || ""}
      inline={false}
      onBackdropClick={canCancelDecision ? () => cancelDecision() : null}
      footer={footer}
    >
      <DecisionRouter
        decision={decision}
        canAct={canAct}
        selectedObjectId={selectedObjectId}
        inlineSubmit={false}
        onSubmitActionChange={isCombatDecision ? null : handleSubmitActionChange}
        hideDescription={false}
        combatInline={isCombatDecision}
        layout="panel"
        showStripSummary={false}
      />
    </MobileDecisionSheet>,
    null
  );
}

function PriorityControlStack({
  actionCount = 0,
  holdEnabled = false,
  confirmEnabled = false,
  onHoldChange,
  onConfirmChange,
  showActionCount = true,
  className = "",
}) {
  const compactLandscapeViewport = typeof window !== "undefined"
    && window.matchMedia("(max-height: 480px) and (orientation: landscape)").matches;
  const checkboxLabelClass =
    "action-strip-toggle flex items-center gap-1.5 text-[11px] uppercase tracking-wider cursor-pointer transition-colors";

  return (
    <div className={cn("priority-control-stack flex shrink-0 flex-col items-start justify-center py-1.5", className)}>
      {showActionCount && !compactLandscapeViewport && (
        <div className="pointer-events-none pl-[18px] text-[11px] font-bold uppercase tracking-[0.14em] text-[#d9c18b]">
            {actionCount} actions
        </div>
      )}
      <div className="flex items-center gap-3">
        <label className={checkboxLabelClass}>
          <Checkbox
            checked={holdEnabled}
            onCheckedChange={(value) => onHoldChange?.(Boolean(value))}
            className="h-3 w-3"
          />
          <span title="Hold">{compactLandscapeViewport ? "H" : "Hold"}</span>
        </label>
        <label className={checkboxLabelClass}>
          <Checkbox
            checked={confirmEnabled}
            onCheckedChange={(value) => onConfirmChange?.(Boolean(value))}
            className="h-3 w-3"
          />
          <span title="Confirm">{compactLandscapeViewport ? "C" : "Confirm"}</span>
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
    triggerOrderingState,
  } = useGame();
  const {
    hoveredObjectId,
    hoverCard,
    clearHover,
    setHoverLinkedObjects,
    clearHoverLinkedObjects,
  } = useHover();
  const decision = state?.decision || null;
  const manaPayment = state?.mana_payment || null;
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
  const inlineRootRef = useRef(null);
  const [manaTabAnchorRect, setManaTabAnchorRect] = useState(null);
  const stackSize = Number(state?.stack_size || 0);
  const showPriorityAdvanceButton = !!passAction;
  const canCancelDecision = canAct && !!state?.cancelable;
  const hasCustomPassLabel = !!passAction?.label && passAction.label !== "Pass priority";
  const passLabel = holdRule === "always" || hasCustomPassLabel
    ? (passAction?.label || "Pass priority")
    : `→ ${nextPriorityAdvanceLabel(state?.phase, state?.step, stackSize)}`;
  const battlefieldFamilies = useMemo(
    () => buildBattlefieldFamilies(state?.players),
    [state?.players]
  );
  const actionGroups = useMemo(
    () => buildPriorityActionGroups(otherActions, battlefieldFamilies),
    [otherActions, battlefieldFamilies]
  );
  const objectNameById = useMemo(
    () => buildObjectNameById(state),
    [state]
  );
  const objectControllerById = useMemo(
    () => buildObjectControllerById(state),
    [state]
  );
  const decisionIdentity = [
    decision?.kind || "",
    decision?.player ?? "",
    decision?.source_id ?? "",
    decision?.source_name || "",
    decision?.reason || "",
    decision?.description || "",
    decision?.context_text || "",
    decision?.consequence_text || "",
  ].join("|");
  const viewedCards = state?.viewed_cards || null;
  const viewedCardsLabel = viewedCards?.visibility === "public" ? "Revealed" : "Look";
  const viewedCardsIdentity = useMemo(
    () => buildViewedCardsIdentity(viewedCards),
    [viewedCards]
  );
  const [acknowledgedViewedCardsToken, setAcknowledgedViewedCardsToken] = useState("");
  const viewedCardsToken = viewedCardsIdentity ? `${decisionIdentity}|${viewedCardsIdentity}` : "";
  const showViewedCardsStep = Boolean(viewedCardsToken)
    && acknowledgedViewedCardsToken !== viewedCardsToken;
  const triggerOrderingDecision = isTriggerOrderingDecision(decision);
  const showStripDecisionSummary = (
    decision?.kind === "targets"
    && !showViewedCardsStep
    && !triggerOrderingDecision
  );
  const toolbarDecisionSummary = useMemo(() => {
    const parts = [
      decision?.description,
      decision?.context_text,
    ]
      .map((value) => normalizeDecisionText(value))
      .filter(Boolean);
    return parts[0] || "";
  }, [decision?.context_text, decision?.description]);
  const viewedCardEntries = useMemo(
    () => {
      if (Array.isArray(viewedCards?.cards) && viewedCards.cards.length > 0) {
        return viewedCards.cards.map((card) => ({
          id: String(card.id),
          name: card.name || `Card #${card.id}`,
          controller: viewedCards?.subject,
        }));
      }
      return (viewedCards?.card_ids || []).map((id) => ({
        id: String(id),
        name: objectNameById.get(String(id)) || `Card #${id}`,
        controller: viewedCards?.subject,
      }));
    },
    [objectNameById, viewedCards]
  );
  const viewedCardsSourceName = (() => {
    if (viewedCards?.source != null) {
      const sourceName = objectNameById.get(String(viewedCards.source));
      if (sourceName) return sourceName;
    }
    return decision?.source_name || "";
  })();
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
  const visibleActionGroups = useMemo(() => {
    if (selectedObjectId == null && !selectedObjectNameLower) return actionGroups;
    return actionGroups.filter((group) => {
      for (const linkedObjectId of group.linkedObjectIds) {
        if (selectedObjectFamilyIds.has(linkedObjectId)) return true;
      }
      for (const actionIndex of group.actionIndices) {
        if (selectedActionIndices.has(actionIndex)) return true;
      }
      return false;
    });
  }, [actionGroups, selectedActionIndices, selectedObjectFamilyIds, selectedObjectId, selectedObjectNameLower]);
  const priorityActionCount = visibleActionGroups.length;
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
  const handleViewedCardHoverStart = useCallback((card) => {
    if (!card?.id) return;
    clearHoverLinkedObjects();
    hoverCard(card.id);
  }, [clearHoverLinkedObjects, hoverCard]);
  const handleViewedCardHoverEnd = useCallback(() => {
    clearHoverLinkedObjects();
    clearHover();
  }, [clearHoverLinkedObjects, clearHover]);
  const [submitState, setSubmitState] = useState({ key: "", action: null });
  const handleSubmitActionChange = useCallback(
    (nextAction) => {
      setSubmitState({ key: decisionIdentity, action: nextAction || null });
    },
    [decisionIdentity]
  );
  const submitAction = submitState.key === decisionIdentity ? submitState.action : null;
  const triggerOrderingSubmitAction = useMemo(() => {
    if (!triggerOrderingDecision) return null;
    const order = triggerOrderingState?.order?.length
      ? normalizeTriggerOrderingOrder(triggerOrderingState.order, decision)
      : defaultTriggerOrderingOrder(decision);
    return {
      label: "Submit Order",
      disabled: !canAct,
      onSubmit: () => {
        clearHover();
        dispatch({ type: "select_options", option_indices: order }, "Order submitted");
      },
    };
  }, [canAct, clearHover, decision, dispatch, triggerOrderingDecision, triggerOrderingState]);
  const effectiveSubmitAction = triggerOrderingSubmitAction || submitAction;
  const canSubmitFocused = canAct
    && !!effectiveSubmitAction
    && !effectiveSubmitAction.disabled
    && typeof effectiveSubmitAction.onSubmit === "function";
  const canAdvanceViewedCardsStep = !!decision;
  const compactLandscapeViewport = typeof window !== "undefined"
    && window.matchMedia("(max-height: 480px) and (orientation: landscape)").matches;
  const completeViewedCardsStep = useCallback(() => {
    if (!viewedCardsToken) return;
    setAcknowledgedViewedCardsToken(viewedCardsToken);
  }, [viewedCardsToken]);

  const updateManaTabAnchorRect = useCallback(() => {
    if (!inline || !inlineRootRef.current) {
      setManaTabAnchorRect(null);
      return;
    }
    const rect = inlineRootRef.current.getBoundingClientRect();
    setManaTabAnchorRect((current) => {
      if (
        current
        && current.left === rect.left
        && current.top === rect.top
        && current.width === rect.width
      ) {
        return current;
      }
      return {
        left: rect.left,
        top: rect.top,
        width: rect.width,
      };
    });
  }, [inline]);

  useLayoutEffect(() => {
    if (!inline) return undefined;

    const node = inlineRootRef.current;
    if (!node || typeof window === "undefined") {
      return undefined;
    }

    let frame = 0;
    const scheduleUpdate = () => {
      if (frame) cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        updateManaTabAnchorRect();
        frame = 0;
      });
    };

    scheduleUpdate();
    const resizeObserver = typeof ResizeObserver !== "undefined"
      ? new ResizeObserver(scheduleUpdate)
      : null;
    resizeObserver?.observe(node);
    window.addEventListener("resize", scheduleUpdate);
    window.addEventListener("scroll", scheduleUpdate, true);

    return () => {
      if (frame) cancelAnimationFrame(frame);
      resizeObserver?.disconnect();
      window.removeEventListener("resize", scheduleUpdate);
      window.removeEventListener("scroll", scheduleUpdate, true);
    };
  }, [inline, updateManaTabAnchorRect]);

  if (!decision || isCombatDecision) return null;
  if (isPriorityDecision && !passAction) return null;

  if (inline) {
    return (
      <div
        ref={inlineRootRef}
        className={cn(
          "pointer-events-none absolute inset-0 z-[120] flex items-start pt-0.5",
          compactLandscapeViewport ? "px-0" : "px-2"
        )}
      >
        <ManaPaymentTab manaPayment={manaPayment} anchorRect={inline ? manaTabAnchorRect : null} />
        <div
          className={cn(
            "priority-inline-panel pointer-events-auto relative flex h-full w-full flex-col py-0",
            compactLandscapeViewport ? "px-0" : "px-2"
          )}
        >
          {isPriorityDecision ? (
            showViewedCardsStep ? (
              <div className="action-strip-layout flex min-h-[46px] items-stretch gap-2">
                <Button
                  variant="ghost"
                  size="sm"
                  className="decision-neon-button decision-submit-button h-full w-[176px] shrink-0 self-stretch rounded-none px-3 text-[14px] font-bold uppercase"
                  disabled={!canAdvanceViewedCardsStep}
                  onPointerDown={(event) => {
                    if (!canAdvanceViewedCardsStep || event.button !== 0) return;
                    event.preventDefault();
                    completeViewedCardsStep();
                  }}
                  onClick={(event) => {
                    if (!canAdvanceViewedCardsStep || event.detail !== 0) return;
                    completeViewedCardsStep();
                  }}
                >
                  Done
                </Button>
                <ViewedCardsStrip
                  label={viewedCardsLabel}
                  description={viewedCards?.description || ""}
                  sourceName={viewedCardsSourceName}
                  cards={viewedCardEntries}
                  players={state?.players || []}
                  perspective={state?.perspective}
                  objectControllerById={objectControllerById}
                  hoveredObjectId={hoveredObjectId}
                  selectedObjectId={selectedObjectId}
                  onCardHoverStart={handleViewedCardHoverStart}
                  onCardHoverEnd={handleViewedCardHoverEnd}
                  compact
                />
              </div>
            ) : (
              <div className="action-strip-layout flex min-h-[46px] items-stretch gap-2">
                {showPriorityAdvanceButton && (
                  <Button
                    variant="ghost"
                    size="sm"
                    className="pass-priority-btn action-strip-advance-button h-full w-[176px] shrink-0 self-stretch rounded-none px-3 text-[14px] font-bold uppercase"
                    disabled={!canAct}
                    onClick={() => triggerPriorityAction(passAction)}
                  >
                    {passLabel}
                  </Button>
                )}
                <PriorityActionStrip
                  groups={visibleActionGroups}
                  canAct={canAct}
                  players={state?.players || []}
                  perspective={state?.perspective}
                  hasPinnedSelection={selectedObjectId != null}
                  objectNameById={objectNameById}
                  objectControllerById={objectControllerById}
                  hoveredObjectFamilyIds={hoveredObjectFamilyIds}
                  selectedObjectFamilyIds={selectedObjectFamilyIds}
                  selectedActionIndices={selectedActionIndices}
                  onActionClick={triggerPriorityAction}
                  onActionHoverStart={handleActionHoverStart}
                  onActionHoverEnd={handleActionHoverEnd}
                />
                <PriorityControlStack
                  actionCount={priorityActionCount}
                  holdEnabled={holdRule === "always"}
                  confirmEnabled={confirmEnabled}
                  onHoldChange={(value) => setHoldRule(value ? "always" : "never")}
                  onConfirmChange={setConfirmEnabled}
                  showActionCount={priorityActionCount > 0}
                  className="ml-auto min-w-[104px]"
                />
              </div>
            )
          ) : (
            <div className="action-strip-decision-stack flex min-h-0 min-w-0 flex-1 flex-col gap-1.5 py-1">
              <div className="action-strip-decision-toolbar flex min-w-0 items-stretch gap-2">
                <div className="flex min-w-0 flex-1 items-stretch gap-2">
                  <div className="flex min-w-0 max-w-[320px] shrink-0 items-stretch gap-2">
                      <Button
                        variant="ghost"
                        size="sm"
                        className="decision-neon-button decision-submit-button h-full min-w-[104px] flex-[1.2_1_0] self-stretch rounded-none px-3 text-[clamp(11px,0.88vw,14px)] font-bold uppercase"
                        disabled={showViewedCardsStep ? !canAdvanceViewedCardsStep : !canSubmitFocused}
                        onPointerDown={(event) => {
                          if (showViewedCardsStep) {
                            if (!canAdvanceViewedCardsStep || event.button !== 0) return;
                            event.preventDefault();
                            completeViewedCardsStep();
                            return;
                          }
                          if (!canSubmitFocused || event.button !== 0) return;
                          event.preventDefault();
                          effectiveSubmitAction.onSubmit();
                        }}
                        onClick={(event) => {
                          if (showViewedCardsStep) {
                            if (!canAdvanceViewedCardsStep || event.detail !== 0) return;
                            completeViewedCardsStep();
                            return;
                          }
                          if (!canSubmitFocused || event.detail !== 0) return;
                          effectiveSubmitAction.onSubmit();
                        }}
                      >
                        {showViewedCardsStep ? "Done" : (effectiveSubmitAction?.label || "Submit")}
                      </Button>
                      <Button
                        type="button"
                        variant="ghost"
                        size="sm"
                        className="decision-neon-button decision-neon-button--danger decision-cancel-button h-full min-w-[82px] flex-[0.75_1_0] self-stretch rounded-none px-2 text-[clamp(10px,0.82vw,13px)] font-bold uppercase tracking-wide"
                        disabled={!canCancelDecision}
                        onPointerDown={(event) => {
                          if (!canCancelDecision || event.button !== 0) return;
                          event.preventDefault();
                          cancelDecision();
                        }}
                        onClick={(event) => {
                          if (!canCancelDecision || event.detail !== 0) return;
                          cancelDecision();
                        }}
                      >
                        Cancel
                      </Button>
                  </div>
                  {!triggerOrderingDecision && (
                    <div className="action-strip-decision-meta flex min-w-0 flex-1 flex-col justify-center px-1">
                      <div className="flex min-w-0 items-baseline gap-2">
                        <div className="action-strip-decision-title text-[11px] font-bold uppercase tracking-[0.14em]">
                          {resolveDecisionTitle(decision)}
                        </div>
                        {toolbarDecisionSummary && (
                          <div className="action-strip-decision-inline-summary truncate text-[11px]">
                            {toolbarDecisionSummary}
                          </div>
                        )}
                      </div>
                      {!toolbarDecisionSummary && decision?.source_name && (
                        <div className="action-strip-decision-source truncate text-[11px]">
                          {normalizeDecisionText(decision.source_name)}
                        </div>
                      )}
                    </div>
                  )}
                </div>
                <PriorityControlStack
                  holdEnabled={holdRule === "always"}
                  confirmEnabled={confirmEnabled}
                  onHoldChange={(value) => setHoldRule(value ? "always" : "never")}
                  onConfirmChange={setConfirmEnabled}
                  showActionCount={false}
                  className="ml-auto min-w-[104px]"
                />
              </div>
              <div className="action-strip-decision-content min-w-0 flex-1 overflow-hidden">
                {canAct ? (
                  showViewedCardsStep ? (
                    <ViewedCardsStrip
                      label={viewedCardsLabel}
                      description={viewedCards?.description || ""}
                      sourceName={viewedCardsSourceName}
                      cards={viewedCardEntries}
                      players={state?.players || []}
                      perspective={state?.perspective}
                      objectControllerById={objectControllerById}
                      hoveredObjectId={hoveredObjectId}
                      selectedObjectId={selectedObjectId}
                      onCardHoverStart={handleViewedCardHoverStart}
                      onCardHoverEnd={handleViewedCardHoverEnd}
                      compact
                    />
                  ) : (!triggerOrderingDecision && (
                    <DecisionRouter
                      decision={decision}
                      canAct={canAct}
                      selectedObjectId={selectedObjectId}
                      inlineSubmit={false}
                      onSubmitActionChange={handleSubmitActionChange}
                      hideDescription
                      layout="strip"
                      showStripSummary={false}
                    />
                  ))
                ) : (
                  <span className="action-strip-waiting text-[12px] whitespace-nowrap">
                    Waiting for opponent
                  </span>
                )}
              </div>
            </div>
          )}
        </div>
      </div>
    );
  }

  return (
    <div
      className={cn(
        "pointer-events-none relative z-[120]",
        anchoredStyle
          ? "fixed"
          : "fixed left-2 bottom-[148px] w-[min(92vw,348px)]"
      )}
      style={anchoredStyle || undefined}
    >
      <ManaPaymentTab manaPayment={manaPayment} />
      <div className={cn(
        "priority-inline-panel pointer-events-auto relative py-0",
        compactLandscapeViewport ? "px-0" : "px-2"
      )}>
        <div className="action-strip-layout flex min-h-[46px] items-start gap-2">
          {isPriorityDecision ? (
            showViewedCardsStep ? (
              <Button
                variant="ghost"
                size="sm"
                className="decision-neon-button decision-submit-button h-full w-[176px] shrink-0 self-stretch rounded-none px-3 text-[14px] font-bold uppercase"
                disabled={!canAdvanceViewedCardsStep}
                onPointerDown={(event) => {
                  if (!canAdvanceViewedCardsStep || event.button !== 0) return;
                  event.preventDefault();
                  completeViewedCardsStep();
                }}
                onClick={(event) => {
                  if (!canAdvanceViewedCardsStep || event.detail !== 0) return;
                  completeViewedCardsStep();
                }}
              >
                Done
              </Button>
            ) : (
              <>
                {showPriorityAdvanceButton && (
                  <Button
                    variant="ghost"
                    size="sm"
                    className="pass-priority-btn action-strip-advance-button h-full w-[176px] shrink-0 self-stretch rounded-none px-3 text-[14px] font-bold uppercase"
                    disabled={!canAct}
                    onClick={() => triggerPriorityAction(passAction)}
                  >
                    {passLabel}
                  </Button>
                )}
              </>
            )
          ) : (
            <>
              <div className="action-strip-decision-stack flex min-w-0 w-full flex-col gap-y-1">
                <div className="action-strip-decision-toolbar flex min-h-[46px] items-stretch gap-2">
                  <Button
                    variant="ghost"
                    size="sm"
                    className="decision-neon-button decision-submit-button h-full min-w-[104px] flex-[1.2_1_0] self-stretch rounded-none px-3 text-[clamp(11px,0.88vw,14px)] font-bold uppercase"
                    disabled={showViewedCardsStep ? !canAdvanceViewedCardsStep : !canSubmitFocused}
                    onPointerDown={(event) => {
                      if (showViewedCardsStep) {
                        if (!canAdvanceViewedCardsStep || event.button !== 0) return;
                        event.preventDefault();
                        completeViewedCardsStep();
                        return;
                      }
                      if (!canSubmitFocused || event.button !== 0) return;
                      event.preventDefault();
                      effectiveSubmitAction.onSubmit();
                    }}
                    onClick={(event) => {
                      if (showViewedCardsStep) {
                        if (!canAdvanceViewedCardsStep || event.detail !== 0) return;
                        completeViewedCardsStep();
                        return;
                      }
                      if (!canSubmitFocused || event.detail !== 0) return;
                      effectiveSubmitAction.onSubmit();
                    }}
                  >
                    {showViewedCardsStep ? "Done" : (effectiveSubmitAction?.label || "Submit")}
                  </Button>
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    className="decision-neon-button decision-neon-button--danger decision-cancel-button h-full min-w-[82px] flex-[0.75_1_0] self-stretch rounded-none px-2 text-[clamp(10px,0.82vw,13px)] font-bold uppercase tracking-wide"
                    disabled={!canCancelDecision}
                    onPointerDown={(event) => {
                      if (!canCancelDecision || event.button !== 0) return;
                        event.preventDefault();
                        cancelDecision();
                    }}
                    onClick={(event) => {
                      if (!canCancelDecision || event.detail !== 0) return;
                      cancelDecision();
                    }}
                  >
                    Cancel
                  </Button>
                  <PriorityControlStack
                    holdEnabled={holdRule === "always"}
                    confirmEnabled={confirmEnabled}
                    onHoldChange={(value) => setHoldRule(value ? "always" : "never")}
                    onConfirmChange={setConfirmEnabled}
                    showActionCount={false}
                    className="ml-auto min-w-[104px]"
                  />
                </div>
              </div>
              {!triggerOrderingDecision && (
                <div className="action-strip-decision-meta self-stretch flex min-w-0 flex-col justify-center py-1.5">
                  <div className="action-strip-decision-title truncate text-[11px] font-bold uppercase tracking-[0.14em]">
                    {resolveDecisionTitle(decision)}
                  </div>
                  {decision?.source_name && (
                    <div className="action-strip-decision-source mt-0.5 truncate text-[11px]">
                      {normalizeDecisionText(decision.source_name)}
                    </div>
                  )}
                </div>
              )}
            </>
          )}
        </div>
      </div>
      <div className={cn("action-strip-body-shell pointer-events-auto flex-1 border-b px-2 py-1.5", !isPriorityDecision && ACTION_STRIP_BODY_CLASS)}>
        {isPriorityDecision ? (
          showViewedCardsStep ? (
            <ViewedCardsStrip
              label={viewedCardsLabel}
              description={viewedCards?.description || ""}
              sourceName={viewedCardsSourceName}
              cards={viewedCardEntries}
              players={state?.players || []}
              perspective={state?.perspective}
              objectControllerById={objectControllerById}
              hoveredObjectId={hoveredObjectId}
              selectedObjectId={selectedObjectId}
              onCardHoverStart={handleViewedCardHoverStart}
              onCardHoverEnd={handleViewedCardHoverEnd}
              compact
            />
          ) : (
            <div className="flex min-h-[46px] items-stretch gap-2">
              <PriorityActionStrip
                groups={visibleActionGroups}
                canAct={canAct}
                players={state?.players || []}
                perspective={state?.perspective}
                hasPinnedSelection={selectedObjectId != null}
                objectNameById={objectNameById}
                objectControllerById={objectControllerById}
                hoveredObjectFamilyIds={hoveredObjectFamilyIds}
                selectedObjectFamilyIds={selectedObjectFamilyIds}
                selectedActionIndices={selectedActionIndices}
                onActionClick={triggerPriorityAction}
                onActionHoverStart={handleActionHoverStart}
                onActionHoverEnd={handleActionHoverEnd}
              />
              <PriorityControlStack
                actionCount={priorityActionCount}
                holdEnabled={holdRule === "always"}
                confirmEnabled={confirmEnabled}
                onHoldChange={(value) => setHoldRule(value ? "always" : "never")}
                onConfirmChange={setConfirmEnabled}
                className="ml-auto min-w-[104px]"
              />
            </div>
          )
        ) : (
          <div className="action-strip-decision-content min-w-0 h-full">
            {showViewedCardsStep ? (
              <ViewedCardsStrip
                label={viewedCardsLabel}
                description={viewedCards?.description || ""}
                sourceName={viewedCardsSourceName}
                cards={viewedCardEntries}
                players={state?.players || []}
                perspective={state?.perspective}
                objectControllerById={objectControllerById}
                hoveredObjectId={hoveredObjectId}
                selectedObjectId={selectedObjectId}
                onCardHoverStart={handleViewedCardHoverStart}
                onCardHoverEnd={handleViewedCardHoverEnd}
                compact
              />
            ) : (!triggerOrderingDecision && (
              <DecisionRouter
                decision={decision}
                canAct={canAct}
                selectedObjectId={selectedObjectId}
                inlineSubmit={false}
                onSubmitActionChange={handleSubmitActionChange}
                hideDescription={false}
                layout="strip"
                showStripSummary={!showStripDecisionSummary}
              />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

function CombatBar({ anchor = null, inline = false, decision, canAct }) {
  const {
    state,
    holdRule,
    setHoldRule,
    confirmEnabled,
    setConfirmEnabled,
    cancelDecision,
  } = useGame();
  const decisionIdentity = [
    decision?.kind || "",
    decision?.player ?? "",
    decision?.source_id ?? "",
    decision?.source_name || "",
    decision?.reason || "",
    decision?.description || "",
    decision?.context_text || "",
    decision?.consequence_text || "",
  ].join("|");
  const [combatActionState, setCombatActionState] = useState({ key: "", action: null });
  const handleCombatActionChange = useCallback(
    (nextAction) => {
      setCombatActionState({ key: decisionIdentity, action: nextAction || null });
    },
    [decisionIdentity]
  );
  if (!decision || (decision.kind !== "attackers" && decision.kind !== "blockers")) return null;

  const anchoredStyle = inline ? null : priorityAnchorStyle(anchor);
  const compactPortraitViewport = typeof window !== "undefined"
    && window.matchMedia("(max-width: 720px) and (orientation: portrait)").matches;
  const combatAction = combatActionState.key === decisionIdentity ? combatActionState.action : null;
  const canCancelDecision = canAct && !!state?.cancelable;
  const canSubmitCombat = canAct
    && !!combatAction
    && !combatAction.disabled
    && typeof combatAction.onSubmit === "function";
  const panelClass = inline
    ? "pointer-events-none absolute inset-0 z-[120] flex items-center px-2"
    : "pointer-events-none fixed left-2 bottom-[148px] z-[120] w-[min(96vw,740px)]";

  const innerClass = cn(
    "priority-inline-panel pointer-events-auto flex w-full items-center gap-2 px-2 py-1.5",
    compactPortraitViewport && "flex-col items-stretch",
    !inline && anchoredStyle ? "fixed" : ""
  );

  return (
    <div className={panelClass}>
      <div className={innerClass} style={anchoredStyle || undefined}>
        <div className={cn("min-w-0 flex-1", compactPortraitViewport && "w-full")}>
          <div className={cn("flex min-h-[46px] items-stretch gap-2", compactPortraitViewport && "flex-col")}>
            <Button
              variant="ghost"
              size="sm"
              className="decision-neon-button decision-submit-button h-full min-w-[140px] flex-[1.2_1_0] self-stretch rounded-none px-3 text-[clamp(11px,0.88vw,14px)] font-bold uppercase"
              disabled={!canSubmitCombat}
              onClick={() => combatAction?.onSubmit?.()}
            >
              {combatAction?.label || (
                decision.kind === "attackers" ? "Confirm Attackers (0)" : "Confirm Blockers (0)"
              )}
            </Button>
            {canCancelDecision ? (
              <Button
                type="button"
                variant="ghost"
                size="sm"
                className="decision-neon-button decision-neon-button--danger decision-cancel-button h-full min-w-[96px] flex-[0.75_1_0] self-stretch rounded-none px-2 text-[clamp(10px,0.82vw,13px)] font-bold uppercase tracking-wide"
                disabled={!canCancelDecision}
                onClick={() => cancelDecision()}
              >
                Cancel
              </Button>
            ) : null}
          </div>
          <DecisionRouter
            decision={decision}
            canAct={canAct}
            combatInline
            onCombatActionChange={handleCombatActionChange}
          />
        </div>
        <PriorityControlStack
          holdEnabled={holdRule === "always"}
          confirmEnabled={confirmEnabled}
          onHoldChange={(value) => setHoldRule(value ? "always" : "never")}
          onConfirmChange={setConfirmEnabled}
          showActionCount={false}
          className={cn(
            "min-w-[104px]",
            compactPortraitViewport && "min-w-0 w-full"
          )}
        />
      </div>
    </div>
  );
}

export default function DecisionPopupLayer({
  anchor = null,
  priorityInline = false,
  selectedObjectId = null,
  mobileBattle = false,
  mobileBattlePortalTarget = null,
  mobileBattleDockInline = false,
  mobileBattleDockHidden = false,
}) {
  const { state } = useGame();
  const decision = state?.decision || null;
  const canAct = !!decision && state?.perspective === decision.player;

  if (!decision) return null;
  if (mobileBattle) {
    return (
      <MobileBattleDecisionLayer
        selectedObjectId={selectedObjectId}
        portalTarget={mobileBattlePortalTarget}
        dockInline={mobileBattleDockInline}
        dockHidden={mobileBattleDockHidden}
      />
    );
  }
  if (decision?.kind === "priority") {
    return <PriorityBar anchor={anchor} inline={priorityInline} selectedObjectId={selectedObjectId} />;
  }
  if (decision?.kind === "attackers" || decision?.kind === "blockers") {
    return <CombatBar anchor={anchor} inline={priorityInline} decision={decision} canAct={canAct} />;
  }
  return <PriorityBar anchor={anchor} inline={priorityInline} selectedObjectId={selectedObjectId} />;
}
