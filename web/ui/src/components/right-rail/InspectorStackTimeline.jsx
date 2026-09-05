import RollingPanel from "@/components/board/RollingPanel";
import { useCallback, useLayoutEffect, useMemo, useRef } from "react";
import { useGame } from "@/context/GameContext";
import PlayerStackAlert from "@/components/board/PlayerStackAlert";
import { ScrollArea } from "@/components/ui/scroll-area";
import useNewCards from "@/hooks/useNewCards";
import useStackStartAlert from "@/hooks/useStackStartAlert";
import StackCard from "@/components/cards/StackCard";
import AnimatedCircuitFrame from "@/components/cards/AnimatedCircuitFrame";
import useScryfallImageUrl from "@/hooks/useScryfallImageUrl";
import { getPlayerAccent, playerAccentVars } from "@/lib/player-colors";
import { ManaCostIcons, SymbolText } from "@/lib/mana-symbols";
import { stagger } from "@/lib/motion/anime";
import useLayoutReflow from "@/lib/motion/useLayoutReflow";
import { cn } from "@/lib/utils";
import { ArrowDown, ArrowUp } from "lucide-react";
import {
  buildTriggerOrderingEntries,
  buildTriggerOrderingKey,
  isTriggerOrderingDecision,
} from "@/lib/trigger-ordering";

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

