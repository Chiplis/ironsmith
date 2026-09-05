import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { useGame } from "@/context/GameContext";
import { useHover } from "@/context/HoverContext";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import DecisionRouter from "@/components/decisions/DecisionRouter";
import DecisionSummary from "@/components/decisions/DecisionSummary";
import PeerWaitPopover, { PeerWaitButtonContent } from "@/components/decisions/PeerWaitPopover";
import useDeferredPeerWait from "@/hooks/useDeferredPeerWait";
import { normalizeDecisionText } from "@/components/decisions/decisionText";
import { animate, cancelMotion, snappySpring, stagger } from "@/lib/motion/anime";
import { KeywordHelpersProvider, ManaSymbol, SymbolText } from "@/lib/mana-symbols";
import { nextPriorityAdvanceLabel } from "@/lib/constants";
import HighlightedDecisionText from "@/components/decisions/HighlightedDecisionText";
import { decisionOptionAccentVars, getPlayerAccent } from "@/lib/player-colors";
import { useDecisionButtonAccent } from "@/lib/decision-button-style";
import useDeclareAttackersButtonTransition from "@/hooks/useDeclareAttackersButtonTransition";
import {
  collectSelectedPriorityActionIndices,
  filterPriorityActionGroups,
  withoutManaAbilityActionGroups,
} from "@/lib/priority-action-filter";
import {
  buildBattlefieldFamilies,
  buildPriorityActionGroups,
} from "@/lib/priority-action-groups";
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
import { playerDisplayName, samePlayerId } from "@/lib/player-display";
import { LoaderCircle, X } from "lucide-react";

const ACTION_STRIP_BODY_CLASS = "min-h-0 h-full";
const MANA_PAYMENT_TAB_EXIT_MS = 320;
const PRIORITY_ACTION_PRIMARY_CYCLE = 0;

function clamp(value, min, max) {
  return Math.min(max, Math.max(min, value));
}

function safeInlineLabel(value, fallback = "") {
  if (value == null || value === false) return fallback;
  if (typeof value === "string" || typeof value === "number" || typeof value === "boolean") {
    return String(value);
  }
  if (typeof value === "object") {
    return String(value.label || value.name || value.description || fallback || "");
  }
  return String(value);
}

function DecisionCardNameTrigger({ objectId, onInspect, children, className = "" }) {
  if (objectId == null || typeof onInspect !== "function") return children;
  return (
    <span
      className={cn("decision-card-name-trigger", className)}
      data-inspector-object-id={String(objectId)}
      role="button"
      tabIndex={0}
      aria-label={`Inspect ${String(children || "card")}`}
      onPointerDown={(event) => {
        event.stopPropagation();
      }}
      onPointerUp={(event) => {
        if (event.button !== 0) return;
        event.stopPropagation();
        onInspect(objectId, event.currentTarget);
      }}
      onClick={(event) => {
        event.stopPropagation();
        if (event.detail !== 0) return;
        onInspect(objectId, event.currentTarget);
      }}
      onKeyDown={(event) => {
        if (event.key !== "Enter" && event.key !== " ") return;
        event.preventDefault();
        event.stopPropagation();
        onInspect(objectId, event.currentTarget);
      }}
    >
      {children}
    </span>
  );
}

function conciseDecisionSummary(value) {
  const normalized = normalizeDecisionText(value);
  if (typeof normalized !== "string") return "";
  const clauses = normalized.split(/\s*;\s*/).filter(Boolean);
  const targetClause = clauses.find((clause) => /\btarget\b/i.test(clause));
  const selectedClause = String(targetClause || clauses[0] || "");
  const colonIndex = selectedClause.indexOf(":");
  const effectClause = colonIndex >= 0 ? selectedClause.slice(colonIndex + 1).trim() : "";
  const summary = effectClause && /\btarget\b/i.test(effectClause) ? effectClause : selectedClause;
  return summary
    .replace(/^(?:choose|select)\s+(?:the\s+)?targets?\s*:\s*/i, "")
    .trim();
}

