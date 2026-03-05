import { useLayoutEffect, useMemo, useRef, useState } from "react";
import { useGame } from "@/context/GameContext";
import { ScrollArea } from "@/components/ui/scroll-area";
import useNewCards from "@/hooks/useNewCards";
import StackCard from "@/components/cards/StackCard";
import { ChevronDown, ChevronUp } from "lucide-react";

export default function StackPanel({
  onInspect,
  expanded = false,
  onToggleExpanded,
  onContentHeightChange,
}) {
  const { state } = useGame();
  const objects = state?.stack_objects || [];
  const previews = state?.stack_preview || [];
  const hasContent = objects.length > 0 || previews.length > 0;
  const itemCount = objects.length || previews.length;
  const stackIds = useMemo(() => objects.map((e) => e.id), [objects]);
  const { newIds } = useNewCards(stackIds);
  const panelRef = useRef(null);
  const [hasOverflow, setHasOverflow] = useState(false);
  const showToggle = hasContent && (expanded || hasOverflow);

  useLayoutEffect(() => {
    const panelEl = panelRef.current;
    if (!panelEl || !hasContent) {
      setHasOverflow(false);
      onContentHeightChange?.(0);
      return;
    }

    const viewport = panelEl.querySelector('[data-slot="scroll-area-viewport"]');
    if (!viewport) {
      setHasOverflow(false);
      onContentHeightChange?.(0);
      return;
    }

    let frame = 0;
    let observedContentEl = null;
    const observer = new ResizeObserver(() => {
      scheduleMeasure();
    });

    const syncObservedContent = () => {
      const nextContentEl = viewport.firstElementChild;
      if (nextContentEl === observedContentEl) return;
      if (observedContentEl) {
        observer.unobserve(observedContentEl);
      }
      observedContentEl = nextContentEl;
      if (observedContentEl) {
        observer.observe(observedContentEl);
      }
    };

    const measureOverflow = () => {
      frame = 0;
      syncObservedContent();
      const nextOverflow = viewport.scrollHeight - viewport.clientHeight > 1;
      setHasOverflow(nextOverflow);
      if (typeof onContentHeightChange === "function" && observedContentEl) {
        // Chrome is panel framing (title row, padding, gaps). Content is card list height.
        const panelChromeHeight = panelEl.clientHeight - viewport.clientHeight;
        const desiredHeight = Math.ceil(panelChromeHeight + observedContentEl.scrollHeight + 2);
        onContentHeightChange(desiredHeight);
      }
    };

    const scheduleMeasure = () => {
      if (frame) return;
      frame = window.requestAnimationFrame(measureOverflow);
    };

    scheduleMeasure();
    observer.observe(viewport);
    observer.observe(panelEl);

    return () => {
      if (frame) window.cancelAnimationFrame(frame);
      if (observedContentEl) {
        observer.unobserve(observedContentEl);
        observedContentEl = null;
      }
      observer.disconnect();
    };
  }, [hasContent, itemCount, expanded, onContentHeightChange]);

  return (
    <section ref={panelRef} className="h-full p-2 flex flex-col gap-1.5 overflow-hidden bg-[#0b1118]">
      <div className="flex items-center gap-1 shrink-0">
        {showToggle ? (
          <button
            type="button"
            onClick={onToggleExpanded}
            aria-label={expanded ? "Collapse stack panel" : "Expand stack panel"}
            className="w-6 h-6 rounded border border-[#32445a] bg-[#111927] text-[#8ec4ff] hover:text-[#c6e4ff] hover:border-[#4f6f90] grid place-items-center transition-colors"
          >
            {expanded ? <ChevronDown className="size-4" /> : <ChevronUp className="size-4" />}
          </button>
        ) : (
          <span className="w-6 h-6 shrink-0" aria-hidden="true" />
        )}
        <h4 className="m-0 ml-auto text-right text-[#8ec4ff] uppercase tracking-widest text-[14px] font-bold">
          Stack{hasContent ? ` (${itemCount})` : ""}
        </h4>
      </div>
      {hasContent && (
        <ScrollArea className="flex-1 min-h-0">
          <div className="grid gap-1.5 pr-0.5">
            {objects.length > 0
              ? objects.map((entry) => (
                  <StackCard key={entry.id} entry={entry} isNew={newIds.has(entry.id)} onClick={onInspect} />
                ))
              : previews.map((name, i) => (
                  <div
                    key={i}
                    className="game-card w-full min-w-0 min-h-[60px] text-[14px] bg-gradient-to-b from-[#132237] to-[#0d1726] p-1.5"
                  >
                    <span className="relative z-2 leading-[1.12] text-shadow-[0_1px_1px_rgba(0,0,0,0.85)]">{name}</span>
                  </div>
                ))}
          </div>
        </ScrollArea>
      )}
    </section>
  );
}