function HorizontalStackEntry({
  entry,
  positionLabel,
  isActive = false,
  accent = null,
  showStackAlert = false,
  onClick,
  reorderControls = null,
  compact = false,
}) {
  const name = entry?.name || `Object#${entry?.id}`;
  const artUrl = useScryfallImageUrl(name, "art_crop");
  const kindLabel = horizontalStackKindLabel(entry);
  const subtitle = String(entry?.__subtitle || "").trim();
  const isTriggerOrderingEntry = !!entry?.__trigger_ordering;
  const isSpell = !entry?.ability_kind;
  const entryMinHeight = compact ? 40 : HORIZONTAL_STACK_ENTRY_MIN_HEIGHT;
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
        "stack-timeline-entry pointer-events-auto relative shrink-0",
        reorderControls && "stack-card-reorderable",
        isTriggerOrderingEntry && "stack-timeline-entry-ordering"
      )}
      style={{
        width: HORIZONTAL_STACK_ENTRY_WIDTH,
        minHeight: `${entryMinHeight}px`,
      }}
      data-arrow-anchor="stack"
      data-arrow-anchor-gap={compact ? "26" : undefined}
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
            <ArrowUp className="size-3.5" />
          </button>
          <button
            type="button"
            className="stack-card-reorder-button stack-card-reorder-button-right"
            disabled={!reorderControls.canMoveRight}
            onClick={() => reorderControls.onMoveRight?.()}
            aria-label={reorderControls.rightLabel || `Move ${name} toward the bottom of the stack`}
            title={reorderControls.rightTitle || "Move toward the bottom of the stack"}
          >
            <ArrowDown className="size-3.5" />
          </button>
        </>
      )}
      <button
        type="button"
        className={cn(
          "stack-timeline-entry-surface stack-timeline-circuit relative grid h-full w-full items-start gap-x-1.5 gap-y-0 overflow-hidden border border-[rgba(224,191,127,0.78)] px-2 text-left transition-transform duration-150",
          compact
            ? "grid-cols-[20px_minmax(0,1fr)] py-1"
            : "grid-cols-[24px_minmax(0,1fr)] py-[5px]",
          reorderControls && "pl-8 pr-8",
          isTriggerOrderingEntry && "stack-timeline-entry-surface-ordering",
          "hover:shadow-none",
          isActive && "stack-timeline-item-active"
        )}
        style={{ minHeight: `${entryMinHeight}px`, ...accentStyle }}
        onClick={() => onClick?.(stackInspectObjectId(entry), {
          source: "stack",
          stackEntry: entry,
        })}
      >
        <div
          className={cn(
            "stack-timeline-entry-fill absolute inset-0 z-0",
            isTriggerOrderingEntry && "stack-timeline-entry-fill-ordering"
          )}
        />
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
          className="stack-entry-badge pointer-events-none absolute left-2 z-[2] rounded-none bg-[rgba(54,43,33,0.9)] px-1 py-[1px] text-[8px] font-bold uppercase leading-none tracking-[0.12em] text-[#f0d7a2]"
          style={{ top: `${compact ? 22 : HORIZONTAL_STACK_BADGE_TOP}px` }}
        >
          {positionLabel}
        </span>
        <div
          className={cn(
            "relative z-[2] shrink-0 overflow-hidden rounded-none bg-[rgba(43,34,27,0.96)]",
            compact ? "h-5 w-5" : "h-6 w-6"
          )}
        >
          {artUrl && (
            <img
              className="h-full w-full object-cover opacity-100 saturate-[1.06] brightness-[1.08]"
              src={artUrl}
              alt=""
              loading="lazy"
              referrerPolicy="no-referrer"
            />
          )}
        </div>
        <div className={cn("relative z-[2] min-w-0", compact ? "h-5" : "h-6")}>
          <div className="absolute inset-x-0 top-0 flex items-start justify-between gap-1.5">
            <div className={cn(
              "stack-entry-title min-w-0 truncate pr-1 font-semibold leading-[1.02] text-[#fff0ca]",
              compact ? "text-[12px]" : "text-[13px]"
            )}>
              {name}
            </div>
            <div className="flex shrink-0 items-start gap-1 pt-[1px]">
              {isSpell && entry?.mana_cost && (
                <span className="shrink-0 scale-[0.82] origin-top-right">
                  <ManaCostIcons cost={entry.mana_cost} />
                </span>
              )}
              {pt && (
                <span className="rounded-none border border-[rgba(196,167,112,0.42)] bg-[rgba(79,61,39,0.24)] px-1 py-0.5 text-[10px] font-bold leading-none tracking-wide text-[#f5d08b]">
                  {pt}
                </span>
              )}
            </div>
          </div>
          <div className={cn(
            "absolute inset-x-0 bottom-0 truncate font-bold uppercase leading-none tracking-[0.12em] text-[#ead9b6]",
            compact ? "text-[8px]" : "text-[9px]"
          )}>
            {subtitle ? (
              <SymbolText text={subtitle} noWrap />
            ) : kindLabel}
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
  compact = false,
}) {
  const {
    state,
    triggerOrderingState,
    moveTriggerOrderingItem,
  } = useGame();
  const players = state?.players || [];
  const bodyRef = useRef(null);
  const horizontalScrollRef = useRef(null);
  const focusedDecision = isFocusedDecision(decision) && canAct;
  const triggerOrderingActive = isTriggerOrderingDecision(decision);
  const triggerOrderingKey = buildTriggerOrderingKey(decision);
  const hasStackEntries = stackObjects.length > 0 || stackPreview.length > 0;
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
    return stackObjects;
  }, [stackObjects]);
  const visibleTimelineEntries = useMemo(
    () => visibleLiveStackObjects.map((entry) => ({
      ...entry,
      __timeline_key: `live-${entry.id}`,
      __leaving: false,
    })),
    [visibleLiveStackObjects]
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
  const horizontalTimelineEntries = useMemo(
    () => [
      ...pendingTriggerEntries,
      ...visibleLiveStackObjects.map((entry) => ({
        ...entry,
        __timeline_key: `live-${entry.id}`,
        __leaving: false,
      })),
    ],
    [pendingTriggerEntries, visibleLiveStackObjects]
  );
  const itemCount = (
    pendingTriggerEntries.length + visibleLiveStackObjects.length
  ) || stackPreview.length;
  const timelineSignature = timelineEntries.map((entry) => entry.__timeline_key).join("|");
  const horizontalTimelineSignature = horizontalTimelineEntries
    .map((entry) => entry.__timeline_key)
    .join("|");
  const isHorizontal = layout === "horizontal";
  const horizontalEntries = useMemo(
    () => horizontalTimelineEntries
      .map((entry, index) => ({
        ...entry,
        __horizontal_source_index: index,
      }))
      .reverse(),
    [horizontalTimelineEntries]
  );
  const horizontalPreviewEntries = useMemo(
    () => [...stackPreview].reverse(),
    [stackPreview]
  );
  const handleInspectStackObject = useCallback((objectId, meta) => {
    dismissStackStartAlert();
    onInspectObject?.(objectId, meta);
  }, [dismissStackStartAlert, onInspectObject]);

  useLayoutReflow(bodyRef, timelineSignature, {
    children: ".stack-timeline-entry",
    // The horizontal stack rail is absolutely positioned and can retain intermediate
    // wrapper opacity from FLIP-style reflow animations, which makes the active item
    // look dim even though its inner surface is fully opaque.
    disabled: collapsed || timelineEntries.length === 0 || isHorizontal || (isHorizontal && triggerOrderingActive),
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
    const syncToRightEdge = () => {
      if (rafId != null) cancelAnimationFrame(rafId);
      rafId = requestAnimationFrame(() => {
        rafId = null;
        scroller.scrollLeft = Math.max(0, scroller.scrollWidth - scroller.clientWidth);
      });
    };

    syncToRightEdge();

    const observer = typeof ResizeObserver !== "undefined"
      ? new ResizeObserver(syncToRightEdge)
      : null;
    observer?.observe(scroller);
    observer?.observe(content);

    return () => {
      if (rafId != null) cancelAnimationFrame(rafId);
      observer?.disconnect();
    };
  }, [isHorizontal, itemCount, horizontalTimelineSignature, horizontalPreviewEntries]);

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
        className={cn(
          "pointer-events-none relative isolate flex w-fit max-w-full items-stretch overflow-hidden",
          compact
            ? "rounded-none bg-transparent shadow-none"
            : "rounded-none bg-transparent shadow-none"
        )}
        style={{ minHeight: `${compact ? 40 : HORIZONTAL_STACK_ENTRY_MIN_HEIGHT + 2}px` }}
        data-inspector-stack-timeline
      >
        <div
          ref={horizontalScrollRef}
          className="stack-timeline-scroll pointer-events-none min-w-0 flex-1 overflow-x-auto overflow-y-hidden px-1 py-0"
        >
          <div
            ref={bodyRef}
            className="flex w-max min-w-full items-stretch justify-end overflow-visible"
          >
            {horizontalEntries.length > 0
              ? horizontalEntries.map((entry, index) => (
                  <HorizontalStackEntry
                    key={entry.__timeline_key}
                    entry={entry}
                    compact={compact}
                    positionLabel={positionLabelForIndex(entry.__horizontal_source_index ?? index)}
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
                      !entry.__leaving ? getPlayerAccent(players, entry.controller, state?.perspective) : null
                    }
                    onClick={entry.__leaving || entry.__trigger_ordering ? undefined : handleInspectStackObject}
                    reorderControls={entry.__trigger_ordering
                      ? {
                          canMoveLeft: canAct && (entry.__horizontal_source_index ?? index) > 0,
                          canMoveRight: canAct
                            && (entry.__horizontal_source_index ?? index) < (pendingTriggerEntries.length - 1),
                          onMoveLeft: () => moveTriggerOrderingItem(entry.__horizontal_source_index ?? index, -1),
                          onMoveRight: () => moveTriggerOrderingItem(entry.__horizontal_source_index ?? index, 1),
                        }
                      : null}
                  />
                ))
              : horizontalPreviewEntries.map((name, index) => (
                <div
                  key={`${name}-${index}`}
                  className={cn(
                    "stack-timeline-entry pointer-events-auto relative flex h-full shrink-0 items-center bg-[linear-gradient(180deg,rgb(13,33,52),rgb(8,18,31))] px-3 text-[13px] font-semibold text-[#d5e7fd]",
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
          ? "pointer-events-auto w-full min-h-0 overflow-hidden rounded-none border border-[#35506c] bg-transparent shadow-none flex flex-col"
          : "pointer-events-none absolute inset-x-0 bottom-0 z-[36] overflow-hidden border-t border-[#35506c] bg-transparent shadow-none",
        compact && !isHorizontal && "stack-timeline-compact"
      )}
      style={embedded ? undefined : { height: `${Math.max(0, timelineHeight)}px` }}
      data-inspector-stack-timeline
    >
      <header className="pointer-events-none flex items-center justify-between gap-2 border-b border-[#2f4864] px-2.5 py-1.5">
        <div className="flex items-center gap-1.5">
          {collapsible && typeof onToggleCollapsed === "function" && (
            <button
              type="button"
              className="pointer-events-auto inline-flex h-4 w-4 items-center justify-center rounded-none border border-[#3a5673] bg-[rgba(9,18,30,0.7)] text-[10px] text-[#9cc8f3] transition-colors hover:border-[#8ec4ff] hover:text-[#d8ecff]"
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
        <RollingPanel open={!collapsed}>
        <div ref={bodyRef} className="pointer-events-auto overflow-hidden">
          <div
            className="stack-timeline-scroll pointer-events-auto grid gap-1.5 overflow-y-auto overscroll-contain p-1.5"
            style={{ maxHeight: `${embeddedExpandedMaxHeight}px` }}
          >
            {timelineEntries.length > 0
              ? timelineEntries.map((entry, index) => (
                  <div
                    key={entry.__timeline_key}
                    className="stack-timeline-entry pointer-events-auto relative"
                  >
                    <span className="pointer-events-none absolute left-1.5 top-1.5 z-10 rounded-none bg-[rgba(8,18,30,0.86)] px-1 py-[2px] text-[10px] font-bold uppercase tracking-[0.12em] text-[#8ec4ff]">
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
                      className={cn(
                        compact ? "pt-3 stack-card--compact-timeline" : "pt-4"
                      )}
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
                    className="pointer-events-auto rounded-none border border-[#304760] bg-[linear-gradient(180deg,rgba(13,33,52,0.8),rgba(8,18,31,0.92))] px-2.5 py-2 text-[14px] text-[#d5e7fd]"
                  >
                    <div className="text-[10px] font-bold uppercase tracking-[0.12em] text-[#8ec4ff]">
                      Preview
                    </div>
                    <div className="mt-0.5 leading-snug">{name}</div>
                  </div>
                ))}
          </div>
        </div>
        </RollingPanel>
      ) : (
        <ScrollArea className="pointer-events-none h-[calc(100%-38px)]">
          <div ref={bodyRef} className="grid gap-1.5 p-1.5">
            {timelineEntries.length > 0
              ? timelineEntries.map((entry, index) => (
                  <div
                    key={entry.__timeline_key}
                    className="stack-timeline-entry pointer-events-auto relative"
                  >
                    <span className="pointer-events-none absolute left-1.5 top-1.5 z-10 rounded-none bg-[rgba(8,18,30,0.86)] px-1 py-[2px] text-[10px] font-bold uppercase tracking-[0.12em] text-[#8ec4ff]">
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
                    className="pointer-events-auto rounded-none border border-[#304760] bg-[linear-gradient(180deg,rgba(13,33,52,0.8),rgba(8,18,31,0.92))] px-2.5 py-2 text-[14px] text-[#d5e7fd]"
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