function decisionStageLabel(decision) {
  switch (decision?.kind) {
    case "targets": return "Target";
    case "select_objects": return "Select";
    case "select_options": return "Choose";
    case "number": return "Number";
    case "mana_payment": return "Payment";
    case "attackers":
    case "blockers": return "Combat";
    default: return "Action";
  }
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

function buildManaPaymentGroups(payment) {
  const pips = Array.isArray(payment?.pips) ? payment.pips : [];
  const groups = [];

  for (let index = 0; index < pips.length; index += 1) {
    const pip = pips[index];

    if (isSingleGenericPip(pip)) {
      let count = 1;
      while (index + count < pips.length && isSingleGenericPip(pips[index + count])) {
        count += 1;
      }

      groups.push({
        key: `generic-${index}`,
        start: index,
        end: index + count,
        kind: "generic",
        displayCount: count,
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
        className="mana-payment-shell relative overflow-visible rounded-none border px-2.5 py-1.5"
        >
          <div className="mana-payment-shell-glow absolute inset-0" />
          <div className="absolute inset-x-0 top-0 h-px bg-[linear-gradient(90deg,transparent,rgba(255,220,176,0.85),transparent)]" />
          <div className="mana-payment-tail absolute left-1/2 top-full h-3.5 w-14 -translate-x-1/2 -translate-y-px overflow-hidden rounded-none border-x border-b" />
          <div className="mana-payment-track relative rounded-none border px-1.5 py-0.5">
            <div className="relative flex items-center gap-1.5">
              {groups.map((group) => (
                  <span
                    key={group.key}
                    className="mana-payment-group relative inline-flex min-w-[28px] items-center justify-center rounded-none px-1 py-0.5 opacity-100"
                  >
                    {group.kind === "generic" ? (
                      <ManaSymbol sym={String(group.displayCount)} size={18} />
                    ) : (
                      <ManaSymbol sym={group.displayCode} size={18} />
                    )}
                  </span>
              ))}
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

function dispatchHandActionHover(objectId = null) {
  if (typeof window === "undefined") return;
  window.dispatchEvent(new CustomEvent("ironsmith:hand-action-hover", {
    detail: { objectId: objectId != null ? String(objectId) : null },
  }));
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

function resolveObjectAccent(
  players,
  perspective,
  controllerById,
  objectId,
  explicitControllerId = null,
  accentOverrides = null
) {
  const controllerId = explicitControllerId != null
    ? Number(explicitControllerId)
    : controllerById.get(String(objectId));
  if (controllerId == null || Number(controllerId) === Number(perspective)) {
    return null;
  }
  return getPlayerAccent(players || [], controllerId, perspective, accentOverrides);
}

function PriorityActionPillLabel({
  text,
  viewportRef,
  carouselResetVersion = 0,
  isHovered = false,
  highlightText = "",
  highlightColor = null,
  onHighlightClick = null,
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

  const shouldAnimate = isHovered && isOverflowing && isVisible;

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
      delay: 0,
      loop: true,
    });
    marqueeAnimationRef.current = animation;

    return () => {
      cancelMotion(animation);
      marqueeEl.style.transform = "translateX(0px)";
    };
  }, [carouselResetVersion, shouldAnimate, travelDistance, travelDuration]);

  useEffect(() => {
    if (shouldAnimate) return;
    const marqueeEl = marqueeRef.current;
    if (!marqueeEl) return;
    marqueeEl.style.transform = "translateX(0px)";
  }, [shouldAnimate]);

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
            onHighlightClick={onHighlightClick}
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
      <span aria-hidden="true" className="invisible block min-w-0 overflow-hidden whitespace-nowrap" style={{ textOverflow: "clip" }}>
        <HighlightedDecisionText
          text={displayText}
          highlightText={highlightText}
          highlightColor={highlightColor}
        />
      </span>
      <span
        ref={marqueeRef}
        className="absolute left-0 top-0 inline-flex whitespace-nowrap will-change-transform"
      >
        <span className="pr-7">
          <HighlightedDecisionText
            text={displayText}
            highlightText={highlightText}
            highlightColor={highlightColor}
            onHighlightClick={onHighlightClick}
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
  decisionPlayer,
  className = "",
  hasPinnedSelection = false,
  objectNameById,
  objectControllerById,
  hoveredObjectFamilyIds,
  selectedObjectFamilyIds,
  selectedActionIndices,
  onActionClick,
  onActionHoverStart,
  onActionHoverEnd,
  onActionCardInspect,
  accentOverrides = null,
}) {
  const { playerAccentOverrides: contextAccentOverrides } = useGame();
  const effectiveAccentOverrides = accentOverrides || contextAccentOverrides;
  const viewportRef = useRef(null);
  const groupNodeRefs = useRef(new Map());
  const displayNodeRefs = useRef(new Map());
  const previousHoveredGroupKeysRef = useRef(new Set());
  const previousSelectedGroupKeysRef = useRef(new Set());
  const stripMotionRef = useRef(null);
  const [carouselResetByGroupKey, setCarouselResetByGroupKey] = useState({});
  const [isPointerInStrip, setIsPointerInStrip] = useState(false);
  const [hoveredPillKey, setHoveredPillKey] = useState(null);
  const { attachScrollableRef, hoverSuppressed } = useHoverSuppressedWhileScrolling({
    onScrollStart: onActionHoverEnd,
  });
  const compactLandscapeViewport = typeof window !== "undefined"
    && window.matchMedia("(max-width: 720px) and (orientation: landscape)").matches;
  const groupKeysSignature = useMemo(
    () => groups.map((group) => group.key).join("|"),
    [groups]
  );
  const displayGroups = useMemo(
    () => groups.map((group) => ({
      cycle: PRIORITY_ACTION_PRIMARY_CYCLE,
      group,
      key: group.key,
    })),
    [groups]
  );

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
  }, [groupKeysSignature]);

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
  }, [displayGroups, groupKeysSignature]);

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
        .map((key) => groupNodeRefs.current.get(key)?.[PRIORITY_ACTION_PRIMARY_CYCLE] || null)
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
  }, [groupKeysSignature, hasPinnedSelection, hoveredGroupKeys, isPointerInStrip, selectedGroupKeys]);

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
      <div className={cn("action-strip-empty-state action-strip-empty-state--waiting flex min-w-0 flex-1 items-center px-3 text-[12px] whitespace-nowrap", className)}>
        Waiting for {playerDisplayName(players, decisionPlayer)}
      </div>
    );
  }

  if (!groups.length) {
    return (
      <div className={cn("action-strip-empty-state action-strip-empty-state--empty flex min-w-0 flex-1 items-center px-3 text-[12px] whitespace-nowrap", className)}>
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
      className={cn("action-strip-scroll min-w-0 flex-1 overflow-x-auto overflow-y-hidden whitespace-nowrap", className)}
      onMouseEnter={() => setIsPointerInStrip(true)}
      onMouseLeave={() => {
        setIsPointerInStrip(false);
        setHoveredPillKey(null);
      }}
    >
      <div className="decision-strip-options-row flex w-max min-w-full min-h-[32px] items-stretch gap-1.5 pr-2">
        {displayGroups.map(({ key, cycle, group }) => {
          const isPrimaryCycle = cycle === PRIORITY_ACTION_PRIMARY_CYCLE;
          const linkedActive = isGroupHoveredLinked(group) || isGroupSelectedLinked(group);
          const highlightName = group.hoverObjectId != null
            ? objectNameById.get(String(group.hoverObjectId)) || ""
            : "";
          const accent = resolveObjectAccent(
            players,
            perspective,
            objectControllerById,
            group.hoverObjectId,
            null,
            effectiveAccentOverrides
          ) || getPlayerAccent(
            players || [],
            perspective,
            perspective,
            effectiveAccentOverrides,
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
            linkedActive && "is-linked-active",
            "text-[#d8ccb4]",
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
                isHovered={hoveredPillKey === key}
                highlightText={highlightName}
                onHighlightClick={group.hoverObjectId != null
                  ? (event) => onActionCardInspect?.(group.hoverObjectId, event.currentTarget)
                  : null}
              />
            </>
          );

          return (
            <button
              key={key}
              type="button"
              data-local-action={canAct && isPrimaryCycle ? "true" : "false"}
              aria-disabled={!canAct || !isPrimaryCycle}
              aria-hidden={isPrimaryCycle ? undefined : true}
              tabIndex={isPrimaryCycle ? undefined : -1}
              ref={setNodeRef}
              className={pillClassName}
              style={{
                textOverflow: "clip",
                ...decisionOptionAccentVars(accent),
              }}
              onPointerDown={(event) => {
                if (event.button !== 0) return;
                if (event.pointerType && event.pointerType !== "mouse") return;
                // Match decision option buttons so a pointer sequence that
                // started on a payment control cannot finish as a click on a newly
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
                setHoveredPillKey(key);
                onActionHoverStart(group);
              }}
              onMouseLeave={() => {
                setHoveredPillKey((currentKey) => (currentKey === key ? null : currentKey));
                onActionHoverEnd();
              }}
              onPointerLeave={() => {
                setHoveredPillKey((currentKey) => (currentKey === key ? null : currentKey));
              }}
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
    case "mana_payment":
      return "Pay Mana";
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

function isInspectorOnlyViewedCards(viewedCards) {
  return Boolean(viewedCards?.inspector_only || viewedCards?.inspectorOnly);
}

function isHiddenCardName(name) {
  return String(name || "").trim().toLowerCase() === "hidden card";
}

function viewedCardDisplayName(card, objectNameById) {
  const objectName = objectNameById.get(String(card?.id));
  if (objectName && !isHiddenCardName(objectName)) return objectName;
  return card?.name || `Card #${card?.id}`;
}

function buildPeerWaitOpeningPreviewEntries(peerWait) {
  const previews = Array.isArray(peerWait?.openingPreviews)
    ? peerWait.openingPreviews
    : [];
  return previews
    .map((preview, index) => {
      const objectId = Number(preview?.objectId ?? preview?.object_id);
      const stableId = Number(preview?.stableId ?? preview?.stable_id);
      const slot = Number(preview?.slot);
      const position = Number(preview?.position);
      const id = Number.isSafeInteger(objectId) && objectId >= 0
        ? objectId
        : Number.isSafeInteger(stableId) && stableId >= 0
          ? `stable-${stableId}`
          : Number.isSafeInteger(slot) && slot >= 0
            ? `slot-${Number(preview.owner)}-${slot}-${index}`
            : `opening-preview-${index}`;
      const name = String(preview?.card || preview?.name || "").trim();
      if (!name) return null;
      return {
        key: [
          preview?.owner ?? "",
          preview?.objectId ?? preview?.object_id ?? "",
          preview?.stableId ?? preview?.stable_id ?? "",
          preview?.slot ?? "",
          preview?.position ?? "",
          name,
          index,
        ].join(":"),
        id,
        name,
        controller: preview?.owner,
        zone: preview?.zone,
        position: Number.isSafeInteger(position) && position >= 0 ? position : null,
      };
    })
    .filter(Boolean);
}

function peerWaitOpeningPreviewDescription(peerWait) {
  const current = Number(peerWait?.progressCurrent);
  const total = Number(peerWait?.progressTotal);
  const cardName = String(peerWait?.cardName || "").trim();
  const zone = String(peerWait?.zone || "").trim();
  const parts = [];
  if (Number.isFinite(current) && Number.isFinite(total) && total > 0) {
    parts.push(`${Math.max(0, Math.min(total, current))}/${total}`);
  }
  if (cardName) parts.push(cardName);
  if (zone) parts.push(zone);
  return parts.join(" / ");
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
  accentOverrides = null,
  className = "",
  objectControllerById = new Map(),
  hoveredObjectId = null,
  selectedObjectId = null,
  onCardHoverStart,
  onCardHoverEnd,
  compact = false,
  wrap = false,
}) {
  const { playerAccentOverrides: contextAccentOverrides } = useGame();
  const effectiveAccentOverrides = accentOverrides || contextAccentOverrides;
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
        "action-strip-scroll min-w-0",
        wrap ? "overflow-x-hidden overflow-y-auto" : "overflow-x-auto overflow-y-hidden",
        compact && "flex-1 self-stretch"
      )}
    >
      <div className={cn(
        "flex min-w-full items-center gap-1.5 pb-0.5",
        wrap ? "flex-wrap" : "w-max"
      )}>
        {cards.length > 0 ? cards.map((card, index) => {
          const cardAccent = resolveObjectAccent(
            players,
            perspective,
            objectControllerById,
            card.id,
            card.controller,
            effectiveAccentOverrides,
          ) || getPlayerAccent(
            players || [],
            perspective,
            perspective,
            effectiveAccentOverrides,
          );
          const cardAccentStyle = decisionOptionAccentVars(cardAccent);
          return (
            <button
              key={card.key || card.id || index}
              type="button"
              className={cn(
                "action-strip-pill action-strip-view-card inline-flex max-w-[220px] items-center px-2 py-1 text-[12px] transition-all",
                String(hoveredObjectId) === String(card.id) || String(selectedObjectId) === String(card.id)
                  ? "is-linked-active"
                  : "is-interactive",
                "text-[#d8ccb4]",
              )}
              style={cardAccentStyle}
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
                />
              </span>
            </button>
          );
        }) : (
          <div className="text-[12px] italic text-[#bda983]">
            No cards visible.
          </div>
        )}
      </div>
    </div>
  );

  return (
    <div className={cn(
      "viewed-cards-strip min-w-0 flex-1 overflow-hidden px-1 py-1",
      wrap && "viewed-cards-strip--wrap",
      className
    )}>
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
  primaryAdvanceLabel = "",
  primaryDisabled = false,
  onPrimary,
  secondaryLabel = "",
  secondaryDisabled = false,
  onSecondary,
  inline = false,
  orientation = "horizontal",
}) {
  const { state, multiplayer, playerAccentOverrides } = useGame();
  const decision = state?.decision || null;
  const attackButtonTransition = useDeclareAttackersButtonTransition(decision);
  const { style: decisionButtonStyle, isLocal: localDecisionButton } =
    useDecisionButtonAccent(state, decision, playerAccentOverrides);
  const isVertical = orientation === "vertical";
  const effectivePrimaryDisabled = primaryDisabled || attackButtonTransition.locked;
  const rawPeerWait = multiplayer?.peerWait || null;
  const peerWait = useDeferredPeerWait(rawPeerWait);
  const peerWaiting = Boolean(peerWait);
  const peerWaitLocked = Boolean(rawPeerWait);
  const primaryText = safeInlineLabel(primaryLabel, "Continue");
  const primaryAdvanceText = safeInlineLabel(primaryAdvanceLabel);
  const subtitleText = safeInlineLabel(subtitle);

  return (
    <div
      className={cn(
        "mobile-decision-dock",
        inline && "mobile-decision-dock--inline",
        isVertical && "mobile-decision-dock--vertical"
      )}
    >
      <div className={cn(
        "mobile-decision-dock-actions",
        isVertical && "mobile-decision-dock-actions--vertical"
      )}>
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
        <PeerWaitPopover peerWait={peerWait}>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            className="mobile-decision-primary-button decision-main-button"
            style={decisionButtonStyle}
            data-local-action={localDecisionButton ? "true" : "false"}
            data-transitioning={attackButtonTransition.transitioning ? "true" : "false"}
            aria-disabled={peerWaitLocked || effectivePrimaryDisabled}
            disabled={peerWaiting ? false : effectivePrimaryDisabled}
            onClick={(event) => {
              if (peerWaitLocked) return;
              onPrimary?.(event);
            }}
          >
            {peerWaiting ? (
              <PeerWaitButtonContent />
            ) : (
              <>
                <span className="mobile-decision-primary-label">
                  {primaryText}
                </span>
                {primaryAdvanceText ? (
                  <span className="mobile-decision-primary-subtitle">
                    {primaryAdvanceText}
                  </span>
                ) : subtitleText ? (
                  <span className="mobile-decision-primary-subtitle">
                    <SymbolText text={normalizeDecisionText(subtitleText)} noWrap />
                  </span>
                ) : null}
              </>
            )}
          </Button>
        </PeerWaitPopover>
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
  dockOrientation = "horizontal",
}) {
  const {
    state,
    multiplayer,
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
  const canAct = !!decision && samePlayerId(state?.perspective, decision.player);
  const peerWait = useDeferredPeerWait(multiplayer?.peerWait || null);
  const peerWaiting = Boolean(peerWait);

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
    decision?.plan_id || "",
    decision?.request_hash || "",
  ].join("|");
  const rawViewedCards = state?.viewed_cards || null;
  const viewedCards = isInspectorOnlyViewedCards(rawViewedCards) ? null : rawViewedCards;
  const viewedCardsLabel = viewedCards?.visibility === "public" ? "Revealed" : "Look";
  const viewedCardsIdentity = useMemo(
    () => buildViewedCardsIdentity(viewedCards),
    [viewedCards]
  );
  const viewedCardsToken = viewedCardsIdentity ? `${decisionIdentity}|${viewedCardsIdentity}` : "";
  const showViewedCardsStep = decision?.kind === "priority"
    && Boolean(viewedCardsToken)
    && acknowledgedViewedCardsToken !== viewedCardsToken;
  const showInlineViewedCards = Boolean(viewedCardsToken) && !showViewedCardsStep;
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
  const selectedObjectFamilyIds = useMemo(
    () => buildObjectFamilyIds(state?.players, selectedObjectId),
    [state?.players, selectedObjectId]
  );
  const selectedActionIndices = useMemo(() => {
    if (selectedObjectId == null) return new Set();
    return collectSelectedPriorityActionIndices(otherActions, selectedObjectFamilyIds);
  }, [otherActions, selectedObjectFamilyIds, selectedObjectId]);
  const manaPaymentActive = Boolean(state?.mana_payment);
  const visibleActionGroups = useMemo(() => {
    if (selectedObjectId == null) {
      // Mana abilities only surface in the default strip while a payment is
      // in progress; otherwise they're reachable by selecting the permanent.
      return isPriorityDecision && !manaPaymentActive
        ? withoutManaAbilityActionGroups(actionGroups)
        : actionGroups;
    }
    return filterPriorityActionGroups(
      actionGroups,
      selectedObjectFamilyIds,
      selectedActionIndices,
    );
  }, [
    actionGroups,
    isPriorityDecision,
    manaPaymentActive,
    selectedActionIndices,
    selectedObjectFamilyIds,
    selectedObjectId,
  ]);
  const showPriorityAdvanceButton = !!passAction;
  const hasCustomPassLabel = !!passAction?.label && passAction.label !== "Pass priority";
  const resolvingStackPriority = stackSize > 0 && !hasCustomPassLabel;
  const passAdvanceLabel = showPriorityAdvanceButton
    ? ""
    : (visibleActionGroups[0]?.label || "Continue");
  const passCurrentLabel = showPriorityAdvanceButton
    ? (
      resolvingStackPriority
        ? "Resolve"
        : hasCustomPassLabel
          ? passAction.label
          : `Go to ${nextPriorityAdvanceLabel(state?.phase, state?.step, stackSize)}`
    )
    : passAdvanceLabel;
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
          key: String(card.id),
          id: String(card.id),
          name: viewedCardDisplayName(card, objectNameById),
          controller: viewedCards?.subject,
        }));
      }
      return (viewedCards?.card_ids || []).map((id) => ({
        key: String(id),
        id: String(id),
        name: objectNameById.get(String(id)) || `Card #${id}`,
        controller: viewedCards?.subject,
      }));
    },
    [objectNameById, viewedCards]
  );
  const peerWaitOpeningPreviewEntries = useMemo(
    () => buildPeerWaitOpeningPreviewEntries(peerWait),
    [peerWait]
  );
  const showPeerWaitOpeningPreviews = peerWaiting && peerWaitOpeningPreviewEntries.length > 0;
  const peerWaitPreviewDescription = useMemo(
    () => peerWaitOpeningPreviewDescription(peerWait),
    [peerWait]
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
      .map((value) => conciseDecisionSummary(value))
      .filter(Boolean);
    return parts[0] || "";
  }, [decision?.context_text, decision?.description]);
  const mobileDockSubtitle = useMemo(() => {
    if (toolbarDecisionSummary) return toolbarDecisionSummary;
    if (hasCustomPassLabel) return "";
    if (stackSize > 0) {
      return `Resolve ${stackSize}`;
    }
    return nextPriorityAdvanceLabel(state?.phase, state?.step, stackSize);
  }, [hasCustomPassLabel, stackSize, state?.phase, state?.step, toolbarDecisionSummary]);

  const triggerPriorityAction = useCallback(
    (action) => {
      if (!canAct || !action) return;
      dispatchHandActionHover(null);
      clearHoverLinkedObjects();
      clearHover();
      if (action.kind === "untap_land") {
        cancelDecision();
        setActionsSheetState({ key: decisionIdentity, open: false });
        return;
      }
      dispatch(
        { type: "priority_action", action_index: action.index, action_ref: action.action_ref },
        action.label
      );
      setActionsSheetState({ key: decisionIdentity, open: false });
    },
    [canAct, cancelDecision, clearHover, clearHoverLinkedObjects, decisionIdentity, dispatch]
  );
  const handleActionHoverStart = useCallback(
    (group) => {
      if (!canAct || !group) return;
      setHoverLinkedObjects(group.linkedObjectIds || []);
      dispatchHandActionHover(group.hoverObjectId);
    },
    [canAct, setHoverLinkedObjects]
  );
  const handleActionHoverEnd = useCallback(() => {
    clearHoverLinkedObjects();
    clearHover();
    dispatchHandActionHover(null);
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

  if (showPeerWaitOpeningPreviews) {
    return renderMobileBattlePortal(
      <MobileDecisionOverlay
        eyebrow={canAct ? "Your Action" : "Opponent Action"}
        title="Opening"
        subtitle={peerWaitPreviewDescription || peerWait?.operation || ""}
      >
        <ViewedCardsStrip
          label="Opening"
          description={peerWaitPreviewDescription}
          sourceName={peerWait?.operation || ""}
          cards={peerWaitOpeningPreviewEntries}
          players={state?.players || []}
          perspective={state?.perspective}
          objectControllerById={objectControllerById}
          hoveredObjectId={hoveredObjectId}
          selectedObjectId={selectedObjectId}
          onCardHoverStart={handleViewedCardHoverStart}
          onCardHoverEnd={handleViewedCardHoverEnd}
          wrap
        />
        <div className="mobile-decision-overlay-footer">
          <Button
            type="button"
            variant="ghost"
            size="sm"
            className="mobile-decision-primary-button mobile-decision-primary-button--full"
            disabled
          >
            <PeerWaitButtonContent peerWait={peerWait} />
          </Button>
        </div>
      </MobileDecisionOverlay>
    );
  }

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
    const singleActionGroup = visibleActionGroups.length === 1 ? visibleActionGroups[0] : null;
    const secondaryAction = showPriorityAdvanceButton
      ? (visibleActionGroups.length > 1
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
      : (visibleActionGroups.length > 1
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
    const primaryDisabled = !canAct || (!showPriorityAdvanceButton && visibleActionGroups.length === 0);
    const resolvedDockSubtitle = decisionTextMatches(mobileDockSubtitle, passAdvanceLabel)
      ? ""
      : mobileDockSubtitle;
    const handlePrimary = () => {
      if (showPriorityAdvanceButton) {
        triggerPriorityAction(passAction);
        return;
      }
      if (visibleActionGroups[0]?.firstAction) {
        triggerPriorityAction(visibleActionGroups[0].firstAction);
      }
    };

    return (
      <>
        {renderMobileBattlePortal(
          <MobileDecisionDock
            subtitle={resolvedDockSubtitle}
            primaryLabel={passCurrentLabel}
            primaryAdvanceLabel={showPriorityAdvanceButton ? passAdvanceLabel : ""}
            primaryDisabled={primaryDisabled}
            onPrimary={handlePrimary}
            secondaryLabel={secondaryAction?.label || ""}
            secondaryDisabled={secondaryAction?.disabled || false}
            onSecondary={secondaryAction?.onClick}
            inline={dockInline}
            orientation={dockOrientation}
          />,
          portalTarget
        )}
        {actionsSheetOpen ? (
          <MobileDecisionSheet
            eyebrow={dockTitle}
            title="Available Actions"
            subtitle={`${visibleActionGroups.length} action${visibleActionGroups.length === 1 ? "" : "s"}`}
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
        {showInlineViewedCards ? (
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
        ) : null}
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
          orientation={dockOrientation}
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
          orientation={dockOrientation}
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
  onHoldChange,
  advanceControlLabel = "",
  showActionCount = true,
  className = "",
}) {
  const compactLandscapeViewport = typeof window !== "undefined"
    && window.matchMedia("(max-width: 720px) and (orientation: landscape)").matches;
  const advanceLabelText = safeInlineLabel(advanceControlLabel);
  const checkboxLabelClass =
    "priority-control-toggle action-strip-toggle flex items-center gap-1.5 text-[11px] uppercase tracking-wider cursor-pointer transition-colors";

  return (
    <div className={cn("priority-control-stack flex shrink-0 flex-col items-start justify-center py-1.5", className)}>
      {showActionCount && !compactLandscapeViewport && (
        <div className="priority-control-count pointer-events-none pl-[18px] text-[11px] font-bold uppercase tracking-[0.14em] text-[#d9c18b]">
          <span className="priority-control-count-number">{actionCount}</span>
          <span className="priority-control-count-label">
            {actionCount === 1 ? "Action" : "Actions"}
          </span>
        </div>
      )}
      <div className="priority-control-toggles flex items-center gap-3">
        {advanceLabelText ? (
          <span className="priority-control-advance-label" title={advanceLabelText}>
            {advanceLabelText}
          </span>
        ) : null}
        <label className={checkboxLabelClass}>
          <span title="Hold">{compactLandscapeViewport ? "H" : "Hold"}</span>
          <Checkbox
            checked={holdEnabled}
            onCheckedChange={(value) => onHoldChange?.(Boolean(value))}
            className="h-3 w-3"
          />
        </label>
      </div>
    </div>
  );
}

const PAYMENT_POOL_SYMBOLS = [
  ["white", "W"],
  ["blue", "U"],
  ["black", "B"],
  ["red", "R"],
  ["green", "G"],
  ["colorless", "C"],
];

function ManaPaymentToolbarPool({ label, pool }) {
  const entries = PAYMENT_POOL_SYMBOLS
    .map(([key, symbol]) => ({ symbol, amount: Number(pool?.[key] || 0) }))
    .filter((entry) => entry.amount > 0);
  return (
    <span className="mana-payment-toolbar-pool" aria-label={`${label} ${entries.length ? "mana" : "Empty"}`}>
      <span className="mana-payment-toolbar-pool-label">{label}</span>
      <span className="mana-payment-toolbar-pool-value">
        {entries.length ? entries.map(({ symbol, amount }) => (
          <span key={symbol} className="mana-payment-toolbar-pool-symbol">
            <ManaSymbol sym={symbol} size={15} />
            {amount > 1 ? <span>×{amount}</span> : null}
          </span>
        )) : <span>Empty</span>}
      </span>
    </span>
  );
}

function ManaPaymentToolbarMeta({ payment, sourceObjectId = null, onInspectObject = null }) {
  if (!payment) return null;
  const sourceName = payment.source_name || "mana cost";
  return (
    <div className="mana-payment-toolbar-meta">
      <div className="mana-payment-toolbar-title" title={`Pay for ${sourceName}`}>
        <span>Pay for </span>
        <DecisionCardNameTrigger objectId={sourceObjectId} onInspect={onInspectObject}>
          {sourceName}
        </DecisionCardNameTrigger>
      </div>
      <div className="mana-payment-toolbar-pools" aria-label="Mana pool payment preview">
        <ManaPaymentToolbarPool label="Pool" pool={payment.pool_before} />
        <span className="mana-payment-toolbar-arrow">→</span>
        <ManaPaymentToolbarPool label="Sources" pool={payment.pool_after_activations} />
        <span className="mana-payment-toolbar-arrow">→</span>
        <ManaPaymentToolbarPool label="After" pool={payment.pool_after_payment} />
      </div>
      {!payment.planning_complete ? (
        <span className="mana-payment-toolbar-planning" title="Checking for a better payment plan">
          <LoaderCircle size={15} className="animate-spin" />
          <span>Planning</span>
        </span>
      ) : null}
    </div>
  );
}

function ActionStripMainTitleText({ children }) {
  const textRef = useRef(null);

  const fitText = useCallback(() => {
    const textElement = textRef.current;
    const container = textElement?.parentElement;
    if (!textElement || !container) return;

    textElement.style.removeProperty("--action-strip-main-title-size");
    const availableWidth = Math.max(1, container.clientWidth);
    const naturalWidth = Math.max(1, textElement.scrollWidth);
    const baseSize = 14;
    const minSize = 7;
    const nextSize = Math.max(minSize, Math.min(baseSize, baseSize * (availableWidth / naturalWidth)));
    textElement.style.setProperty("--action-strip-main-title-size", `${nextSize}px`);
  }, []);

  useLayoutEffect(() => {
    fitText();
    if (typeof ResizeObserver === "undefined") return undefined;

    const textElement = textRef.current;
    const container = textElement?.parentElement;
    const observer = new ResizeObserver(() => fitText());
    if (container) observer.observe(container);
    if (textElement) observer.observe(textElement);
    return () => observer.disconnect();
  }, [children, fitText]);

  return (
    <span ref={textRef} className="action-strip-main-title-text">
      {children}
    </span>
  );
}

function PriorityBar({
  anchor = null,
  inline = false,
  replaceMiddleControls = false,
  selectedObjectId = null,
}) {
  const {
    state,
    dispatch,
    holdRule,
    setHoldRule,
    cancelDecision,
    triggerOrderingState,
    multiplayer,
    playerAccentOverrides,
  } = useGame();
  const [decisionToolbarSearchTarget, setDecisionToolbarSearchTarget] = useState(null);
  const {
    hoveredObjectId,
    hoverCard,
    clearHover,
    setHoverLinkedObjects,
    clearHoverLinkedObjects,
    showAnchoredCardPreview,
  } = useHover();
  const decision = state?.decision || null;
  const manaPayment = state?.mana_payment || null;
  const canAct = !!decision && samePlayerId(state?.perspective, decision.player);
  const { style: decisionButtonStyle, isLocal: localDecisionButton } =
    useDecisionButtonAccent(state, decision, playerAccentOverrides);
  const rawPeerWait = multiplayer?.peerWait || null;
  const peerWait = useDeferredPeerWait(rawPeerWait);
  const peerWaiting = Boolean(peerWait);
  const peerWaitLocked = Boolean(rawPeerWait);
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
  const resolvingStackPriority = stackSize > 0 && !hasCustomPassLabel;
  const passControlAdvanceLabel = "";
  const passCurrentLabel = resolvingStackPriority
    ? "Resolve"
    : (
      hasCustomPassLabel
        ? passAction.label
        : `Go to ${nextPriorityAdvanceLabel(state?.phase, state?.step, stackSize)}`
    );
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
  const rawViewedCards = state?.viewed_cards || null;
  const viewedCards = isInspectorOnlyViewedCards(rawViewedCards) ? null : rawViewedCards;
  const viewedCardsLabel = viewedCards?.visibility === "public" ? "Revealed" : "Look";
  const viewedCardsIdentity = useMemo(
    () => buildViewedCardsIdentity(viewedCards),
    [viewedCards]
  );
  const [acknowledgedViewedCardsToken, setAcknowledgedViewedCardsToken] = useState("");
  const viewedCardsToken = viewedCardsIdentity ? `${decisionIdentity}|${viewedCardsIdentity}` : "";
  const showViewedCardsStep = isPriorityDecision
    && Boolean(viewedCardsToken)
    && acknowledgedViewedCardsToken !== viewedCardsToken;
  const showInlineViewedCards = Boolean(viewedCardsToken) && !showViewedCardsStep;
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
      .map((value) => conciseDecisionSummary(value))
      .filter(Boolean);
    return parts[0] || "";
  }, [decision?.context_text, decision?.description]);
  const viewedCardEntries = useMemo(
    () => {
      if (Array.isArray(viewedCards?.cards) && viewedCards.cards.length > 0) {
        return viewedCards.cards.map((card) => ({
          key: String(card.id),
          id: String(card.id),
          name: viewedCardDisplayName(card, objectNameById),
          controller: viewedCards?.subject,
        }));
      }
      return (viewedCards?.card_ids || []).map((id) => ({
        key: String(id),
        id: String(id),
        name: objectNameById.get(String(id)) || `Card #${id}`,
        controller: viewedCards?.subject,
      }));
    },
    [objectNameById, viewedCards]
  );
  const peerWaitOpeningPreviewEntries = useMemo(
    () => buildPeerWaitOpeningPreviewEntries(peerWait),
    [peerWait]
  );
  const showPeerWaitOpeningPreviews = peerWaiting && peerWaitOpeningPreviewEntries.length > 0;
  const peerWaitPreviewDescription = useMemo(
    () => peerWaitOpeningPreviewDescription(peerWait),
    [peerWait]
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
  const selectedActionIndices = useMemo(() => {
    if (selectedObjectId == null) return new Set();
    return collectSelectedPriorityActionIndices(otherActions, selectedObjectFamilyIds);
  }, [otherActions, selectedObjectFamilyIds, selectedObjectId]);
  const visibleActionGroups = useMemo(() => {
    if (selectedObjectId == null) {
      // Mana abilities only surface in the default strip while a payment is
      // in progress; otherwise they're reachable by selecting the permanent.
      return isPriorityDecision && !manaPayment
        ? withoutManaAbilityActionGroups(actionGroups)
        : actionGroups;
    }
    return filterPriorityActionGroups(
      actionGroups,
      selectedObjectFamilyIds,
      selectedActionIndices,
    );
  }, [
    actionGroups,
    isPriorityDecision,
    manaPayment,
    selectedActionIndices,
    selectedObjectFamilyIds,
    selectedObjectId,
  ]);
  const priorityActionCount = visibleActionGroups.length;
  const triggerPriorityAction = useCallback(
    (action) => {
      if (peerWaitLocked || !canAct || !action) return;
      dispatchHandActionHover(null);
      clearHover();
      if (action.kind === "untap_land") {
        cancelDecision();
        return;
      }
      dispatch(
        { type: "priority_action", action_index: action.index, action_ref: action.action_ref },
        action.label
      );
    },
    [canAct, cancelDecision, clearHover, dispatch, peerWaitLocked]
  );
  const triggerPassActionFromPointer = useCallback(
    (event) => {
      if (!canAct || !passAction || event.button !== 0) return;
      event.preventDefault();
      triggerPriorityAction(passAction);
    },
    [canAct, passAction, triggerPriorityAction]
  );
  const triggerPassActionFromClick = useCallback(
    (event) => {
      if (!canAct || !passAction || event.detail !== 0) return;
      dispatchHandActionHover(null);
      triggerPriorityAction(passAction);
    },
    [canAct, passAction, triggerPriorityAction]
  );
  const handleActionHoverStart = useCallback(
    (group) => {
      if (!canAct || !group) return;
      setHoverLinkedObjects(group.linkedObjectIds || []);
      dispatchHandActionHover(group.hoverObjectId);
    },
    [canAct, setHoverLinkedObjects]
  );
  const handleActionHoverEnd = useCallback(() => {
    if (!canAct) {
      dispatchHandActionHover(null);
      return;
    }
    clearHoverLinkedObjects();
    clearHover();
    dispatchHandActionHover(null);
  }, [canAct, clearHoverLinkedObjects, clearHover]);
  const handleActionCardInspect = useCallback((objectId, anchor) => {
    if (objectId == null || !anchor) return;
    clearHoverLinkedObjects();
    clearHover();
    dispatchHandActionHover(null);
    showAnchoredCardPreview(objectId, anchor);
  }, [clearHover, clearHoverLinkedObjects, showAnchoredCardPreview]);
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
  const secondarySubmitAction = effectiveSubmitAction?.secondaryAction || null;
  const canSubmitSecondary = canAct
    && !!secondarySubmitAction
    && !secondarySubmitAction.disabled
    && typeof secondarySubmitAction.onSubmit === "function";
  const canAdvanceViewedCardsStep = !!decision;
  const compactLandscapeViewport = typeof window !== "undefined"
    && window.matchMedia("(max-width: 720px) and (orientation: landscape)").matches;
  const completeViewedCardsStep = useCallback(() => {
    if (!viewedCardsToken) return;
    setAcknowledgedViewedCardsToken(viewedCardsToken);
  }, [viewedCardsToken]);

  const topbarMainDecisionHost = inline
    && !isPriorityDecision
    && !replaceMiddleControls
    && typeof document !== "undefined"
    ? document.querySelector('[data-topbar-main-decision-host="true"]')
    : null;
  const renderExpandedPrimaryControl = (ported = false) => (
    (peerWaiting || showViewedCardsStep || effectiveSubmitAction) ? (
      <PeerWaitPopover peerWait={peerWait}>
        <Button
          variant="ghost"
          size="sm"
          className={cn(
            "decision-neon-button decision-main-button decision-submit-button h-full self-stretch rounded-none font-bold uppercase",
            ported
              ? "topbar-ported-decision-button w-full min-w-0 px-2 text-[11px]"
              : cn(
                  manaPayment
                    ? "mana-payment-pay-button min-w-[82px] flex-[0.75_1_0] px-2 text-[clamp(11px,0.88vw,14px)]"
                    : "min-w-[104px] flex-[1.2_1_0] px-3 text-[clamp(11px,0.88vw,14px)]",
                  replaceMiddleControls && "decision-main-button--middle-replacement"
                )
          )}
          style={decisionButtonStyle}
          data-local-action={localDecisionButton ? "true" : "false"}
          aria-disabled={peerWaitLocked || (showViewedCardsStep ? !canAdvanceViewedCardsStep : !canSubmitFocused)}
          disabled={peerWaiting ? false : (showViewedCardsStep ? !canAdvanceViewedCardsStep : !canSubmitFocused)}
          title={peerWaiting ? "Waiting for peers" : (showViewedCardsStep ? "Done" : (effectiveSubmitAction?.label || "Submit"))}
          onPointerDown={(event) => {
            if (peerWaitLocked) return;
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
            if (peerWaitLocked) return;
            if (showViewedCardsStep) {
              if (!canAdvanceViewedCardsStep || event.detail !== 0) return;
              completeViewedCardsStep();
              return;
            }
            if (!canSubmitFocused || event.detail !== 0) return;
            effectiveSubmitAction.onSubmit();
          }}
        >
          {peerWaiting ? (
            <PeerWaitButtonContent />
          ) : (
            showViewedCardsStep ? "Done" : (effectiveSubmitAction?.label || "Submit")
          )}
        </Button>
      </PeerWaitPopover>
    ) : null
  );

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
    const updateDecisionOverflowMode = () => {
      const content = node.querySelector(".action-strip-decision-content");
      if (!content || !replaceMiddleControls) return;
      const optionNodes = Array.from(content.querySelectorAll(
        ".action-strip-pill, .decision-option-row--strip, .decision-target-requirement, .decision-selected-chip"
      ));
      if (optionNodes.length === 0) {
        content.removeAttribute("data-option-rows");
        return;
      }
      const rowTops = new Set(
        optionNodes.map((option) => Math.round(option.getBoundingClientRect().top / 4) * 4)
      );
      const nextMode = rowTops.size > 1 ? "multiple" : "single";
      if (content.dataset.optionRows !== nextMode) {
        content.dataset.optionRows = nextMode;
      }
    };
    const scheduleUpdate = () => {
      if (frame) cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        updateManaTabAnchorRect();
        updateDecisionOverflowMode();
        frame = 0;
      });
    };

    scheduleUpdate();
    const resizeObserver = typeof ResizeObserver !== "undefined"
      ? new ResizeObserver(scheduleUpdate)
      : null;
    const mutationObserver = typeof MutationObserver !== "undefined"
      ? new MutationObserver(scheduleUpdate)
      : null;
    resizeObserver?.observe(node);
    mutationObserver?.observe(node, { childList: true, subtree: true });
    window.addEventListener("resize", scheduleUpdate);
    window.addEventListener("scroll", scheduleUpdate, true);

    return () => {
      if (frame) cancelAnimationFrame(frame);
      resizeObserver?.disconnect();
      mutationObserver?.disconnect();
      window.removeEventListener("resize", scheduleUpdate);
      window.removeEventListener("scroll", scheduleUpdate, true);
    };
  }, [inline, replaceMiddleControls, updateManaTabAnchorRect]);

  if (!decision || isCombatDecision) return null;
  if (isPriorityDecision && !passAction) return null;

  if (inline) {
    return (
      <>
        {topbarMainDecisionHost
          ? createPortal(
              <div
                className="topbar-ported-decision-action action-strip-command-region h-full w-full"
                style={decisionButtonStyle}
              >
                <div
                  className="action-strip-main-region h-full w-full"
                  style={decisionButtonStyle}
                  data-local-action={localDecisionButton ? "true" : "false"}
                >
                  {renderExpandedPrimaryControl(true)}
                </div>
              </div>,
              topbarMainDecisionHost
            )
          : null}
        <div
          ref={inlineRootRef}
          className={cn(
            "pointer-events-none absolute inset-0 z-[120] flex",
            isPriorityDecision ? "items-stretch pt-0" : "items-start pt-0.5",
            compactLandscapeViewport || isPriorityDecision ? "px-0" : "px-2"
          )}
        >
        <ManaPaymentTab manaPayment={manaPayment} anchorRect={inline ? manaTabAnchorRect : null} />
        <div
          className={cn(
            "priority-inline-panel pointer-events-auto relative flex h-full w-full flex-col py-0",
            isPriorityDecision && "priority-inline-panel--segmented",
            isPriorityDecision && !showViewedCardsStep && "priority-inline-panel--main-only",
            compactLandscapeViewport || isPriorityDecision ? "px-0" : "px-2"
          )}
          data-replaces-middle-controls={replaceMiddleControls ? "true" : "false"}
        >
          {isPriorityDecision ? (
            showViewedCardsStep ? (
              <div
                className="priority-main-action-only flex min-h-[46px] items-stretch"
                style={decisionButtonStyle}
              >
                <div className="action-strip-command-region shrink-0 self-stretch" style={decisionButtonStyle}>
                  <div
                    className="action-strip-main-region h-full w-[132px] shrink-0 self-stretch"
                    style={decisionButtonStyle}
                    data-local-action={localDecisionButton ? "true" : "false"}
                  >
                    <PeerWaitPopover peerWait={peerWait}>
                      <Button
                        variant="ghost"
                        size="sm"
                        className="decision-neon-button decision-main-button decision-submit-button h-full w-full rounded-none px-3 text-[14px] font-bold uppercase"
                        style={decisionButtonStyle}
                        data-local-action={localDecisionButton ? "true" : "false"}
                        aria-disabled={peerWaitLocked || !canAdvanceViewedCardsStep}
                        disabled={peerWaiting ? false : !canAdvanceViewedCardsStep}
                        onPointerDown={(event) => {
                          if (peerWaitLocked) return;
                          if (!canAdvanceViewedCardsStep || event.button !== 0) return;
                          event.preventDefault();
                          completeViewedCardsStep();
                        }}
                        onClick={(event) => {
                          if (peerWaitLocked) return;
                          if (!canAdvanceViewedCardsStep || event.detail !== 0) return;
                          completeViewedCardsStep();
                        }}
                      >
                        {peerWaiting ? <PeerWaitButtonContent /> : "Done"}
                      </Button>
                    </PeerWaitPopover>
                  </div>
                </div>
                <ViewedCardsStrip
                  label={viewedCardsLabel}
                  description={viewedCards?.description || ""}
                  sourceName={viewedCardsSourceName}
                  cards={viewedCardEntries}
                  players={state?.players || []}
                  perspective={state?.perspective}
                  accentOverrides={playerAccentOverrides}
                  className="action-strip-options-region self-stretch"
                  objectControllerById={objectControllerById}
                  hoveredObjectId={hoveredObjectId}
                  selectedObjectId={selectedObjectId}
                  onCardHoverStart={handleViewedCardHoverStart}
                  onCardHoverEnd={handleViewedCardHoverEnd}
                  compact
                />
              </div>
            ) : (
              <div
                className="action-strip-layout action-strip-layout--segmented flex min-h-[46px] items-stretch gap-2"
                style={decisionButtonStyle}
              >
                <div className="action-strip-command-region shrink-0 self-stretch" style={decisionButtonStyle}>
                  {showPriorityAdvanceButton && (
                    <div
                      className="action-strip-main-region relative h-full w-[132px] shrink-0 self-stretch"
                      style={decisionButtonStyle}
                      data-local-action={localDecisionButton ? "true" : "false"}
                    >
                      <PeerWaitPopover peerWait={peerWait}>
                        <Button
                          variant="ghost"
                          size="sm"
                          className="pass-priority-btn decision-main-button action-strip-advance-button h-full w-full rounded-none px-3 text-[14px] font-bold uppercase"
                          style={decisionButtonStyle}
                          data-local-action={localDecisionButton ? "true" : "false"}
                          aria-disabled={peerWaitLocked || !canAct}
                          aria-label={peerWaiting ? "Waiting for peers" : passCurrentLabel}
                          onPointerDown={peerWaiting ? undefined : triggerPassActionFromPointer}
                          onClick={peerWaiting ? undefined : triggerPassActionFromClick}
                        >
                          {peerWaiting ? (
                            <PeerWaitButtonContent />
                          ) : (
                            <span className="sr-only">{passCurrentLabel}</span>
                          )}
                        </Button>
                      </PeerWaitPopover>
                      {!peerWaiting && (
                        <div className="action-strip-main-text-stack absolute left-2 top-2 z-20">
                          <div className="action-strip-main-title-row">
                            <ActionStripMainTitleText>{passCurrentLabel}</ActionStripMainTitleText>
                          </div>
                          <div
                            className="action-strip-main-controls"
                            onPointerDown={(event) => event.stopPropagation()}
                            onClick={(event) => event.stopPropagation()}
                          >
                            <PriorityControlStack
                              actionCount={priorityActionCount}
                              holdEnabled={holdRule === "always"}
                              onHoldChange={(value) => setHoldRule(value ? "always" : "never")}
                              advanceControlLabel={passControlAdvanceLabel}
                              showActionCount={false}
                            />
                          </div>
                        </div>
                      )}
                    </div>
                  )}
                </div>
              </div>
            )
          ) : (
            <div className="action-strip-decision-stack flex min-h-0 min-w-0 flex-1 flex-col gap-1.5 py-1">
              <div className="action-strip-decision-toolbar flex min-w-0 items-stretch gap-2">
                <div className="flex min-w-0 flex-1 items-stretch gap-2">
                  {manaPayment ? (
                    <ManaPaymentToolbarMeta
                      payment={manaPayment}
                      sourceObjectId={decision?.source_id}
                      onInspectObject={handleActionCardInspect}
                    />
                  ) : !triggerOrderingDecision && (
                    <div className="action-strip-decision-meta flex min-w-0 flex-1 flex-col justify-center px-1">
                      <div className="flex min-w-0 items-baseline gap-2">
                        <span className="decision-stage-chip">{decisionStageLabel(decision)}</span>
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
                          <DecisionCardNameTrigger
                            objectId={decision?.source_id}
                            onInspect={handleActionCardInspect}
                          >
                            {normalizeDecisionText(decision.source_name)}
                          </DecisionCardNameTrigger>
                        </div>
                      )}
                    </div>
                  )}
                  <div
                    ref={setDecisionToolbarSearchTarget}
                    className="action-strip-decision-toolbar-search min-w-0"
                  />
                  <div className={cn(
                    "flex min-w-0 shrink-0 items-stretch gap-2",
                    manaPayment ? "max-w-[360px]" : "max-w-[320px]"
                  )}>
                    {!topbarMainDecisionHost ? renderExpandedPrimaryControl(false) : null}
                    {manaPayment && secondarySubmitAction ? (
                      <Button
                        type="button"
                        variant="ghost"
                        size="sm"
                        className={cn(
                          "decision-neon-button decision-plan-button h-full min-w-[82px] flex-[0.75_1_0] self-stretch rounded-none px-2 text-[clamp(10px,0.82vw,13px)] font-bold uppercase tracking-wide",
                          secondarySubmitAction.active && "is-active"
                        )}
                        disabled={!canSubmitSecondary}
                        onPointerDown={(event) => {
                          if (!canSubmitSecondary || event.button !== 0) return;
                          event.preventDefault();
                          secondarySubmitAction.onSubmit();
                        }}
                        onClick={(event) => {
                          if (!canSubmitSecondary || event.detail !== 0) return;
                          secondarySubmitAction.onSubmit();
                        }}
                      >
                        {secondarySubmitAction.label || "Plan"}
                      </Button>
                    ) : null}
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
                </div>
              </div>
              <div className="action-strip-decision-content min-w-0 flex-1 overflow-hidden">
                {showPeerWaitOpeningPreviews ? (
                  <ViewedCardsStrip
                    label="Opening"
                    description={peerWaitPreviewDescription}
                    sourceName={peerWait?.operation || ""}
                    cards={peerWaitOpeningPreviewEntries}
                    players={state?.players || []}
                    perspective={state?.perspective}
                    objectControllerById={objectControllerById}
                    hoveredObjectId={hoveredObjectId}
                    selectedObjectId={selectedObjectId}
                    onCardHoverStart={handleViewedCardHoverStart}
                    onCardHoverEnd={handleViewedCardHoverEnd}
                    wrap
                  />
                ) : canAct ? (
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
                    <>
                      {showInlineViewedCards ? (
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
                      ) : null}
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
                    </>
                  ))
                ) : (
                  <span className="action-strip-waiting text-[12px] whitespace-nowrap">
                    Waiting for {playerDisplayName(state?.players || [], decision?.player)}
                  </span>
                )}
              </div>
            </div>
          )}
        </div>
        </div>
      </>
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
        <div
          className="action-strip-layout action-strip-layout--segmented flex min-h-[46px] items-start gap-2"
          style={isPriorityDecision ? decisionButtonStyle : undefined}
        >
          {isPriorityDecision ? (
            showViewedCardsStep ? (
              <div
                    className="action-strip-main-region h-full w-[132px] shrink-0 self-stretch"
                    style={decisionButtonStyle}
                    data-local-action={localDecisionButton ? "true" : "false"}
                  >
                <PeerWaitPopover peerWait={peerWait}>
                  <Button
                    variant="ghost"
                    size="sm"
                    className="decision-neon-button decision-main-button decision-submit-button h-full w-full rounded-none px-3 text-[14px] font-bold uppercase"
                    style={decisionButtonStyle}
                    data-local-action={localDecisionButton ? "true" : "false"}
                    aria-disabled={peerWaitLocked || !canAdvanceViewedCardsStep}
                    disabled={peerWaiting ? false : !canAdvanceViewedCardsStep}
                    onPointerDown={(event) => {
                      if (peerWaitLocked) return;
                      if (!canAdvanceViewedCardsStep || event.button !== 0) return;
                      event.preventDefault();
                      completeViewedCardsStep();
                    }}
                    onClick={(event) => {
                      if (peerWaitLocked) return;
                      if (!canAdvanceViewedCardsStep || event.detail !== 0) return;
                      completeViewedCardsStep();
                    }}
                  >
                    {peerWaiting ? <PeerWaitButtonContent /> : "Done"}
                  </Button>
                </PeerWaitPopover>
              </div>
            ) : (
              <>
                {showPriorityAdvanceButton && (
                  <div
                      className="action-strip-main-region relative h-full w-[132px] shrink-0 self-stretch"
                      style={decisionButtonStyle}
                      data-local-action={localDecisionButton ? "true" : "false"}
                    >
                    <PeerWaitPopover peerWait={peerWait}>
                      <Button
                        variant="ghost"
                        size="sm"
                        className="pass-priority-btn decision-main-button action-strip-advance-button h-full w-full rounded-none px-3 text-[14px] font-bold uppercase"
                        style={decisionButtonStyle}
                        data-local-action={localDecisionButton ? "true" : "false"}
                        aria-disabled={peerWaitLocked || !canAct}
                        aria-label={peerWaiting ? "Waiting for peers" : passCurrentLabel}
                        onPointerDown={peerWaiting ? undefined : triggerPassActionFromPointer}
                        onClick={peerWaiting ? undefined : triggerPassActionFromClick}
                      >
                        {peerWaiting ? (
                          <PeerWaitButtonContent />
                        ) : (
                          <span className="sr-only">{passCurrentLabel}</span>
                        )}
                      </Button>
                    </PeerWaitPopover>
                    {!peerWaiting && (
                      <div className="action-strip-main-text-stack absolute left-2 top-2 z-20">
                        <div className="action-strip-main-title-row">
                          <ActionStripMainTitleText>{passCurrentLabel}</ActionStripMainTitleText>
                        </div>
                        <div
                          className="action-strip-main-controls"
                          onPointerDown={(event) => event.stopPropagation()}
                          onClick={(event) => event.stopPropagation()}
                        >
                          <PriorityControlStack
                            actionCount={priorityActionCount}
                            holdEnabled={holdRule === "always"}
                            onHoldChange={(value) => setHoldRule(value ? "always" : "never")}
                            advanceControlLabel={passControlAdvanceLabel}
                            showActionCount={false}
                          />
                        </div>
                      </div>
                    )}
                  </div>
                )}
              </>
            )
          ) : (
            <>
              <div className="action-strip-decision-stack flex min-w-0 w-full flex-col gap-y-1">
                <div className="action-strip-decision-toolbar flex min-h-[46px] items-stretch gap-2">
                  {manaPayment ? (
                    <ManaPaymentToolbarMeta
                      payment={manaPayment}
                      sourceObjectId={decision?.source_id}
                      onInspectObject={handleActionCardInspect}
                    />
                  ) : !triggerOrderingDecision && (
                    <div className="action-strip-decision-meta flex min-w-0 flex-1 flex-col justify-center py-1.5">
                      <div className="flex min-w-0 items-center gap-2">
                        <span className="decision-stage-chip">{decisionStageLabel(decision)}</span>
                        <div className="action-strip-decision-title truncate text-[11px] font-bold uppercase tracking-[0.14em]">
                          {resolveDecisionTitle(decision)}
                        </div>
                      </div>
                      {decision?.source_name && (
                        <div className="action-strip-decision-source mt-0.5 truncate text-[11px]">
                          <DecisionCardNameTrigger
                            objectId={decision?.source_id}
                            onInspect={handleActionCardInspect}
                          >
                            {normalizeDecisionText(decision.source_name)}
                          </DecisionCardNameTrigger>
                        </div>
                      )}
                    </div>
                  )}
                  <div className={cn(
                    "flex min-w-0 shrink-0 items-stretch gap-2",
                    manaPayment ? "max-w-[360px]" : "max-w-[320px]"
                  )}>
                    {(peerWaiting || showViewedCardsStep || effectiveSubmitAction) ? (
                    <PeerWaitPopover peerWait={peerWait}>
                      <Button
                        variant="ghost"
                        size="sm"
                        className={cn(
                          "decision-neon-button decision-main-button decision-submit-button h-full self-stretch rounded-none text-[clamp(11px,0.88vw,14px)] font-bold uppercase",
                          manaPayment
                            ? "mana-payment-pay-button min-w-[82px] flex-[0.75_1_0] px-2"
                            : "min-w-[104px] flex-[1.2_1_0] px-3"
                        )}
                        style={decisionButtonStyle}
                        data-local-action={localDecisionButton ? "true" : "false"}
                        aria-disabled={peerWaitLocked || (showViewedCardsStep ? !canAdvanceViewedCardsStep : !canSubmitFocused)}
                        disabled={peerWaiting ? false : (showViewedCardsStep ? !canAdvanceViewedCardsStep : !canSubmitFocused)}
                        onPointerDown={(event) => {
                          if (peerWaitLocked) return;
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
                          if (peerWaitLocked) return;
                          if (showViewedCardsStep) {
                            if (!canAdvanceViewedCardsStep || event.detail !== 0) return;
                            completeViewedCardsStep();
                            return;
                          }
                          if (!canSubmitFocused || event.detail !== 0) return;
                          effectiveSubmitAction.onSubmit();
                        }}
                      >
                        {peerWaiting ? (
                          <PeerWaitButtonContent />
                        ) : (
                          showViewedCardsStep ? "Done" : (effectiveSubmitAction?.label || "Submit")
                        )}
                      </Button>
                    </PeerWaitPopover>
                    ) : null}
                    {manaPayment && secondarySubmitAction ? (
                      <Button
                        type="button"
                        variant="ghost"
                        size="sm"
                        className={cn(
                          "decision-neon-button decision-plan-button h-full min-w-[82px] flex-[0.75_1_0] self-stretch rounded-none px-2 text-[clamp(10px,0.82vw,13px)] font-bold uppercase tracking-wide",
                          secondarySubmitAction.active && "is-active"
                        )}
                        disabled={!canSubmitSecondary}
                        onPointerDown={(event) => {
                          if (!canSubmitSecondary || event.button !== 0) return;
                          event.preventDefault();
                          secondarySubmitAction.onSubmit();
                        }}
                        onClick={(event) => {
                          if (!canSubmitSecondary || event.detail !== 0) return;
                          secondarySubmitAction.onSubmit();
                        }}
                      >
                        {secondarySubmitAction.label || "Plan"}
                      </Button>
                    ) : null}
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
                  {!manaPayment ? (
                    <PriorityControlStack
                      holdEnabled={holdRule === "always"}
                      onHoldChange={(value) => setHoldRule(value ? "always" : "never")}
                      showActionCount={false}
                      className="ml-auto min-w-[104px]"
                    />
                  ) : null}
                </div>
              </div>
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
                decisionPlayer={decision?.player}
                hasPinnedSelection={selectedObjectId != null}
                objectNameById={objectNameById}
                objectControllerById={objectControllerById}
                hoveredObjectFamilyIds={hoveredObjectFamilyIds}
                selectedObjectFamilyIds={selectedObjectFamilyIds}
                selectedActionIndices={selectedActionIndices}
                onActionClick={triggerPriorityAction}
                onActionHoverStart={handleActionHoverStart}
                onActionHoverEnd={handleActionHoverEnd}
                onActionCardInspect={handleActionCardInspect}
              />
              <PriorityControlStack
                actionCount={priorityActionCount}
                holdEnabled={holdRule === "always"}
                onHoldChange={(value) => setHoldRule(value ? "always" : "never")}
                className="ml-auto min-w-[104px]"
              />
            </div>
          )
        ) : (
          <div className="action-strip-decision-content min-w-0 h-full">
            {showPeerWaitOpeningPreviews ? (
              <ViewedCardsStrip
                label="Opening"
                description={peerWaitPreviewDescription}
                sourceName={peerWait?.operation || ""}
                cards={peerWaitOpeningPreviewEntries}
                players={state?.players || []}
                perspective={state?.perspective}
                objectControllerById={objectControllerById}
                hoveredObjectId={hoveredObjectId}
                selectedObjectId={selectedObjectId}
                onCardHoverStart={handleViewedCardHoverStart}
                onCardHoverEnd={handleViewedCardHoverEnd}
                wrap
              />
            ) : showViewedCardsStep ? (
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
              <>
                {showInlineViewedCards ? (
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
                ) : null}
                <DecisionRouter
                  decision={decision}
                  canAct={canAct}
                  selectedObjectId={selectedObjectId}
                  inlineSubmit={false}
                  onSubmitActionChange={handleSubmitActionChange}
                  hideDescription={false}
                  layout="strip"
                  showStripSummary={!showStripDecisionSummary}
                  toolbarSearchTarget={decisionToolbarSearchTarget}
                />
              </>
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
    cancelDecision,
    multiplayer,
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
  const attackButtonTransition = useDeclareAttackersButtonTransition(decision);
  const rawPeerWait = multiplayer?.peerWait || null;
  const peerWait = useDeferredPeerWait(rawPeerWait);
  const peerWaiting = Boolean(peerWait);
  const peerWaitLocked = Boolean(rawPeerWait);
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
    && !attackButtonTransition.locked
    && typeof combatAction.onSubmit === "function";
  const combatPrimaryDisabled = !combatAction
    || combatAction.disabled
    || attackButtonTransition.locked;
  const panelClass = inline
    ? "pointer-events-none absolute inset-0 z-[120] flex items-center px-2"
    : "pointer-events-none fixed left-2 bottom-[148px] z-[120] w-[min(96vw,740px)]";

  const innerClass = cn(
    "priority-inline-panel pointer-events-auto flex w-full items-stretch gap-2 px-2 py-0",
    compactPortraitViewport && "flex-col items-stretch",
    !inline && anchoredStyle ? "fixed" : ""
  );

  return (
    <div className={panelClass}>
      <div className={innerClass} style={anchoredStyle || undefined}>
        <div className={cn("min-w-0 flex-1", compactPortraitViewport && "w-full")}>
          <div className={cn("action-strip-layout flex min-h-[46px] items-stretch justify-end gap-2", compactPortraitViewport && "flex-col")}>
            <PeerWaitPopover peerWait={peerWait}>
              <Button
                variant="ghost"
                size="sm"
                className={cn(
                  "decision-option-row decision-option-row--strip combat-confirm-option h-9 w-[224px] min-w-[224px] max-w-[360px] shrink-0 justify-center self-center px-3 text-center text-[12px] font-semibold",
                  decision.kind === "attackers"
                    ? "combat-confirm-option--attackers"
                    : "combat-confirm-option--blockers",
                  inline ? "combat-inline-primary my-auto" : "h-full self-stretch",
                  compactPortraitViewport && "w-full min-w-0"
                )}
                data-transitioning={attackButtonTransition.transitioning ? "true" : "false"}
                aria-disabled={peerWaitLocked || !canSubmitCombat}
                disabled={peerWaiting ? false : combatPrimaryDisabled}
                onClick={() => {
                  if (peerWaitLocked || !canSubmitCombat) return;
                  combatAction?.onSubmit?.();
                }}
              >
                {peerWaiting ? (
                  <PeerWaitButtonContent />
                ) : (
                  combatAction?.label || (
                    decision.kind === "attackers" ? "Confirm Attackers (0)" : "Confirm Blockers (0)"
                  )
                )}
              </Button>
            </PeerWaitPopover>
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
          onHoldChange={(value) => setHoldRule(value ? "always" : "never")}
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
  replaceMiddleControls = false,
  selectedObjectId = null,
  mobileBattle = false,
  mobileBattlePortalTarget = null,
  mobileBattleDockInline = false,
  mobileBattleDockHidden = false,
  mobileBattleDockOrientation = "horizontal",
}) {
  const { state } = useGame();
  const decision = state?.decision || null;
  const canAct = !!decision && samePlayerId(state?.perspective, decision.player);

  if (!decision) return null;
  let content = null;
  if (mobileBattle) {
    content = (
      <MobileBattleDecisionLayer
        selectedObjectId={selectedObjectId}
        portalTarget={mobileBattlePortalTarget}
        dockInline={mobileBattleDockInline}
        dockHidden={mobileBattleDockHidden}
        dockOrientation={mobileBattleDockOrientation}
      />
    );
  } else if (decision?.kind === "priority") {
    content = (
      <PriorityBar
        anchor={anchor}
        inline={priorityInline}
        replaceMiddleControls={replaceMiddleControls}
        selectedObjectId={selectedObjectId}
      />
    );
  } else if (decision?.kind === "attackers" || decision?.kind === "blockers") {
    content = <CombatBar anchor={anchor} inline={priorityInline} decision={decision} canAct={canAct} />;
  } else {
    content = (
      <PriorityBar
        anchor={anchor}
        inline={priorityInline}
        replaceMiddleControls={replaceMiddleControls}
        selectedObjectId={selectedObjectId}
      />
    );
  }

  return (
    <KeywordHelpersProvider enabled={false}>
      {content}
    </KeywordHelpersProvider>
  );
}
