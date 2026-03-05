import { useMemo } from "react";
import { ScrollArea } from "@/components/ui/scroll-area";
import useNewCards from "@/hooks/useNewCards";
import StackCard from "@/components/cards/StackCard";

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
  selectedObjectId = null,
  onInspectObject,
}) {
  const focusedDecision = isFocusedDecision(decision) && canAct;
  const hasStackEntries = stackObjects.length > 0 || stackPreview.length > 0;
  const stackIds = useMemo(() => stackObjects.map((entry) => entry.id), [stackObjects]);
  const { newIds } = useNewCards(stackIds);
  const itemCount = stackObjects.length || stackPreview.length;

  if (!hasStackEntries) return null;

  return (
    <section
      className="absolute inset-x-0 bottom-0 z-[36] h-[176px] overflow-hidden border-t border-[#35506c] bg-[linear-gradient(180deg,rgba(6,14,24,0.64),rgba(5,10,18,0.9))] backdrop-blur-[2.2px] pointer-events-auto"
      data-inspector-stack-timeline
    >
      <header className="flex items-center justify-between gap-2 border-b border-[#2f4864] px-2.5 py-1.5">
        <div className="text-[11px] font-bold uppercase tracking-[0.14em] text-[#8ec4ff]">
          Stack Timeline
        </div>
        <div className="text-[11px] text-[#c5d9f2]">
          {focusedDecision ? `${itemCount} stack entr${itemCount === 1 ? "y" : "ies"}` : `${itemCount} entr${itemCount === 1 ? "y" : "ies"}`}
        </div>
      </header>
      <ScrollArea className="h-[calc(100%-38px)]">
        <div className="grid gap-1.5 p-1.5">
          {stackObjects.length > 0
            ? stackObjects.map((entry, index) => (
                <div key={entry.id} className="relative">
                  <span className="pointer-events-none absolute left-1.5 top-1.5 z-10 rounded bg-[rgba(8,18,30,0.86)] px-1 py-[2px] text-[10px] font-bold uppercase tracking-[0.12em] text-[#8ec4ff]">
                    {index === 0
                      ? (focusedDecision ? "Resolving" : "Top")
                      : `#${stackObjects.length - index}`}
                  </span>
                  <StackCard
                    entry={entry}
                    isNew={newIds.has(entry.id)}
                    isActive={selectedObjectId != null && String(selectedObjectId) === String(entry.id)}
                    className="pt-4"
                    onClick={onInspectObject}
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
    </section>
  );
}
