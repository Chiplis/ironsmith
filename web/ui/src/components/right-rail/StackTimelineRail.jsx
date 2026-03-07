import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { useGame } from "@/context/GameContext";
import InspectorStackTimeline from "./InspectorStackTimeline";
import { cn } from "@/lib/utils";

const STACK_RAIL_WIDTH = "clamp(240px, 24vw, 360px)";
const STACK_HORIZONTAL_ENTRY_WIDTH = 230;
const STACK_HORIZONTAL_GAP = 0;
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
  const [availableHeight, setAvailableHeight] = useState(STACK_DEFAULT_MAX_HEIGHT);
  const [horizontalTopOffset, setHorizontalTopOffset] = useState(0);
  const [horizontalMaxWidth, setHorizontalMaxWidth] = useState(null);
  const railRef = useRef(null);

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
    let frame = 0;
    if (isCollapsed && changed && rawStackEntryCount > 0) {
      frame = window.requestAnimationFrame(() => {
        setIsCollapsed(false);
      });
    }
    previousStackSignatureRef.current = stackSignature;
    return () => {
      if (frame) window.cancelAnimationFrame(frame);
    };
  }, [stackSignature, isCollapsed, rawStackEntryCount]);

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
        : (rootRect.height - STACK_EDGE_MARGIN);

      const computedAvailableHeight = Math.max(
        150,
        Math.round(myBottom - opponentsTop - (STACK_EDGE_MARGIN * 2))
      );

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
  }, [floating, anchorRef, rawStackEntryCount, state?.players?.length]);

  useLayoutEffect(() => {
    if (floating) return undefined;

    const railEl = railRef.current;
    if (!railEl) return undefined;

    const root = railEl.closest("[data-drop-zone]") ?? railEl.closest("main");
    const myZone = railEl.closest("[data-my-zone]") ?? root?.querySelector("[data-my-zone]");
    const strip = root?.querySelector(".priority-inline-panel");
    const headerContent = myZone?.querySelector("[data-my-zone-header-content]");
    const railHost = railEl.parentElement;
    if (!root || !myZone || !strip || !headerContent || !railHost) return undefined;
    const headerItems = Array.from(headerContent.children);

    let rafId = null;
    const measureOffset = () => {
      const myZoneRect = myZone.getBoundingClientRect();
      const stripRect = strip.getBoundingClientRect();
      const railHostRect = railHost.getBoundingClientRect();
      const contentRight = headerItems.length > 0
        ? Math.max(...headerItems.map((item) => item.getBoundingClientRect().right))
        : headerContent.getBoundingClientRect().left;
      if (!myZoneRect.height || !stripRect.height || !railHostRect.width) return;

      const nextOffset = Math.max(0, myZoneRect.top - stripRect.bottom);
      const nextMaxWidth = Math.max(0, railHostRect.right - contentRight - 8);
      setHorizontalTopOffset((currentOffset) => (
        Math.abs(currentOffset - nextOffset) >= 0.5 ? nextOffset : currentOffset
      ));
      setHorizontalMaxWidth((currentWidth) => (
        currentWidth == null || Math.abs(currentWidth - nextMaxWidth) >= 0.5 ? nextMaxWidth : currentWidth
      ));
    };

    const scheduleMeasure = () => {
      if (rafId != null) cancelAnimationFrame(rafId);
      rafId = requestAnimationFrame(() => {
        rafId = null;
        measureOffset();
      });
    };

    scheduleMeasure();

    const observer = typeof ResizeObserver !== "undefined"
      ? new ResizeObserver(scheduleMeasure)
      : null;
    observer?.observe(root);
    observer?.observe(myZone);
    observer?.observe(strip);
    observer?.observe(headerContent);
    observer?.observe(railHost);
    headerItems.forEach((item) => observer?.observe(item));
    observer?.observe(railEl);
    window.addEventListener("resize", scheduleMeasure);

    return () => {
      if (rafId != null) cancelAnimationFrame(rafId);
      observer?.disconnect();
      window.removeEventListener("resize", scheduleMeasure);
    };
  }, [floating, rawStackEntryCount, state?.players?.length]);

  const shouldShowRail = rawStackEntryCount > 0;
  const collapsedPanelHeight = STACK_MIN_HEIGHT;
  const stackPanelMaxHeight = useMemo(
    () => Math.max(STACK_MIN_HEIGHT, Math.round(availableHeight)),
    [availableHeight]
  );
  const stackBodyMaxHeight = useMemo(
    () => Math.max(96, stackPanelMaxHeight - 38),
    [stackPanelMaxHeight]
  );
  const horizontalRailWidth = useMemo(() => {
    const entryCount = Math.max(displayStackEntryCount, stackPreview.length, 1);
    const entriesWidth = entryCount * STACK_HORIZONTAL_ENTRY_WIDTH;
    const gapsWidth = Math.max(0, entryCount - 1) * STACK_HORIZONTAL_GAP;
    return entriesWidth + gapsWidth + 8;
  }, [displayStackEntryCount, stackPreview.length]);
  const horizontalVisibleWidth = useMemo(() => {
    if (!Number.isFinite(horizontalMaxWidth) || horizontalMaxWidth == null) return horizontalRailWidth;
    return Math.min(horizontalRailWidth, Math.max(0, horizontalMaxWidth));
  }, [horizontalMaxWidth, horizontalRailWidth]);

  if (floating) {
    return (
      <aside
        className={cn(
          "pointer-events-none absolute right-2 z-[56] transition-[transform,opacity] duration-280 ease-out",
          shouldShowRail ? "translate-y-0 opacity-100" : "translate-y-2 opacity-0"
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
      ref={railRef}
      className={cn(
        "pointer-events-none absolute right-0 z-[72] overflow-visible transition-[width,transform,opacity] duration-220 ease-out",
        shouldShowRail ? "translate-x-0 opacity-100" : "translate-x-6 opacity-0"
      )}
      style={{
        width: shouldShowRail ? `${horizontalVisibleWidth}px` : "0px",
        top: `${-horizontalTopOffset}px`,
      }}
      aria-hidden={!shouldShowRail}
    >
      <div
        className={cn(
          "pointer-events-auto overflow-visible transition-opacity duration-220 ease-out",
          shouldShowRail ? "opacity-100" : "opacity-0",
          shouldShowRail ? "pointer-events-auto" : "pointer-events-none"
        )}
      >
        {shouldShowRail && (
          <InspectorStackTimeline
            embedded
            layout="horizontal"
            title="Stack"
            decision={decision}
            canAct={canAct}
            stackObjects={stackObjects}
            stackPreview={stackPreview}
            selectedObjectId={selectedObjectId}
            onInspectObject={onInspectObject}
          />
        )}
      </div>
    </aside>
  );
}
