import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { useGame } from "@/context/GameContext";
import PlayerStackAlert from "@/components/board/PlayerStackAlert";
import { ScrollArea } from "@/components/ui/scroll-area";
import useNewCards from "@/hooks/useNewCards";
import useStackStartAlert from "@/hooks/useStackStartAlert";
import StackCard from "@/components/cards/StackCard";
import AnimatedCircuitFrame from "@/components/cards/AnimatedCircuitFrame";
import { getPlayerAccent, playerAccentVars } from "@/lib/player-colors";
import { ManaCostIcons } from "@/lib/mana-symbols";
import { scryfallImageUrl } from "@/lib/scryfall";
import { stagger } from "@/lib/motion/anime";
import useLayoutReflow from "@/lib/motion/useLayoutReflow";
import { cn } from "@/lib/utils";
import { ArrowLeft, ArrowRight } from "lucide-react";
import {
  buildTriggerOrderingEntries,
  buildTriggerOrderingKey,
  isTriggerOrderingDecision,
} from "@/lib/trigger-ordering";

const STACK_LEAVE_ANIMATION_MS = 360;
const HORIZONTAL_STACK_ENTRY_WIDTH = "clamp(180px, 17vw, 230px)";
const HORIZONTAL_STACK_ENTRY_MIN_HEIGHT = 50;
const HORIZONTAL_STACK_BADGE_TOP = 27;
const HORIZONTAL_STACK_CIRCUIT_PATH = "M9.5 2.5H90.5 M9.5 47.5H90.5";

function isFocusedDecision(decision) {
  return (
    !!decision
    && decision.kind !== "priority"
    && decision.kind !== "attackers"
    && decision.kind !== "blockers"
  );
}

function stackInspectObjectId(entry) {
  return entry?.inspect_object_id ?? entry?.id ?? null;
}

function resolveActiveStackInspectId(stackObjects = [], selectedObjectId = null) {
  const selectedKey = selectedObjectId == null ? null : String(selectedObjectId);
  if (selectedKey != null) {
    const selectedEntry = stackObjects.find((entry) => (
      String(stackInspectObjectId(entry)) === selectedKey
      || String(entry?.id) === selectedKey
    ));
    if (selectedEntry) return String(stackInspectObjectId(selectedEntry));
  }

  const topEntry = stackObjects[0] || null;
  return topEntry ? String(stackInspectObjectId(topEntry)) : null;
}

function horizontalStackKindLabel(entry) {
  const abilityKind = String(entry?.ability_kind || "").trim();
  const normalized = abilityKind.toLowerCase();
  if (!abilityKind) return "Spell";
  if (normalized === "triggered") return "Trigger";
  if (normalized === "activated") return "Activation";
  return `${abilityKind} ability`;
}

function mergeTimelineLeavingEntries(liveEntries = [], leavingEntries = []) {
  if (!Array.isArray(leavingEntries) || leavingEntries.length === 0) {
    return liveEntries.map((entry) => ({
      ...entry,
      __timeline_key: `live-${entry.id}`,
      __leaving: false,
    }));
  }

  const mergedEntries = [];
  const sortedLeavingEntries = [...leavingEntries].sort((left, right) => (
    (left.previousIndex ?? Number.MAX_SAFE_INTEGER)
    - (right.previousIndex ?? Number.MAX_SAFE_INTEGER)
  ));
  let liveIndex = 0;

  for (const leavingItem of sortedLeavingEntries) {
    const targetIndex = Math.max(0, Number(leavingItem.previousIndex ?? mergedEntries.length));
    while (mergedEntries.length < targetIndex && liveIndex < liveEntries.length) {
      const liveEntry = liveEntries[liveIndex];
      mergedEntries.push({
        ...liveEntry,
        __timeline_key: `live-${liveEntry.id}`,
        __leaving: false,
      });
      liveIndex += 1;
    }

    mergedEntries.push({
      ...leavingItem.entry,
      __timeline_key: leavingItem.key,
      __leaving: true,
    });
  }

  while (liveIndex < liveEntries.length) {
    const liveEntry = liveEntries[liveIndex];
    mergedEntries.push({
      ...liveEntry,
      __timeline_key: `live-${liveEntry.id}`,
      __leaving: false,
    });
    liveIndex += 1;
  }

  return mergedEntries;
}

