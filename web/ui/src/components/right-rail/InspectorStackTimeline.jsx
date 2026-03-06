import { useEffect, useMemo, useRef, useState } from "react";
import { ScrollArea } from "@/components/ui/scroll-area";
import useNewCards from "@/hooks/useNewCards";
import StackCard from "@/components/cards/StackCard";
import { stagger } from "@/lib/motion/anime";
import useLayoutReflow from "@/lib/motion/useLayoutReflow";
import { cn } from "@/lib/utils";

const STACK_LEAVE_ANIMATION_MS = 360;

function isFocusedDecision(decision) {
  return (
    !!decision
    && decision.kind !== "priority"
    && decision.kind !== "attackers"
    && decision.kind !== "blockers"
  );
}

export default function InspectorStackTimeline({
  decision = null,
  canAct = false,
  stackObjects = [],
  stackPreview = [],
  timelineHeight = 176,
  embedded = false,
  selectedObjectId = null,
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
  const focusedDecision = isFocusedDecision(decision) && canAct;
  const hasStackEntries = stackObjects.length > 0 || leavingEntries.length > 0 || stackPreview.length > 0;
  const stackIds = useMemo(() => stackObjects.map((entry) => entry.id), [stackObjects]);
  const { newIds } = useNewCards(stackIds);
  const timelineEntries = useMemo(
    () => [
      ...leavingEntries.map((item) => ({
        ...item.entry,
        __timeline_key: item.key,
        __leaving: true,
      })),
      ...stackObjects.map((entry) => ({
        ...entry,
        __timeline_key: `live-${entry.id}`,
        __leaving: false,
      })),
    ],
    [leavingEntries, stackObjects]
  );
  const itemCount = stackObjects.length || leavingEntries.length || stackPreview.length;
  const timelineSignature = timelineEntries.map((entry) => entry.__timeline_key).join("|");

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
    enterFrom: { opacity: 0, y: 16, scale: 0.97 },
    leaveTo: { opacity: 0, y: -14, scale: 0.96 },
  });

  if (!hasStackEntries) return null;

  const embeddedExpandedMaxHeight = Number.isFinite(maxBodyHeight) && maxBodyHeight > 0
    ? Math.max(96, Math.round(maxBodyHeight))
    : 380;

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
            className="grid gap-1.5 p-1.5 overflow-y-auto overscroll-contain"
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
                      isActive={!entry.__leaving && selectedObjectId != null && String(selectedObjectId) === String(entry.id)}
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
                      isActive={!entry.__leaving && selectedObjectId != null && String(selectedObjectId) === String(entry.id)}
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
