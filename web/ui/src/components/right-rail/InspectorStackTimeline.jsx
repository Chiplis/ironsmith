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
  buildTriggerOrderingEntries,
  buildTriggerOrderingKey,
  isTriggerOrderingDecision,
} from "@/lib/trigger-ordering";

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
    triggerOrderingState,
    moveTriggerOrderingItem,
  } = useGame();
  const bodyRef = useRef(null);
  const focusedDecision = isFocusedDecision(decision) && canAct;
  const triggerOrderingActive = isTriggerOrderingDecision(decision);
  const triggerOrderingKey = buildTriggerOrderingKey(decision);
  const hasStackEntries = stackObjects.length > 0 || stackPreview.length > 0;
  const stackIds = useMemo(
    () => stackEntryRenderKeys(stackObjects).map((key) => `live-${key}`),
    [stackObjects]
  );
  const { newIds } = useNewCards(stackIds);
  const activeStackInspectId = useMemo(
    () => resolveActiveStackInspectId(stackObjects, selectedObjectId),
    [selectedObjectId, stackObjects]
  );
  // Triggers waiting to be ordered sit above the live stack: the order a
  // player arranges them in here is the order they will land in.
  const pendingTriggerEntries = useMemo(() => {
    if (!triggerOrderingActive || triggerOrderingState?.key !== triggerOrderingKey) return [];
    return buildTriggerOrderingEntries(decision, triggerOrderingState.order).map((entry) => ({
      ...entry,
      __timeline_key: `pending-${entry.__trigger_ordering_option_index}`,
      __leaving: false,
    }));
  }, [decision, triggerOrderingActive, triggerOrderingKey, triggerOrderingState]);
  const liveTimelineEntries = useMemo(
    () => stackObjects.map((entry, index) => ({
      ...entry,
      __timeline_key: stackIds[index],
      __leaving: false,
    })),
    [stackObjects, stackIds]
  );
  const timelineEntries = useMemo(
    () => [...pendingTriggerEntries, ...liveTimelineEntries],
    [pendingTriggerEntries, liveTimelineEntries]
  );
  const itemCount = timelineEntries.length || stackPreview.length;
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

  if (!hasStackEntries && pendingTriggerEntries.length === 0) return null;

  const embeddedExpandedMaxHeight = Number.isFinite(maxBodyHeight) && maxBodyHeight > 0
    ? Math.max(96, Math.round(maxBodyHeight))
    : 380;

  const positionLabelForIndex = (index) => {
    if (index !== 0) return `#${timelineEntries.length - index}`;
    if (focusedDecision && !triggerOrderingActive) return "Resolving";
    return "Top";
  };

  const renderEntry = (entry, index) => {
    const isPending = Boolean(entry.__trigger_ordering);
    // A pending trigger can be previewed and inspected through the object it
    // came from, once the engine has named one for it.
    const canInspect = !entry.__leaving && (!isPending || stackInspectObjectId(entry) != null);
    return (
      <div
        key={entry.__timeline_key}
        className="stack-timeline-entry pointer-events-auto relative"
      >
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
                canMoveRight: canAct && index < (pendingTriggerEntries.length - 1),
                onMoveLeft: () => moveTriggerOrderingItem(index, -1),
                onMoveRight: () => moveTriggerOrderingItem(index, 1),
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

  const countLabel = focusedDecision
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
      data-ordering={triggerOrderingActive ? "true" : "false"}
    >
      <header className="stack-panel-header pointer-events-none">
        <span className="stack-panel-title">{ui(title)}</span>
        {/* The count reads as a bare number; the full label stays in the
            accessible text (and in innerText, which the e2e suites match). */}
        <span className="stack-panel-count" title={countLabel}>
          <span aria-hidden="true">{itemCount}</span>
          <span className="sr-only">{countLabel}</span>
        </span>
        {triggerOrderingActive && (
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