function HorizontalStackEntry({
  entry,
  positionLabel,
  isActive = false,
  accent = null,
  showStackAlert = false,
  onClick,
  reorderControls = null,
}) {
  const name = entry?.name || `Object#${entry?.id}`;
  const artUrl = scryfallImageUrl(name, "art_crop");
  const kindLabel = horizontalStackKindLabel(entry);
  const subtitle = String(entry?.__subtitle || "").trim();
  const isSpell = !entry?.ability_kind;
  const pt = entry?.power_toughness
    || (entry?.power != null && entry?.toughness != null
      ? `${entry.power}/${entry.toughness}`
      : null);
  const accentStyle = accent
    ? {
      ...playerAccentVars(accent),
      "--glow-rgb": accent.rgb,
    }
    : undefined;

  return (
    <div
      className={cn(
        "stack-timeline-entry relative shrink-0",
        reorderControls && "stack-card-reorderable"
      )}
      style={{
        width: HORIZONTAL_STACK_ENTRY_WIDTH,
        minHeight: `${HORIZONTAL_STACK_ENTRY_MIN_HEIGHT}px`,
      }}
      data-arrow-anchor="stack"
      data-object-id={entry?.id}
    >
      {reorderControls && (
        <>
          <button
            type="button"
            className="stack-card-reorder-button stack-card-reorder-button-left"
            disabled={!reorderControls.canMoveLeft}
            onClick={() => reorderControls.onMoveLeft?.()}
            aria-label={reorderControls.leftLabel || `Move ${name} toward the top of the stack`}
            title={reorderControls.leftTitle || "Move toward the top of the stack"}
          >
            <ArrowLeft className="size-3.5" />
          </button>
          <button
            type="button"
            className="stack-card-reorder-button stack-card-reorder-button-right"
            disabled={!reorderControls.canMoveRight}
            onClick={() => reorderControls.onMoveRight?.()}
            aria-label={reorderControls.rightLabel || `Move ${name} toward the bottom of the stack`}
            title={reorderControls.rightTitle || "Move toward the bottom of the stack"}
          >
            <ArrowRight className="size-3.5" />
          </button>
        </>
      )}
      <button
        type="button"
        className={cn(
          "stack-timeline-entry-surface stack-timeline-circuit relative grid h-full w-full grid-cols-[24px_minmax(0,1fr)] items-start gap-x-1.5 gap-y-0 overflow-hidden bg-[linear-gradient(180deg,rgba(7,16,27,0.94),rgba(6,12,21,0.98))] px-2 py-[5px] text-left transition-[background,box-shadow,transform] duration-150",
          reorderControls && "pl-8 pr-8",
          !isActive && "hover:shadow-none",
          isActive && "stack-timeline-item-active",
          isActive && "bg-[linear-gradient(180deg,rgba(10,22,37,0.98),rgba(7,16,28,1))]"
        )}
        style={{ minHeight: `${HORIZONTAL_STACK_ENTRY_MIN_HEIGHT}px`, ...accentStyle }}
        onClick={() => onClick?.(stackInspectObjectId(entry), {
          source: "stack",
          stackEntry: entry,
        })}
      >
        <AnimatedCircuitFrame
          seed={`stack-timeline:${entry?.id}:${entry?.controller}:${name}`}
          path={HORIZONTAL_STACK_CIRCUIT_PATH}
          viewBox="0 0 100 50"
          overlayClassName="stack-circuit-overlay"
        />
        <PlayerStackAlert
          visible={showStackAlert}
          className="pointer-events-none absolute right-2 top-1/2 z-[3] -translate-y-1/2"
        />
        <span
          className="stack-entry-badge pointer-events-none absolute left-2 z-[2] rounded bg-[rgba(8,18,30,0.9)] px-1 py-[1px] text-[8px] font-bold uppercase leading-none tracking-[0.12em] text-[#8ec4ff]"
          style={{ top: `${HORIZONTAL_STACK_BADGE_TOP}px` }}
        >
          {positionLabel}
        </span>
        <div className="relative z-[2] h-6 w-6 shrink-0 overflow-hidden rounded-md bg-[#0b121b]">
          {artUrl && (
            <img
              className="h-full w-full object-cover opacity-90"
              src={artUrl}
              alt=""
              loading="lazy"
              referrerPolicy="no-referrer"
            />
          )}
        </div>
        <div className="relative z-[2] h-6 min-w-0">
          <div className="absolute inset-x-0 top-0 flex items-start justify-between gap-1.5">
            <div className="stack-entry-title min-w-0 truncate pr-1 text-[13px] font-semibold leading-[1.02] text-[#edf5ff]">
              {name}
            </div>
            <div className="flex shrink-0 items-start gap-1 pt-[1px]">
              {isSpell && entry?.mana_cost && (
                <span className="shrink-0 scale-[0.82] origin-top-right">
                  <ManaCostIcons cost={entry.mana_cost} />
                </span>
              )}
              {pt && (
                <span className="rounded-sm border border-[#f5d08b]/35 bg-[rgba(245,208,139,0.08)] px-1 py-0.5 text-[10px] font-bold leading-none tracking-wide text-[#f5d08b]">
                  {pt}
                </span>
              )}
            </div>
          </div>
          <div className="absolute inset-x-0 bottom-0 truncate text-[9px] font-bold uppercase leading-none tracking-[0.12em] text-[#8ec4ff]">
            {subtitle || kindLabel}
          </div>
        </div>
      </button>
    </div>
  );
}

