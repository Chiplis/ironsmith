import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { ScrollArea } from "@/components/ui/scroll-area";
import useNewCards from "@/hooks/useNewCards";
import StackCard from "@/components/cards/StackCard";
import { ManaCostIcons } from "@/lib/mana-symbols";
import { scryfallImageUrl } from "@/lib/scryfall";
import { stagger } from "@/lib/motion/anime";
import useLayoutReflow from "@/lib/motion/useLayoutReflow";
import { cn } from "@/lib/utils";

const STACK_LEAVE_ANIMATION_MS = 360;
const HORIZONTAL_STACK_ENTRY_WIDTH = "clamp(180px, 17vw, 230px)";

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
  showLeadingBorder = true,
  isActive = false,
  onClick,
}) {
  const name = entry?.name || `Object#${entry?.id}`;
  const artUrl = scryfallImageUrl(name, "art_crop");
  const kindLabel = horizontalStackKindLabel(entry);
  const isSpell = !entry?.ability_kind;
  const pt = entry?.power_toughness
    || (entry?.power != null && entry?.toughness != null
      ? `${entry.power}/${entry.toughness}`
      : null);

  return (
    <div
      className="stack-timeline-entry relative shrink-0"
      style={{ width: HORIZONTAL_STACK_ENTRY_WIDTH }}
    >
      <button
        type="button"
        className={cn(
          "relative grid min-h-[74px] w-full grid-cols-[24px_minmax(0,1fr)_auto] items-start gap-1.5 overflow-hidden bg-[linear-gradient(180deg,rgba(7,16,27,0.94),rgba(6,12,21,0.98))] px-2 py-2 text-left transition-[background,box-shadow,transform] duration-150",
          showLeadingBorder && "shadow-[inset_1px_0_0_rgba(53,80,108,0.65),0_10px_18px_rgba(0,0,0,0.22)]",
          showLeadingBorder && !isActive && "hover:shadow-[inset_1px_0_0_rgba(127,190,244,0.92),-10px_0_18px_-14px_rgba(127,190,244,0.95),0_10px_18px_rgba(0,0,0,0.22)]",
          !showLeadingBorder && "shadow-[0_10px_18px_rgba(0,0,0,0.22)]",
          isActive && "stack-timeline-item-active",
          isActive && showLeadingBorder && "bg-[linear-gradient(180deg,rgba(10,22,37,0.98),rgba(7,16,28,1))] shadow-[inset_1px_0_0_rgba(142,196,255,0.95),-10px_0_18px_-14px_rgba(142,196,255,0.98),0_12px_22px_rgba(0,0,0,0.3)]",
          isActive && !showLeadingBorder && "bg-[linear-gradient(180deg,rgba(10,22,37,0.98),rgba(7,16,28,1))] shadow-[0_12px_22px_rgba(0,0,0,0.3)]"
        )}
        onClick={() => onClick?.(stackInspectObjectId(entry))}
      >
        <span className="pointer-events-none absolute left-2 top-[34px] rounded bg-[rgba(8,18,30,0.9)] px-1 py-[1px] text-[8px] font-bold uppercase leading-none tracking-[0.12em] text-[#8ec4ff]">
          {positionLabel}
        </span>
        <div className="relative h-6 w-6 shrink-0 overflow-hidden rounded-md border border-[#29425b]/75 bg-[#0b121b]">
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
        <div className="min-w-0">
          <div className="truncate text-[13px] font-semibold leading-[1.02] text-[#edf5ff]">
            {name}
          </div>
          <div className="mt-0.5 truncate text-[9px] font-bold uppercase leading-none tracking-[0.12em] text-[#8ec4ff]">
            {kindLabel}
          </div>
        </div>
        <div className="flex shrink-0 items-center gap-1 self-center">
          {isSpell && entry?.mana_cost && (
            <span className="shrink-0 scale-[0.86] origin-right">
              <ManaCostIcons cost={entry.mana_cost} />
            </span>
          )}
          {pt && (
            <span className="rounded-sm border border-[#f5d08b]/35 bg-[rgba(245,208,139,0.08)] px-1 py-0.5 text-[10px] font-bold leading-none tracking-wide text-[#f5d08b]">
              {pt}
            </span>
          )}
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
  const [leavingEntries, setLeavingEntries] = useState([]);
  const previousStackRef = useRef([]);
  const leaveTimeoutsRef = useRef(new Map());
  const bodyRef = useRef(null);
  const horizontalScrollRef = useRef(null);
  const focusedDecision = isFocusedDecision(decision) && canAct;
  const hasStackEntries = stackObjects.length > 0 || leavingEntries.length > 0 || stackPreview.length > 0;
  const stackIds = useMemo(() => stackObjects.map((entry) => entry.id), [stackObjects]);
  const { newIds } = useNewCards(stackIds);
  const topLiveEntryId = useMemo(
    () => (stackObjects.length > 0 ? String(stackInspectObjectId(stackObjects[0])) : null),
    [stackObjects]
  );
  const timelineEntries = useMemo(
    () => [
      ...stackObjects.map((entry) => ({
        ...entry,
        __timeline_key: `live-${entry.id}`,
        __leaving: false,
      })),
      ...leavingEntries.map((item) => ({
        ...item.entry,
        __timeline_key: item.key,
        __leaving: true,
      })),
    ],
    [leavingEntries, stackObjects]
  );
  const itemCount = stackObjects.length || leavingEntries.length || stackPreview.length;
  const timelineSignature = timelineEntries.map((entry) => entry.__timeline_key).join("|");
  const isHorizontal = layout === "horizontal";
  const horizontalEntries = useMemo(
    () => (isHorizontal ? [...timelineEntries].reverse() : timelineEntries),
    [isHorizontal, timelineEntries]
  );
  const horizontalPreviewEntries = useMemo(
    () => (isHorizontal ? [...stackPreview].reverse() : stackPreview),
    [isHorizontal, stackPreview]
  );

  useEffect(() => {
    const previousStack = previousStackRef.current || [];
    const nextIds = new Set(stackObjects.map((entry) => String(entry.id)));
    const removed = previousStack.filter((entry) => !nextIds.has(String(entry.id)));

    if (removed.length > 0) {
      const additions = removed.map((entry) => ({
        key: `leaving-${entry.id}-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`,
        entry,
      }));
      queueMicrotask(() => {
        setLeavingEntries((prev) => {
          const existing = new Set(prev.map((item) => String(item.entry.id)));
          const deduped = additions.filter((item) => !existing.has(String(item.entry.id)));
          return deduped.length > 0 ? [...deduped, ...prev] : prev;
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
    enterFrom: isHorizontal ? { opacity: 0, x: 20, scale: 0.97 } : { opacity: 0, y: 16, scale: 0.97 },
    leaveTo: isHorizontal ? { opacity: 0, x: 18, scale: 0.96 } : { opacity: 0, y: -14, scale: 0.96 },
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
  }, [isHorizontal, itemCount, timelineSignature, horizontalPreviewEntries]);

  if (!hasStackEntries) return null;

  const embeddedExpandedMaxHeight = Number.isFinite(maxBodyHeight) && maxBodyHeight > 0
    ? Math.max(96, Math.round(maxBodyHeight))
    : 380;

  if (isHorizontal) {
    return (
      <section
        className="relative flex min-h-[74px] items-stretch overflow-visible rounded-[14px] border border-[#35506c]/80 bg-[linear-gradient(180deg,rgba(6,14,24,0.86),rgba(5,10,18,0.98))] backdrop-blur-[2.2px] shadow-[0_14px_30px_rgba(0,0,0,0.38)]"
        data-inspector-stack-timeline
      >
        <div
          ref={horizontalScrollRef}
          className="stack-timeline-scroll min-w-0 flex-1 overflow-x-auto overflow-y-hidden px-1 py-0"
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
                  positionLabel={index === horizontalEntries.length - 1 ? "Top" : `#${index + 1}`}
                  showLeadingBorder={index < horizontalEntries.length - 1}
                  isActive={
                    !entry.__leaving
                    && topLiveEntryId != null
                    && String(topLiveEntryId) === String(stackInspectObjectId(entry))
                  }
                  onClick={entry.__leaving ? undefined : onInspectObject}
                />
              ))
              : horizontalPreviewEntries.map((name, index) => (
                <div
                  key={`${name}-${index}`}
                  className={cn(
                    "stack-timeline-entry relative flex h-full shrink-0 items-center bg-[linear-gradient(180deg,rgba(13,33,52,0.84),rgba(8,18,31,0.96))] px-3 text-[13px] font-semibold text-[#d5e7fd]",
                    index < horizontalPreviewEntries.length - 1
                      ? "shadow-[inset_1px_0_0_rgba(53,80,108,0.65)]"
                      : ""
                  )}
                  style={{ width: HORIZONTAL_STACK_ENTRY_WIDTH }}
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
                      {index === 0
                        ? (focusedDecision ? "Resolving" : "Top")
                        : `#${timelineEntries.length - index}`}
                    </span>
                    <StackCard
                      entry={entry}
                      isNew={!entry.__leaving && newIds.has(entry.id)}
                      isLeaving={entry.__leaving}
                      isActive={
                        !entry.__leaving
                        && topLiveEntryId != null
                        && String(topLiveEntryId) === String(stackInspectObjectId(entry))
                      }
                      className="pt-4"
                      onClick={entry.__leaving ? undefined : onInspectObject}
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
                      {index === 0
                        ? (focusedDecision ? "Resolving" : "Top")
                        : `#${timelineEntries.length - index}`}
                    </span>
                    <StackCard
                      entry={entry}
                      isNew={!entry.__leaving && newIds.has(entry.id)}
                      isLeaving={entry.__leaving}
                      isActive={
                        !entry.__leaving
                        && topLiveEntryId != null
                        && String(topLiveEntryId) === String(stackInspectObjectId(entry))
                      }
                      className="pt-4"
                      onClick={entry.__leaving ? undefined : onInspectObject}
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
