import useUiText from "@/i18n/useUiText";
import RollingPanel from "@/components/board/RollingPanel";
import { useLayoutEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { useGame } from "@/context/GameContext";
import InspectorStackTimeline from "./InspectorStackTimeline";
import { cn } from "@/lib/utils";
import { getVisibleStackObjects } from "@/lib/stack-targets";
import { isTriggerOrderingDecision } from "@/lib/trigger-ordering";
import { samePlayerId } from "@/lib/player-display";

const STACK_EDGE_MARGIN = 6;
const STACK_MIN_HEIGHT = 44;
const STACK_DEFAULT_MAX_HEIGHT = 320;
const STACK_INLINE_MAX_HEIGHT = 236;
const STACK_INLINE_LEFT_OFFSET = 58;
// The header row plus the panel's own padding: what the list cannot use.
const STACK_PANEL_CHROME_HEIGHT = 34;

// The desktop board docks the stack beside the zone piles (inlineFlow); the
// merged mobile/tablet header floats it over the board from a portal.
export default function StackTimelineRail({
  selectedObjectId = null,
  onInspectObject = null,
  inlineFlow = false,
  className = "",
}) {
  const ui = useUiText();
  const { state } = useGame();
  const decision = state?.decision || null;
  const canAct = !!decision && samePlayerId(decision.player, state?.perspective);
  const stackObjects = getVisibleStackObjects(state);
  const stackPreview = state?.stack_preview || [];
  const rawStackEntryCount = Math.max(stackObjects.length, stackPreview.length);
  const orderingEntryCount = useMemo(
    () =>
      isTriggerOrderingDecision(decision)
        ? rawStackEntryCount + (decision?.options || []).length
        : rawStackEntryCount,
    [decision, rawStackEntryCount],
  );
  const [availableHeight, setAvailableHeight] = useState(
    STACK_DEFAULT_MAX_HEIGHT,
  );
  const inlineAnchorRef = useRef(null);
  const flowRailRef = useRef(null);
  const [inlineRect, setInlineRect] = useState(null);
  const shouldShowRail = orderingEntryCount > 0;

  useLayoutEffect(() => {
    if (inlineFlow) return undefined;

    let rafId = null;
    const publishRect = () => {
      const anchor = inlineAnchorRef.current;
      const rect =
        shouldShowRail && anchor
          ? anchor.getBoundingClientRect()
          : null;
      const nextRect =
        rect && rect.width > 0 && rect.height > 0
          ? {
              left: Math.round(rect.left),
              top: Math.round(rect.top),
              width: Math.round(rect.width),
              height: Math.round(rect.height),
            }
          : null;

      setInlineRect((prev) => {
        if (
          prev?.left === nextRect?.left
          && prev?.top === nextRect?.top
          && prev?.width === nextRect?.width
          && prev?.height === nextRect?.height
        ) {
          return prev;
        }
        return nextRect;
      });
    };

    const scheduleRect = () => {
      if (rafId != null) cancelAnimationFrame(rafId);
      rafId = requestAnimationFrame(() => {
        rafId = null;
        publishRect();
      });
    };

    scheduleRect();

    const anchor = inlineAnchorRef.current;
    const resizeObserver =
      typeof ResizeObserver !== "undefined"
        ? new ResizeObserver(scheduleRect)
        : null;
    if (anchor) resizeObserver?.observe(anchor);

    window.addEventListener("resize", scheduleRect);
    window.addEventListener("scroll", scheduleRect, true);
    return () => {
      if (rafId != null) cancelAnimationFrame(rafId);
      resizeObserver?.disconnect();
      window.removeEventListener("resize", scheduleRect);
      window.removeEventListener("scroll", scheduleRect, true);
    };
  }, [inlineFlow, shouldShowRail]);

  useLayoutEffect(() => {
    if (!inlineFlow || !shouldShowRail) return undefined;

    const railEl = flowRailRef.current;
    if (!railEl) return undefined;

    let rafId = null;
    const measureHeight = () => {
      const nextHeight = Math.max(
        150,
        Math.round(railEl.clientHeight || STACK_DEFAULT_MAX_HEIGHT),
      );
      setAvailableHeight((currentHeight) => (
        Math.abs(currentHeight - nextHeight) >= 1 ? nextHeight : currentHeight
      ));
    };

    const scheduleMeasure = () => {
      if (rafId != null) cancelAnimationFrame(rafId);
      rafId = requestAnimationFrame(() => {
        rafId = null;
        measureHeight();
      });
    };

    scheduleMeasure();

    const resizeObserver =
      typeof ResizeObserver !== "undefined"
        ? new ResizeObserver(scheduleMeasure)
        : null;
    resizeObserver?.observe(railEl);

    window.addEventListener("resize", scheduleMeasure);
    return () => {
      if (rafId != null) cancelAnimationFrame(rafId);
      resizeObserver?.disconnect();
      window.removeEventListener("resize", scheduleMeasure);
    };
  }, [inlineFlow, shouldShowRail]);

  const stackPanelMaxHeight = useMemo(
    () => Math.max(STACK_MIN_HEIGHT, Math.round(availableHeight)),
    [availableHeight],
  );
  const stackBodyMaxHeight = useMemo(
    () => Math.max(96, stackPanelMaxHeight - STACK_PANEL_CHROME_HEIGHT),
    [stackPanelMaxHeight],
  );

  if (inlineFlow) {
    return (
      <RollingPanel open={shouldShowRail} className="stack-flow-reveal">
      <aside
        ref={flowRailRef}
        className={cn(
          "stack-inline-flow-rail stack-inline-vertical-rail pointer-events-none h-full min-w-0 overflow-hidden",
          className
        )}
        aria-hidden={!shouldShowRail}
      >
        <InspectorStackTimeline
          embedded
          title={ui("Stack")}
          decision={decision}
          canAct={canAct}
          stackObjects={stackObjects}
          stackPreview={stackPreview}
          selectedObjectId={selectedObjectId}
          onInspectObject={onInspectObject}
          maxBodyHeight={stackBodyMaxHeight}
          compact
        />
      </aside>
      </RollingPanel>
    );
  }

  if (!shouldShowRail) return null;

  const inlineViewportHeight = typeof window !== "undefined"
    ? window.innerHeight
    : inlineRect
      ? inlineRect.top + STACK_DEFAULT_MAX_HEIGHT
      : STACK_DEFAULT_MAX_HEIGHT;
  const inlinePanelMaxHeight = inlineRect
    ? Math.max(
        STACK_MIN_HEIGHT,
        Math.min(
          STACK_INLINE_MAX_HEIGHT,
          Math.floor(inlineViewportHeight - inlineRect.top - STACK_EDGE_MARGIN)
        )
      )
    : STACK_DEFAULT_MAX_HEIGHT;
  const inlineBodyMaxHeight = Math.max(96, inlinePanelMaxHeight - STACK_PANEL_CHROME_HEIGHT);
  const inlinePanelLeft = inlineRect
    ? Math.max(STACK_EDGE_MARGIN, inlineRect.left - STACK_INLINE_LEFT_OFFSET)
    : 0;

  return (
    <>
      <div
        ref={inlineAnchorRef}
        className={cn("min-w-0 pointer-events-none", className)}
        aria-hidden="true"
      />
      {inlineRect && typeof document !== "undefined"
        ? createPortal(
            <div
              className="pointer-events-none fixed z-[96] flex items-start justify-start bg-transparent"
              style={{
                left: `${inlinePanelLeft}px`,
                top: `${inlineRect.top}px`,
                width: `${inlineRect.width}px`,
                maxHeight: `${inlinePanelMaxHeight}px`,
              }}
            >
              <div
                className="stack-inline-vertical-rail pointer-events-none min-w-0 pl-2"
                style={{
                  width: "min(clamp(188px, 18vw, 260px), 100%)",
                  maxHeight: `${inlinePanelMaxHeight}px`,
                }}
              >
                <InspectorStackTimeline
                  embedded
                  title={ui("Stack")}
                  decision={decision}
                  canAct={canAct}
                  stackObjects={stackObjects}
                  stackPreview={stackPreview}
                  selectedObjectId={selectedObjectId}
                  onInspectObject={onInspectObject}
                  maxBodyHeight={inlineBodyMaxHeight}
                  compact
                />
              </div>
            </div>,
            document.body,
          )
        : null}
    </>
  );
}
