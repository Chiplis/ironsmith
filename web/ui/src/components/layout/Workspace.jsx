import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { useGame } from "@/context/GameContext";
import { useDragActions } from "@/context/DragContext";
import TableCore from "@/components/board/TableCore";
import RightRail from "@/components/right-rail/RightRail";
import HandZone from "@/components/board/HandZone";
import StackPanel from "@/components/right-rail/StackPanel";
import DragOverlay from "@/components/overlays/DragOverlay";
import CastParticles from "@/components/overlays/CastParticles";
import ArrowOverlay from "@/components/overlays/ArrowOverlay";

export default function Workspace({
  zoneViews,
  setZoneViews,
  deckLoadingMode,
  onLoadDecks,
  onCancelDeckLoading,
}) {
  const [selectedObjectId, setSelectedObjectId] = useState(null);
  const [stackExpanded, setStackExpanded] = useState(false);
  const [stackHeightCap, setStackHeightCap] = useState(520);
  const { state, dispatch, status } = useGame();
  const { endDrag } = useDragActions();
  const workspaceRef = useRef(null);
  const stackDockRef = useRef(null);

  const players = state?.players || [];
  const perspective = state?.perspective;
  const me = players.find((p) => p.id === perspective) || players[0];
  const handRowHeight = 140;
  const stackObjectsCount = state?.stack_objects?.length || 0;
  const stackPreviewCount = state?.stack_preview?.length || 0;
  const stackCount = stackObjectsCount > 0 ? stackObjectsCount : stackPreviewCount;
  const hasStackContent = stackCount > 0;
  const stackCompactHeight = handRowHeight;
  const stackItemHeight = stackObjectsCount > 0 ? 80 : 60;
  const stackExpandedHeightUncapped = Math.max(
    stackCompactHeight,
    44 + (stackCount * stackItemHeight) + (Math.max(stackCount - 1, 0) * 6)
  );
  const stackExpandedHeight = Math.min(Math.max(stackCompactHeight, stackHeightCap), stackExpandedHeightUncapped);
  const stackPanelHeight = stackExpanded ? stackExpandedHeight : stackCompactHeight;
  const stackPanelWidth = "clamp(160px, 20vw, 280px)";
  const addCardError = status?.isError && typeof status?.msg === "string" && status.msg.startsWith("Add card failed:")
    ? status.msg
    : null;
  const [dismissedAddCardError, setDismissedAddCardError] = useState(false);

  useEffect(() => {
    if (addCardError) setDismissedAddCardError(false);
  }, [status, addCardError]);

  useEffect(() => {
    if (!hasStackContent && stackExpanded) {
      setStackExpanded(false);
    }
  }, [hasStackContent, stackExpanded]);

  useLayoutEffect(() => {
    const root = workspaceRef.current;
    const stackDock = stackDockRef.current;
    if (!root || !stackDock) return;

    const recalcCap = () => {
      const rootRect = root.getBoundingClientRect();
      const stackRect = stackDock.getBoundingClientRect();
      const opponentsZone = root.querySelector("[data-opponents-zones]");
      const topLimit = opponentsZone
        ? opponentsZone.getBoundingClientRect().top
        : rootRect.top + 8;
      const nextCap = Math.floor(stackRect.bottom - topLimit - 8);
      const normalizedCap = Math.max(stackCompactHeight, nextCap);
      setStackHeightCap((prev) => (Math.abs(prev - normalizedCap) > 1 ? normalizedCap : prev));
    };

    recalcCap();
    const observer = new ResizeObserver(recalcCap);
    observer.observe(root);
    observer.observe(stackDock);
    const opponentsZone = root.querySelector("[data-opponents-zones]");
    if (opponentsZone) observer.observe(opponentsZone);
    window.addEventListener("resize", recalcCap);

    return () => {
      observer.disconnect();
      window.removeEventListener("resize", recalcCap);
    };
  }, [stackCompactHeight, stackCount, zoneViews, deckLoadingMode, stackExpanded]);

  // Handle drag drop — if user drops on the battlefield area, dispatch the action
  useEffect(() => {
    const onPointerUp = (e) => {
      const ds = endDrag();
      if (!ds || !ds.actions || ds.actions.length === 0) return;

      // Check if dropped over the table area (anywhere above the hand)
      const el = document.elementFromPoint(e.clientX, e.clientY);
      const isOverTable = el && (
        el.closest("[data-drop-zone]") ||
        el.closest(".table-gradient") ||
        el.closest(".board-zone-bg")
      );

      if (isOverTable) {
        // If single action, dispatch immediately. If multiple, use first.
        const action = ds.actions[0];
        dispatch(
          { type: "priority_action", action_index: action.index },
          action.label
        );
        // Spawn particles at drop point
        window.__castParticles?.(e.clientX, e.clientY, ds.glowKind || "spell");
      }
    };

    const onPointerCancel = () => {
      endDrag();
    };

    const onWindowBlur = () => {
      endDrag();
    };

    document.addEventListener("pointerup", onPointerUp);
    document.addEventListener("pointercancel", onPointerCancel);
    window.addEventListener("blur", onWindowBlur);
    return () => {
      document.removeEventListener("pointerup", onPointerUp);
      document.removeEventListener("pointercancel", onPointerCancel);
      window.removeEventListener("blur", onWindowBlur);
    };
  }, [endDrag, dispatch]);

  return (
    <section
      ref={workspaceRef}
      className="relative grid gap-2 min-h-0 h-full"
      style={{
        gridTemplateColumns: "clamp(143px,12vw,195px) minmax(0,1fr)",
        gridTemplateRows: `minmax(0,1fr) ${handRowHeight}px`,
      }}
    >
      <DragOverlay />
      <CastParticles />
      <ArrowOverlay />
      {addCardError && !dismissedAddCardError && (
        <button
          type="button"
          className="add-card-error-toast absolute top-2 right-2 z-[120] max-w-[min(420px,48vw)] rounded border border-[#9f2b2b] bg-[rgba(24,8,8,0.96)] px-3 py-2 text-left text-[13px] font-semibold leading-tight text-[#ff7f7f] shadow-[0_10px_26px_rgba(0,0,0,0.45)] hover:border-[#c04040] hover:text-[#ff9f9f] transition-colors"
          onClick={() => setDismissedAddCardError(true)}
          title="Click to dismiss"
        >
          {addCardError}
        </button>
      )}
      <RightRail pinnedObjectId={selectedObjectId} />
      <TableCore
        selectedObjectId={selectedObjectId}
        onInspect={setSelectedObjectId}
        zoneViews={zoneViews}
        setZoneViews={setZoneViews}
        deckLoadingMode={deckLoadingMode}
        onLoadDecks={onLoadDecks}
        onCancelDeckLoading={onCancelDeckLoading}
      />
      <div className="col-span-2 flex min-h-0 h-full overflow-visible items-end">
        <div className="flex-1 h-full min-w-0 overflow-visible">
          <HandZone player={me} selectedObjectId={selectedObjectId} onInspect={setSelectedObjectId} />
        </div>
        <div
          ref={stackDockRef}
          className="relative z-20 ml-2 shrink-0 self-end rounded overflow-hidden border border-[#2a3647] bg-[#0b1118] shadow-[0_16px_34px_rgba(0,0,0,0.46)] transition-[height] duration-200 ease-out"
          style={{
            width: stackPanelWidth,
            height: `${stackPanelHeight}px`,
          }}
        >
          <StackPanel
            onInspect={setSelectedObjectId}
            expanded={stackExpanded}
            onToggleExpanded={() => setStackExpanded((v) => !v)}
          />
        </div>
      </div>
    </section>
  );
}