export default function InspectorStackTimeline({
  decision = null,
  canAct = false,
  stackObjects = [],
  stackPreview = [],
  selectedObjectId = null,
  timelineHeight = 176,
  embedded = false,
  layout = "vertical",
  onInspectObject,
  title = "Stack",
  collapsible = false,
  collapsed = false,
  onToggleCollapsed = null,
  maxBodyHeight = null,
}) {
  const {
    state,
    triggerOrderingState,
    moveTriggerOrderingItem,
  } = useGame();
  const players = state?.players || [];
  const [leavingEntries, setLeavingEntries] = useState([]);
  const previousStackRef = useRef([]);
  const leaveTimeoutsRef = useRef(new Map());
  const bodyRef = useRef(null);
  const horizontalScrollRef = useRef(null);
  const focusedDecision = isFocusedDecision(decision) && canAct;
  const triggerOrderingActive = isTriggerOrderingDecision(decision);
  const triggerOrderingKey = buildTriggerOrderingKey(decision);
  const hasStackEntries = stackObjects.length > 0 || leavingEntries.length > 0 || stackPreview.length > 0;
  const stackIds = useMemo(() => stackObjects.map((entry) => entry.id), [stackObjects]);
  const { newIds } = useNewCards(stackIds);
  const activeStackInspectId = useMemo(
    () => resolveActiveStackInspectId(stackObjects, selectedObjectId),
    [selectedObjectId, stackObjects]
  );
  const pendingTriggerEntries = useMemo(() => {
    if (!triggerOrderingActive || triggerOrderingState?.key !== triggerOrderingKey) return [];
    return buildTriggerOrderingEntries(decision, triggerOrderingState.order).map((entry) => ({
      ...entry,
      __timeline_key: `pending-${entry.__trigger_ordering_option_index}`,
      __leaving: false,
    }));
  }, [decision, triggerOrderingActive, triggerOrderingKey, triggerOrderingState]);
  const visibleLiveStackObjects = useMemo(() => {
    if (!triggerOrderingActive || pendingTriggerEntries.length === 0) {
      return stackObjects;
    }
    return stackObjects.slice(pendingTriggerEntries.length);
  }, [pendingTriggerEntries.length, stackObjects, triggerOrderingActive]);
  const visibleTimelineEntries = useMemo(
    () => mergeTimelineLeavingEntries(visibleLiveStackObjects, leavingEntries),
    [leavingEntries, visibleLiveStackObjects]
  );
  const { alertEntryId: stackStartAlertId, dismissAlert: dismissStackStartAlert } = useStackStartAlert(
    visibleLiveStackObjects,
    state?.perspective
  );
  const timelineEntries = useMemo(
    () => [
      ...pendingTriggerEntries,
      ...visibleTimelineEntries,
    ],
    [pendingTriggerEntries, visibleTimelineEntries]
  );
  const itemCount = (
    pendingTriggerEntries.length + visibleLiveStackObjects.length
  ) || stackPreview.length;
  const timelineSignature = timelineEntries.map((entry) => entry.__timeline_key).join("|");
  const isHorizontal = layout === "horizontal";
  const horizontalEntries = timelineEntries;
  const horizontalPreviewEntries = stackPreview;
  const handleInspectStackObject = useCallback((objectId, meta) => {
    dismissStackStartAlert();
    onInspectObject?.(objectId, meta);
  }, [dismissStackStartAlert, onInspectObject]);

  useEffect(() => {
    const previousStack = previousStackRef.current || [];
    const nextIds = new Set(stackObjects.map((entry) => String(entry.id)));
    const previousIndexById = new Map(previousStack.map((entry, index) => [String(entry.id), index]));
    const removed = previousStack.filter((entry) => !nextIds.has(String(entry.id)));

    if (removed.length > 0) {
      const additions = removed.map((entry) => ({
        key: `live-${entry.id}`,
        entry,
        previousIndex: previousIndexById.get(String(entry.id)) ?? previousStack.length,
      }));
      queueMicrotask(() => {
        setLeavingEntries((prev) => {
          const existing = new Set(prev.map((item) => String(item.entry.id)));
          const deduped = additions.filter((item) => !existing.has(String(item.entry.id)));
          if (deduped.length === 0) return prev;
          return [...prev, ...deduped];
        });
      });

      for (const addition of additions) {
        const timeout = setTimeout(() => {
          leaveTimeoutsRef.current.delete(addition.key);
          setLeavingEntries((prev) => prev.filter((item) => item.key !== addition.key));
        }, STACK_LEAVE_ANIMATION_MS);
        leaveTimeoutsRef.current.set(addition.key, timeout);
      }
    }

    previousStackRef.current = stackObjects;
  }, [stackObjects]);

  useEffect(() => () => {
    for (const timeout of leaveTimeoutsRef.current.values()) {
      clearTimeout(timeout);
    }
    leaveTimeoutsRef.current.clear();
  }, []);

  useLayoutReflow(bodyRef, timelineSignature, {
    children: ".stack-timeline-entry",
    disabled: collapsed || timelineEntries.length === 0,
    delay: stagger(34),
    duration: 320,
    bounce: 0.12,
    enterFrom: isHorizontal ? { opacity: 0, y: 8, scale: 0.97 } : { opacity: 0, y: 16, scale: 0.97 },
    leaveTo: isHorizontal ? { opacity: 0, y: -8, scale: 0.96 } : { opacity: 0, y: -14, scale: 0.96 },
  });

  useLayoutEffect(() => {
    if (!isHorizontal) return;
    const scroller = horizontalScrollRef.current;
    const content = bodyRef.current;
    if (!scroller || !content) return;

    let rafId = null;
    const syncToLeftEdge = () => {
      if (rafId != null) cancelAnimationFrame(rafId);
      rafId = requestAnimationFrame(() => {
        rafId = null;
        scroller.scrollLeft = 0;
      });
    };

    syncToLeftEdge();

    const observer = typeof ResizeObserver !== "undefined"
      ? new ResizeObserver(syncToLeftEdge)
      : null;
    observer?.observe(scroller);
    observer?.observe(content);

    return () => {
      if (rafId != null) cancelAnimationFrame(rafId);
      observer?.disconnect();
    };
  }, [isHorizontal, itemCount, timelineSignature, horizontalPreviewEntries]);

  if (!hasStackEntries && pendingTriggerEntries.length === 0) return null;

  const embeddedExpandedMaxHeight = Number.isFinite(maxBodyHeight) && maxBodyHeight > 0
    ? Math.max(96, Math.round(maxBodyHeight))
    : 380;

  const positionLabelForIndex = (index) => {
    if (index !== 0) return `#${timelineEntries.length - index}`;
    if (focusedDecision && !triggerOrderingActive) return "Resolving";
    return "Top";
  };

  if (isHorizontal) {
    return (
      <section
        className="relative flex items-stretch overflow-hidden rounded-[14px] bg-[linear-gradient(180deg,rgba(6,14,24,0.86),rgba(5,10,18,0.98))] backdrop-blur-[2.2px] shadow-[0_14px_30px_rgba(0,0,0,0.38)]"
        style={{ minHeight: `${HORIZONTAL_STACK_ENTRY_MIN_HEIGHT + 2}px` }}
        data-inspector-stack-timeline
      >
        <div
          ref={horizontalScrollRef}
          className="stack-timeline-scroll min-w-0 flex-1 overflow-x-auto overflow-y-hidden px-1 py-0"
        >
          <div
            ref={bodyRef}
            className="flex w-max min-w-full items-stretch justify-start overflow-visible"
          >
            {horizontalEntries.length > 0
              ? horizontalEntries.map((entry, index) => (
                  <HorizontalStackEntry
                    key={entry.__timeline_key}
                    entry={entry}
                    positionLabel={positionLabelForIndex(index)}
                    showStackAlert={
                      !entry.__leaving
                      && !entry.__trigger_ordering
                      && stackStartAlertId != null
                      && String(entry.id) === String(stackStartAlertId)
                    }
                    isActive={
                      !entry.__leaving
                      && !entry.__trigger_ordering
                      && activeStackInspectId != null
                      && String(activeStackInspectId) === String(stackInspectObjectId(entry))
                    }
                    accent={
                      !entry.__leaving ? getPlayerAccent(players, entry.controller) : null
                    }
                    onClick={entry.__leaving || entry.__trigger_ordering ? undefined : handleInspectStackObject}
                    reorderControls={entry.__trigger_ordering
                      ? {
                          canMoveLeft: canAct && index > 0,
                          canMoveRight: canAct && index < (pendingTriggerEntries.length - 1),
                          onMoveLeft: () => moveTriggerOrderingItem(index, -1),
                          onMoveRight: () => moveTriggerOrderingItem(index, 1),
                        }
                      : null}
                  />
                ))
              : horizontalPreviewEntries.map((name, index) => (
                <div
                  key={`${name}-${index}`}
                  className={cn(
                    "stack-timeline-entry relative flex h-full shrink-0 items-center bg-[linear-gradient(180deg,rgba(13,33,52,0.84),rgba(8,18,31,0.96))] px-3 text-[13px] font-semibold text-[#d5e7fd]",
                    index > 0
                      ? "shadow-[inset_1px_0_0_rgba(53,80,108,0.65)]"
                      : ""
                  )}
                  style={{
                    width: HORIZONTAL_STACK_ENTRY_WIDTH,
                    minHeight: `${HORIZONTAL_STACK_ENTRY_MIN_HEIGHT}px`,
                  }}
                >
                  <span className="truncate">{name}</span>
                </div>
              ))}
          </div>
        </div>
      </section>
    );
  }

  return (
    <section
      className={cn(
        embedded
          ? "w-full min-h-0 overflow-hidden rounded-l rounded-r-sm border border-[#35506c] bg-[linear-gradient(180deg,rgba(6,14,24,0.84),rgba(5,10,18,0.95))] backdrop-blur-[2.2px] pointer-events-auto shadow-[0_14px_30px_rgba(0,0,0,0.45)] flex flex-col"
          : "absolute inset-x-0 bottom-0 z-[36] overflow-hidden border-t border-[#35506c] bg-[linear-gradient(180deg,rgba(6,14,24,0.64),rgba(5,10,18,0.9))] backdrop-blur-[2.2px] pointer-events-auto"
      )}
      style={embedded ? undefined : { height: `${Math.max(0, timelineHeight)}px` }}
      data-inspector-stack-timeline
    >
      <header className="flex items-center justify-between gap-2 border-b border-[#2f4864] px-2.5 py-1.5">
        <div className="flex items-center gap-1.5">
          {collapsible && typeof onToggleCollapsed === "function" && (
            <button
              type="button"
              className="inline-flex h-4 w-4 items-center justify-center rounded-sm border border-[#3a5673] bg-[rgba(9,18,30,0.7)] text-[10px] text-[#9cc8f3] transition-colors hover:border-[#8ec4ff] hover:text-[#d8ecff]"
              onClick={onToggleCollapsed}
              aria-label={collapsed ? "Expand stack" : "Collapse stack"}
              title={collapsed ? "Expand stack" : "Collapse stack"}
            >
              {collapsed ? "▸" : "▾"}
            </button>
          )}
          <div className="text-[11px] font-bold uppercase tracking-[0.14em] text-[#8ec4ff]">
            {title}
          </div>
        </div>
        <div className="text-[11px] text-[#c5d9f2]">
          {focusedDecision ? `${itemCount} stack entr${itemCount === 1 ? "y" : "ies"}` : `${itemCount} entr${itemCount === 1 ? "y" : "ies"}`}
        </div>
      </header>
      {embedded ? (
        <div
          ref={bodyRef}
          className={cn(
            "overflow-hidden transition-[max-height,opacity] duration-300 ease-out",
            collapsed ? "opacity-0" : "opacity-100"
          )}
          style={{ maxHeight: collapsed ? "0px" : `${embeddedExpandedMaxHeight}px` }}
        >
          <div
            className="stack-timeline-scroll grid gap-1.5 overflow-y-auto overscroll-contain p-1.5"
            style={{ maxHeight: `${embeddedExpandedMaxHeight}px` }}
          >
            {timelineEntries.length > 0
              ? timelineEntries.map((entry, index) => (
                  <div
                    key={entry.__timeline_key}
                    className="stack-timeline-entry relative"
                  >
                    <span className="pointer-events-none absolute left-1.5 top-1.5 z-10 rounded bg-[rgba(8,18,30,0.86)] px-1 py-[2px] text-[10px] font-bold uppercase tracking-[0.12em] text-[#8ec4ff]">
                      {positionLabelForIndex(index)}
                    </span>
                    <StackCard
                      entry={entry}
                      isNew={!entry.__leaving && !entry.__trigger_ordering && newIds.has(entry.id)}
                      isLeaving={entry.__leaving}
                      showStackAlert={
                        !entry.__leaving
                        && !entry.__trigger_ordering
                        && stackStartAlertId != null
                        && String(entry.id) === String(stackStartAlertId)
                      }
                      isActive={
                        !entry.__leaving
                        && !entry.__trigger_ordering
                        && activeStackInspectId != null
                        && String(activeStackInspectId) === String(stackInspectObjectId(entry))
                      }
                      className="pt-4"
                      onClick={entry.__leaving || entry.__trigger_ordering ? undefined : handleInspectStackObject}
                      reorderControls={entry.__trigger_ordering
                        ? {
                            canMoveLeft: canAct && index > 0,
                            canMoveRight: canAct && index < (pendingTriggerEntries.length - 1),
                            onMoveLeft: () => moveTriggerOrderingItem(index, -1),
                            onMoveRight: () => moveTriggerOrderingItem(index, 1),
                          }
                        : null}
                    />
                  </div>
                ))
              : stackPreview.map((name, index) => (
                  <div
                    key={`${name}-${index}`}
                    className="rounded border border-[#304760] bg-[linear-gradient(180deg,rgba(13,33,52,0.8),rgba(8,18,31,0.92))] px-2.5 py-2 text-[14px] text-[#d5e7fd]"
                  >
                    <div className="text-[10px] font-bold uppercase tracking-[0.12em] text-[#8ec4ff]">
                      Preview
                    </div>
                    <div className="mt-0.5 leading-snug">{name}</div>
                  </div>
                ))}
          </div>
        </div>
      ) : (
        <ScrollArea className="h-[calc(100%-38px)]">
          <div ref={bodyRef} className="grid gap-1.5 p-1.5">
            {timelineEntries.length > 0
              ? timelineEntries.map((entry, index) => (
                  <div
                    key={entry.__timeline_key}
                    className="stack-timeline-entry relative"
                  >
                    <span className="pointer-events-none absolute left-1.5 top-1.5 z-10 rounded bg-[rgba(8,18,30,0.86)] px-1 py-[2px] text-[10px] font-bold uppercase tracking-[0.12em] text-[#8ec4ff]">
                      {positionLabelForIndex(index)}
                    </span>
                    <StackCard
                      entry={entry}
                      isNew={!entry.__leaving && !entry.__trigger_ordering && newIds.has(entry.id)}
                      isLeaving={entry.__leaving}
                      showStackAlert={
                        !entry.__leaving
                        && !entry.__trigger_ordering
                        && stackStartAlertId != null
                        && String(entry.id) === String(stackStartAlertId)
                      }
                      isActive={
                        !entry.__leaving
                        && !entry.__trigger_ordering
                        && activeStackInspectId != null
                        && String(activeStackInspectId) === String(stackInspectObjectId(entry))
                      }
                      className="pt-4"
                      onClick={entry.__leaving || entry.__trigger_ordering ? undefined : handleInspectStackObject}
                      reorderControls={entry.__trigger_ordering
                        ? {
                            canMoveLeft: canAct && index > 0,
                            canMoveRight: canAct && index < (pendingTriggerEntries.length - 1),
                            onMoveLeft: () => moveTriggerOrderingItem(index, -1),
                            onMoveRight: () => moveTriggerOrderingItem(index, 1),
                          }
                        : null}
                    />
                  </div>
                ))
              : stackPreview.map((name, index) => (
                  <div
                    key={`${name}-${index}`}
                    className="rounded border border-[#304760] bg-[linear-gradient(180deg,rgba(13,33,52,0.8),rgba(8,18,31,0.92))] px-2.5 py-2 text-[14px] text-[#d5e7fd]"
                  >
                    <div className="text-[10px] font-bold uppercase tracking-[0.12em] text-[#8ec4ff]">
                      Preview
                    </div>
                    <div className="mt-0.5 leading-snug">{name}</div>
                  </div>
                ))}

          </div>
        </ScrollArea>
      )}
    </section>
  );
}
