import useUiText from "@/i18n/useUiText";
import RollingPanel from "@/components/board/RollingPanel";
import { useMemo, useRef } from "react";
import { useGame } from "@/context/GameContext";
import { ScrollArea } from "@/components/ui/scroll-area";
import useNewCards from "@/hooks/useNewCards";
import StackCard from "@/components/cards/StackCard";
import { stagger } from "@/lib/motion/anime";
import useLayoutReflow from "@/lib/motion/useLayoutReflow";
import { cn } from "@/lib/utils";
import { stackEntryRenderKeys } from "@/lib/stack-targets";
import {
  buildEffectOrderingEntries,
  buildEffectOrderingKey,
  isEffectOrderingDecision,
  isReplacementOrderingDecision,
} from "@/lib/effect-ordering";

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

export default function InspectorStackTimeline({
  decision = null,
  canAct = false,
  stackObjects = [],
  stackPreview = [],
  selectedObjectId = null,
  timelineHeight = 176,
  embedded = false,
  onInspectObject,
  title = "Stack",
  maxBodyHeight = null,
  compact = false,
}) {
  const ui = useUiText();
  const {
    state,
    effectOrderingState,
    moveEffectOrderingItem,
  } = useGame();
  const bodyRef = useRef(null);
  const focusedDecision = isFocusedDecision(decision) && canAct;
  const effectOrderingActive = isEffectOrderingDecision(decision);
  const replacementOrderingActive = isReplacementOrderingDecision(decision);
  const effectOrderingKey = buildEffectOrderingKey(decision);
  const hasStackEntries = stackObjects.length > 0 || stackPreview.length > 0;
  const stackIds = useMemo(
    () => stackEntryRenderKeys(stackObjects).map((key, index) => stackObjects[index].__timeline_key ?? `live-${key}`),
    [stackObjects]
  );
  const { newIds } = useNewCards(stackIds);
  const activeStackInspectId = useMemo(
    () => resolveActiveStackInspectId(stackObjects.filter(entry => !entry.__leaving), selectedObjectId),
    [selectedObjectId, stackObjects]
  );
  // Pending choices share the same cards and arrows. Replacements are kept
  // above a separate Stack heading because they change an event directly.
  const pendingOrderingEntries = useMemo(() => {
    if (!effectOrderingActive || effectOrderingState?.key !== effectOrderingKey) return [];
    return buildEffectOrderingEntries(decision, effectOrderingState.order, state).map((entry) => ({
      ...entry,
      __timeline_key: `pending-${entry.id}`,
      __leaving: false,
    }));
  }, [decision, effectOrderingActive, effectOrderingKey, effectOrderingState, state]);
  const liveTimelineEntries = useMemo(
    () => stackObjects.map((entry, index) => ({
      ...entry,
      __timeline_key: stackIds[index],
      __leaving: Boolean(entry.__leaving),
    })),
    [stackObjects, stackIds]
  );
  const timelineEntries = useMemo(
    () => [...pendingOrderingEntries, ...liveTimelineEntries],
    [pendingOrderingEntries, liveTimelineEntries]
  );
  const liveEntryCount = timelineEntries.filter(entry => !entry.__leaving).length;
  const itemCount = liveEntryCount || stackPreview.length;
  const timelineSignature = timelineEntries.map((entry) => entry.__timeline_key).join("|");
  useLayoutReflow(bodyRef, timelineSignature, {
    children: ".stack-timeline-entry",
    disabled: timelineEntries.length === 0,
    delay: stagger(34),
    duration: 320,
    bounce: 0.12,
    enterFrom: { opacity: 0, y: 16, scale: 0.97 },
    leaveTo: { opacity: 0, y: -14, scale: 0.96 },
  });

  if (!hasStackEntries && pendingOrderingEntries.length === 0) return null;

  const embeddedExpandedMaxHeight = Number.isFinite(maxBodyHeight) && maxBodyHeight > 0
    ? Math.max(96, Math.round(maxBodyHeight))
    : 380;

  const positionLabelForIndex = (index) => {
    if (timelineEntries[index].__leaving) return "";
    index = timelineEntries.slice(0, index).filter(entry => !entry.__leaving).length;
    if (replacementOrderingActive) {
      if (index < pendingOrderingEntries.length) return index === 0 ? "Apply first" : `#${index + 1}`;
      return index === pendingOrderingEntries.length ? "Resolving" : `#${liveEntryCount - index}`;
    }
    if (index !== 0) return `#${liveEntryCount - index}`;
    if (focusedDecision && !effectOrderingActive) return "Resolving";
    return "Top";
  };

  const renderEntry = (entry, index) => {
    const isPending = Boolean(entry.__effect_ordering);
    // A pending effect can be previewed and inspected through the object it
    // came from, once the engine has named one for it.
    const canInspect = !entry.__leaving && (!isPending || stackInspectObjectId(entry) != null);
    return (
      <div
        key={entry.__timeline_key}
        className="stack-timeline-entry pointer-events-auto relative"
        data-leaving={entry.__leaving ? "true" : undefined}
        inert={entry.__leaving || undefined}
        aria-hidden={entry.__leaving || undefined}
      >
        {replacementOrderingActive && index === pendingOrderingEntries.length && (
          <div className="stack-panel-header">
            <span className="stack-panel-title">{ui("Stack")}</span>
          </div>
        )}
        <StackCard
          entry={entry}
          density={compact ? "compact" : "default"}
          positionLabel={positionLabelForIndex(index)}
          isNew={!entry.__leaving && !isPending && newIds.has(entry.__timeline_key)}
          isLeaving={entry.__leaving}
          isActive={
            !entry.__leaving
            && !isPending
            && activeStackInspectId != null
            && String(activeStackInspectId) === String(stackInspectObjectId(entry))
          }
          onClick={canInspect ? onInspectObject : undefined}
          reorderControls={isPending
            ? {
                canMoveLeft: canAct && index > 0,
                canMoveRight: canAct && index < (pendingOrderingEntries.length - 1),
                onMoveLeft: () => moveEffectOrderingItem(index, -1),
                onMoveRight: () => moveEffectOrderingItem(index, 1),
                ...(replacementOrderingActive ? {
                  leftLabel: ui("Move {0} earlier", { 0: entry.name }),
                  rightLabel: ui("Move {0} later", { 0: entry.name }),
                  leftTitle: "Apply earlier",
                  rightTitle: "Apply later",
                } : {}),
              }
            : null}
        />
      </div>
    );
  };

  const renderPreview = (name, index) => (
    <div
      key={`${name}-${index}`}
      className="stack-preview-tile pointer-events-auto"
    >
      <span className="stack-card-position">{ui("Preview")}</span>
      <span className="stack-preview-tile-name">{name}</span>
    </div>
  );

  const body = (
    <div
      ref={bodyRef}
      className="stack-timeline-scroll pointer-events-auto"
      style={embedded ? { maxHeight: `${embeddedExpandedMaxHeight}px` } : undefined}
    >
      {timelineEntries.length > 0
        ? timelineEntries.map(renderEntry)
        : stackPreview.map(renderPreview)}
    </div>
  );

  const displayedCount = replacementOrderingActive ? pendingOrderingEntries.length : itemCount;
  const countLabel = replacementOrderingActive
    ? ui("Replacement effects: {0}", { 0: displayedCount })
    : focusedDecision
    ? ui("Stack entries: {0}", { 0: itemCount })
    : ui("Entries: {0}", { 0: itemCount });

  return (
    <section
      className={cn(
        "stack-panel",
        embedded
          ? "stack-panel--embedded pointer-events-auto w-full min-h-0 overflow-hidden flex flex-col"
          : "stack-panel--docked pointer-events-none absolute inset-x-0 bottom-0 z-[36] overflow-hidden",
        compact && "stack-timeline-compact"
      )}
      style={embedded ? undefined : { height: `${Math.max(0, timelineHeight)}px` }}
      data-inspector-stack-timeline
      data-density={compact ? "compact" : "default"}
      data-ordering={effectOrderingActive ? "true" : "false"}
      data-ordering-kind={replacementOrderingActive ? "replacement" : effectOrderingActive ? "trigger" : undefined}
    >
      <header className="stack-panel-header pointer-events-none">
        <span className="stack-panel-title">{ui(replacementOrderingActive ? "Replacement effects" : title)}</span>
        {/* The count reads as a bare number; the full label stays in the
            accessible text (and in innerText, which the e2e suites match). */}
        <span className="stack-panel-count" title={countLabel}>
          <span aria-hidden="true">{displayedCount}</span>
          <span className="sr-only">{countLabel}</span>
        </span>
        {effectOrderingActive && (
          <span className="stack-panel-state">{ui("Order")}</span>
        )}
      </header>
      {embedded ? (
        <RollingPanel open className="stack-timeline-body min-h-0 flex-1">
          <div className="pointer-events-auto flex min-h-0 flex-col overflow-hidden">
            {body}
          </div>
        </RollingPanel>
      ) : (
        <ScrollArea className="pointer-events-none h-[calc(100%-34px)]">
          {body}
        </ScrollArea>
      )}
    </section>
  );
}
