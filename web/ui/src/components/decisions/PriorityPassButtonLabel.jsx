import { useCallback, useLayoutEffect, useRef, useState } from "react";
import { cn } from "@/lib/utils";

export default function PriorityPassButtonLabel({
  currentLabel,
  advanceLabel,
  className = "",
}) {
  const minScale = 0.45;
  const maxAdvanceRatioScale = 0.75;
  const minAdvanceRatio = 0.55;
  const maxAdvanceRatio = 0.9;
  const containerRef = useRef(null);
  const currentRef = useRef(null);
  const advanceRef = useRef(null);
  const [scale, setScale] = useState(1);
  const scaleProgress = Math.min(1, Math.max(0, (1 - scale) / (1 - maxAdvanceRatioScale)));
  const advanceRatio = minAdvanceRatio + ((maxAdvanceRatio - minAdvanceRatio) * scaleProgress);
  const advanceFontSizeEm = 1.5625 * advanceRatio;

  const recomputeScale = useCallback(() => {
    const container = containerRef.current;
    if (!container) return;

    const parentWidth = container.parentElement?.clientWidth || container.clientWidth;
    const availableWidth = Math.max(1, parentWidth - 52);
    const currentWidth = currentRef.current?.scrollWidth || 0;
    const advanceWidth = advanceRef.current?.scrollWidth || 0;
    const widestLine = Math.max(currentWidth, advanceWidth, 1);
    const nextScale = Math.min(1, Math.max(minScale, availableWidth / widestLine));
    setScale((prev) => (Math.abs(prev - nextScale) > 0.01 ? nextScale : prev));
  }, [minScale]);

  useLayoutEffect(() => {
    recomputeScale();
    if (typeof ResizeObserver === "undefined") return undefined;

    const observer = new ResizeObserver(() => recomputeScale());
    if (containerRef.current?.parentElement) observer.observe(containerRef.current.parentElement);
    if (containerRef.current) observer.observe(containerRef.current);
    if (currentRef.current) observer.observe(currentRef.current);
    if (advanceRef.current) observer.observe(advanceRef.current);
    return () => observer.disconnect();
  }, [advanceLabel, currentLabel, recomputeScale]);

  return (
    <span
      ref={containerRef}
      className={cn(
        "flex min-w-0 max-w-full flex-col items-start justify-center leading-none",
        className
      )}
      style={{
        "--priority-pass-advance-size": `${advanceFontSizeEm}em`,
        transform: scale < 1 ? `scale(${scale})` : undefined,
        transformOrigin: "left center",
      }}
    >
      <span ref={currentRef} className="priority-pass-label-current block max-w-none whitespace-nowrap text-[1em] font-bold leading-[1.05] uppercase">
        {currentLabel || "Priority"}
      </span>
      {advanceLabel ? (
        <span ref={advanceRef} className="priority-pass-label-advance mt-0.5 block max-w-none whitespace-nowrap text-[0.68em] font-bold leading-[1.05] uppercase">
          {advanceLabel}
        </span>
      ) : null}
    </span>
  );
}
