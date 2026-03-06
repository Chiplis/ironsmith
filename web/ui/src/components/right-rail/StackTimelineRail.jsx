import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { useGame } from "@/context/GameContext";
import InspectorStackTimeline from "./InspectorStackTimeline";
import { cn } from "@/lib/utils";

const STACK_RAIL_WIDTH = "clamp(240px, 24vw, 360px)";
const STACK_LEAVE_ANIMATION_MS = 360;
const STACK_EDGE_MARGIN = 6;
const STACK_MIN_HEIGHT = 44;
const STACK_DEFAULT_MAX_HEIGHT = 320;

export default function StackTimelineRail({
  selectedObjectId = null,
  onInspectObject = null,
  floating = false,
  anchorRef = null,
}) {
  const { state } = useGame();
  const decision = state?.decision || null;
  const canAct = !!decision && decision.player === state?.perspective;
  const stackObjects = state?.stack_objects || [];
  const stackPreview = state?.stack_preview || [];
  const stackSignature = stackObjects.map((entry) => String(entry.id)).join("|");
  const rawStackEntryCount = Math.max(stackObjects.length, stackPreview.length);
  const [displayStackEntryCount, setDisplayStackEntryCount] = useState(rawStackEntryCount);
  const [isCollapsed, setIsCollapsed] = useState(false);
  const previousStackSignatureRef = useRef(stackSignature);
  const [anchorBottomOffset, setAnchorBottomOffset] = useState(STACK_EDGE_MARGIN);
  const [availableHeight, setAvailableHeight] = useState(STACK_DEFAULT_MAX_HEIGHT);
  const anchorNode = anchorRef?.current ?? null;

  useEffect(() => {
    if (rawStackEntryCount === displayStackEntryCount) return undefined;

    if (rawStackEntryCount > displayStackEntryCount) {
      const timeout = setTimeout(() => {
        setDisplayStackEntryCount(rawStackEntryCount);
      }, 0);
      return () => clearTimeout(timeout);
    }

    const timeout = setTimeout(() => {
      setDisplayStackEntryCount(rawStackEntryCount);
    }, STACK_LEAVE_ANIMATION_MS);
    return () => clearTimeout(timeout);
  }, [rawStackEntryCount, displayStackEntryCount]);

  useEffect(() => {
    const changed = stackSignature !== previousStackSignatureRef.current;
    if (isCollapsed && changed && rawStackEntryCount > 0) {
      setIsCollapsed(false);
    }
    previousStackSignatureRef.current = stackSignature;
  }, [stackSignature, isCollapsed, rawStackEntryCount]);

  useLayoutEffect(() => {
    if (!floating) return undefined;

    const root = anchorNode;
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
        : (rootRect.height - STACK_EDGE_MARGIN);

      const computedBottomOffset = Math.max(
        STACK_EDGE_MARGIN,
        Math.round(rootRect.height - myBottom + STACK_EDGE_MARGIN)
      );
      const computedAvailableHeight = Math.max(
        150,
        Math.round(myBottom - opponentsTop - (STACK_EDGE_MARGIN * 2))
      );

      setAnchorBottomOffset(computedBottomOffset);
      setAvailableHeight(computedAvailableHeight);
    };

    const scheduleBounds = () => {
      if (rafId != null) cancelAnimationFrame(rafId);
      rafId = requestAnimationFrame(computeBounds);
    };

    scheduleBounds();

    const resizeObserver = typeof ResizeObserver !== "undefined"
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
  }, [floating, anchorNode, rawStackEntryCount, state?.players?.length]);

  const shouldShowRail = displayStackEntryCount > 0;
  const containerStyle = useMemo(
    () => ({ width: shouldShowRail ? STACK_RAIL_WIDTH : "0px" }),
    [shouldShowRail]
  );
  const collapsedPanelHeight = STACK_MIN_HEIGHT;
  const stackPanelMaxHeight = useMemo(
    () => Math.max(STACK_MIN_HEIGHT, Math.round(availableHeight)),
    [availableHeight]
  );
  const stackBodyMaxHeight = useMemo(
    () => Math.max(96, stackPanelMaxHeight - 38),
    [stackPanelMaxHeight]
  );

  if (floating) {
    return (
      <aside
        className={cn(
          "pointer-events-none absolute right-2 z-[56] transition-[transform,opacity] duration-280 ease-out",
          shouldShowRail ? "translate-y-0 opacity-100" : "translate-y-2 opacity-0"
        )}
        style={{
          width: STACK_RAIL_WIDTH,
          bottom: `${anchorBottomOffset}px`,
          maxHeight: `${stackPanelMaxHeight}px`,
        }}
        aria-hidden={!shouldShowRail}
      >
        <div
          className={cn(
            "pointer-events-auto overflow-hidden transition-[max-height] duration-320 ease-out",
            shouldShowRail ? "max-h-[90vh]" : "max-h-0"
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

  return (
    <aside
      className={cn(
        "pointer-events-none relative h-full shrink-0 overflow-hidden transition-[width,transform,opacity] duration-220 ease-out",
        shouldShowRail ? "translate-x-0 opacity-100" : "-translate-x-[110%] opacity-0"
      )}
      style={containerStyle}
      aria-hidden={!shouldShowRail}
    >
      <div className={cn("h-full overflow-hidden", shouldShowRail ? "pointer-events-auto" : "pointer-events-none")}>
        {shouldShowRail && (
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
        )}
      </div>
    </aside>
  );
}
