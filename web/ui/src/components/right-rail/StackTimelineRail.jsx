import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { useGame } from "@/context/GameContext";
import InspectorStackTimeline from "./InspectorStackTimeline";
import { cn } from "@/lib/utils";
import { getVisibleStackObjects } from "@/lib/stack-targets";
import { isTriggerOrderingDecision } from "@/lib/trigger-ordering";

const STACK_RAIL_WIDTH = "clamp(240px, 24vw, 360px)";
const STACK_EDGE_MARGIN = 6;
const STACK_MIN_HEIGHT = 44;
const STACK_DEFAULT_MAX_HEIGHT = 320;

export default function StackTimelineRail({
  selectedObjectId = null,
  onInspectObject = null,
  floating = false,
  anchorRef = null,
  className = "",
}) {
  const { state } = useGame();
  const decision = state?.decision || null;
  const canAct = !!decision && decision.player === state?.perspective;
  const stackObjects = getVisibleStackObjects(state);
  const stackPreview = state?.stack_preview || [];
  const stackSignature = stackObjects
    .map((entry) => String(entry.id))
    .join("|");
  const rawStackEntryCount = Math.max(stackObjects.length, stackPreview.length);
  const orderingEntryCount = useMemo(
    () =>
      isTriggerOrderingDecision(decision)
        ? rawStackEntryCount + (decision?.options || []).length
        : rawStackEntryCount,
    [decision, rawStackEntryCount],
  );
  const [isCollapsed, setIsCollapsed] = useState(false);
  const previousStackSignatureRef = useRef(stackSignature);
  const [availableHeight, setAvailableHeight] = useState(
    STACK_DEFAULT_MAX_HEIGHT,
  );
  const inlineAnchorRef = useRef(null);
  const [inlineRect, setInlineRect] = useState(null);
  const shouldShowRail = orderingEntryCount > 0;

  useEffect(() => {
    const changed = stackSignature !== previousStackSignatureRef.current;
    let frame = 0;
    if (isCollapsed && changed && orderingEntryCount > 0) {
      frame = window.requestAnimationFrame(() => {
        setIsCollapsed(false);
      });
    }
    previousStackSignatureRef.current = stackSignature;
    return () => {
      if (frame) window.cancelAnimationFrame(frame);
    };
  }, [stackSignature, isCollapsed, orderingEntryCount]);

  useLayoutEffect(() => {
    if (!floating) return undefined;

    const root = anchorRef?.current ?? null;
    if (!root) return undefined;

    let rafId = null;
    const computeBounds = () => {
      const rootRect = root.getBoundingClientRect();
      if (!rootRect || rootRect.height <= 0) return;

      const opponents = root.querySelector("[data-opponents-zones]");
      const myZone = root.querySelector("[data-my-zone]");

      const opponentsTop = opponents
        ? opponents.getBoundingClientRect().top - rootRect.top
        : STACK_EDGE_MARGIN;
      const myBottom = myZone
        ? myZone.getBoundingClientRect().bottom - rootRect.top
        : rootRect.height - STACK_EDGE_MARGIN;

      const computedAvailableHeight = Math.max(
        150,
        Math.round(myBottom - opponentsTop - STACK_EDGE_MARGIN * 2),
      );

      setAvailableHeight(computedAvailableHeight);
    };

    const scheduleBounds = () => {
      if (rafId != null) cancelAnimationFrame(rafId);
      rafId = requestAnimationFrame(computeBounds);
    };

    scheduleBounds();

    const resizeObserver =
      typeof ResizeObserver !== "undefined"
        ? new ResizeObserver(scheduleBounds)
        : null;
    resizeObserver?.observe(root);
    const opponents = root.querySelector("[data-opponents-zones]");
    const myZone = root.querySelector("[data-my-zone]");
    if (opponents) resizeObserver?.observe(opponents);
    if (myZone) resizeObserver?.observe(myZone);

    window.addEventListener("resize", scheduleBounds);
    return () => {
      if (rafId != null) cancelAnimationFrame(rafId);
      resizeObserver?.disconnect();
      window.removeEventListener("resize", scheduleBounds);
    };
  }, [floating, anchorRef, rawStackEntryCount, state?.players?.length]);

  useLayoutEffect(() => {
    if (floating) return undefined;

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
  }, [floating, shouldShowRail]);

  const collapsedPanelHeight = STACK_MIN_HEIGHT;
  const stackPanelMaxHeight = useMemo(
    () => Math.max(STACK_MIN_HEIGHT, Math.round(availableHeight)),
    [availableHeight],
  );
  const stackBodyMaxHeight = useMemo(
    () => Math.max(96, stackPanelMaxHeight - 38),
    [stackPanelMaxHeight],
  );

  if (floating) {
    return (
      <aside
        className={cn(
          "pointer-events-none absolute right-2 z-[56] transition-[transform,opacity] duration-280 ease-out",
          shouldShowRail
            ? "translate-y-0 opacity-100"
            : "translate-y-2 opacity-0",
        )}
        style={{
          width: STACK_RAIL_WIDTH,
          bottom: `${STACK_EDGE_MARGIN}px`,
          maxHeight: `${stackPanelMaxHeight}px`,
        }}
        aria-hidden={!shouldShowRail}
      >
        <div
          className={cn(
            "pointer-events-auto overflow-hidden transition-[max-height] duration-320 ease-out",
            shouldShowRail ? "max-h-[90vh]" : "max-h-0",
          )}
          style={{
            maxHeight: shouldShowRail
              ? `${isCollapsed ? collapsedPanelHeight : stackPanelMaxHeight}px`
              : "0px",
          }}
        >
          <InspectorStackTimeline
            embedded
            title="Stack"
            collapsible
            collapsed={isCollapsed}
            onToggleCollapsed={() => setIsCollapsed((prev) => !prev)}
            decision={decision}
            canAct={canAct}
            stackObjects={stackObjects}
            stackPreview={stackPreview}
            selectedObjectId={selectedObjectId}
            onInspectObject={onInspectObject}
            maxBodyHeight={stackBodyMaxHeight}
          />
        </div>
      </aside>
    );
  }

  if (!shouldShowRail) return null;

  const horizontalTimeline = (
    <div className="flex h-full w-full min-w-0 items-stretch justify-start pointer-events-auto pl-2">
      <div className="inline-flex h-full items-center justify-start">
        <InspectorStackTimeline
          embedded
          layout="horizontal"
          compact
          title="Stack"
          decision={decision}
          canAct={canAct}
          stackObjects={stackObjects}
          stackPreview={stackPreview}
          selectedObjectId={selectedObjectId}
          onInspectObject={onInspectObject}
        />
      </div>
    </div>
  );

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
              className="pointer-events-none fixed z-[96] flex items-stretch justify-start bg-[linear-gradient(180deg,rgba(56,49,42,0.98),rgba(49,44,39,0.98))]"
              style={{
                left: `${inlineRect.left}px`,
                top: `${inlineRect.top}px`,
                width: `${inlineRect.width}px`,
                height: `${inlineRect.height}px`,
              }}
            >
              {horizontalTimeline}
            </div>,
            document.body,
          )
        : null}
    </>
  );
}
